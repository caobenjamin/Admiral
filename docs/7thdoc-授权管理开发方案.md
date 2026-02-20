# 7thdoc — 授权管理（账户管理）开发方案

> 版本：v1.0  
> 日期：2026-02-20  
> 依据：6thdoc-账户管理功能.md、Admiral-客户端交互设计规范.md、Admiral_v1_Architecture.md  
> 前置条件：OAuth 与 MAPI 联调已完成，现有接口已调通

---

## 1. 需求摘要

将「媒体账户」板块重构为「授权管理」，实现：

- 导航更名：LeftTab「媒体账户」→「授权管理」
- 平台可展开：点击下拉展开媒体平台列表，当前仅 B 站
- B 站 Tab 布局：搜索筛选 + 新增账户（按 BID 选源）+ 新增授权 + 账户列表（含删除）
- 账户数量上限：服务端可配置，默认 10，可精细到用户级
- 授权流程：完成后自动刷新账户列表

---

## 2. 已调通接口清单

以下接口已实现并可用，开发时直接复用：

| 方法 | 路径 | 说明 | 客户端调用 |
|------|------|------|------------|
| POST | `/api/v1/oauth/bilibili/initiate` | 发起 OAuth 授权 | `tauri.oauthInitiate()` |
| GET | `/api/v1/oauth/bilibili/callback` | OAuth 回调（B 站调用） | — |
| GET | `/api/v1/oauth/bilibili/status` | 授权会话列表 | `tauri.oauthStatus()` |
| GET | `/api/v1/oauth/bilibili/bindings` | 已绑定账户列表 | `tauri.oauthBindingsList()` |
| GET | `/api/v1/oauth/bilibili/accounts?oauth_session_id=` | 可绑定账户列表 | `tauri.oauthAccountsList(oauthSessionId?)` |
| POST | `/api/v1/oauth/bilibili/bind-accounts` | 绑定账户 | `tauri.oauthBindAccounts()` |
| DELETE | `/api/v1/oauth/bilibili/sessions/{id}` | 取消授权会话 | `tauri.oauthDeleteSession()` |

---

## 3. 待实现能力

### 3.1 服务端

| 序号 | 能力 | 说明 |
|------|------|------|
| 1 | OAuth 回调中获取 BID/用户名 | 换到 token 后调用 B 站 userinfo API，并持久化 bid、user_name |
| 2 | Status API 返回 bid、user_name | TokenMeta 扩展字段，供「新增账户」下拉使用 |
| 3 | 解绑单个账户 API | `DELETE /api/v1/oauth/bilibili/bindings/{account_id}` |
| 4 | 账户数量上限 | 默认 10，可配置；bind_accounts 时校验，超限返回 400 + 明确提示 |
| 5 | 用户级上限配置 | 可选：user_account_limits 表或配置项，按用户覆盖默认值 |

### 3.2 客户端

| 序号 | 能力 | 说明 |
|------|------|------|
| 1 | LeftTab 更名与展开 | 「媒体账户」→「授权管理」；点击展开平台列表，当前仅 B 站 |
| 2 | B 站 Tab 布局 | 按交互规范实现：搜索筛选 + 新增账户下拉 + 新增授权 + 账户列表 |
| 3 | 新增账户流程 | 下拉展示已授权 BID/user_name，选 BID → 弹窗拉取该 token 下账户列表 → 勾选保存 |
| 4 | 账户删除 | 表格每行操作按钮「删除」，调用解绑 API |
| 5 | 上限提示 | 达到上限时提示「已达账户数量上限，请删除部分账户后再添加」 |
| 6 | 授权后自动刷新 | 授权成功后自动刷新账户列表（轮询或 WebSocket 事件） |

---

## 4. 详细设计

### 4.1 数据库变更

#### 4.1.1 oauth_tokens 表扩展

```sql
ALTER TABLE oauth_tokens ADD COLUMN IF NOT EXISTS bid BIGINT;
ALTER TABLE oauth_tokens ADD COLUMN IF NOT EXISTS user_name VARCHAR(200);
```

- `bid`：B 站 userinfo 返回的 bid
- `user_name`：B 站 userinfo 返回的 user_name

#### 4.1.2 用户账户上限配置（可选）

**方案 A：配置文件 + 环境变量**

```toml
# config/default.toml
[oauth]
default_max_accounts_per_user = 10
```

**方案 B：user_account_limits 表（支持按用户覆盖）**

