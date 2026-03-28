# Admiral

> 广告操作的智能指挥系统（Intelligent Control System for AdOps）

Admiral 是一个服务于广告代理公司优化师的本地 Agent 控制系统。通过自然语言对话与任务系统，自动执行广告平台操作与监控，由智能执行引擎统一调度。

---

## 项目结构

```
admiral/
├── crates/
│   ├── admiral-types/      # 共享类型定义（Server + Client 复用）
│   ├── admiral-server/     # 服务端（Axum + PostgreSQL），部署于远端
│   └── admiral-client/     # 桌面客户端（Tauri 2 + React），本地运行
├── knowledge/              # RAG 知识库源文件（Markdown）
├── prd/                    # 需求与架构文档
├── docs/                   # 开发与部署文档
├── scripts/                # 辅助脚本
└── docker-compose.yml      # PostgreSQL 编排（可选）
```

## 技术栈

| 组件 | 技术 |
|------|------|
| 共享类型 | Rust, serde, uuid, chrono |
| 服务端 | Rust, Axum, SeaORM, PostgreSQL, JWT RS256, AES-256-GCM |
| 客户端 Rust | Rust, Tauri 2, rusqlite (SQLite), tokio, governor (rate limiting) |
| 客户端 UI | React 19, TypeScript, Vite 6, Tailwind CSS 4, Zustand 5 |

---

## 核心模块

### admiral-types — 共享类型库

服务端与客户端共用的领域类型，分为三个模块：

- **api** — REST 请求/响应 DTO：认证、OAuth、MAPI 代理、LLM 聊天
- **skill** — Skill 定义：`SkillDefinition`、`SkillCall`、`SkillResult`，含风险等级与 LLM function calling schema
- **task** — 任务域核心类型：`Task`、`TaskPlan`、`TaskStep`、`TaskRun`、`TaskProgress`，以及 `RetryPolicy`、`StepMode`（Single / ForEach）、条件执行等

### admiral-server — 中心服务

部署于远端，提供认证、OAuth Token 保管、MAPI 代理、LLM 代理、知识库检索等功能。

**主要模块：**

| 模块 | 职责 |
|------|------|
| `api/` | HTTP 路由（auth、oauth、mapi、llm、knowledge、admin） |
| `service/` | 业务逻辑（认证、OAuth Token 管理、MAPI 转发、LLM 路由、知识库 ingest/检索） |
| `infra/` | 基础设施（数据库连接、JWT 签发/校验、AES 加密） |
| `middleware/` | 请求 ID、日志、JWT 鉴权 |
| `migration/` | SeaORM 数据库迁移 |

