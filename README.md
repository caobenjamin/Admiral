# Admiral

> 广告操作的智能指挥系统（Control System for AdOps）

Admiral 是一个服务于广告代理公司优化师的本地 Agent 控制系统。通过自然语言和任务系统，自动执行广告操作与监控，由智能执行引擎统一调度。

## 项目结构

```
admiral/
├── crates/
│   ├── admiral-types/      # 共享类型定义 (Server + Client 复用)
│   ├── admiral-server/     # 服务端 (Axum + PostgreSQL)
│   └── admiral-client/     # 桌面客户端 (Tauri 2 + React)
├── docker-compose.yml      # 本地开发环境 (PostgreSQL)
└── prd/                    # 需求与架构文档
```

## 技术栈

| 组件 | 技术 |
|------|------|
| 服务端 | Rust, Axum, SeaORM, PostgreSQL |
| 客户端 Rust | Rust, Tauri 2, SQLite |
| 客户端 UI | React 19, TypeScript, Vite, shadcn/ui, Tailwind CSS |
| 进程内缓存 | moka, dashmap, arc-swap |

---

## 服务端已实现功能

服务端按 [4thdoc-服务端开发方案](prd/4thdoc-服务端开发方案.md) 与 [Admiral_v1_Architecture](prd/Admiral_v1_Architecture.md) 实现，包含以下模块。

### 实现方式概览

- **配置**：config-rs 合并 `config/default.toml` 与按环境的 toml，敏感项（数据库、JWT 路径、Token 加密密钥、B 站 OAuth、LLM API Key）仅从环境变量读取。
- **数据库**：PostgreSQL + SeaORM，启动时自动执行迁移（users / invites / entitlements / devices → oauth_tokens → ad_account_bindings）。
- **认证**：JWT RS256（PEM 文件）、密码 argon2 哈希；邀请码一次性使用（`used_count` 校验与递增）。
- **OAuth Token**：AES-256-GCM 加密落库，按用户多 Token 支持；后台每 10 小时刷新 B 站 token。
- **MAPI**：按请求体中的 `account_id` + 当前用户查绑定表取 token，注入 `Authorization` 后转发到 B 站 `https://cm.bilibili.com/takumi/api/{path}`。
- **LLM**：单默认厂商（deepseek / doubao / chatgpt / gemini），从环境变量读 API Key，转发到对应 OpenAI 兼容或厂商 API，支持流式与非流式。

### 服务端模块结构

```
crates/admiral-server/
├── config/                 # default.toml, development.toml, production.toml
├── src/
│   ├── main.rs             # 启动：配置、tracing、DB、迁移、JWT/OAuth/LLM 状态、路由、OAuth 刷新任务
│   ├── config.rs           # 配置加载与 env 敏感项
│   ├── error.rs            # AppError 与 HTTP 错误体
│   ├── state.rs            # AppState（db, jwt, oauth, llm, config）
│   ├── migration/          # SeaORM 迁移（用户/邀请/权益/设备 → oauth_tokens → ad_account_bindings）
│   ├── entities/           # SeaORM 实体（user, invite, entitlement, device, oauth_token, ad_account_binding）
│   ├── api/                # HTTP 路由层
│   │   ├── router.rs       # 路由汇总
│   │   ├── auth.rs         # 注册 / 登录 / 刷新 / me / license
│   │   ├── oauth.rs        # B 站 OAuth：initiate / callback / status / accounts / bind-accounts / delete session
│   │   ├── mapi.rs         # MAPI 代理
│   │   ├── llm.rs          # LLM 聊天代理
│   │   └── admin.rs        # 管理员：邀请码创建与列表、用户权益延期
│   ├── service/            # 业务逻辑
│   │   ├── auth_service.rs
│   │   ├── oauth_service.rs # 含 list_advertisers、bind_accounts、get_access_token_for_account、refresh 周期
│   │   ├── token_vault.rs  # Token 加密存储与按用户/过期列表
│   │   ├── mapi_proxy.rs
│   │   └── llm_router.rs
│   ├── infra/              # 基础设施
│   │   ├── db.rs           # SeaORM 连接池
│   │   ├── jwt.rs          # RS256 签发与校验
│   │   └── crypto.rs       # AES-256-GCM
│   └── middleware/         # request_id、logging、auth_guard（JWT 提取器）
```

### API 列表

