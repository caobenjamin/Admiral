# Admiral

> 广告操作的智能指挥系统（Control System for AdOps）

Admiral 是一个服务于广告代理公司优化师的本地 Agent 控制系统。通过自然语言和任务系统，自动执行广告操作与监控，由智能执行引擎统一调度。

## 项目结构

```
admiral/
├── crates/
│   ├── admiral-types/      # 共享类型定义 (Server + Client 复用)
│   ├── admiral-server/     # 服务端 (Axum + PostgreSQL)，部署于远端运行
│   └── admiral-client/     # 桌面客户端 (Tauri 2 + React)，本地运行
├── docker-compose.yml      # 可选：远端服务器上可用的 PostgreSQL 编排
└── prd/                    # 需求与架构文档
```

## 技术栈

| 组件 | 技术 |
|------|------|
| 服务端 | Rust, Axum, SeaORM, PostgreSQL |
| 客户端 Rust | Rust, Tauri 2, SQLite |
| 客户端 UI | React 19, TypeScript, Vite 6, Tailwind CSS 4, Zustand |
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

**B 站 OAuth 说明**：回调地址必须为 HTTPS。本仓库采用「服务端在远端运行」的联调模式，服务端部署在具备 HTTPS 的远端服务器，详见 [服务器部署与 OAuth 联调](docs/服务器部署与OAuth联调.md)。

### 服务端日志

#### 如何开启日志

日志默认已开启，输出到 **stdout**。通过以下方式控制：

- **日志级别**：环境变量 `ADMIRAL_LOG_LEVEL` 覆盖配置，例如 `info`、`debug`、`trace`。未设置时使用 `config/*.toml` 中 `[logging].level`（development 默认 `debug`，production 默认 `info`）。
- **输出格式**：由 `config/*.toml` 中 `[logging].format` 控制：
  - `pretty`：人类可读（development 默认），适合本地调试
  - `json`：JSON 行（production 默认），便于采集到 ELK/Loki 等

```bash
# 联调时使用 debug 查看完整请求/响应
export ADMIRAL_LOG_LEVEL=debug
cargo run -p admiral-server
```

#### 如何查看日志

- **本地运行**：日志直接打印到终端
- **systemd 等托管**：输出到 stdout，由 systemd 或日志收集器接管（如 `journalctl -u admiral-server -f`）
- **Docker/K8s**：`docker logs` 或 kubectl logs 查看容器 stdout

#### 记录了哪些日志

| 类别 | 内容 | 示例 / 说明 |
|------|------|-------------|
| **客户端请求** | 方法、完整 URI、请求体、响应状态、耗时 | `client request method=POST uri=/api/v1/mapi/proxy request_body={...} response_status=200 elapsed_ms=123` |
| **给客户端的返回** | MAPI/LLM 返回给客户端的 HTTP 状态、响应体预览 | MAPI、LLM handler 中记录 `response_status`、`response_body`（超长截断） |
| **MAPI 外部调用** | 请求：目标 URL、Method、请求体；响应：状态、响应体 | `MAPI external call mapi_url=... mapi_method=POST` / `MAPI external response mapi_status=200 mapi_response_body=...` |
| **LLM 外部调用** | 请求：URL、provider、请求体、是否流式；响应：状态、响应体或流式标记 | `LLM external call llm_url=... llm_provider=deepseek` / `LLM external response llm_status=200 llm_response_body=...` |

**说明**：请求体 / 响应体超过阈值会截断（如 2048 字符），避免日志过大。不会记录 `Authorization`、API Key 等敏感 header。请求体若超过 1MB 读取 limit，会标记为 `(omitted)`，handler 可能收到空 body，生产环境可根据需要调整或关闭 body 日志。

---

## 客户端已实现功能

客户端按 [客户端开发文档](prd/5thdoc-2026Feb20-客户端开发文档.md) 与 [客户端开发细节设计计划](.cursor/plans/客户端开发细节设计_d29e9d40.plan.md) 完成样式与布局重设计，采用 Element Plus 风格设计 token，布局为 Header + LeftTab + 主操作两栏（任务列表 + 任务详情/对话）+ StatusBar。

### 实现方式概览

- **类型**：在 `admiral-types/src/api.rs` 中定义 auth / oauth / mapi / llm 请求与响应类型，服务端与客户端共用。
- **配置**：`ClientConfig` 从环境变量读取 `ADMIRAL_API_BASE_URL`（默认 `http://127.0.0.1:8080`）、`ADMIRAL_CREDENTIAL_FILE`（凭证文件路径）。
- **凭证**：Rust 侧 `CredentialStore` 抽象，本期实现为明文 JSON 文件；Token 仅存 Rust，前端不持 token，通过 Tauri command 调 `me()` / `license()` 等获取用户信息。
- **网络**：`ApiClient` 封装全部 REST 调用（auth、oauth、mapi、llm），Bearer 注入与 401 时 refresh 重试在 Command 层统一处理。
- **Tauri**：Tauri 2 + tauri-plugin-shell；所有命令通过 `tauri_bridge` 注册（含 `open_url` 用于在系统浏览器打开 B 站授权链接）。