**API 概览：**

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/v1/auth/register` | 注册（需邀请码） |
| POST | `/api/v1/auth/login` | 登录 |
| POST | `/api/v1/auth/refresh` | 刷新 Token |
| GET | `/api/v1/auth/me` | 当前用户 |
| GET | `/api/v1/auth/license` | 权益状态 |
| POST | `/api/v1/oauth/bilibili/initiate` | 发起 B 站授权 |
| GET | `/api/v1/oauth/bilibili/callback` | OAuth 回调 |
| GET | `/api/v1/oauth/bilibili/status` | 授权列表 |
| GET | `/api/v1/oauth/bilibili/bindings` | 已绑定账户 |
| POST | `/api/v1/oauth/bilibili/bind-accounts` | 绑定广告账户 |
| POST | `/api/v1/mapi/proxy` | MAPI 代理转发 |
| POST | `/api/v1/llm/chat` | LLM 对话代理 |
| POST | `/api/v1/knowledge/query` | 知识库全文检索 |
| POST | `/api/v1/admin/knowledge/ingest` | 知识库导入 |

### admiral-client — 桌面客户端

本地运行的 Tauri 桌面应用，包含任务引擎、Agent 运行时和 React 前端。

**Rust 模块：**

```
src/
├── agent/          # Agent 运行时
│   ├── profile     # Agent 人格配置（含对话驱动策略）
│   ├── context     # 上下文组装（System + Memory + TaskSpec + Knowledge + History）
│   ├── runtime     # ReAct 循环 + SSE 流式解析
│   ├── memory      # 记忆系统（Markdown + SQLite FTS5）
│   ├── memory_updater  # 对话后自动记忆提取
│   └── spec_updater    # 对话后自动更新任务规格 (TASK_SPEC.md)
├── skill/          # Skill 工具体系
│   ├── loader      # 从 skills/ 目录加载 skill.toml
│   ├── registry    # 技能注册表与 LLM tools 生成
│   ├── validator   # 参数校验
│   └── executor    # 执行分发（MAPI/本地），含 per-account 速率限制
├── task/           # 任务引擎
│   ├── engine      # 任务生命周期管理与持久化
│   ├── executor    # 计划执行器（StepContext、ForEach、条件、重试）
│   ├── scheduler   # Heartbeat / Cron 定时调度
│   ├── state_machine  # 对话驱动状态机
│   ├── spec        # 任务规格文档 (TaskSpec ↔ Markdown)
│   ├── completeness  # 完整度评估引擎
│   └── report      # 执行报告生成（Markdown）
├── commands/       # Tauri 业务命令（auth、oauth、mapi、llm、task）
├── network/        # REST 客户端（api_client）
├── db/             # SQLite 迁移与数据访问
├── tauri_bridge.rs # AppState 与 Tauri command 注册
└── config.rs       # 客户端配置
```

**前端 UI：**

```
ui/src/
├── components/
│   ├── layout/     # AppShell、Header、LeftTab、MainContent、StatusBar
│   ├── auth/       # 登录 / 注册表单
│   ├── task/       # TaskList、TaskDetail、TaskCreate、TaskTimeline、ConfirmActions
│   ├── chat/       # ChatPanel、ChatInput、MessageBubble、MarkdownContent
│   └── settings/   # 平台授权、偏好设置
├── pages/          # HomePage、SetupPage、AdminPage、MapiDebugPage、LlmChatPage
├── stores/         # Zustand stores（auth、task、ui、ws、toast）
├── hooks/          # useAuth、useTasks、useAgent、useWebSocket
└── lib/            # tauri.ts 封装、types.ts 类型定义
```

---

## 对话驱动任务系统

Admiral 的任务系统采用**对话驱动**模式：用户只需提供最少信息创建任务（名称 + 触发类型），随后通过持续对话逐步明确目标，系统自动评估信息完整度并推进生命周期。

### 核心理念

- **极简创建**：创建任务只需名称 + 选择手动/定时，不需要填写复杂表单
- **引导对话**：系统主动提问，帮助用户逐步定义任务目标和实现路径
- **规格沉淀**：每个任务维护一份 `TASK_SPEC.md`，记录目标、上下文、步骤、参数、运行历史，防止 LLM 失忆
- **完整度评估**：系统持续评估信息完整度（Incomplete → Partial → Sufficient → Complete），自动推进状态
- **试运行验证**：信息足够时建议试运行，根据结果继续完善配置
- **执行报告**：每次执行生成结构化 Markdown 报告

### 任务生命周期

```
Draft → Exploring → Configuring → Testing → Ready → Active
             (引导对话)    (细化配置)   (试运行)  (可发布)  (定时运行)
                              ↑            |              |
                              └────────────┘              |
                              ↑                           |
                              └───────────────────────────┘