```sql
CREATE TABLE IF NOT EXISTS user_account_limits (
    user_id UUID PRIMARY KEY REFERENCES users(id),
    max_accounts INTEGER NOT NULL DEFAULT 10,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

- 若表中无记录，则使用 default_max_accounts_per_user
- MVP 建议先用方案 A，后续再扩展方案 B

### 4.2 服务端 API 设计

#### 4.2.1 TokenMeta 扩展

```rust
// admiral-types/src/api.rs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenMeta {
    pub id: Uuid,
    pub platform: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bid: Option<i64>,        // 新增：B站 userinfo 的 bid
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>, // 新增：B站 userinfo 的 user_name
}
```

#### 4.2.2 解绑单个账户

| 方法 | 路径 | 说明 | 响应 |
|------|------|------|------|
| DELETE | `/api/v1/oauth/bilibili/bindings/{account_id}` | 解绑指定 account_id | 204 No Content |

- 校验：account_id 属于当前用户且已绑定
- 成功后删除 `ad_account_bindings` 中对应记录

#### 4.2.3 账户数量上限校验

在 `POST /api/v1/oauth/bilibili/bind-accounts` 中：

1. 查询当前用户已绑定账户数：`COUNT(*) FROM ad_account_bindings WHERE user_id = ?`
2. 计算：`current_count + len(req.account_ids)` 去重后的新增数
3. 若超过 `max_accounts_per_user`：
   - HTTP 400
   - Body：`{ "code": "ACCOUNT_LIMIT_EXCEEDED", "message": "已达账户数量上限（10），请删除部分账户后再添加。", "max_accounts": 10 }`

#### 4.2.4 OAuth 回调中获取 userinfo

在 `oauth_service::callback` 中，换取 token 成功后：

1. 调用 `GET https://cm.bilibili.com/takumi/api/open_api/oauth2/userinfo`，Header：`Authorization: Bearer {access_token}`
2. 解析 `data.bid`、`data.user_name`
3. 写入 `oauth_tokens` 的 `bid`、`user_name` 列

### 4.3 客户端 UI 设计（遵循交互规范）

#### 4.3.1 LeftTab 调整

| 变更项 | 规范依据 | 实现 |
|--------|----------|------|
| 文案 | 6thdoc | 「媒体账户」→「授权管理」 |
| 展开态 | 设计规范 §4.2 | 点击展开平台列表；当前仅展示「B站」，未来可扩展 |
| 子状态 | 设计规范 | 「B站 已授权 ✓」/「B站 未授权」 |

#### 4.3.2 平台授权页 `/settings/platforms` 布局

```
┌────────────────────────────────────────────────────────────────────┐
│  授权管理                                                            │
│                                                                     │
│  [▼ B站]  ← 下拉展开平台（当前仅 B站）                                │
│                                                                     │
│  展开 B站 Tab 后：                                                    │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │  [账户名称多选搜索] [账户ID精确匹配]     [新增账户 ▾] [新增授权]  │ │
│  ├────────────────────────────────────────────────────────────────┤ │
│  │  账户名称      │  账户ID    │  操作                             │ │
│  │  ─────────────┼────────────┼────────────────────────────────   │ │
│  │  品牌旗舰店    │  10001     │  [删除]                            │ │
│  │  效果广告A     │  10002     │  [删除]                            │ │
│  └────────────────────────────────────────────────────────────────┘ │
│                                                                     │
│  空状态：「暂无已绑定账户」                                           │
│  上限提示：「已达账户数量上限（10），请删除部分账户后再添加。」         │
└────────────────────────────────────────────────────────────────────┘
```

#### 4.3.3 新增账户流程

1. 点击「新增账户」→ 下拉展示已授权 BID / user_name 列表（来自 `status` API）
2. 点击某条（BID）→ 打开弹窗
3. 弹窗内：`GET /accounts?oauth_session_id={token_id}` 拉取该 token 下的广告账户列表
4. 用户勾选账户 → 点击「保存」→ `POST /bind-accounts`
5. 若返回 ACCOUNT_LIMIT_EXCEEDED，Toast 提示上限信息
6. 成功则关闭弹窗并刷新列表

#### 4.3.4 删除账户

- 表格每行「删除」按钮，样式：Ghost/Text 型，红色文字（`--color-danger`）
- 点击 → 确认弹窗「确定要解绑该账户吗？」→ 确认后 `DELETE /bindings/{account_id}`
- 成功后刷新列表

#### 4.3.5 新增授权