| 方法 | 路径 | 说明 | 认证 |
|------|------|------|------|
| GET | `/health` | 健康检查 | 否 |
| POST | `/api/v1/auth/register` | 注册（邮箱+密码+邀请码，邀请码一次性） | 否 |
| POST | `/api/v1/auth/login` | 登录 | 否 |
| POST | `/api/v1/auth/refresh` | 刷新 access_token | Body: refresh_token |
| GET | `/api/v1/auth/me` | 当前用户信息 | JWT |
| GET | `/api/v1/auth/license` | 试用期/有效期状态 | JWT |
| POST | `/api/v1/oauth/bilibili/initiate` | 获取 B 站授权跳转 URL | JWT |
| GET | `/api/v1/oauth/bilibili/callback` | B 站 OAuth 回调（常驻） | 否 |
| GET | `/api/v1/oauth/bilibili/status` | 当前用户各次授权列表 | JWT |
| GET | `/api/v1/oauth/bilibili/accounts` | 可绑定广告账户列表（可选 `?oauth_session_id=`） | JWT |
| POST | `/api/v1/oauth/bilibili/bind-accounts` | 提交绑定（body: oauth_token_id, account_ids） | JWT |
| DELETE | `/api/v1/oauth/bilibili/sessions/:id` | 取消某次授权 | JWT |
| POST | `/api/v1/mapi/proxy` | MAPI 代理（body: account_id, method, path, query?, body?） | JWT |
| POST | `/api/v1/llm/chat` | 大模型对话（body: messages, stream?, model?, max_tokens?） | JWT |
| GET | `/api/v1/admin/invites` | 邀请码列表 | JWT (admin) |
| POST | `/api/v1/admin/invites` | 创建邀请码 | JWT (admin) |
| POST | `/api/v1/admin/users/:id/extend` | 延长用户权益（body: days, reason?） | JWT (admin) |

### 环境变量

完整清单见 [.env.example](.env.example)。必配项示例：

- `ADMIRAL_DATABASE_URL`：PostgreSQL 连接串
- `ADMIRAL_JWT_PRIVATE_KEY_PATH` / `ADMIRAL_JWT_PUBLIC_KEY_PATH`：RS256 密钥路径
- OAuth 与 MAPI：`ADMIRAL_TOKEN_ENCRYPTION_KEY`（32 字节 hex）、`ADMIRAL_BILI_APP_ID`、`ADMIRAL_BILI_APP_SECRET`、`ADMIRAL_BILI_CALLBACK_URL`
- LLM：`ADMIRAL_LLM_DEFAULT_PROVIDER`、对应厂商的 `ADMIRAL_*_API_KEY`

**B 站 OAuth 说明**：回调地址必须为 HTTPS，本地无法满足。若需完成授权联调，请将服务端部署到具备 HTTPS 的服务器，并参见 [服务器部署与 OAuth 联调](docs/服务器部署与OAuth联调.md)。

---

## 客户端已实现功能（联调阶段）

客户端按 [客户端联调实现方案](.cursor/plans/客户端联调实现方案_304a7a99.plan.md) 实现，与当前服务端 REST API 联调，验证认证、OAuth、MAPI、LLM 基本流程；界面从简，技术栈与架构 §7 一致，便于后续升级为精美交互。

### 实现方式概览

- **类型**：在 `admiral-types/src/api.rs` 中定义 auth / oauth / mapi / llm 请求与响应类型，服务端与客户端共用。
- **配置**：`ClientConfig` 从环境变量读取 `ADMIRAL_API_BASE_URL`（默认 `http://127.0.0.1:8080`）、`ADMIRAL_CREDENTIAL_FILE`（凭证文件路径）。
- **凭证**：Rust 侧 `CredentialStore` 抽象，本期实现为明文 JSON 文件；Token 仅存 Rust，前端不持 token，通过 Tauri command 调 `me()` / `license()` 等获取用户信息。
- **网络**：`ApiClient` 封装全部 REST 调用（auth、oauth、mapi、llm），Bearer 注入与 401 时 refresh 重试在 Command 层统一处理。
- **Tauri**：Tauri 2 + tauri-plugin-shell；所有命令通过 `tauri_bridge` 注册（含 `open_url` 用于在系统浏览器打开 B 站授权链接）。

### 客户端模块结构