```

| 状态 | 说明 |
|------|------|
| **Draft** | 刚创建，仅有名称和触发类型 |
| **Exploring** | 系统通过对话收集任务目标和需求 |
| **Configuring** | 信息基本明确，细化配置和实现路径 |
| **Testing** | 进行试运行验证配置 |
| **Ready** | 试运行通过，可发布/调度 |
| **Active** | 已发布，定时任务运行中 |

### 任务规格文档 (TASK_SPEC.md)

每个任务在 `~/.admiral/memory/tasks/{uuid}/` 下维护规格文档，结构包括：

- **目标** — 从对话中提取的任务目标
- **上下文** — 平台、关联账户、触发方式
- **实现路径** — 有序步骤，每步关联技能和参数
- **信息完整度评估** — 检查清单（已确认/待确认）
- **运行历史摘要** — 每次执行的结果记录

### 执行引擎

| 特性 | 说明 |
|------|------|
| **三阶段执行** | DryRun（验证）→ TestRun（试运行）→ Production（正式执行） |
| **StepContext 数据管道** | 步骤输出通过 `output_key` 存入上下文，后续步骤通过 `param_bindings` 引用 |
| **ForEach 批量执行** | 对列表并发执行，支持 `max_concurrency` 控制 |
| **条件执行** | 步骤可设置条件表达式，不满足时跳过或失败 |
| **指数退避重试** | 可配置 `RetryPolicy`（max_retries、backoff_ms、backoff_multiplier） |
| **操作权限审批** | 写操作自动检查权限白名单，未授权时暂停等待用户审批 |
| **实时进度事件** | `TaskProgress` 事件推送到前端 |
| **执行报告** | 每次执行后自动生成 Markdown 报告并持久化 |
| **定时调度** | Scheduler 每 30s 检查 Active 状态的 Cron/Heartbeat 任务 |

---

## Agent 运行时

Agent 基于 ReAct 循环与 LLM function calling 实现自然语言到操作的转换。

**执行流程：**

1. 用户消息 → Knowledge RAG 检索相关知识片段
2. Context 组装：System Prompt + User Memory + **Task Spec** + Knowledge + 历史对话 + 完整度评估
3. LLM 生成回复（支持流式 SSE），包含对话驱动策略和新任务引导
4. 若包含 `tool_calls` → Skill Executor 执行 → 结果回传 → 继续循环
5. 对话结束后异步执行：
   - 记忆提取更新（User Memory + Task Memory）
   - **任务规格更新**（从对话中提取结构化信息，合并到 TASK_SPEC.md）
   - **完整度评估**（规则引擎检查，自动推进任务状态）

**记忆系统：** `~/.admiral/memory/` 下按用户 / 每日 / 任务维度存储 Markdown，辅以 SQLite FTS5 搜索。每个任务额外维护 `TASK_SPEC.md`（规格文档）和 `runs/{run_id}.md`（执行报告）。

---

## 前置要求

- Rust 1.85+（2024 edition）
- Node.js 20+
- macOS 或 Windows（客户端运行环境）

## 快速开始

### 启动客户端

确保远端服务端已部署运行（参见 [部署文档](docs/部署到GCP-从零到运行.md)）。

```bash
export ADMIRAL_API_BASE_URL=https://your-server-domain
cargo tauri dev
```

Tauri 会自动构建前端并启动桌面窗口。凭证文件与本地数据库在首次使用时自动创建。

### 服务端部署

详见 [开始运行](docs/开始运行-服务端远端与客户端本机.md) 与 [部署到 GCP](docs/部署到GCP-从零到运行.md)。

### 环境变量

完整清单见 [.env.example](.env.example)。必配项：

| 变量 | 说明 |
|------|------|
| `ADMIRAL_DATABASE_URL` | PostgreSQL 连接串 |
| `ADMIRAL_JWT_PRIVATE_KEY_PATH` | RS256 私钥路径 |
| `ADMIRAL_JWT_PUBLIC_KEY_PATH` | RS256 公钥路径 |
| `ADMIRAL_TOKEN_ENCRYPTION_KEY` | Token 加密密钥（32 字节 hex） |
| `ADMIRAL_BILI_APP_ID` / `APP_SECRET` | B 站 OAuth 应用凭证 |
| `ADMIRAL_BILI_CALLBACK_URL` | OAuth 回调地址（须 HTTPS） |
| `ADMIRAL_DEEPSEEK_API_KEY` | LLM API Key |
| `ADMIRAL_API_BASE_URL` | 客户端连接的服务端地址 |

---

## 日志

服务端日志输出到 stdout，通过 `ADMIRAL_LOG_LEVEL` 控制级别（默认 `info`），`config/*.toml` 中 `[logging].format` 控制格式（`pretty` / `json`）。

记录内容：客户端请求、MAPI/LLM 外部调用与响应（超长截断），不记录敏感 header。

---

## 开发阶段

| 阶段 | 内容 | 状态 |
|------|------|------|
| Phase 0–1 | 项目骨架、认证系统 | 已完成 |
| Phase 2–3 | B 站 OAuth、广告账户绑定、MAPI 代理 | 已完成 |
| Phase 4 | LLM 代理（DeepSeek） | 已完成 |
| 管理端 | 邀请码、用户权益管理 | 已完成 |
| 客户端联调 | 类型共享、API 客户端、Tauri 桥接、基础 UI | 已完成 |
| UI 设计 | Element Plus 风格设计系统、布局重设计 | 已完成 |
| **任务系统 v1** | Agent Runtime、Skill 体系、任务引擎（含状态机、StepContext、ForEach、条件执行、重试、进度事件）、Knowledge RAG、定时调度 | 已完成 |
| 流式对话 | LLM 端到端 SSE 流式交互 | 已完成 |
| **对话驱动任务系统 v2** | 重构为对话驱动模式：极简创建 → 引导对话 → TASK_SPEC.md 规格沉淀 → 完整度评估 → 试运行 → 执行报告。新增 spec_updater、completeness、report 模块。前端简化创建流程，任务详情增加规格/历史标签页 | 已完成 |
| 下一步 | WebSocket 通知、日志闭环、Heartbeat 配置 UI、cron 表达式解析 | 规划中 |

## 文档

| 文档 | 说明 |
|------|------|
| [开始运行](docs/开始运行-服务端远端与客户端本机.md) | 完整初始化步骤与自检清单 |
| [任务系统实现报告](docs/任务系统实现报告.md) | Agent / Skill / 任务引擎 / RAG 详细实现 |
| [知识库初始化](docs/知识库初始化.md) | RAG 知识导入与检索 |
| [联调环境与配置](docs/联调环境与配置说明.md) | 服务端远端 + 客户端本地联调 |
| [部署到 GCP](docs/部署到GCP-从零到运行.md) | 从零到运行的服务端部署 |
| [服务器部署与 OAuth](docs/服务器部署与OAuth联调.md) | HTTPS 与 OAuth 回调配置 |
| [DEPLOY-GITHUB](docs/DEPLOY-GITHUB.md) | Git 推送部署流程 |

## License

MIT
