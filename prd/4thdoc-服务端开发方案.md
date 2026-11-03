# Admiral 服务端基础功能 — 开发方案文档

> **基于**：`3nddoc-服务端基础功能.md`（需求）、`Admiral_v1_Architecture.md`（架构与技术选型）  
> **版本**：v1.0  
> **状态**：开发就绪  
> **最后更新**：2026-02-19

---

## 目录

- [1. 文档目的与范围](#1-文档目的与范围)
- [2. 技术栈与工程结构](#2-技术栈与工程结构)
- [3. 用户登录、注册与鉴权](#3-用户登录注册与鉴权)
- [4. 广告平台授权（MAPI OAuth2.0）](#4-广告平台授权mapi-oauth20)
- [5. 广告账户绑定](#5-广告账户绑定)
- [6. MAPI 接口转发服务](#6-mapi-接口转发服务)
- [7. 大模型调用服务](#7-大模型调用服务)
- [8. 数据库与存储](#8-数据库与存储)
- [9. 配置与环境变量](#9-配置与环境变量)
- [10. 开发任务拆解与顺序](#10-开发任务拆解与顺序)

---

## 1. 文档目的与范围

### 1.1 目的

本文档在《3nddoc-服务端基础功能》需求基础上，结合《Admiral_v1_Architecture》的技术框架，给出**服务端**可执行的开发方案，包括：

- 模块划分与目录结构
- 接口定义与请求/响应格式
- 与 B 站 MAPI 的对接细节（URL、参数、错误处理）
- 数据表设计与扩展点（多 Token 区分）
- 配置项与环境变量
- 开发顺序与依赖关系

### 1.2 范围（本期实现）

| 模块 | 说明 |
|------|------|
| 用户注册/登录/鉴权 | 按架构文档「8. MVP 认证流程」实现 |
| MAPI OAuth2.0 | 发起授权、回调换 Token、Token 存储与 10 小时自刷新 |
| 广告账户绑定 | 拉取账户列表、用户多选、建立 用户-账户-Token 绑定 |
| MAPI 接口转发 | 按 account_id 匹配 Token，注入 Header，转发到 B 站 |
| 大模型调用 | 转发请求到 DeepSeek / Doubao / ChatGPT / Gemini，管理员配置、暂不做路由 |

### 1.3 与架构文档的对应关系

| 架构文档章节 | 本方案使用 |
|-------------|------------|
| §4 技术选型 | Rust / Axum / Tokio / PostgreSQL / SeaORM / JWT / Argon2 / aes-gcm |
| §6 服务端架构 | api/、service/、domain/、infra/、middleware/ 模块划分 |
| §8 认证与用户管理 | 注册/登录流程、JWT 双 Token、邀请码、权益表 |
| §10 B站 OAuth | 流程与 Token 生命周期；本方案细化 URL、参数与多 Token 存储 |
| §11 MAPI Gateway | 统一代理、Token 注入、限流预留；本方案细化「按 account_id 匹配 Token」 |
| §16 数据库设计 | 在 oauth_tokens / ad_accounts 上做多 Token 与绑定扩展 |

---

## 2. 技术栈与工程结构

### 2.1 技术栈（与架构一致）

| 层次 | 技术 | 用途 |
|------|------|------|
| 语言 / 运行时 | Rust 1.82+ (edition 2025), Tokio | 服务端主程序 |
| Web 框架 | Axum 0.8+ | HTTP / WebSocket |
| 数据库 | PostgreSQL 16+ | 持久化 |
| ORM | SeaORM 1.x | 迁移与读写 |
| 认证 | jsonwebtoken (JWT), argon2 | 密码哈希与 Token |
| 加密 | aes-gcm | OAuth Token 加密存储 |
| HTTP 客户端 | reqwest | 调用 B 站 MAPI、大模型 API |
| 配置 | config-rs | 多环境配置 |
| 日志 | tracing + tracing-subscriber | 结构化日志 |

### 2.2 服务端工程结构（本期涉及）

```
admiral-server/
├── Cargo.toml
├── config/
│   ├── default.toml
│   ├── development.toml
│   └── production.toml
├── migrations/
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── config.rs
│   ├── error.rs
│   │
│   ├── api/
│   │   ├── mod.rs
│   │   ├── router.rs
│   │   ├── auth.rs          # 注册/登录/刷新/me
│   │   ├── oauth.rs         # B站 initiate/callback/status
│   │   ├── accounts.rs      # 账户列表、绑定
│   │   ├── mapi.rs          # MAPI 转发
│   │   ├── llm.rs           # 大模型转发
│   │   └── admin.rs         # 管理员：用户/邀请码/LLM 配置
│   │
│   ├── service/
│   │   ├── mod.rs
│   │   ├── auth_service.rs
│   │   ├── user_service.rs
│   │   ├── oauth_service.rs      # 授权链接、callback、换 Token
│   │   ├── token_vault.rs        # 加密存/取、按 account 查 Token
│   │   ├── token_refresh_job.rs  # 每 10 小时刷新
│   │   ├── mapi_proxy.rs         # 转发 + Token 注入
│   │   └── llm_proxy.rs          # 大模型转发
│   │
│   ├── domain/
│   │   ├── mod.rs
│   │   ├── user.rs
│   │   ├── invite.rs
│   │   ├── entitlement.rs
│   │   ├── oauth_token.rs
│   │   └── ad_account_binding.rs
│   │
│   ├── infra/
│   │   ├── mod.rs
│   │   ├── db.rs
│   │   ├── crypto.rs
│   │   └── http_client.rs
│   │
│   └── middleware/
│       ├── mod.rs
│       ├── auth_guard.rs
│       ├── request_id.rs
│       └── logging.rs
```

---

## 3. 用户登录、注册与鉴权

### 3.1 需求来源

《3nddoc》第 1 条：按《Admiral_v1_Architecture》**「8. MVP 认证流程（简化版）」**实现。

### 3.2 流程概要

- **注册**：邮箱 + 密码 + 姓名 + 公司 + 邀请码 → 校验邀请码（**一次性**，见下）→ 创建用户 → 绑定权益（试用期）→ 返回 JWT。
- **登录**：邮箱 + 密码 → 校验 → 返回 JWT。
- **鉴权**：REST 使用 `Authorization: Bearer <access_token>`；后续 WebSocket 可复用同一 access_token 或 device_token 方案。

### 3.3 Token 机制（与架构一致）

| Token 类型 | 有效期 | 用途 |
|-----------|--------|------|
| access_token | 2 小时 | API 调用认证 |
| refresh_token | 30 天 | 刷新 access_token |

### 3.4 接口定义

| Method | Path | 说明 | 认证 |
|--------|------|------|------|
| POST | `/api/v1/auth/register` | 注册 | 否 |
| POST | `/api/v1/auth/login` | 登录 | 否 |
| POST | `/api/v1/auth/refresh` | 刷新 access_token | Body 中 refresh_token |
| POST | `/api/v1/auth/logout` | 登出（可选：服务端拉黑 token） | JWT |
| GET | `/api/v1/auth/me` | 当前用户信息 | JWT |
| GET | `/api/v1/auth/license` | 试用期/有效期状态 | JWT |

### 3.5 请求/响应示例

**注册**  
`POST /api/v1/auth/register`

```json
{
  "email": "user@example.com",
  "password": "strong_password",
  "name": "张三",
  "company": "某某公司",
  "invite_code": "INV-XXXX"
}
```

成功：`201` + 与登录相同的 Token 体。

**登录**  
`POST /api/v1/auth/login`

```json
{
  "email": "user@example.com",
  "password": "strong_password"
}
```

成功：`200`

```json
{
  "access_token": "eyJ...",
  "refresh_token": "eyJ...",
  "expires_in": 7200,
  "token_type": "Bearer",
  "user": {
    "id": "uuid",
    "email": "user@example.com",
    "name": "张三",
    "role": "user"
  }
}
```

**刷新**  
`POST /api/v1/auth/refresh`

```json
{
  "refresh_token": "eyJ..."
}
```

成功：返回新的 `access_token`、`expires_in`、`token_type`。

### 3.6 实现要点

- 密码存储：argon2（与架构 19.2 一致）。
- JWT：RS256，私钥/公钥路径从配置读取。
- **邀请码：仅可使用一次，兑换后即作废。** 查 `invites` 表时校验：未使用（`used_count < 1` 或 `status = 'active'`）、未过期（`expires_at`）；注册成功后立即将该邀请码标记为已使用（如 `used_count = 1` 或 `status = 'used'`），之后任何人再次使用同一邀请码注册均视为无效。
- 错误：邀请码无效、已使用、已过期、邮箱已存在、密码错误等返回明确 HTTP 状态与错误码，便于前端区分。

---

## 4. 广告平台授权（MAPI OAuth2.0）

### 4.1 需求来源与回调监听要求

《3nddoc》第 2.1 条：用户在客户端点击「授权」→ 服务端生成跳转链接 → 用户在浏览器完成授权 → 回调带 code/state → 服务端用 code 换 token → 存库并支持多 Token 区分 → 每 10 小时刷新 token。

**OAuth 回调监听**：服务端必须**常驻开启** OAuth 回调接口的 HTTP 监听。用户授权成功后，B 站会将浏览器重定向到服务端配置的 `CALLBACK_URL` 并携带 `code` 与 `state`；服务端需随时能收到该请求并立即用 `code` 换取 token。因此回调地址对应的接口（如 `GET /api/v1/oauth/bilibili/callback`）须在服务运行期间一直可用，不可仅在「发起授权后」临时开放。

### 4.2 B 站 MAPI 相关 URL（固定）

| 步骤 | URL / 用途 |
|------|------------|
| 授权页（浏览器打开） | `https://ad.bilibili.com/developer/developer-authorization?client_id={APP_ID}&state={STATE}` |
| 换取 Token | `POST https://cm.bilibili.com/takumi/api/open_api/oauth2/token` |
| 刷新 Token | 同上，`grant_type=refresh_token` |

服务端需配置：`APP_ID`、`APP_SECRET`、`CALLBACK_URL`（即服务端回调地址，见下）。

**OAuth 回调地址（CALLBACK_URL）**  
- **路径（固定）**：`/api/v1/oauth/bilibili/callback`  
- **代码书写完毕后，配置时使用如下格式（IP 拼接）**：  
  `http://<SERVER_IP>:<PORT>/api/v1/oauth/bilibili/callback`  
  其中 `<SERVER_IP>` 为服务端对外 IP，`<PORT>` 为服务监听端口（如 `8080`）。示例：`http://192.168.1.100:8080/api/v1/oauth/bilibili/callback`  
- 后续接入域名后，改为例如：`https://api.your-domain.com/api/v1/oauth/bilibili/callback`。  
- 在 B 站开放平台配置的「授权回调页」必须与此 URL 完全一致（含协议、主机、端口、路径）。

### 4.3 流程一：获取授权跳转链接

1. 客户端请求（需已登录）：  
   `POST /api/v1/oauth/bilibili/initiate`  
   Body 可为空或预留扩展（如 `scope`）。
2. 服务端：
   - 校验 JWT，取 `user_id`。
   - 生成随机 `state`（如 UUID），将 `(state, user_id)` 存入缓存（如 moka），TTL 建议 10 分钟。
   - 拼接：  
     `https://ad.bilibili.com/developer/developer-authorization?client_id={APP_ID}&state={STATE}`  
   - 返回：  
     `{ "authorization_url": "https://..." }`
3. 客户端收到后，打开该 URL 供用户在浏览器中授权。

### 4.4 流程二：回调与换取 Token（服务端常驻监听）

1. 用户在 B 站授权成功后，浏览器跳转到：  
   `GET {CALLBACK_URL}?code=xxx&state=xxx`
2. **服务端需常驻监听该路径**：`GET /api/v1/oauth/bilibili/callback` 作为普通 HTTP 路由在服务启动时即注册并一直对外提供；任意时刻收到带 `code`、`state` 的请求即执行下述逻辑：
   - 从 query 取 `code`、`state`。
   - 用 `state` 从缓存取出 `user_id`，若不存在或已过期返回 400。
   - 请求 B 站换 Token：  
     `POST https://cm.bilibili.com/takumi/api/open_api/oauth2/token`  
     Body（application/x-www-form-urlencoded）：  
     `grant_type=authorization_code`、`code=...`、`client_id=...`、`client_secret=...`、`redirect_uri=...`
   - 若 B 站返回非 0（如 `code != 0`），记录日志并返回 502 或 400，并带 B 站错误信息。
   - 解析 `data.access_token`、`data.refresh_token`、`data.expires_in`（秒）。
   - **多 Token 区分**：同一用户可多次授权，每次产生一条新 Token 记录（见 §8）。本次写入一条新 `oauth_tokens` 记录并关联 `user_id`。
   - 使用 `token_vault` 将 access_token、refresh_token 用 AES-256-GCM 加密后写入 DB。
   - 可选：通过 WebSocket 向该用户推送 `oauth.success`（架构 §6.3）。
   - 回调页可返回 HTML：提示「授权成功，请关闭窗口」或重定向到客户端 deep link。

### 4.5 流程三：Token 自刷新（每 10 小时）

- **需求**：《3nddoc》要求「每隔 10 个小时就要完成一次 token 的刷新」。
- **实现**：
  - 后台任务（如 tokio 定时或 cron 每 10 小时）扫描 `oauth_tokens` 中需要刷新的记录（可按 `expires_at` 或单独字段「下次刷新时间」判断）。
  - 对每条记录：用 `refresh_token` 调 B 站：  
    `POST https://cm.bilibili.com/takumi/api/open_api/oauth2/token`  
    Body：`grant_type=refresh_token`、`client_id=...`、`client_secret=...`、`refresh_token=...`
  - 若返回成功，用新 `access_token`、`refresh_token`、`expires_in` 更新该条记录并重新加密写入。
  - 若失败（如 refresh_token 失效），标记该记录为失效或删除，并可选通知用户重新授权。
- **B 站刷新响应**：与换 Token 结构相同，见《3nddoc》示例。

### 4.6 接口汇总（OAuth）

| Method | Path | 说明 | 认证 |
|--------|------|------|------|
| POST | `/api/v1/oauth/bilibili/initiate` | 获取授权跳转 URL | JWT |
| GET | `/api/v1/oauth/bilibili/callback` | B 站回调（浏览器访问） | 否 |
| GET | `/api/v1/oauth/bilibili/status` | 当前用户各次授权状态列表 | JWT |
| DELETE | `/api/v1/oauth/bilibili/sessions/{id}` | 取消某次授权（删除对应 token） | JWT |

---

## 5. 广告账户绑定

### 5.1 需求来源

《3nddoc》第 2.2 条：用户点击「绑定账户」→ 服务端用某次授权的 access_token 调 B 站账户列表 → 展示 → 用户多选确认 → 建立 用户 - 账户 - token 绑定并落库。

### 5.2 获取账户列表（B 站接口）

- 请求：  
  `GET https://cm.bilibili.com/takumi/api/open_api/oauth2/advertisers?page=1&size=100`  
  Header：`Authorization: Bearer {access_token}`
- 响应示例（《3nddoc》）：  
  `code: 0`，`data.records[]` 含 `account_id`、`account_name`、`account_role`，`data.total`、`data.page`、`data.size`。

服务端需先确定「用哪一次授权的 token」：可由前端在「绑定账户」时指定某次授权（如 `oauth_token_id` 或 session 标识），或默认用最近一次有效授权。

### 5.3 服务端接口设计

**获取可绑定的账户列表（需指定某次授权）**

- `GET /api/v1/oauth/bilibili/accounts?oauth_session_id={id}` 或  
  `GET /api/v1/oauth/bilibili/accounts`（默认最近一次授权）
- 逻辑：
  - 根据 JWT 取 user_id，再根据 oauth_session_id 或默认规则取一条有效 `oauth_tokens` 记录。
  - 从 token_vault 解密得到 access_token，调用 B 站 `advertisers` 接口。
  - 返回 `{ "records": [...], "total": n }`，可与 B 站格式一致。

**提交绑定**

- `POST /api/v1/oauth/bilibili/bind-accounts`
- Body：  
  `{ "oauth_token_id": "uuid", "account_ids": [431037, ...] }`
- 逻辑：
  - 校验该 `oauth_token_id` 属于当前 user_id 且有效。
  - 对每个 `account_id`：若尚未绑定则插入/更新「用户-账户-该 token」绑定（见 §8 表设计）。
  - 返回 200 或 201。

这样即可实现「用户 - 账户 - token/refresh_token」的绑定，且支持一个用户多次授权、多组 token、每组 token 下多账户。

### 5.4 数据关系（概念）

- 一次 OAuth 授权 → 一条 `oauth_tokens`（含加密的 access/refresh）。
- 用户可选多个账户绑定到「某一条 oauth_tokens」→ 通过 `ad_account_bindings`（或等价表）记录 (user_id, account_id, oauth_token_id)。
- MAPI 转发时根据 `account_id` 查绑定表得到 `oauth_token_id`，再取对应 token 注入。

---

## 6. MAPI 接口转发服务

### 6.1 需求来源

《3nddoc》第 2.3 条：请求内容不变，只做 **token 注入** 和 **URL 拼接**；根据请求体中的 `account_id` 匹配可用的 token。

### 6.2 规则（与 3nddoc 一致）

- **Base URL**：`https://cm.bilibili.com/takumi/api/{接口路径}`  
  例如客户端传路径 `v2/campaign/list`，则最终请求：  
  `https://cm.bilibili.com/takumi/api/v2/campaign/list`
- **Header**：`Authorization: Bearer {access_token}`  
  `access_token` 来源：根据请求体中的 `account_id` + 当前 user_id，查「用户-账户-Token」绑定，取该账户对应的 `oauth_tokens` 记录的 access_token（解密后）。
- 若未找到绑定或 token 已失效，返回 403 或 401，并提示重新授权/绑定。

### 6.3 统一入口设计（与架构 §11 对齐）

- **入口**：`POST /api/v1/mapi/proxy` 或 `POST /api/v1/mapi/execute`（与架构保持一致）。
- **请求体**：透传客户端请求，但必须包含 `account_id`（用于查 token）；同时需要「接口路径」和「方法、query、body」。
  - 示例：  
    `{ "account_id": 431037, "method": "GET", "path": "v2/campaign/list", "query": {...}, "body": null }`  
    或  
    `{ "account_id": 431037, "method": "POST", "path": "v2/campaign/create", "body": { ... } }`
- **处理步骤**：
  1. JWT 校验，取 user_id。
  2. 从 body 取 `account_id`，查绑定表得到 `oauth_token_id`，再查 token_vault 得到 access_token（若过期可先尝试刷新再查）。
  3. 拼接 URL：`https://cm.bilibili.com/takumi/api/{path}`，若有 query 则拼到 URL。
  4. 构造 HTTP 请求：Method、URL、Header `Authorization: Bearer {access_token}`，Body 原样转发。
  5. 发起到 B 站的请求，将响应状态码与 body 原样（或仅做脱敏日志）返回给客户端。

### 6.4 错误处理

- 无绑定 / token 失效：403，body 中 `code` 与 `message` 说明需重新授权或绑定。
- B 站 4xx/5xx：可返回 502 并将 B 站错误信息转述给客户端（注意脱敏）。
- 请求格式错误（缺少 account_id 或 path）：400。

---

## 7. 大模型调用服务

### 7.1 需求来源

《3nddoc》第 3 条：将用户请求转发给大模型，大模型返回结果后返回给客户端。支持 DeepSeek、Doubao、ChatGPT、Gemini；通过官方 API 调用；初期不做模型路由，由管理员在服务端配置当前使用的模型及 API Key。

### 7.2 行为约定

- **路由**：不做智能路由，仅「单模型」：管理员在服务端配置「当前默认模型」及对应 API Key。
- **调用方式**：服务端作为代理，接收客户端请求（如消息列表 + 当前一条用户消息），转发到对应厂商 API，将响应（或 SSE 流）返回给客户端。
- **认证**：所有请求需 JWT，仅登录用户可使用 LLM 代理。

### 7.3 接口设计

- `POST /api/v1/llm/responses`  
  - 请求体：至少包含「消息列表」和可选「模型覆盖」（若暂不实现路由，可忽略模型覆盖）。  
    示例：  
    `{ "messages": [ {"role": "user", "content": "..."}, ... ], "stream": true }`  
  - 若 `stream: true`：响应为 SSE 流，与所选厂商流式格式一致或做一层统一封装。  
  - 若 `stream: false`：响应为 JSON，如 `{ "choices": [ { "message": { "role": "assistant", "content": "..." } } ] }`。

### 7.4 支持的模型与配置

| 模型 | 配置键（示例） | 说明 |
|------|----------------|------|
| DeepSeek | `DEEPSEEK_API_KEY`, `DEEPSEEK_BASE_URL`（可选） | 官方 API |
| Doubao | `DOUBAO_API_KEY`, `DOUBAO_BASE_URL`（可选） | 字节豆包 |
| ChatGPT | `OPENAI_API_KEY`, `OPENAI_BASE_URL`（可选） | OpenAI 兼容 |
| Gemini | `GEMINI_API_KEY` | Google Gemini |

管理员在服务端配置：当前使用的**默认模型**（如 `deepseek` / `doubao` / `chatgpt` / `gemini`）及对应 API Key；若某模型未配置 Key，则该模型不可用。

### 7.5 实现要点

- 使用 `reqwest` 调用各厂商 HTTP/SSE 接口；可抽象 `LlmProvider` trait，每种模型一个实现（与架构 §6.4 LLM Router 的远期路由可兼容）。
- 配置来源：配置文件（如 `config/default.toml`）或环境变量，敏感信息仅从环境变量读取。
- 错误：API Key 未配置、厂商返回错误时，记录日志并返回 502 或 503，body 中可带简短原因（不泄露 Key）。

---

## 8. 数据库与存储

### 8.1 与架构文档的差异说明

- 架构 §16.1 中 `oauth_tokens` 为 `UNIQUE(user_id, platform)`，即每用户每平台仅一条。  
- 《3nddoc》要求「1 个用户可多次授权，获取多个 token，需区分」。因此需要支持**每用户每平台多条** Token 记录。

### 8.2 oauth_tokens 表（扩展后）

- 去掉 `UNIQUE(user_id, platform)`，改为允许 (user_id, platform) 多行。
- 每行表示「一次授权」：一次 OAuth 流程产生一条记录，含加密的 access_token、refresh_token、expires_at、refresh_expires_at 等。
- 建议增加字段：`id` (UUID) 作为主键，便于「账户绑定」时关联到某一次授权。  
  （若架构已有 `id`，则保持不变。）

### 8.3 用户 - 账户 - Token 绑定表

需要明确「某个 account_id 使用哪一条 oauth_tokens 记录」：

- **方案 A**：在 `ad_accounts` 表增加 `oauth_token_id`（FK 到 `oauth_tokens.id`），表示该账户当前使用的授权记录。
- **方案 B**：新建 `ad_account_bindings` 表：  
  `(user_id, platform, account_id, oauth_token_id)`，唯一约束 (user_id, platform, account_id)。

推荐方案 B 或与现有 `ad_accounts` 合并为「账户绑定即 ad_accounts 行 + oauth_token_id」。最终表结构需满足：

- 根据 (user_id, account_id) 可唯一确定一条 oauth_tokens 记录，用于 MAPI 转发时取 token。

### 8.4 其他表

- `users`、`devices`、`entitlements` 按架构 §16.1 实现即可。
- **邀请码表 `invites`**：邀请码**仅可使用一次，兑换后即作废**。表结构需支持「一次性」语义：例如 `max_uses = 1`，且注册成功后将该条记录的 `used_count` 置为 1（或设置 `status = 'used'`）；之后校验时需排除已使用的邀请码（`used_count >= max_uses` 或 `status = 'used'`）。
- 若本期不做「任务元数据、日志同步、MAPI 审计」，可暂不建 `task_meta`、`log_summaries`、`mapi_audit_logs`，或在实现 MAPI 转发时仅加 `mapi_audit_logs` 以便排查。

---

## 9. 配置与环境变量

### 9.1 环境变量清单（本期）

```bash
# 数据库
ADMIRAL_DATABASE_URL=postgres://user:pass@localhost:5432/admiral

# JWT（RS256）
ADMIRAL_JWT_PRIVATE_KEY_PATH=/path/to/private.pem
ADMIRAL_JWT_PUBLIC_KEY_PATH=/path/to/public.pem

# OAuth Token 加密存储（AES-256）
ADMIRAL_TOKEN_ENCRYPTION_KEY=<32-byte-hex>

# B站 MAPI OAuth
ADMIRAL_BILI_APP_ID=<APP_ID>
ADMIRAL_BILI_APP_SECRET=<APP_SECRET>
# 当前可用 IP 拼接，例如：http://<SERVER_IP>:8080/api/v1/oauth/bilibili/callback ；接入域名后改为 https 域名
ADMIRAL_BILI_CALLBACK_URL=http://<SERVER_IP>:<PORT>/api/v1/oauth/bilibili/callback

# 大模型（按需配置）
ADMIRAL_LLM_DEFAULT_PROVIDER=deepseek   # deepseek | doubao | chatgpt | gemini
ADMIRAL_DEEPSEEK_API_KEY=
ADMIRAL_DOUBAO_API_KEY=
ADMIRAL_OPENAI_API_KEY=
ADMIRAL_GEMINI_API_KEY=

# 服务
ADMIRAL_SERVER_HOST=0.0.0.0
ADMIRAL_SERVER_PORT=8080
ADMIRAL_LOG_LEVEL=info
```

### 9.2 配置文件（config-rs）

- 非敏感项可放在 `config/default.toml`（如 server host/port、log level、B 站 base URL、LLM default provider 名称）。
- 敏感项（API Key、DB URL、JWT 路径、encryption key）仅从环境变量读取，并在文档中说明。

---

## 10. 开发任务拆解与顺序

### 10.1 阶段一：认证与用户

1. 搭建 Axum 路由、config、error、middleware（request_id、logging）。
2. 实现 auth_service：注册（邀请码校验**且一次性作废**、创建用户、创建 entitlement）、登录（密码校验、签发 JWT）、刷新（refresh_token 校验、签发新 access_token）。
3. 实现 api/auth：POST register/login/refresh，GET me、license；统一错误码与响应格式。
4. JWT 中间件：从 Header 解析 Bearer token，校验签名与过期，注入 user_id。
5. 数据库：users、invites、entitlements 表及 SeaORM 迁移；devices 表若本期不做踢设备可后做。

### 10.2 阶段二：MAPI OAuth2.0 与 Token 存储

1. 配置项：B 站 APP_ID、APP_SECRET、CALLBACK_URL；Token 加密密钥。
2. infra/crypto：AES-256-GCM 加密/解密；token_vault：加密写入、解密读取，按 id 或 user_id 列表读取。
3. oauth_tokens 表：支持多行 (user_id, platform)；字段含 id、user_id、platform、access_token_encrypted、refresh_token_encrypted、expires_at 等；SeaORM 迁移。
4. oauth_service：initiate（生成 state、写缓存、返回 URL）；callback（换 token、写库、可选 WS 推送）。
5. api/oauth：POST initiate；**GET callback 作为常驻路由注册**（服务端启动即开启监听，随时接收 B 站重定向并完成 code→token 转换）；GET status；DELETE sessions/:id。
6. token_refresh_job：定时（每 10 小时）扫描 oauth_tokens，调用 B 站刷新接口并更新库。

### 10.3 阶段三：广告账户绑定与 MAPI 转发

1. ad_accounts 或 ad_account_bindings 表：user_id、platform、account_id、oauth_token_id（及可选 account_name 等）；唯一 (user_id, platform, account_id)。
2. service：根据 oauth_token_id 取 token，请求 B 站 `advertisers`；绑定：写入 ad_accounts/bindings。
3. api/accounts：GET accounts（可选 oauth_session_id）；POST bind-accounts。
4. mapi_proxy：根据 account_id + user_id 查绑定得到 oauth_token_id，取 access_token；拼接 URL、注入 Header、转发请求；写 mapi_audit_logs（可选）。
5. api/mapi：POST /api/v1/mapi/proxy（或 execute），body 透传并必须含 account_id、method、path、body/query。

### 10.4 阶段四：大模型调用

1. 配置：default provider、各厂商 API Key（环境变量）。
2. llm_proxy：根据配置选择厂商，使用 reqwest 调用对应 API（DeepSeek/OpenAI 兼容/Doubao/Gemini）；支持 stream 与 non-stream。
3. api/llm：POST /api/v1/llm/responses，JWT 校验，转发到 llm_proxy，返回 SSE 或 JSON。
4. （可选）admin API：GET/PUT 当前 default provider（若存在配置表或配置文件热加载）。

### 10.5 依赖关系

- 阶段一独立可交付（注册/登录/鉴权）。
- 阶段二依赖阶段一（JWT、user_id）。
- 阶段三依赖阶段二（oauth_tokens、token_vault、绑定表）。
- 阶段四仅依赖阶段一（JWT）。

建议顺序：一 → 二 → 三，阶段四可与三并行或稍后。

---

## 附录 A：B 站 MAPI 请求示例（供实现参考）

**换取 Token（callback 后）**

```bash
curl -X POST 'https://cm.bilibili.com/takumi/api/open_api/oauth2/token' \
  -H 'Content-Type: application/x-www-form-urlencoded' \
  -d 'grant_type=authorization_code' \
  -d 'code=xxx' \
  -d 'client_id=APP_ID' \
  -d 'client_secret=APP_SECRET' \
  -d 'redirect_uri=CALLBACK_URL'
```

**刷新 Token**

```bash
curl -X POST 'https://cm.bilibili.com/takumi/api/open_api/oauth2/token' \
  -H 'Content-Type: application/x-www-form-urlencoded' \
  -d 'grant_type=refresh_token' \
  -d 'client_id=APP_ID' \
  -d 'client_secret=APP_SECRET' \
  -d 'refresh_token=xxx'
```

**获取广告主列表**

```bash
curl -X GET 'https://cm.bilibili.com/takumi/api/open_api/oauth2/advertisers?page=1&size=100' \
  -H 'Authorization: Bearer YOUR_YOUR_ACCESS_TOKEN'
```

---

## 附录 B：与架构文档的对照表

| 需求点（3nddoc） | 架构文档章节 | 本方案章节 |
|-----------------|-------------|------------|
| 用户登录/注册/鉴权 | §8 MVP 认证流程 | §3 |
| MAPI OAuth 跳转与回调 | §10 B站 OAuth | §4 |
| Token 存储与刷新 | §10, §19.3 | §4.5, §8 |
| 多 Token 区分 | — | §4.4, §8.2 |
| 账户列表与绑定 | §16.1 ad_accounts | §5, §8.3 |
| MAPI 转发与 Token 注入 | §11 MAPI Gateway | §6 |
| 大模型转发、多模型支持 | §6.4 LLM Router | §7 |
| 技术栈与目录结构 | §4, §6.1 | §2 |

---

> 本文档为《3nddoc-服务端基础功能》的细化开发方案，与《Admiral_v1_Architecture》保持一致，可直接作为服务端实现的依据。