### 设计系统

- **色彩**：主色 #409EFF，成功 #67C23A，警告 #E6A23C，危险 #F56C6C，文字层级、边框、背景等 token 定义于 `globals.css`。
- **布局**：左侧可伸缩 LeftTab（媒体账户 | 任务 | 能力栏），主操作区两栏（任务列表 300px + 任务详情/对话）。
- **输入**：飞书风格多行 ChatInput，Enter 发送、Shift+Enter 换行。

### 任务系统（Agent + Skill + Task Engine）

基于 [6th-任务系统](prd/6th-任务系统.md) 与 OpenClaw 思路实现的完整任务系统已落地，包含：

- **记忆系统**：`~/.admiral/memory/` 下 USER_MEMORY.md、每日/任务记忆，SQLite FTS5 检索
- **Skill 工具体系**：`skills/` 目录 skill.toml 解析，Registry/Validator/Executor，MAPI 类走服务端代理
- **Agent Runtime**：Profile、Context 组装、ReAct 循环，LLM tool_call → Skill 执行 → 结果回传
- **任务引擎**：状态机、DryRun/TestRun/生产执行、Scheduler（心跳），SQLite 持久化
- **Knowledge RAG**：服务端 `/api/v1/knowledge/query` 与客户端调用（当前为桩，可扩展向量检索）
- **前端**：任务列表/创建/详情、ConfirmActions 执行 DryRun/TestRun/生产，首页 Agent 对话接 `agent_chat`

详细实现说明见 **[任务系统实现报告](docs/任务系统实现报告.md)**。

### 客户端模块结构

```
crates/admiral-client/
├── src/
│   ├── main.rs             # Tauri 入口，注册命令与 AppState、迁移与 AppState 初始化
│   ├── config.rs           # ClientConfig（base_url、credential_path）
│   ├── credential.rs       # CredentialStore trait、PlainFileCredentialStore、StoredCredential
│   ├── tauri_bridge.rs     # AppState、Tauri command 桥（auth、oauth、mapi、llm、task_*、agent_chat、open_url）
│   ├── commands/           # 业务命令（auth、oauth、mapi、llm、task）
│   ├── network/
│   │   └── api_client.rs   # REST 客户端（auth、oauth、mapi、llm、knowledge_query）
│   ├── db/                 # SQLite 迁移与 models（tasks、task_runs、conversations、memory、logs）
│   ├── agent/              # profile、memory、context、runtime（ReAct + tool_call）
│   ├── skill/              # loader、registry、validator、executor
│   ├── task/               # state_machine、executor、engine、scheduler
│   └── ...
├── ui/                     # React 19 + TypeScript + Vite 6 + Tailwind 4
│   ├── src/
│   │   ├── App.tsx、router.tsx
│   │   ├── components/
│   │   │   ├── layout/     # AppShell、LeftTab、MainContent、Header、StatusBar
│   │   │   ├── auth/       # LoginForm、InviteCodeInput
│   │   │   ├── task/       # TaskList、TaskDetail、TaskCreate、TaskTimeline、ConfirmActions
│   │   │   ├── chat/       # ChatPanel、ChatInput、MessageBubble
│   │   │   └── settings/   # PlatformAuth、Preferences
│   │   ├── pages/          # HomePage、SetupPage、AdminPage、MapiDebugPage、LlmChatPage
│   │   ├── stores/         # authStore、taskStore、uiStore、wsStore
│   │   ├── hooks/          # useAuth、useTasks、useAgent、useWebSocket
│   │   └── lib/            # tauri.ts、types.ts
│   └── ...
├── tauri.conf.json
└── icons/
```

### 前端路由与页面

| 路径 | 说明 |
|------|------|
| `/login` | 登录 / 注册（Tab），表单提交后调用 Tauri `login` / `register`，成功跳转首页 |
| `/` | 首页：LeftTab + 任务列表栏 + 任务详情（展示区 + 飞书风格输入区） |
| `/setup` | 初始化向导：权限申请与平台授权入口 |
| `/task/new` | 新建任务：任务创建向导 |
| `/settings` | 设置入口：平台授权、偏好设置、MAPI 调试、LLM 对话、管理 |
| `/settings/platforms` | B 站授权：发起授权、授权列表、绑定账户、取消授权 |
| `/settings/preferences` | 偏好设置：主题、语言等（待完善） |
| `/settings/mapi` | MAPI 调试：填写 account_id、method、path、query/body，发送并展示响应 |
| `/settings/llm` | LLM 对话：简单对话页，非流式，验证 LLM 代理 |
| `/admin` | 管理：用户与邀请码管理，仅管理员（Phase 5 接入完整 API） |