```
crates/admiral-client/
├── src/
│   ├── main.rs             # Tauri 入口，注册命令与 AppState
│   ├── config.rs           # ClientConfig（base_url、credential_path）
│   ├── credential.rs       # CredentialStore trait、PlainFileCredentialStore、StoredCredential
│   ├── tauri_bridge.rs     # AppState、Tauri command 桥（auth_*、oauth_*、mapi_proxy、llm_chat、open_url）
│   ├── commands/           # 业务命令（调用 api_client + CredentialStore）
│   │   ├── auth.rs         # register、login、refresh、logout、me、license
│   │   ├── oauth.rs        # oauth_initiate、status、accounts_list、bind_accounts、delete_session
│   │   ├── mapi.rs         # mapi_proxy
│   │   └── llm.rs          # llm_chat
│   ├── network/
│   │   └── api_client.rs   # REST 客户端（auth、oauth、mapi、llm）
│   └── ...
├── ui/                     # React 19 + TypeScript + Vite 6 + Tailwind 4
│   ├── src/
│   │   ├── App.tsx、router.tsx
│   │   ├── components/    # layout（AppShell、Sidebar、Header）、auth（LoginForm）、settings（PlatformAuth）
│   │   ├── pages/         # HomePage、MapiDebugPage、LlmChatPage
│   │   ├── stores/        # authStore（完整）、taskStore/wsStore/uiStore（占位）
│   │   ├── hooks/         # useAuth
│   │   └── lib/           # tauri.ts（invoke 封装）、types.ts（与 admiral-types 对齐）
│   └── ...
├── tauri.conf.json
└── icons/
```

### 前端路由与页面

| 路径 | 说明 |
|------|------|
| `/login` | 登录 / 注册（Tab），表单提交后调用 Tauri `login` / `register`，成功跳转首页 |
| `/` | 首页：AppShell + Sidebar + Header，主区展示 `me()`、`license()` |
| `/settings/platforms` | B 站授权：发起授权（打开浏览器）、授权列表、绑定账户、取消授权 |
| `/settings/mapi` | MAPI 调试：填写 account_id、method、path、query/body，发送并展示响应 |
| `/settings/llm` | LLM 对话：简单对话页，非流式，验证 LLM 代理 |

### 客户端环境变量（可选）

- `ADMIRAL_API_BASE_URL`：服务端 API 地址，默认 `http://127.0.0.1:8080`
- `ADMIRAL_CREDENTIAL_FILE`：凭证文件路径，默认当前目录 `credentials.json`

---

## 前置要求

- Rust 1.85+ (2024 edition)
- Node.js 20+
- Docker & Docker Compose
- macOS 或 Windows（客户端运行环境）

## 快速开始

### 1. 启动数据库

```bash
docker compose up -d
```

### 2. 配置环境变量

```bash
cp .env.example .env
# 编辑 .env 填入 ADMIRAL_DATABASE_URL、JWT 路径等（见上方环境变量）
```

### 3. 启动服务端

```bash
cargo run -p admiral-server
```

（若从仓库根目录运行，配置会从 `crates/admiral-server/config` 或当前目录的 `config/` 加载；也可设置 `ADMIRAL_CONFIG_DIR`。）

### 4. 启动客户端

```bash
cd crates/admiral-client
cargo tauri dev
```

## 开发阶段

| 阶段 | 内容 | 状态 |
|------|------|------|
| Phase 0 | 项目骨架 + 配置/错误/DB/中间件 | 已完成 |
| Phase 1 | 认证 + 用户（注册/登录/JWT/邀请码一次性） | 已完成 |
| Phase 2 | B 站 OAuth + Token 存储与 10 小时刷新 | 已完成 |
| Phase 3 | 广告账户绑定 + MAPI 代理 | 已完成 |
| Phase 4 | 大模型代理（单默认厂商） | 已完成 |
| 管理端 | 邀请码创建/列表、用户权益延期 | 已完成 |
| **客户端联调** | 类型共享、CredentialStore、api_client、Tauri 命令、登录/首页/B 站授权/MAPI/LLM 简单 UI | 已完成 |
| Phase 5 | WebSocket、Agent Runtime、Skill、任务系统、日志闭环 | 待开始 |

## 文档

- 服务端开发方案与接口细节：[prd/4thdoc-服务端开发方案.md](prd/4thdoc-服务端开发方案.md)
- 客户端联调方案（技术栈与未来扩展一致性）：[.cursor/plans/客户端联调实现方案_304a7a99.plan.md](.cursor/plans/客户端联调实现方案_304a7a99.plan.md)
- 需求与架构：[prd/Admiral_v1_Architecture.md](prd/Admiral_v1_Architecture.md)

## License

MIT