- 与现有「发起授权」一致：`POST /initiate` → 打开浏览器 → 授权完成
- 授权成功后：客户端轮询 `status` 或监听 WebSocket `oauth.success`，自动刷新账户列表
- 超时 5 分钟未完成 → Toast「授权超时，请重试」

### 4.4 组件与文件映射

| 组件/功能 | 文件路径 | 备注 |
|-----------|----------|------|
| LeftTab | `crates/admiral-client/ui/src/components/layout/LeftTab.tsx` | 更名、展开平台 |
| PlatformAuth | `crates/admiral-client/ui/src/components/settings/PlatformAuth.tsx` | 重构为 6thdoc 布局 |
| BindAccountModal | 同上或拆分为独立组件 | 支持按 oauth_session_id 拉取 |
| 解绑 API | `crates/admiral-client/.../api_client.rs`、`commands/oauth.rs`、`tauri.ts` | 新增 oauth_unbind_account |

---

## 5. 开发任务拆分

### Phase A：服务端（约 1–2 天）

| 任务 | 描述 | 产出 |
|------|------|------|
| A1 | oauth_tokens 表增加 bid、user_name 列 | migration |
| A2 | OAuth 回调中调用 B站 userinfo 并写入 | oauth_service.rs |
| A3 | TokenMeta 与 status 返回 bid、user_name | admiral-types + oauth_service |
| A4 | 解绑单个账户 API | router + oauth handler + oauth_service |
| A5 | 账户数量上限配置与 bind_accounts 校验 | config + oauth_service |

### Phase B：客户端（约 2–3 天）

| 任务 | 描述 | 产出 |
|------|------|------|
| B1 | LeftTab「授权管理」更名 + 平台下拉结构 | LeftTab.tsx |
| B2 | PlatformAuth 布局：搜索筛选 + 按钮区 + 表格 | PlatformAuth.tsx |
| B3 | 新增账户下拉 + 按 BID 弹窗拉取账户 | BindAccountModal 重构 |
| B4 | 表格行「删除」+ 确认弹窗 + 解绑 API 调用 | PlatformAuth + tauri |
| B5 | 上限提示 Toast、授权后自动刷新 | PlatformAuth |
| B6 | 新增 oauth_unbind_account 的 Tauri/IPC 封装 | api_client + commands + tauri.ts + types |

### Phase C：联调与验收（约 0.5 天）

| 任务 | 描述 |
|------|------|
| C1 | 端到端：授权 → 获取 bid/user_name → 新增账户 → 删除账户 |
| C2 | 上限场景：绑定至 10 个后继续添加，校验返回与前端提示 |
| C3 | 按 Admiral-客户端交互设计规范 做视觉与交互检查 |

---

## 6. 设计规范符合性检查

开发完成后需满足：

- [ ] 色彩：语义色一致（成功绿、危险红、主色蓝）
- [ ] 按钮：Primary / Secondary / Danger / Ghost 使用正确
- [ ] 弹窗：ESC 可关闭、遮罩点击可关闭、按钮右对齐
- [ ] Toast：成功 3 秒、错误 5 秒，顶部居中
- [ ] 表格：合适的表格组件，表头与内容对齐
- [ ] 空状态：无账户时明确提示与引导
- [ ] 加载态：接口请求中显示 loading
- [ ] ARIA：Dialog、按钮等具备必要无障碍属性

---

## 7. 风险与注意事项

1. **数据库**：开发阶段可清空数据库，migration 需考虑 `ADD COLUMN IF NOT EXISTS` 以兼容已有库。
2. **多 Token**：同一用户可多次授权，status 返回多条；「新增账户」下拉需正确展示每条 token 的 bid/user_name，并传对应 `oauth_session_id`。
3. **WebSocket**：若 oauth.success 事件尚未实现，授权成功后可先用轮询（如每 2 秒查 status）实现自动刷新，后续再切 WebSocket。

---

## 8. 附录：B站 userinfo API

```
GET https://cm.bilibili.com/takumi/api/open_api/oauth2/userinfo
Authorization: Bearer {access_token}
```

响应示例：

```json
{
  "message": "success",
  "code": 0,
  "data": {
    "bid": 11906268,
    "user_name": "雪雯11770V1"
  }
}
```

---

> 本方案严格遵循 6thdoc-账户管理功能.md、Admiral-客户端交互设计规范.md、Admiral_v1_Architecture.md，作为下一步开发的执行依据。