### 客户端环境变量

- `ADMIRAL_API_BASE_URL`：**必设**，远端服务端 API 地址（如 `https://admiral.example.com`），联调时客户端连接此地址
- `ADMIRAL_CREDENTIAL_FILE`：凭证文件路径，默认当前目录 `credentials.json`

---

## 前置要求

- Rust 1.85+ (2024 edition)
- Node.js 20+
- macOS 或 Windows（客户端运行环境）

## 开发与联调模式

**服务端**：统一部署并运行在**远端服务器**（如 GCP），本地仅做代码编辑，不部署、不运行。  
**客户端**：本地运行，通过 `ADMIRAL_API_BASE_URL` 连接远端服务端 API 进行联调。

详细说明见 [联调环境与配置说明](docs/联调环境与配置说明.md)。

## 快速开始

**服务端在远端、本机只跑客户端**时，需要先初始化远端服务端（数据库、.env、JWT、可选 B 站 OAuth / LLM），再在本机设 `ADMIRAL_API_BASE_URL` 并启动客户端。**完整步骤与自检清单**见 **[开始运行：服务端远端与客户端本机](docs/开始运行-服务端远端与客户端本机.md)**。

### 1. 启动客户端（连接远端服务端）

确保远端服务端已部署并运行（参见 [部署到 GCP](docs/部署到GCP-从零到运行.md) 或 [服务器部署与 OAuth 联调](docs/服务器部署与OAuth联调.md)）。

在**仓库根目录**执行：

```bash
# 指定远端 API 地址（替换为你的服务端域名）
export ADMIRAL_API_BASE_URL=https://admiral.example.com

cargo tauri dev
```

Tauri 会先在前端目录 `crates/admiral-client/ui` 下执行 `npm run dev`，再启动桌面窗口。客户端将向上述地址发起登录、OAuth、MAPI、LLM 等请求。**凭证文件、本地 SQLite、记忆目录**无需事先创建，首次登录与使用时会自动初始化。

### 2. 服务端代码修改与部署

本地修改 `crates/admiral-server/` 下的代码后，通过 Git 推送到远端，在服务器上执行 `git pull` 并重新编译、重启服务端。参见 [DEPLOY-GITHUB](docs/DEPLOY-GITHUB.md)。

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
| **客户端样式与布局** | 设计系统 token、LeftTab、MainContent 两栏、TaskList/TaskDetail、ChatInput 飞书风格、ConfirmActions、路由补全 | 已完成 |
| **任务系统（Phase 5 核心）** | 记忆系统（MD+FTS5）、Skill 工具体系、Agent Runtime（ReAct+tool_call）、任务状态机与 DryRun/TestRun/执行、Knowledge RAG 桩、前端任务/对话 UI 联通 | 已完成 |
| Phase 5 其余 | WebSocket、日志闭环、Heartbeat 配置 UI 等 | 待开始 |

## 文档

- **开始运行**（服务端远端 + 客户端本机：初始化步骤与自检清单）：[docs/开始运行-服务端远端与客户端本机.md](docs/开始运行-服务端远端与客户端本机.md)
- **知识库初始化**（Markdown 目录、管理员 ingest API、全文检索）：[docs/知识库初始化.md](docs/知识库初始化.md)
- **任务系统实现报告**（记忆/Skill/Agent/任务引擎/RAG/前端对接）：[docs/任务系统实现报告.md](docs/任务系统实现报告.md)
- 联调环境与配置说明（服务端远端、客户端本地）：[docs/联调环境与配置说明.md](docs/联调环境与配置说明.md)
- 服务端部署：[docs/部署到GCP-从零到运行.md](docs/部署到GCP-从零到运行.md)、[docs/服务器部署与OAuth联调.md](docs/服务器部署与OAuth联调.md)、[docs/DEPLOY-GITHUB.md](docs/DEPLOY-GITHUB.md)
- 服务端开发方案与接口细节：[prd/4thdoc-服务端开发方案.md](prd/4thdoc-服务端开发方案.md)
- 客户端开发文档（PRD）：[prd/5thdoc-2026Feb20-客户端开发文档.md](prd/5thdoc-2026Feb20-客户端开发文档.md)
- 客户端开发细节设计计划：[.cursor/plans/客户端开发细节设计_d29e9d40.plan.md](.cursor/plans/客户端开发细节设计_d29e9d40.plan.md)
- 客户端联调方案（技术栈与未来扩展一致性）：[.cursor/plans/客户端联调实现方案_304a7a99.plan.md](.cursor/plans/客户端联调实现方案_304a7a99.plan.md)
- 需求与架构：[prd/Admiral_v1_Architecture.md](prd/Admiral_v1_Architecture.md)

## License

MIT
