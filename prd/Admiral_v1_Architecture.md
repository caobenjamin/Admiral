# Admiral v1.0 — 需求规格 + 系统架构文档

> **项目代号**：Admiral  
> **定位**：本地执行型广告自动化 Agent 控制系统  
> **版本**：v1.0 MVP  
> **文档状态**：Development-Ready（可直接进入编码）  
> **最后更新**：2026-02-19

---

## 目录

- [0. 第一性原理与项目愿景](#0-第一性原理与项目愿景)
- [1. MVP 范围界定](#1-mvp-范围界定)
- [2. 系统总体架构](#2-系统总体架构)
- [3. 架构原则](#3-架构原则)
- [4. 技术选型](#4-技术选型)
- [5. 参考 OpenClaw 的 Agent 架构设计](#5-参考-openclaw-的-agent-架构设计)
- [6. 服务端架构详细设计](#6-服务端架构详细设计)
- [7. 客户端架构详细设计](#7-客户端架构详细设计)
- [8. 认证与用户管理](#8-认证与用户管理)
- [9. 设备绑定与在线机制](#9-设备绑定与在线机制)
- [10. B站 OAuth 授权](#10-b站-oauth-授权)
- [11. MAPI Gateway](#11-mapi-gateway)
- [12. 任务系统](#12-任务系统)
- [13. Skill 系统](#13-skill-系统)
- [14. dry-run 与 test-run](#14-dry-run-与-test-run)
- [15. 日志体系](#15-日志体系)
- [16. 数据库设计](#16-数据库设计)
- [17. 接口设计总览](#17-接口设计总览)
- [18. 未来扩展预留](#18-未来扩展预留)
- [19. 安全设计](#19-安全设计)
- [20. 开发阶段规划](#20-开发阶段规划)
- [21. AI 编码规则](#21-ai-编码规则)

---

## 0. 第一性原理与项目愿景

Admiral 是一个服务于广告代理公司(Agency)优化师的本地 Agent 控制系统。

**核心命题**：让广告优化师通过自然语言 + 任务系统，自动执行广告操作与监控，由智能执行引擎统一调度。

**本质定位**：广告操作的智能指挥系统（Control System for AdOps）

**长期愿景**（MVP 不实现，但架构预留）：
- 接入更多广告媒体平台（巨量引擎、腾讯广告、快手等）
- 通过微信服务号向用户推送任务状态、告警等消息
- 通过微信小程序查看任务运行情况、进行简单操控
- 开源客户端，付费用户使用云端服务
- 支持团队协作与多设备

---

## 1. MVP 范围界定

### 本期做

| 模块 | 范围 |
|------|------|
| 桌面客户端 | macOS + Windows（Tauri 2） |
| 服务端 | 中央 API + WebSocket 网关 |
| 广告平台 | 仅 Bilibili MAPI |
| 用户体系 | 邀请码注册，管理员手动创建/管理 |
| 认证方式 | 邮箱 + 密码（MVP 简化版） |
| 付费模式 | 管理员手动控制试用期/有效期 |
| 设备策略 | 单用户单设备 |
| 执行模式 | 单执行节点（本地） |
| 网络要求 | 必须在线 |

### 本期不做（但架构预留接口）

| 模块 | 说明 |
|------|------|
| 微信扫码登录 | 预留 `AuthProvider` trait |
| 手机号注册 | 预留 `AuthProvider` trait |
| 在线支付 | 预留 `PaymentProvider` trait |
| 微信服务号推送 | 预留 `NotificationChannel` trait |
| 微信小程序 | 预留 `ClientChannel` trait + REST API |
| 多广告平台 | 预留 `MediaPlatform` trait |
| 免费/开源版本 | 预留本地 LLM 配置接口 |
| 多设备同步 | 数据模型已考虑 |
| 团队协作 | 数据模型已考虑 |

---

## 2. 系统总体架构

```
┌────────────────────────────────────────────────────────────┐
│                     Admiral 系统架构                        │
│                                                            │
│  ┌──────────────┐    WebSocket/HTTPS    ┌───────────────┐  │
│  │  Admiral      │◄───────────────────►│  Admiral       │  │
│  │  Desktop      │                      │  Server        │  │
│  │  (Tauri 2)    │                      │  (Axum)        │  │
│  │              │                      │               │  │
│  │  ┌─────────┐ │                      │ ┌───────────┐ │  │
│  │  │Agent    │ │                      │ │Auth       │ │  │
│  │  │Runtime  │ │                      │ │Service    │ │  │
│  │  ├─────────┤ │                      │ ├───────────┤ │  │
│  │  │Task     │ │                      │ │Token      │ │  │
│  │  │Engine   │ │                      │ │Vault      │ │  │
│  │  ├─────────┤ │                      │ ├───────────┤ │  │
│  │  │Skill    │ │                      │ │MAPI       │ │  │
│  │  │Registry │ │                      │ │Gateway    │ │  │
│  │  ├─────────┤ │                      │ ├───────────┤ │  │
│  │  │Local DB │ │                      │ │LLM        │ │  │
│  │  │(SQLite) │ │                      │ │Router     │ │  │
│  │  └─────────┘ │                      │ ├───────────┤ │  │
│  └──────────────┘                      │ │WS Gateway │ │  │
│                                        │ ├───────────┤ │  │
│  ┌──────────────┐ (未来)               │ │DB         │ │  │
│  │ WeChat Mini  │◄─── REST API ───────►│ │(Postgres) │ │  │
│  │ Program      │                      │ └───────────┘ │  │
│  └──────────────┘                      └───────────────┘  │
│                                              │            │
│  ┌──────────────┐ (未来)                     │            │
│  │ WeChat       │◄─── Webhook ───────────────┘            │
│  │ Service Acct │                                         │
│  └──────────────┘                                         │
│                                              │            │
│                                        ┌─────┴─────────┐  │
│                                        │ Bilibili MAPI │  │
│                                        │ (External)     │  │
│                                        └───────────────┘  │
└────────────────────────────────────────────────────────────┘
```

### 数据流概览

```
用户输入 → Agent Runtime (意图识别 + Skill 匹配)
         → Task Engine (任务编排 + 状态机)
         → Skill 执行 (本地)
         → MAPI Gateway (服务端代理) → Bilibili API
         → 结果回传 → UI 更新
```

---

## 3. 架构原则

### 3.1 单设备原则
同一账号只允许一个客户端在线，新设备登录自动踢旧设备（通过 WebSocket 下发 `force_logout` 事件）。

### 3.2 单执行节点原则
所有任务仅在客户端本地执行，服务端只做中继（Token 注入、API 代理、LLM 转发）。这保证了：
- 用户数据本地化
- 执行不依赖服务端算力
- 天然隔离不同用户的操作

### 3.3 安全原则
- 客户端 **不得** 保存任何广告平台 Token
- 所有 MAPI 调用 **必须** 经服务端代理
- 服务端 Token 加密存储（AES-256-GCM）
- 所有 HTTP 通信强制 HTTPS

### 3.4 在线强制原则
- 客户端必须维持 WebSocket 长连接
- 断网 > 5 分钟进入 offline 状态，禁止执行任务
- 恢复连接后自动重连 + 状态同步

### 3.5 可扩展原则（参考 OpenClaw）
- 所有外部依赖通过 trait（接口）抽象
- 插件化的 Skill 系统，支持动态加载
- Channel 抽象层，未来可接入微信小程序、服务号等

---

## 4. 技术选型

### 服务端

| 层次 | 技术 | 版本 | 理由 |
|------|------|------|------|
| 语言 | Rust | 1.82+ (2025 edition) | 高性能、类型安全、无 GC |
| Web 框架 | Axum | 0.8+ | Tokio 生态原生、Tower middleware、社区活跃 |
| 异步运行时 | Tokio | 1.x | Rust 异步标准 |
| 数据库 | PostgreSQL | 16+ | JSONB 支持、扩展生态丰富、比 MySQL 更适合复杂查询 |
| ORM | SeaORM | 1.x | Rust 原生异步 ORM，支持 migration |
| WebSocket | axum + tokio-tungstenite | — | Axum 原生 WebSocket 支持 |
| 进程内缓存 | moka | 0.12+ | TTL 缓存（限流计数、token 缓存），无外部依赖 |
| 并发数据结构 | dashmap | 6+ | 线程安全 HashMap（设备在线状态、会话管理） |
| 热加载配置 | arc-swap | 1+ | 无锁原子指针交换（限流配置热加载） |
| LLM 调用 | reqwest + async-openai | — | 灵活路由多模型 |
| 配置管理 | config-rs | — | 多环境配置 |
| 日志 | tracing + tracing-subscriber | — | 结构化日志 |
| 认证 | jsonwebtoken (JWT) | — | 无状态认证，access_token + refresh_token |
| 密码哈希 | argon2 | — | 当前最安全的密码哈希算法 |
| 加密 | aes-gcm | — | Token 加密存储 |

> **为什么选 PostgreSQL 而非 MySQL**：PostgreSQL 的 JSONB 类型天然适合存储 Skill 配置、任务参数等半结构化数据；其 `LISTEN/NOTIFY` 机制在未来可用于实时事件分发；扩展生态（如 pgvector）在未来可支持语义搜索等 AI 场景。

> **为什么 MVP 不用 Redis**：MVP 阶段为单机部署、用户规模极小（个位数），Redis 引入的运维成本和架构复杂度不值得。会话状态、限流计数、设备在线管理等场景使用 Rust 进程内方案完全满足需求：`moka` 提供高性能 TTL 缓存（内置 LRU 淘汰），`dashmap` 提供并发安全的 HashMap，`arc-swap` 提供无锁配置热加载。进程重启时，持久化状态从 PostgreSQL 恢复，瞬时状态（限流计数、在线列表）允许丢失后重建。未来用户规模增长或需要多实例部署时，可透明替换为 Redis 而不改变业务层接口（通过抽象 `CacheStore` trait）。

### 客户端

| 层次 | 技术 | 版本 | 理由 |
|------|------|------|------|
| 框架 | Tauri | 2.x | 跨平台桌面应用，体积小，安全模型优秀 |
| 后端语言 | Rust | 1.82+ | 与服务端统一，复用类型定义 |
| 前端框架 | React | 19+ | 生态成熟，配合 TypeScript |
| 前端构建 | Vite | 6+ | 极速 HMR，ES module 原生 |
| UI 组件库 | shadcn/ui + Tailwind CSS | 4.x | 现代设计系统，可定制性高 |
| 状态管理 | Zustand | 5+ | 轻量，TypeScript 友好 |
| 本地数据库 | SQLite (via rusqlite) | — | 嵌入式，零配置 |
| Agent 运行时 | 自建（参考 OpenClaw 模式） | — | 见第 5 节详细设计 |

### 通信协议

| 场景 | 协议 | 说明 |
|------|------|------|
| 客户端 ↔ 服务端 API | HTTPS REST | 标准 CRUD 操作 |
| 实时通信 | WebSocket (WSS) | 状态推送、事件下发、心跳 |
| 服务端 ↔ Bilibili MAPI | HTTPS REST | 代理调用 |
| 服务端 ↔ LLM | HTTPS (SSE) | 流式响应 |

### 部署

| 组件 | 方案 |
|------|------|
| 服务端 | Docker Compose（单机 MVP），未来迁移 k8s |
| 数据库 | PostgreSQL，Docker 容器化 |
| 域名/证书 | Let's Encrypt + Caddy 反向代理 |
| CI/CD | GitHub Actions |

---

## 5. 参考 OpenClaw 的 Agent 架构设计

Admiral 的 Agent 引擎借鉴 OpenClaw 的核心理念，但针对广告自动化场景进行了重新设计。

### 5.1 Agent 核心概念映射

| OpenClaw 概念 | Admiral 对应 | 说明 |
|---------------|-------------|------|
| SOUL.md | `agent_profile.toml` | Agent 的人格、规则、约束定义 |
| IDENTITY.md | 内嵌于 profile | Agent 的专业领域定义（广告优化） |
| MEMORY.md | `memory_store` (SQLite) | 长期记忆：用户偏好、历史操作模式 |
| HEARTBEAT.md | `scheduler` | 定时任务触发器 |
| Skills | `skills/` 目录 | 以 TOML + Rust/TS 实现的能力包 |
| Channels | `ClientChannel` trait | 桌面 UI / 未来微信小程序 |
| Gateway | Server WS Gateway | WebSocket 网关，消息路由 |
| Agent Runtime | `AgentRuntime` (本地) | 上下文组装 → LLM 调用 → 工具执行循环 |

### 5.2 Agent Runtime 架构（客户端本地）

```
┌─────────────────────────────────────────────────┐
│                  Agent Runtime                   │
│                                                  │
│  ┌──────────────────────────────────────────┐   │
│  │           Context Assembler               │   │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐  │   │
│  │  │ Agent    │ │ Session  │ │ Memory   │  │   │
│  │  │ Profile  │ │ History  │ │ Store    │  │   │
│  │  └──────────┘ └──────────┘ └──────────┘  │   │
│  └──────────────────────────────────────────┘   │
│                      │                           │
│                      ▼                           │
│  ┌──────────────────────────────────────────┐   │
│  │            LLM Interaction Loop           │   │
│  │                                           │   │
│  │  User Input → Context Build → LLM Call    │   │
│  │       ↑          (via Server Proxy)       │   │
│  │       │                │                  │   │
│  │       │                ▼                  │   │
│  │  Result ← Tool Execution ← Tool Choice   │   │
│  └──────────────────────────────────────────┘   │
│                      │                           │
│                      ▼                           │
│  ┌──────────────────────────────────────────┐   │
│  │           Skill Executor                  │   │
│  │  ┌────────────┐ ┌───────────────────┐    │   │
│  │  │ Local      │ │ Remote            │    │   │
│  │  │ Skills     │ │ Skills            │    │   │
│  │  │ (解析/工具)│ │ (MAPI via Server) │    │   │
│  │  └────────────┘ └───────────────────┘    │   │
│  └──────────────────────────────────────────┘   │
└─────────────────────────────────────────────────┘
```

### 5.3 Agent 交互循环（ReAct 模式）

```
loop {
    1. 组装上下文
       - 加载 agent_profile（系统提示词 + 规则）
       - 加载当前会话历史
       - 加载相关记忆（向量检索或关键词匹配）
       - 加载可用 Skill 列表（以 tool_use 格式）

    2. 调用 LLM（经服务端代理）
       - 发送完整上下文 → 服务端 /llm/chat
       - 服务端路由到合适的模型（按 Skill 复杂度）
       - 流式返回响应

    3. 解析 LLM 响应
       - 如果是纯文本 → 直接呈现给用户
       - 如果包含 tool_call → 进入 Skill 执行

    4. 执行 Skill
       - 本地 Skill：直接执行（解析、计算、文件操作等）
       - 远程 Skill（MAPI）：通过服务端代理调用
       - 记录执行日志

    5. 回传结果
       - 将 Skill 执行结果拼回上下文
       - 继续循环（直至 LLM 给出最终回答）

    6. 更新记忆
       - 提取本次交互的关键信息
       - 写入 memory_store
}
```

### 5.4 Agent Profile 定义（agent_profile.toml）

```toml
[agent]
name = "Admiral"
version = "1.0.0"
description = "广告操作智能助手"

[personality]
role = "资深广告优化师助手"
tone = "专业、简洁、主动确认"
language = "zh-CN"

[rules]
# 强制规则 — LLM 必须遵守
must_dry_run = true                    # 所有写操作必须先 dry-run
must_test_run_before_deploy = true     # 部署前必须 test-run
max_batch_size = 100                   # 单次批量操作上限
require_user_confirm_for_write = true  # 写操作需用户确认

[constraints]
supported_platforms = ["bilibili"]     # MVP 仅 B 站
max_concurrent_tasks = 5
task_timeout_seconds = 3600

[llm]
default_model = "claude-sonnet-4-20250514"
planning_model = "claude-sonnet-4-20250514"    # 任务规划用更强模型
execution_model = "claude-haiku"               # 简单执行用快速模型
temperature = 0.1                              # 广告操作需要确定性
```

### 5.5 Skill 定义格式（参考 OpenClaw 的 SKILL.md）

每个 Skill 以独立目录存在于 `skills/` 下：

```
skills/
├── semantic/                   # 语义解析类
│   ├── intent_parser/
│   │   ├── skill.toml
│   │   └── prompt.md
│   └── entity_extractor/
│       ├── skill.toml
│       └── prompt.md
├── tools/                      # 通用工具类
│   ├── excel_export/
│   │   ├── skill.toml
│   │   └── handler.ts
│   └── email_sender/
│       ├── skill.toml
│       └── handler.ts
└── mapi/                       # 广告 API 类（核心）
    └── bilibili/
        ├── get_account_list/
        │   ├── skill.toml
        │   └── handler.rs
        ├── get_campaign_list/
        │   ├── skill.toml
        │   └── handler.rs
        ├── create_campaign/
        │   ├── skill.toml
        │   └── handler.rs
        ├── get_ad_data/
        │   ├── skill.toml
        │   └── handler.rs
        └── ...
```

**skill.toml 示例**（MAPI 类）：

```toml
[skill]
id = "bili.campaign.create"
name = "创建B站广告计划"
category = "mapi"
platform = "bilibili"
version = "1.0.0"

[skill.description]
short = "在指定B站广告账户下创建新的广告计划"
long = """
此 Skill 调用 Bilibili Marketing API 创建广告计划。
支持设置计划名称、预算、出价策略、投放时段等。
"""

[skill.risk]
level = "high"           # low / medium / high / critical
requires_dry_run = true
requires_test_run = true
requires_user_confirm = true
is_reversible = false

[skill.input]
required = ["account_id", "campaign_name", "budget", "bid_type"]
optional = ["schedule", "targeting", "delivery_speed"]

[skill.api]
method = "POST"
endpoint = "/v2/campaign/create"
rate_limit = "10/min"

[skill.validation]
budget_min = 100
budget_max = 999999
campaign_name_max_length = 50

[skill.llm_tool_definition]
# 此部分会在 Agent Runtime 组装上下文时，
# 转换为 LLM tool_use 格式注入 system prompt
```

---

## 6. 服务端架构详细设计

### 6.1 模块划分

```
admiral-server/
├── Cargo.toml
├── config/
│   ├── default.toml
│   ├── development.toml
│   └── production.toml
├── migrations/                    # SeaORM 数据库迁移
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── config.rs                  # 配置加载
│   ├── error.rs                   # 统一错误类型
│   │
│   ├── api/                       # HTTP API 层 (Axum handlers)
│   │   ├── mod.rs
│   │   ├── router.rs              # 路由注册
│   │   ├── auth.rs                # 认证相关接口
│   │   ├── oauth.rs               # B站 OAuth 接口
│   │   ├── mapi.rs                # MAPI 代理接口
│   │   ├── llm.rs                 # LLM 代理接口
│   │   ├── user.rs                # 用户管理接口
│   │   ├── task.rs                # 任务元数据接口
│   │   └── admin.rs               # 管理后台接口
│   │
│   ├── ws/                        # WebSocket 网关
│   │   ├── mod.rs
│   │   ├── gateway.rs             # 连接管理 + 消息路由
│   │   ├── handler.rs             # 消息处理器
│   │   └── protocol.rs            # 消息协议定义
│   │
│   ├── service/                   # 业务逻辑层
│   │   ├── mod.rs
│   │   ├── auth_service.rs        # 认证逻辑
│   │   ├── user_service.rs        # 用户管理
│   │   ├── device_service.rs      # 设备管理
│   │   ├── entitlement_service.rs # 权益/试用期管理
│   │   ├── oauth_service.rs       # OAuth 流程
│   │   ├── token_vault.rs         # Token 加密存储
│   │   ├── mapi_proxy.rs          # MAPI 代理逻辑
│   │   ├── llm_router.rs          # LLM 调用路由
│   │   └── log_service.rs         # 日志聚合
│   │
│   ├── domain/                    # 领域模型
│   │   ├── mod.rs
│   │   ├── user.rs
│   │   ├── device.rs
│   │   ├── entitlement.rs
│   │   ├── invite.rs
│   │   ├── oauth_token.rs
│   │   └── task_meta.rs
│   │
│   ├── infra/                     # 基础设施层
│   │   ├── mod.rs
│   │   ├── db.rs                  # 数据库连接池
│   │   ├── cache.rs               # 进程内缓存 (moka + DashMap)
│   │   ├── crypto.rs              # 加密工具
│   │   └── http_client.rs         # 外部 HTTP 调用
│   │
│   ├── middleware/                 # Axum 中间件
│   │   ├── mod.rs
│   │   ├── auth_guard.rs          # JWT 认证中间件
│   │   ├── rate_limiter.rs        # 限流中间件
│   │   ├── request_id.rs          # 请求 ID 注入
│   │   └── logging.rs             # 请求日志
│   │
│   └── traits/                    # 扩展接口（未来插件化）
│       ├── mod.rs
│       ├── auth_provider.rs       # 认证提供者 trait
│       ├── payment_provider.rs    # 支付提供者 trait
│       ├── notification_channel.rs # 通知渠道 trait
│       ├── media_platform.rs      # 媒体平台 trait
│       └── client_channel.rs      # 客户端渠道 trait
```

### 6.2 关键 Trait 定义（扩展点）

```rust
/// 认证提供者 — MVP 实现邮箱密码，未来扩展微信/手机号
#[async_trait]
pub trait AuthProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    async fn authenticate(&self, credential: AuthCredential) -> Result<AuthResult>;
    async fn register(&self, info: RegistrationInfo) -> Result<UserId>;
}

/// 媒体平台 — MVP 实现 Bilibili，未来扩展巨量/腾讯/快手
#[async_trait]
pub trait MediaPlatform: Send + Sync {
    fn platform_id(&self) -> &str;
    fn platform_name(&self) -> &str;
    async fn init_oauth(&self, user_id: UserId) -> Result<OAuthUrl>;
    async fn handle_callback(&self, code: &str) -> Result<OAuthToken>;
    async fn refresh_token(&self, token: &OAuthToken) -> Result<OAuthToken>;
    async fn execute_api(&self, req: ApiRequest) -> Result<ApiResponse>;
}

/// 通知渠道 — 未来扩展微信服务号、邮件等
#[async_trait]
pub trait NotificationChannel: Send + Sync {
    fn channel_id(&self) -> &str;
    async fn send(&self, user_id: UserId, message: Notification) -> Result<()>;
}

/// 支付提供者 — 未来扩展微信支付/支付宝
#[async_trait]
pub trait PaymentProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    async fn create_order(&self, plan: SubscriptionPlan) -> Result<PaymentOrder>;
    async fn verify_payment(&self, order_id: &str) -> Result<PaymentStatus>;
}

/// 客户端渠道 — 桌面端/未来小程序
#[async_trait]
pub trait ClientChannel: Send + Sync {
    fn channel_id(&self) -> &str;
    async fn push_event(&self, user_id: UserId, event: ClientEvent) -> Result<()>;
}
```

### 6.3 WebSocket Gateway 设计

Gateway 是系统实时通信的核心枢纽，参考 OpenClaw 的 hub-and-spoke 模式：

**消息协议** (JSON over WebSocket)：

```json
{
    "type": "event_type",
    "request_id": "uuid",
    "timestamp": "ISO8601",
    "payload": { ... }
}
```

**事件类型**：

| 方向 | 事件类型 | 说明 |
|------|---------|------|
| S→C | `auth.force_logout` | 被新设备踢下线 |
| S→C | `auth.license_expired` | 试用期/有效期到期 |
| S→C | `oauth.success` | B站授权成功通知 |
| S→C | `oauth.token_refreshed` | Token 刷新通知 |
| S→C | `task.status_changed` | 任务状态变更（服务端感知的） |
| C→S | `heartbeat.ping` | 心跳 |
| S→C | `heartbeat.pong` | 心跳响应 |
| C→S | `log.sync` | 日志上报 |
| S→C | `system.announcement` | 系统公告 |

**连接管理**：

```
客户端连接 → JWT 认证 → 设备注册/踢旧设备
     │
     ├─ 心跳（每 30s）
     │    └─ 超时（2 次未收到 = 60s）→ 标记离线
     │
     ├─ 断线重连（指数退避：1s, 2s, 4s, 8s, max 30s）
     │    └─ 重连成功 → 重新认证 → 增量同步
     │
     └─ 状态机：Connected → Authenticated → Online ↔ Offline → Disconnected
```

### 6.4 LLM Router 设计

```
Client Request
    │
    ▼
┌─────────────────────┐
│    LLM Router       │
│                     │
│  根据请求类型路由：  │
│  - planning → 强模型 │
│  - execution → 快模型│
│  - parsing → 快模型  │
│                     │
│  支持 fallback：     │
│  主模型失败 → 备用   │
└─────────────────────┘
    │
    ├──► Claude API (Anthropic)
    ├──► OpenAI API (备用)
    └──► (未来) 本地模型
```

**路由策略**：

```toml
[llm.routes]
# 任务规划、复杂意图理解 → 强模型
planning = { primary = "claude-sonnet", fallback = "gpt-4o" }
# 简单的参数提取、格式化 → 快速模型
extraction = { primary = "claude-haiku", fallback = "gpt-4o-mini" }
# 数据分析、报告生成 → 强模型
analysis = { primary = "claude-sonnet", fallback = "gpt-4o" }
```

---

## 7. 客户端架构详细设计

### 7.1 目录结构

```
admiral-client/
├── src-tauri/                     # Rust 后端
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/              # Tauri 2 权限声明
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── commands/              # Tauri IPC commands
│   │   │   ├── mod.rs
│   │   │   ├── auth.rs
│   │   │   ├── task.rs
│   │   │   ├── skill.rs
│   │   │   └── settings.rs
│   │   │
│   │   ├── agent/                 # Agent 运行时
│   │   │   ├── mod.rs
│   │   │   ├── runtime.rs         # 核心交互循环
│   │   │   ├── context.rs         # 上下文组装
│   │   │   ├── memory.rs          # 记忆存储
│   │   │   └── profile.rs         # Agent Profile 加载
│   │   │
│   │   ├── task/                  # 任务引擎
│   │   │   ├── mod.rs
│   │   │   ├── engine.rs          # 任务调度引擎
│   │   │   ├── state_machine.rs   # 状态机
│   │   │   ├── scheduler.rs       # Cron 调度器
│   │   │   └── executor.rs        # 执行器
│   │   │
│   │   ├── skill/                 # Skill 系统
│   │   │   ├── mod.rs
│   │   │   ├── registry.rs        # Skill 注册表
│   │   │   ├── loader.rs          # Skill 加载器
│   │   │   ├── validator.rs       # 输入校验
│   │   │   └── executor.rs        # Skill 执行器
│   │   │
│   │   ├── network/               # 网络层
│   │   │   ├── mod.rs
│   │   │   ├── ws_client.rs       # WebSocket 客户端
│   │   │   ├── api_client.rs      # REST API 客户端
│   │   │   └── connectivity.rs    # 网络状态监控
│   │   │
│   │   ├── db/                    # 本地数据库
│   │   │   ├── mod.rs
│   │   │   ├── migrations.rs
│   │   │   └── models.rs
│   │   │
│   │   └── utils/
│   │       ├── mod.rs
│   │       └── device.rs          # 设备指纹生成
│   │
│   └── skills/                    # Skill 定义文件
│       ├── semantic/
│       ├── tools/
│       └── mapi/
│           └── bilibili/
│
├── src/                           # 前端 (React + TypeScript)
│   ├── main.tsx
│   ├── App.tsx
│   ├── router.tsx
│   │
│   ├── components/                # UI 组件
│   │   ├── ui/                    # shadcn/ui 基础组件
│   │   ├── layout/
│   │   │   ├── AppShell.tsx       # 主布局壳
│   │   │   ├── Sidebar.tsx        # 左侧任务列表
│   │   │   └── Header.tsx
│   │   ├── task/
│   │   │   ├── TaskList.tsx       # 任务列表
│   │   │   ├── TaskDetail.tsx     # 任务详情
│   │   │   ├── TaskCreate.tsx     # 任务创建
│   │   │   └── TaskTimeline.tsx   # 执行时间线
│   │   ├── chat/
│   │   │   ├── ChatPanel.tsx      # Agent 对话面板
│   │   │   └── MessageBubble.tsx
│   │   ├── auth/
│   │   │   ├── LoginForm.tsx
│   │   │   └── InviteCodeInput.tsx
│   │   └── settings/
│   │       ├── PlatformAuth.tsx   # 平台授权管理
│   │       └── Preferences.tsx
│   │
│   ├── hooks/                     # React Hooks
│   │   ├── useWebSocket.ts
│   │   ├── useAgent.ts
│   │   ├── useTasks.ts
│   │   └── useAuth.ts
│   │
│   ├── stores/                    # Zustand stores
│   │   ├── authStore.ts
│   │   ├── taskStore.ts
│   │   ├── wsStore.ts
│   │   └── uiStore.ts
│   │
│   ├── lib/                       # 工具函数
│   │   ├── tauri.ts               # Tauri IPC 封装
│   │   └── types.ts               # 共享类型
│   │
│   └── styles/
│       └── globals.css
│
├── package.json
├── tsconfig.json
├── tailwind.config.ts
└── vite.config.ts
```

### 7.2 UI 布局设计

```
┌─────────────────────────────────────────────────────────┐
│  Header: [Admiral Logo]  [连接状态●]  [账户▾]  [⚙设置]  │
├───────────────┬─────────────────────────────────────────┤
│               │                                         │
│  任务列表      │          任务详情 / Agent 对话            │
│               │                                         │
│  [+ 新建任务]  │  ┌─────────────────────────────────┐   │
│               │  │  任务名称: 批量创建B站广告计划     │   │
│  ┌───────────┐│  │  状态: ● Running                 │   │
│  │📋 任务A    ││  │  进度: ████████░░ 80%            │   │
│  │  ● Running ││  ├─────────────────────────────────┤   │
│  └───────────┘│  │                                   │   │
│  ┌───────────┐│  │  [执行时间线]                     │   │
│  │📋 任务B    ││  │  14:01 - 开始执行                │   │
│  │  ✓ Done   ││  │  14:02 - 账户A 计划创建成功       │   │
│  └───────────┘│  │  14:03 - 账户B 计划创建中...      │   │
│  ┌───────────┐│  │                                   │   │
│  │📋 任务C    ││  ├─────────────────────────────────┤   │
│  │  ◷ Scheduled│  │                                   │   │
│  └───────────┘│  │  [Agent 对话区域]                  │   │
│               │  │  🤖: 我已识别到您要创建5个计划...   │   │
│  ─────────── │  │  👤: 确认，开始执行吧               │   │
│  平台连接     │  │  🤖: 正在执行dry-run检查...        │   │
│  ┌───────────┐│  │                                   │   │
│  │ Bilibili  ││  │  [输入框] ________________ [发送]  │   │
│  │  ✓ 已授权  ││  │                                   │   │
│  └───────────┘│  └─────────────────────────────────┘   │
│               │                                         │
├───────────────┴─────────────────────────────────────────┤
│  状态栏: [在线] [License: 剩余28天] [版本 v1.0.0]        │
└─────────────────────────────────────────────────────────┘
```

### 7.3 前端路由

| 路径 | 页面 | 说明 |
|------|------|------|
| `/login` | 登录页 | 邮箱+密码+邀请码 |
| `/setup` | 初始化向导 | 权限申请、平台授权 |
| `/` | 主页 | 任务列表 + 详情/对话 |
| `/settings` | 设置 | 平台管理、偏好设置 |
| `/settings/platforms` | 平台授权 | B站授权管理 |
| `/admin` | 管理（仅管理员） | 用户管理、邀请码管理 |

---

## 8. 认证与用户管理

### 8.1 MVP 认证流程（简化版）

由于微信扫码登录、手机号注册需要相关资质（微信开放平台、短信服务商），MVP 阶段采用以下简化方案：

**注册流程**：
1. 管理员在服务端后台创建邀请码（含试用天数配置）
2. 用户打开客户端 → 登录/注册页面
3. 首次用户选择「注册」→ 填写：邮箱、密码、姓名、公司名称、邀请码
4. 服务端校验邀请码有效性 → 创建用户 → 绑定权益（试用期）
5. 自动登录 → 返回 JWT (access_token + refresh_token)

**登录流程**：
1. 输入邮箱 + 密码
2. 服务端校验 → 返回 JWT
3. 客户端存储 token → 建立 WebSocket 连接

**Token 机制**：

| Token 类型 | 有效期 | 用途 |
|-----------|--------|------|
| access_token | 2 小时 | API 调用认证 |
| refresh_token | 30 天 | 刷新 access_token |
| device_token | 随 session | WebSocket 连接认证 |

**试用期/有效期管理（MVP 手动）**：
- 管理员通过 Admin API 或数据库直接操作
- `POST /admin/users/{id}/extend` — 延长有效期
- `POST /admin/invites` — 创建邀请码
- 客户端每 10 分钟校验一次 license 状态

### 8.2 未来认证扩展路径

```
MVP:       EmailPasswordProvider implements AuthProvider
Phase 2:   WeChatScanProvider   implements AuthProvider  (需要微信开放平台)
Phase 3:   PhoneSmsProvider     implements AuthProvider  (需要短信服务商)
```

---

## 9. 设备绑定与在线机制

### 9.1 设备标识

客户端首次启动时生成 `device_id`，基于以下信息的 SHA-256 哈希：
- 机器 hostname
- OS 类型 + 版本
- 随机 UUID（首次生成后持久化到本地）

### 9.2 设备状态机

```
未注册 ──login──► 已注册/在线 ◄──reconnect── 断线重连中
                    │     ▲                       ▲
                    │     │                       │
                 disconnect                    timeout
                    │     │                       │
                    ▼     │                       │
                 离线 (≤5min) ─────超时────► 强制离线 (禁止操作)
                    │
                 被踢下线 ◄── 新设备登录
```

### 9.3 踢设备流程

```
新设备 login →
  服务端检查 active_device →
    如已有设备在线:
      1. 通过 WebSocket 向旧设备发送 force_logout
      2. 断开旧设备 WebSocket 连接
      3. 更新 active_device 为新设备
      4. 新设备 login 成功
```

---

## 10. B站 OAuth 授权

### 10.1 授权流程

```
┌──────────┐     ┌──────────┐     ┌──────────┐
│  Client   │     │  Server   │     │ Bilibili  │
└────┬─────┘     └────┬─────┘     └────┬─────┘
     │                │                │
     │ POST /oauth/   │                │
     │ bili/initiate   │                │
     │───────────────►│                │
     │                │                │
     │  auth_url      │                │
     │◄───────────────│                │
     │                │                │
     │ 打开浏览器      │                │
     │ 用户授权 ──────────────────────►│
     │                │                │
     │                │  callback?code │
     │                │◄───────────────│
     │                │                │
     │                │ 换取 token     │
     │                │───────────────►│
     │                │                │
     │                │ token 响应     │
     │                │◄───────────────│
     │                │                │
     │                │ 加密存储 token  │
     │                │                │
     │ WS: oauth.     │                │
     │     success     │                │
     │◄───────────────│                │
     │                │                │
     │ GET /accounts  │                │
     │───────────────►│                │
     │                │ 注入 token 调用 │
     │                │───────────────►│
     │  账户列表      │                │
     │◄───────────────│◄───────────────│
```

### 10.2 Token 生命周期管理

- 服务端加密存储 Token（AES-256-GCM，密钥从环境变量加载）
- Token 过期前自动刷新（TTL 检测 + 提前刷新窗口）
- 刷新失败 → 通知客户端重新授权
- 用户可主动取消授权（清除 Token）

### 10.3 未来多平台扩展

每个平台实现 `MediaPlatform` trait：

```
MediaPlatform
├── BilibiliPlatform     (MVP)
├── OceanEnginePlatform  (未来 — 巨量引擎)
├── TencentAdsPlatform   (未来 — 腾讯广告)
└── KuaishouPlatform     (未来 — 快手)
```

---

## 11. MAPI Gateway

### 11.1 统一代理入口

```
POST /mapi/execute
Authorization: Bearer <access_token>

{
    "request_id": "uuid",
    "skill_id": "bili.campaign.create",
    "account_id": "123456",
    "platform": "bilibili",
    "payload": {
        "campaign_name": "测试计划",
        "budget": 1000,
        "bid_type": "cpc"
    },
    "dry_run": false
}
```

### 11.2 服务端处理流程

```
接收请求
  │
  ├─ 1. 认证校验 (JWT)
  ├─ 2. 权限校验 (用户是否有该账户的权限)
  ├─ 3. 限流检查 (per user + per account)
  ├─ 4. 从 Token Vault 获取并解密 access_token
  ├─ 5. 检查 token 是否需要刷新
  ├─ 6. 注入 token → 构造实际 API 请求
  ├─ 7. 发送请求到 Bilibili MAPI
  ├─ 8. 记录请求/响应日志 (脱敏)
  ├─ 9. 重试逻辑 (指数退避, 最多 3 次)
  └─ 10. 返回结果给客户端
```

### 11.3 限流策略

Admiral 的 MAPI 限流采用 **三层限流 + 接口级精细控制** 的方案，所有限流配置均在服务端 TOML 文件中统一管理，支持运行时热加载。

#### 11.3.1 限流层次模型

```
请求进入
  │
  ▼
Layer 1: 全局限流 (保护服务端整体吞吐)
  │
  ▼
Layer 2: 用户/账户级限流 (防止单用户/单账户滥用)
  │
  ▼
Layer 3: 接口级限流 (匹配 B站各接口的 QPS 限制)
  │
  ▼
放行 → MAPI 调用
```

#### 11.3.2 限流维度

| 层次 | 维度 | 默认值 | 说明 |
|------|------|--------|------|
| L1 全局 | 服务端总请求量 | 600 req/min | 所有用户所有接口合计，保护 B站 API 总限额 |
| L2 用户级 | 单用户全接口 | 60 req/min | 防止单用户占满带宽 |
| L2 账户级 | 单广告账户全接口 | 30 req/min | 保护单个广告账户 |
| L3 接口级 | 单接口 per 账户 | 按接口配置 | 匹配 B站各接口的实际 QPS/QPM 限制 |

#### 11.3.3 接口级限流配置（服务端热加载）

限流规则定义在服务端配置文件 `config/rate_limits.toml` 中，支持按平台、按接口粒度配置。服务端启动时加载，支持通过 Admin API 触发热重载（无需重启）。

```toml
# config/rate_limits.toml — B站 MAPI 接口级限流配置

[global]
max_requests_per_minute = 600

[defaults]
# 未单独配置的接口使用默认值
user_rpm = 60           # 单用户每分钟请求数
account_rpm = 30        # 单账户每分钟请求数
endpoint_rpm = 20       # 单接口 per 账户每分钟请求数
endpoint_qps = 5        # 单接口 per 账户每秒请求数

[platforms.bilibili]
# 按 B站实际接口限制，逐接口配置
# 格式: [platforms.bilibili.endpoints."接口路径"]

[platforms.bilibili.endpoints."/v2/campaign/list"]
endpoint_rpm = 60
endpoint_qps = 10
category = "read"
description = "查询计划列表"

[platforms.bilibili.endpoints."/v2/campaign/create"]
endpoint_rpm = 10
endpoint_qps = 2
category = "write"
description = "创建广告计划"
cooldown_ms = 500       # 写操作间强制冷却时间

[platforms.bilibili.endpoints."/v2/campaign/update"]
endpoint_rpm = 10
endpoint_qps = 2
category = "write"
description = "修改广告计划"
cooldown_ms = 500

[platforms.bilibili.endpoints."/v2/adgroup/list"]
endpoint_rpm = 60
endpoint_qps = 10
category = "read"
description = "查询单元列表"

[platforms.bilibili.endpoints."/v2/adgroup/create"]
endpoint_rpm = 10
endpoint_qps = 2
category = "write"
description = "创建广告单元"
cooldown_ms = 500

[platforms.bilibili.endpoints."/v2/creative/list"]
endpoint_rpm = 60
endpoint_qps = 10
category = "read"
description = "查询创意列表"

[platforms.bilibili.endpoints."/v2/creative/create"]
endpoint_rpm = 5
endpoint_qps = 1
category = "write"
description = "创建广告创意"
cooldown_ms = 1000      # 创意创建间隔更长

[platforms.bilibili.endpoints."/v2/report/query"]
endpoint_rpm = 30
endpoint_qps = 5
category = "read"
description = "查询数据报表"

[platforms.bilibili.endpoints."/v2/account/list"]
endpoint_rpm = 30
endpoint_qps = 5
category = "read"
description = "获取账户列表"

[platforms.bilibili.endpoints."/v2/account/balance"]
endpoint_rpm = 30
endpoint_qps = 5
category = "read"
description = "查询账户余额"
```

#### 11.3.4 限流实现方案

MVP 使用 Rust 进程内的 **令牌桶算法**（Token Bucket），通过 `governor` crate 实现：

```rust
/// 分层限流器，所有状态存储在进程内存中
pub struct RateLimiterStack {
    global: RateLimiter,                                    // L1
    per_user: DashMap<UserId, RateLimiter>,                 // L2
    per_account: DashMap<AccountId, RateLimiter>,           // L2
    per_endpoint: DashMap<(AccountId, String), RateLimiter>,// L3
    config: ArcSwap<RateLimitConfig>,                       // 热加载配置
}

impl RateLimiterStack {
    /// 按优先级依次检查各层限流
    pub async fn check(&self, req: &MapiRequest) -> Result<(), RateLimitError> {
        self.check_global()?;
        self.check_user(req.user_id)?;
        self.check_account(req.account_id)?;
        self.check_endpoint(req.account_id, &req.endpoint)?;
        self.check_cooldown(req.account_id, &req.endpoint).await?;
        Ok(())
    }

    /// 热加载配置（Admin API 触发）
    pub fn reload_config(&self, new_config: RateLimitConfig) {
        self.config.store(Arc::new(new_config));
        // 清空旧的 per-endpoint limiter，下次请求时按新配置重建
        self.per_endpoint.clear();
    }
}
```

#### 11.3.5 限流管理 Admin API

| Method | Path | 说明 |
|--------|------|------|
| GET | `/api/v1/admin/rate-limits` | 查看当前限流配置 |
| PUT | `/api/v1/admin/rate-limits` | 更新限流配置（热加载） |
| GET | `/api/v1/admin/rate-limits/stats` | 查看各维度的限流统计 |

#### 11.3.6 被限流时的客户端行为

- 服务端返回 `429 Too Many Requests`，响应头包含 `Retry-After`（秒）
- 客户端收到 429 后自动退避等待，不计入任务失败
- 任务引擎感知限流，自动调整批量操作的并发度和间隔

---

## 12. 任务系统

### 12.1 任务类型

| 类型 | 触发方式 | MVP 支持 |
|------|---------|---------|
| 手动任务 (Instant) | 用户手动触发 | ✅ |
| 定时任务 (Scheduled) | Cron 表达式 | ✅ (基础) |

两种类型可互转。定时任务在客户端本地由 Scheduler 管理（客户端需在线运行）。

### 12.2 任务状态机

```
                    ┌──────────────────────────────────────┐
                    │           任务生命周期                │
                    │                                      │
   创建 ──► Draft ──► DryRunning ──► DryRunPassed         │
                         │              │                  │
                     DryRunFailed       │                  │
                     (回到 Draft,       ▼                  │
                      补充信息)    TestRunReady             │
                                       │                  │
                                       ▼                  │
                                 TestRunRunning           │
                                   │        │             │
                            TestRunFailed  TestRunPassed   │
                            (回到 Draft)       │          │
                                               ▼          │
                                          Deployed        │
                                               │          │
                                 ┌─────────────┤          │
                                 ▼             ▼          │
                              Running       Scheduled     │
                              │    ▲          │           │
                              │    │      (cron触发)      │
                              │  Resume        │          │
                              │    │           ▼          │
                              ├─► Paused   Running        │
                              │                           │
                              ├──► Succeeded              │
                              └──► Failed                 │
                    └──────────────────────────────────────┘
```

### 12.3 任务数据模型

```typescript
interface Task {
    id: string;                    // UUID
    name: string;                  // 用户命名
    description: string;           // 自然语言描述
    type: "instant" | "scheduled";
    status: TaskStatus;
    
    // Agent 解析后的结构化信息
    plan: TaskPlan;                // 执行计划 (Skill 调用序列)
    
    // 关联信息
    platform: string;              // "bilibili"
    account_ids: string[];         // 关联广告账户
    
    // 调度信息
    cron_expression?: string;      // 定时任务
    next_run_at?: DateTime;
    
    // 执行历史
    runs: TaskRun[];
    
    // 元数据
    created_at: DateTime;
    updated_at: DateTime;
}

interface TaskPlan {
    steps: TaskStep[];             // 有序的执行步骤
    estimated_duration: number;    // 预估耗时（秒）
    risk_level: "low" | "medium" | "high" | "critical";
}

interface TaskStep {
    seq: number;
    skill_id: string;              // 调用的 Skill
    params: Record<string, any>;   // 参数
    depends_on: number[];          // 依赖的前置步骤
    retry_policy: RetryPolicy;
}

interface TaskRun {
    id: string;
    task_id: string;
    run_type: "dry_run" | "test_run" | "production";
    status: RunStatus;
    started_at: DateTime;
    finished_at?: DateTime;
    step_results: StepResult[];
    error?: string;
}
```

### 12.4 任务创建完整流程

```
用户点击「新建任务」
    │
    ▼
输入任务名称
    │
    ▼
选择任务类型（或用自然语言描述）
  ┌──────────────────────────────────────┐
  │ 预置模板:                            │
  │ • 监控和告警（数据监控）              │
  │ • 批量创建广告                       │
  │ • 批量调整出价/预算                   │
  │ • 数据报表导出                       │
  │ • 自定义（自然语言输入）              │
  └──────────────────────────────────────┘
    │
    ▼
Agent 交互循环（多轮对话）
  ┌──────────────────────────────────────┐
  │ 1. 意图识别 (LLM)                    │
  │ 2. 匹配可用 Skills                   │
  │ 3. 检查信息完整性                     │
  │    - 缺少信息 → 向用户提问            │
  │    - 有歧义 → 向用户澄清              │
  │ 4. 生成执行计划 (TaskPlan)            │
  │ 5. 标准化展示给用户确认               │
  └──────────────────────────────────────┘
    │
    ▼
Dry-Run（逻辑校验 + 风险提示）
    │
    ├─ 失败 → 回到 Agent 对话，补充/修正
    │
    ▼
Test-Run（真实调用，最小范围）
    │
    ├─ 失败 → 回到 Agent 对话
    │
    ▼
用户确认 → Deploy
    │
    ├─ 即时任务 → 立即执行
    └─ 定时任务 → 加入调度器
```

---

## 13. Skill 系统

### 13.1 Skill 分类

| 分类 | 说明 | 执行位置 | 示例 |
|------|------|---------|------|
| semantic | 语义理解 | 本地 (LLM) | 意图解析、实体提取 |
| tools | 通用工具 | 本地 | Excel 导出、邮件发送、数据计算 |
| mapi | 广告 API | 远程 (经服务端) | B站 MAPI 全部接口 |

### 13.2 Skill 注册与加载

参考 OpenClaw 的三级加载顺序：

1. **内置 Skills** — 随客户端打包发布（`src-tauri/skills/`）
2. **用户 Skills** — 用户自定义（`~/.admiral/skills/`）— 未来支持
3. **远程 Skills** — 从服务端下载（SkillHub）— 未来支持

加载流程：

```
启动时:
  1. 扫描 skills/ 目录
  2. 解析每个 skill.toml
  3. 校验依赖和前置条件
  4. 注册到 SkillRegistry
  5. 生成 LLM tool_use 定义列表

运行时:
  Agent Runtime 向 LLM 发送当前可用的 tool 列表
  LLM 选择 tool → Skill Executor 执行对应 Skill
```

### 13.3 MVP 首批 Skill 清单

| Skill ID | 名称 | 分类 | 优先级 |
|----------|------|------|--------|
| `semantic.intent_parse` | 意图解析 | semantic | P0 |
| `semantic.entity_extract` | 实体提取 | semantic | P0 |
| `tools.excel_export` | 导出 Excel | tools | P1 |
| `bili.account.list` | 获取账户列表 | mapi | P0 |
| `bili.account.balance` | 查询账户余额 | mapi | P0 |
| `bili.campaign.list` | 查询计划列表 | mapi | P0 |
| `bili.campaign.detail` | 查询计划详情 | mapi | P0 |
| `bili.campaign.create` | 创建计划 | mapi | P0 |
| `bili.campaign.update` | 修改计划 | mapi | P0 |
| `bili.adgroup.list` | 查询单元列表 | mapi | P0 |
| `bili.adgroup.create` | 创建单元 | mapi | P0 |
| `bili.adgroup.update` | 修改单元 | mapi | P0 |
| `bili.creative.list` | 查询创意列表 | mapi | P0 |
| `bili.creative.create` | 创建创意 | mapi | P0 |
| `bili.creative.update` | 修改创意 | mapi | P1 |
| `bili.report.query` | 查询数据报表 | mapi | P0 |

### 13.4 Skill 执行器

```rust
pub struct SkillExecutor {
    registry: SkillRegistry,
    mapi_client: MapiClient,      // 通过服务端代理
    local_runtime: LocalRuntime,   // 本地执行环境
}

impl SkillExecutor {
    pub async fn execute(&self, call: SkillCall) -> Result<SkillResult> {
        let skill = self.registry.get(&call.skill_id)?;
        
        // 输入校验
        skill.validate_input(&call.params)?;
        
        match skill.category {
            Category::Semantic => {
                // 纯 LLM 调用，无副作用
                self.execute_semantic(skill, call).await
            }
            Category::Tools => {
                // 本地工具执行
                self.local_runtime.execute(skill, call).await
            }
            Category::Mapi => {
                // 经服务端代理调用广告 API
                self.mapi_client.execute(skill, call).await
            }
        }
    }
}
```

---

## 14. dry-run 与 test-run

### 14.1 dry-run（逻辑校验）

**目的**：在不调用任何真实 API 的情况下，验证任务计划的可行性。

**校验项**：

| 校验项 | 说明 |
|--------|------|
| 参数完整性 | 所有必填参数已提供 |
| 参数合法性 | 值在合法范围内（如预算 ≥ 100） |
| Skill 可用性 | 所有需要的 Skill 均已注册 |
| 权限校验 | 用户对目标账户有操作权限 |
| 逻辑冲突 | 步骤间无逻辑矛盾 |
| 风险评估 | 标记高风险操作（批量删除等） |

**输出**：`DryRunReport`，包含所有校验结果和风险提示，呈现给用户确认。

### 14.2 test-run（最小范围真实执行）

**目的**：以最小范围真实调用 API，验证实际可行性。

**缩小范围策略**：

| 任务类型 | 缩小策略 |
|---------|---------|
| 批量创建 | 只创建 1 个对象（1 个计划/单元/创意） |
| 批量修改 | 只修改 1 个对象 |
| 数据查询 | 只查询 1 个账户 |
| 监控任务 | 执行 1 次完整检查循环 |

**test-run 后处理**：
- 成功：展示实际结果，用户确认后 Deploy
- 失败：展示错误信息，用户可修改任务参数后重试
- test-run 创建的广告对象会标记为「测试」，用户可选择保留或删除

---

## 15. 日志体系

### 15.1 日志层次

```
┌─────────────────────────────────────┐
│           日志架构                    │
│                                     │
│  Client (本地 SQLite)               │
│  ├─ 完整执行日志 (唯一真实源)         │
│  ├─ Agent 对话历史                   │
│  ├─ Skill 调用详情                   │
│  └─ 系统运行日志                     │
│           │                         │
│           │ 单向同步 (摘要)           │
│           ▼                         │
│  Server (PostgreSQL)                │
│  ├─ 日志摘要 (status, duration)     │
│  ├─ 错误日志 (排障用)               │
│  └─ 审计日志 (MAPI 调用记录)         │
└─────────────────────────────────────┘
```

### 15.2 日志记录规范

每条日志必须包含：

```json
{
    "log_id": "uuid",
    "request_id": "uuid",        // 贯穿整个请求链路
    "user_id": "uuid",
    "task_id": "uuid",
    "task_run_id": "uuid",
    "skill_id": "bili.campaign.create",
    "level": "info",
    "category": "mapi_call",
    "message": "创建计划成功",
    "details": { ... },          // 结构化数据
    "timestamp": "ISO8601",
    "duration_ms": 234
}
```

### 15.3 日志同步策略

- 同步方向：Client → Server 单向
- 同步频率：每 5 分钟批量上报，或 WebSocket 实时推送摘要
- 同步内容：仅摘要级别（不含完整 payload，保护用户数据）
- 失败重试：3 次指数退避，仍失败则本地排队等待下次

---

## 16. 数据库设计

### 16.1 服务端 PostgreSQL

```sql
-- 用户表
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    name VARCHAR(100) NOT NULL,
    company VARCHAR(200),
    role VARCHAR(20) DEFAULT 'user',       -- 'user' | 'admin'
    status VARCHAR(20) DEFAULT 'active',   -- 'active' | 'suspended' | 'expired'
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 设备表
CREATE TABLE devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    device_id VARCHAR(64) NOT NULL,        -- 客户端生成的设备指纹
    device_name VARCHAR(100),
    os_type VARCHAR(20),
    is_active BOOLEAN DEFAULT FALSE,
    last_seen_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id, device_id)
);

-- 邀请码表
CREATE TABLE invites (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(20) UNIQUE NOT NULL,
    trial_days INTEGER NOT NULL DEFAULT 7,
    max_uses INTEGER DEFAULT 1,
    used_count INTEGER DEFAULT 0,
    created_by UUID REFERENCES users(id),
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 权益表 (用户的使用期限)
CREATE TABLE entitlements (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    invite_id UUID REFERENCES invites(id),
    type VARCHAR(20) DEFAULT 'trial',      -- 'trial' | 'paid' | 'manual'
    starts_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    extended_by UUID REFERENCES users(id), -- 管理员延期时记录
    extend_reason TEXT
);

-- B站 OAuth Token 表 (加密存储)
CREATE TABLE oauth_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    platform VARCHAR(50) NOT NULL DEFAULT 'bilibili',
    access_token_encrypted BYTEA NOT NULL,    -- AES-256-GCM 加密
    refresh_token_encrypted BYTEA NOT NULL,
    token_nonce BYTEA NOT NULL,               -- GCM nonce
    scopes TEXT,
    expires_at TIMESTAMPTZ NOT NULL,
    refresh_expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id, platform)
);

-- 广告账户绑定表
CREATE TABLE ad_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    platform VARCHAR(50) NOT NULL DEFAULT 'bilibili',
    account_id VARCHAR(100) NOT NULL,          -- 平台侧的账户ID
    account_name VARCHAR(200),
    status VARCHAR(20) DEFAULT 'active',
    metadata JSONB,                            -- 平台特定的扩展信息
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id, platform, account_id)
);

-- 任务元数据 (服务端只存摘要)
CREATE TABLE task_meta (
    id UUID PRIMARY KEY,                       -- 与客户端 task.id 一致
    user_id UUID NOT NULL REFERENCES users(id),
    name VARCHAR(200),
    status VARCHAR(30),
    platform VARCHAR(50),
    last_run_at TIMESTAMPTZ,
    last_run_status VARCHAR(30),
    synced_at TIMESTAMPTZ DEFAULT NOW()
);

-- 日志摘要表
CREATE TABLE log_summaries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    task_id UUID,
    task_run_id UUID,
    level VARCHAR(10),
    category VARCHAR(50),
    message TEXT,
    timestamp TIMESTAMPTZ,
    duration_ms INTEGER,
    synced_at TIMESTAMPTZ DEFAULT NOW()
);

-- MAPI 调用审计日志
CREATE TABLE mapi_audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    request_id UUID NOT NULL,
    platform VARCHAR(50),
    account_id VARCHAR(100),
    skill_id VARCHAR(100),
    endpoint VARCHAR(500),
    method VARCHAR(10),
    status_code INTEGER,
    duration_ms INTEGER,
    error_message TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 索引
CREATE INDEX idx_devices_user ON devices(user_id);
CREATE INDEX idx_entitlements_user ON entitlements(user_id, is_active);
CREATE INDEX idx_oauth_tokens_user ON oauth_tokens(user_id, platform);
CREATE INDEX idx_ad_accounts_user ON ad_accounts(user_id, platform);
CREATE INDEX idx_task_meta_user ON task_meta(user_id);
CREATE INDEX idx_log_summaries_user_task ON log_summaries(user_id, task_id);
CREATE INDEX idx_mapi_audit_user ON mapi_audit_logs(user_id, created_at);
```

### 16.2 客户端 SQLite

```sql
-- 任务表
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,                     -- UUID
    name TEXT NOT NULL,
    description TEXT,
    type TEXT NOT NULL DEFAULT 'instant',     -- 'instant' | 'scheduled'
    status TEXT NOT NULL DEFAULT 'draft',
    platform TEXT NOT NULL DEFAULT 'bilibili',
    account_ids TEXT,                         -- JSON array
    plan TEXT,                               -- JSON: TaskPlan
    cron_expression TEXT,
    next_run_at TEXT,                         -- ISO8601
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 任务执行记录
CREATE TABLE task_runs (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id),
    run_type TEXT NOT NULL,                   -- 'dry_run' | 'test_run' | 'production'
    status TEXT NOT NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    step_results TEXT,                        -- JSON array
    error TEXT,
    FOREIGN KEY (task_id) REFERENCES tasks(id)
);

-- 日志表 (完整日志)
CREATE TABLE logs (
    id TEXT PRIMARY KEY,
    request_id TEXT,
    task_id TEXT,
    task_run_id TEXT,
    skill_id TEXT,
    level TEXT NOT NULL,
    category TEXT,
    message TEXT NOT NULL,
    details TEXT,                             -- JSON
    timestamp TEXT NOT NULL,
    duration_ms INTEGER,
    synced INTEGER DEFAULT 0                  -- 是否已同步到服务端
);

-- Agent 会话记忆
CREATE TABLE memory (
    id TEXT PRIMARY KEY,
    key TEXT NOT NULL UNIQUE,                 -- 记忆键
    value TEXT NOT NULL,                      -- 记忆内容
    category TEXT,                            -- 'preference' | 'pattern' | 'fact'
    importance REAL DEFAULT 0.5,              -- 重要度 0~1
    last_accessed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Agent 对话历史
CREATE TABLE conversations (
    id TEXT PRIMARY KEY,
    task_id TEXT,                             -- 关联任务（可选）
    role TEXT NOT NULL,                       -- 'user' | 'assistant' | 'system' | 'tool'
    content TEXT NOT NULL,
    tool_calls TEXT,                          -- JSON: tool call 信息
    tool_result TEXT,                         -- JSON: tool 执行结果
    timestamp TEXT NOT NULL
);

-- 配置表
CREATE TABLE config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 索引
CREATE INDEX idx_task_runs_task ON task_runs(task_id);
CREATE INDEX idx_logs_task ON logs(task_id, timestamp);
CREATE INDEX idx_logs_synced ON logs(synced, timestamp);
CREATE INDEX idx_conversations_task ON conversations(task_id, timestamp);
CREATE INDEX idx_memory_category ON memory(category);
```

---

## 17. 接口设计总览

### 17.1 REST API

#### 认证相关

| Method | Path | 说明 | 认证 |
|--------|------|------|------|
| POST | `/api/v1/auth/register` | 注册（邮箱+密码+邀请码） | 否 |
| POST | `/api/v1/auth/login` | 登录 | 否 |
| POST | `/api/v1/auth/refresh` | 刷新 Token | refresh_token |
| POST | `/api/v1/auth/logout` | 登出 | JWT |
| GET | `/api/v1/auth/me` | 获取当前用户信息 | JWT |
| GET | `/api/v1/auth/license` | 检查 license 状态 | JWT |

#### OAuth 相关

| Method | Path | 说明 | 认证 |
|--------|------|------|------|
| POST | `/api/v1/oauth/{platform}/initiate` | 发起 OAuth 授权 | JWT |
| GET | `/api/v1/oauth/{platform}/callback` | OAuth 回调（B站调用） | 否 |
| GET | `/api/v1/oauth/{platform}/status` | 查询授权状态 | JWT |
| DELETE | `/api/v1/oauth/{platform}` | 取消授权 | JWT |

#### MAPI 代理

| Method | Path | 说明 | 认证 |
|--------|------|------|------|
| POST | `/api/v1/mapi/execute` | 统一 MAPI 调用入口 | JWT |

#### LLM 代理

| Method | Path | 说明 | 认证 |
|--------|------|------|------|
| POST | `/api/v1/llm/chat` | LLM 对话（SSE 流式） | JWT |

#### 日志同步

| Method | Path | 说明 | 认证 |
|--------|------|------|------|
| POST | `/api/v1/logs/sync` | 批量上报日志摘要 | JWT |

#### 任务同步

| Method | Path | 说明 | 认证 |
|--------|------|------|------|
| POST | `/api/v1/tasks/sync` | 同步任务元数据 | JWT |

#### 管理后台（Admin）

| Method | Path | 说明 | 认证 |
|--------|------|------|------|
| GET | `/api/v1/admin/users` | 用户列表 | JWT (admin) |
| POST | `/api/v1/admin/users` | 创建用户 | JWT (admin) |
| PUT | `/api/v1/admin/users/{id}` | 更新用户 | JWT (admin) |
| POST | `/api/v1/admin/users/{id}/extend` | 延长有效期 | JWT (admin) |
| POST | `/api/v1/admin/invites` | 创建邀请码 | JWT (admin) |
| GET | `/api/v1/admin/invites` | 邀请码列表 | JWT (admin) |

### 17.2 WebSocket 协议

连接地址：`wss://api.admiral.app/ws?token=<access_token>`

消息格式：

```json
{
    "type": "string",
    "request_id": "uuid",
    "timestamp": "ISO8601",
    "payload": {}
}
```

详细事件类型见 [6.3 节](#63-websocket-gateway-设计)。

---

## 18. 未来扩展预留

### 18.1 多媒体平台接入

**架构预留**：`MediaPlatform` trait 已抽象。新增平台只需：
1. 实现 `MediaPlatform` trait
2. 在 `skills/mapi/` 下新增对应平台的 Skill 目录
3. 在服务端注册平台 OAuth 配置

**数据模型预留**：`oauth_tokens` 和 `ad_accounts` 表均有 `platform` 字段，天然支持多平台。

### 18.2 微信服务号消息推送

**架构预留**：`NotificationChannel` trait。

**未来实现路径**：
1. 服务端新增微信服务号模块，处理消息推送
2. 用户在客户端扫码关注服务号并绑定账号
3. 关键事件（任务完成、失败、告警）通过服务号推送模板消息
4. 推送内容类型：
   - 任务执行完成通知
   - 任务失败告警
   - 账户余额不足预警
   - license 到期提醒

### 18.3 微信小程序

**架构预留**：`ClientChannel` trait + 服务端 REST API 已设计为平台无关。

**未来实现路径**：
1. 小程序复用服务端 REST API（`/api/v1/tasks/sync` 等）
2. 主要功能：
   - 任务列表查看
   - 任务执行状态实时刷新
   - 简单的任务控制（暂停/恢复/取消）
   - 个人信息管理
3. 不包含任务创建（创建过程需要 Agent 交互，体验在桌面端更好）

### 18.4 开源/免费版

**架构预留**：客户端 Agent Runtime 设计为可独立运行。

**未来实现路径**：
1. LLM 调用支持本地配置（API key + endpoint）
2. OAuth 授权支持本地回调模式
3. 去掉服务端强依赖，改为可选连接

---

## 19. 安全设计

### 19.1 传输安全

- 全站 HTTPS (TLS 1.3)
- WebSocket 走 WSS
- 证书自动续期 (Let's Encrypt)

### 19.2 认证安全

- 密码使用 Argon2id 哈希（memory=64MB, iterations=3, parallelism=4）
- JWT 签名使用 RS256（非对称加密）
- access_token 短有效期（2h），refresh_token 长有效期（30d）
- 登录失败限流：5 次/15 分钟，超出锁定 30 分钟

### 19.3 数据安全

- 广告平台 Token 使用 AES-256-GCM 加密存储
- 加密密钥从环境变量加载，不入库
- 日志中不记录完整 Token，仅记录前 8 位
- MAPI 请求/响应日志脱敏（去除敏感字段）

### 19.4 客户端安全

- Tauri 2 的 Capability 权限模型，最小化系统权限
- IPC 通信类型安全（Rust ↔ TypeScript 类型共享）
- 本地 SQLite 数据存放在用户专属目录
- 不在客户端保存任何广告平台 Token（广告平台 Token 仅存服务端）

### 19.5 客户端 JWT 存储安全方案

客户端需要持久化存储 JWT（access_token + refresh_token）以实现免登录体验。以下是方案评估和最终选型：

#### 方案对比

| 方案 | 安全性 | 持久性 | 跨平台 | 实现复杂度 | 评价 |
|------|--------|--------|--------|-----------|------|
| **A. OS 原生密钥链** | ★★★★★ | ✅ | macOS Keychain + Windows Credential Manager | 中 | 最安全，但跨平台 API 差异大 |
| **B. 加密文件存储** | ★★★★ | ✅ | ✅ 统一 | 低 | 安全性好，实现简单，推荐 |
| **C. 纯内存存储** | ★★★★★ | ❌ 重启丢失 | ✅ | 最低 | 每次启动需重新登录，体验差 |
| **D. 明文文件存储** | ★★ | ✅ | ✅ | 最低 | 不推荐，token 裸露 |
| **E. SQLite 明文字段** | ★★ | ✅ | ✅ | 低 | 不推荐，与 D 本质相同 |

#### 选定方案：B. 加密文件存储 + 设备绑定密钥

**理由**：在"平均线以上"的安全要求下，方案 B 在安全性、跨平台一致性、实现复杂度之间取得最佳平衡。方案 A（OS 密钥链）虽更安全，但 macOS Keychain 与 Windows Credential Manager 的 API 差异大，增加维护成本，且对 MVP 而言安全增益有限。

**具体实现**：

```
存储位置: {app_data_dir}/credentials.enc

加密方式:
  算法: AES-256-GCM
  密钥派生: HKDF-SHA256(device_secret, "admiral-credential-key")
  device_secret: 首次启动时生成的 32 字节随机值
                 存储在 {app_data_dir}/.device_secret（文件权限 600）

文件内容 (加密前):
  {
      "access_token": "eyJ...",
      "refresh_token": "eyJ...",
      "access_expires_at": "ISO8601",
      "refresh_expires_at": "ISO8601",
      "user_id": "uuid"
  }
```

**安全特性**：

| 特性 | 说明 |
|------|------|
| 静态加密 | 文件内容 AES-256-GCM 加密，无法直接读取 |
| 设备绑定 | 密钥由 device_secret 派生，拷贝文件到其他机器无法解密 |
| 完整性校验 | GCM 模式自带 AEAD，篡改即失效 |
| 文件权限 | Unix 系统设置 600，仅当前用户可读写 |
| 自动清理 | 登出时安全擦除文件内容（覆写后删除） |
| 内存保护 | Rust 侧使用 `zeroize` crate，token 离开作用域自动清零 |

**威胁模型与接受的风险**：

| 威胁 | 防护 | 残余风险 |
|------|------|---------|
| 磁盘被盗/读取 | AES-256-GCM 加密 | device_secret 同机存储，有物理访问权限的攻击者理论上可提取 |
| 内存 dump | zeroize 清零 | token 使用期间仍在内存中 |
| 恶意软件读取 | 文件权限 600 | 同用户权限的恶意软件可读取 |
| token 泄漏后果 | access_token 2h 过期 | 泄漏窗口有限；refresh_token 泄漏需配合 device_id |

**结论**：此方案安全性处于行业平均水平之上（高于明文存储、高于 localStorage、与 Electron safeStorage 同级），对于 B 端工具软件属于合理选型。未来如需提升，可在不改变接口的情况下切换到方案 A（OS 密钥链）。

### 19.6 API 安全

- 所有请求必须携带 `request_id` (UUID)
- 全局限流 + 用户级限流 + 账户级限流
- CORS 严格限制
- 输入校验在 handler 层统一处理

---

## 20. 开发阶段规划

### Phase 0: 项目骨架（1 周）
- [ ] 初始化 Rust workspace（server + client 共享类型）
- [ ] 搭建 Tauri 2 + React + Vite + shadcn/ui 项目
- [ ] 搭建 Axum 服务端骨架
- [ ] Docker Compose（PostgreSQL）
- [ ] 配置 CI/CD 基础
- [ ] 定义共享类型（`admiral-types` crate）

### Phase 1: 认证 + 用户管理 + WebSocket（2 周）
- [ ] 邮箱+密码注册/登录
- [ ] JWT 签发/校验/刷新
- [ ] 邀请码系统
- [ ] 权益/试用期管理
- [ ] Admin API（创建用户、邀请码、延期）
- [ ] WebSocket Gateway（连接、心跳、设备管理）
- [ ] 客户端登录 UI + 网络状态监控
- [ ] License 定时校验

### Phase 2: B站 OAuth + MAPI Gateway（2 周）
- [ ] B站 OAuth 流程（initiate → callback → token 存储）
- [ ] Token 加密存储 + 自动刷新
- [ ] MAPI 代理入口（统一调用 + token 注入）
- [ ] 限流中间件
- [ ] 客户端平台授权 UI
- [ ] 首个 MAPI Skill 实现（获取账户列表）

### Phase 3: Agent Runtime + Skill 系统（3 周）
- [ ] Agent Profile 加载
- [ ] Skill 注册表 + 加载器
- [ ] LLM Router（服务端代理 + 流式响应）
- [ ] Agent 交互循环（ReAct 模式）
- [ ] Context Assembler（上下文组装）
- [ ] 实现首批 MAPI Skills（P0 列表）
- [ ] 客户端 Agent 对话 UI

### Phase 4: 任务系统（2 周）
- [ ] 任务状态机实现
- [ ] 任务创建流程（模板 + 自定义）
- [ ] dry-run 引擎
- [ ] test-run 引擎
- [ ] 任务部署 + 执行
- [ ] 定时任务调度器
- [ ] 客户端任务列表 + 详情 UI

### Phase 5: 日志 + 打磨 + 闭环（1 周）
- [ ] 本地日志记录
- [ ] 日志同步到服务端
- [ ] 首个完整自动化任务闭环验证
- [ ] 错误处理完善
- [ ] UI 交互打磨
- [ ] 端到端测试

**预计总开发周期**：~10 周（1 人全职）

---

## 21. AI 编码规则

以下规则在开发过程中必须严格遵守：

1. **Token 安全**：所有广告平台 Token 仅在服务端存储，客户端禁止持有
2. **写操作保护**：所有写操作（创建/修改/删除广告对象）必须先 dry-run
3. **测试验证**：任务部署前必须通过 test-run
4. **日志完整**：所有操作必须在本地记录完整日志
5. **请求追踪**：所有请求必须携带 `request_id`，贯穿整个调用链
6. **类型安全**：Server ↔ Client 共享类型定义（`admiral-types` crate），前端通过 Tauri IPC 自动生成 TypeScript 类型
7. **错误处理**：所有 `unwrap()` 禁止出现在生产代码中，使用 `Result` + `?` 传播
8. **Trait 优先**：所有外部依赖通过 trait 抽象，便于测试和未来扩展
9. **配置外置**：所有环境相关配置通过配置文件/环境变量管理，不硬编码
10. **幂等设计**：所有 API 调用设计为幂等或安全可重试

---

## 附录 A：关键技术决策记录

| 决策 | 选择 | 理由 |
|------|------|------|
| 数据库 | PostgreSQL (非 MySQL) | JSONB、LISTEN/NOTIFY、扩展生态 |
| Web 框架 | Axum (非 Actix) | Tokio 原生、Tower 生态、代码更简洁 |
| 前端框架 | React (非 Vue/Svelte) | 生态最成熟、shadcn/ui 支持好 |
| 状态管理 | Zustand (非 Redux) | 更轻量、TS 友好、无 boilerplate |
| ORM | SeaORM (非 Diesel) | 异步原生、migration 好用 |
| Agent 模式 | ReAct (非 Plan-Execute) | 更适合交互式场景、可中途修正 |
| Skill 格式 | TOML (非 YAML/JSON) | Rust 生态标准、人类可读性好 |
| MVP 认证 | 邮箱密码 (非微信扫码) | 无需第三方资质，快速上线 |
| MVP 缓存 | moka + DashMap (非 Redis) | 单机部署无需外部依赖，未来可透明替换 |
| 客户端 JWT 存储 | 加密文件 (非 OS 密钥链) | 跨平台一致、实现简单、安全性高于平均水平 |

## 附录 B：环境变量清单

```bash
# 服务端
ADMIRAL_DATABASE_URL=postgres://user:pass@localhost:5432/admiral
ADMIRAL_JWT_PRIVATE_KEY_PATH=/path/to/private.pem
ADMIRAL_JWT_PUBLIC_KEY_PATH=/path/to/public.pem
ADMIRAL_TOKEN_ENCRYPTION_KEY=<32-byte-hex>    # AES-256 密钥
ADMIRAL_BILI_CLIENT_ID=<bilibili_client_id>
ADMIRAL_BILI_CLIENT_SECRET=<bilibili_client_secret>
ADMIRAL_BILI_CALLBACK_URL=https://api.admiral.app/api/v1/oauth/bilibili/callback
ADMIRAL_LLM_ANTHROPIC_KEY=<anthropic_api_key>
ADMIRAL_LLM_OPENAI_KEY=<openai_api_key>       # 备用
ADMIRAL_SERVER_HOST=0.0.0.0
ADMIRAL_SERVER_PORT=8080
ADMIRAL_LOG_LEVEL=info
```

## 附录 C：Cargo Workspace 结构

```
admiral/
├── Cargo.toml                 # workspace root
├── crates/
│   ├── admiral-types/         # 共享类型定义 (Server + Client 复用)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── user.rs
│   │       ├── task.rs
│   │       ├── skill.rs
│   │       ├── ws_protocol.rs
│   │       └── api.rs
│   │
│   ├── admiral-server/        # 服务端
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── (见 6.1 节)
│   │
│   └── admiral-client/        # 客户端 (Tauri)
│       ├── Cargo.toml
│       ├── tauri.conf.json
│       ├── src/               # Rust 后端
│       │   └── (见 7.1 节)
│       ├── skills/            # Skill 定义
│       └── ui/                # 前端代码
│           └── (见 7.1 节 src/)
│
├── docker-compose.yml
├── .env.example
└── README.md
```

---

> **Admiral v1.0 需求 + 架构文档 — 可进入开发**  
> 此文档为唯一权威开发依据
