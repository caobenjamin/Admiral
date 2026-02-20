# Admiral 客户端开发文档

> 基于 [5thdoc-2026Feb20](./5thdoc-2026Feb20.md) 样式与功能需求，对齐 [Admiral_v1_Architecture](./Admiral_v1_Architecture.md)，参考 [README](../README.md) 当前实现  
> 版本：v1.0  
> 日期：2026-02-20

---

## 1. 文档目标

本文档指导 Admiral 客户端的样式和功能重新设计，**忽略当前联调阶段的简陋 UI**，从零设计现代、专业的客户端界面。功能设计遵循架构文档，实现状态以 README 为准。

---

## 2. 设计风格与设计系统

### 2.1 整体风格

- **基调**：现代、专业，参考飞书客户端气质
- **推荐设计系统**：Element Plus（最新版）UI 设计风格；若有更合适方案可替代
- **气质**：简洁、高效、可信赖，面向 B 端广告优化师

### 2.2 色彩规范（建议）

| 用途 | 色值建议 | 说明 |
|------|----------|------|
| 主色 | `#409EFF` / blue-500 | 按钮、链接、选中态 |
| 成功 | `#67C23A` | 完成、已授权、在线 |
| 警告 | `#E6A23C` | 进行中、需注意 |
| 危险 | `#F56C6C` | 失败、错误、登出 |
| 文字主色 | `#303133` | 正文 |
| 文字次色 | `#606266` | 辅助说明 |
| 文字弱化 | `#909399` | 占位、禁用 |
| 边框 | `#DCDFE6` | 分割线、边框 |
| 背景 | `#F5F7FA` | 页面底色 |
| 卡片背景 | `#FFFFFF` | 内容区 |

### 2.3 圆角与阴影

- 卡片：`border-radius: 8px`，`box-shadow`  subtle
- 按钮：`border-radius: 6px`
- 输入框：`border-radius: 4px`
- 状态指示点：`border-radius: 50%`

### 2.4 字体

- 系统字体栈：`-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif`
- 标题：`font-weight: 600`
- 正文：`font-size: 14px`，行高 1.5

---

## 3. 布局结构

### 3.1 整体布局示意

```
┌──────────────────────────────────────────────────────────────────────────┐
│  Header: [Logo] [连接状态] [账户▾] [⚙设置]                                 │
├─────────┬────────────────────────────────────────────────────────────────┤
│         │                                                                │
│  左侧   │                     主操作区域                                   │
│  可伸缩  │                                                                │
│  Tab    │   ┌──────────────────────┬──────────────────────────────────┐  │
│         │   │    任务列表            │    任务详情 / 对话区               │  │
│         │   │                      │                                  │  │
│         │   │  [+ 新建任务]         │  展示区域                          │  │
│         │   │  ┌──────────────────┐│  - 执行过程、结果、确认交互         │  │
│         │   │  │ 任务 A  ● 运行中  ││                                  │  │
│         │   │  └──────────────────┘│  ───────────────────────────────  │  │
│         │   │  ┌──────────────────┐│  输入区域（飞书风格）               │  │
│         │   │  │ 任务 B  ✓ 已完成  ││  [________________________] [发送] │  │
│         │   │  └──────────────────┘│                                  │  │
│         │   │  …                   │                                  │  │
│         │   └──────────────────────┴──────────────────────────────────┘  │
│         │                                                                │
├─────────┴────────────────────────────────────────────────────────────────┤
│  状态栏: [在线] [License: 剩余28天] [版本 v1.0.0]                          │
└──────────────────────────────────────────────────────────────────────────┘
```

### 3.2 左侧可伸缩 Tab（主导航）

**位置**：主界面最左侧，支持折叠/展开。

**Tab 项**（自上而下）：

| Tab 项 | 图标/标识 | 说明 |
|--------|-----------|------|
| 媒体账户 | B 站图标 | 当前仅 Bilibili，展示授权状态；点击进入平台授权 |
| 任务 | 任务图标 | 默认选中，首个 Tab；对应主操作区的任务列表 |
| 能力栏 | 能力图标 | Skill 列表/快捷入口，预留 |
| 其他 | — | 预留：帮助、快捷键等 |

**交互**：
- 展开时：约 200–240px 宽，显示图标 + 文字
- 折叠时：约 56px 宽，仅显示图标
- 点击 Tab 切换主区展示内容

### 3.3 主操作区域（两栏）

主操作区分为 **任务列表** 和 **任务详情** 两栏。

#### 3.3.1 任务列表栏

- **宽度**：约 280–320px，可拖拽微调
- **内容**：
  - 顶部：「+ 新建任务」按钮
  - 任务卡片列表，每项展示：
    - 任务名称
    - 任务状态（带颜色点）
    - 上次执行结果（成功/失败/无）
  - 支持搜索/筛选（可后续迭代）

#### 3.3.2 任务详情栏

包含两部分：**展示区域** 和 **输入区域**。

**展示区域**：
- 任务名称、状态、进度条
- 执行时间线（各步骤及结果）
- Agent 执行过程、任务结果
- 需要用户确认的交互（dry-run 结果确认、test-run 结果确认等）

**输入区域**（飞书风格）：
- 多行文本输入框，支持 Enter 发送、Shift+Enter 换行
- 占位符示例：「输入指令或描述任务…」
- 发送按钮
- 可选：工具栏（@提及、插入模板等，后续扩展）

### 3.4 Header

- 左：Logo + 产品名 Admiral
- 中：连接状态（在线/离线/连接中）
- 右：账户下拉（邮箱/姓名）、设置入口

### 3.5 状态栏

- 左：连接状态、License 剩余天数
- 右：版本号

---

## 4. 页面与路由

| 路径 | 页面 | 说明 | 认证 |
|------|------|------|------|
| `/login` | 登录页 | 邮箱+密码，含邀请码（注册时） | 否 |
| `/setup` | 初始化向导 | 权限申请、平台授权（可选，可合并到 settings） | JWT |
| `/` | 主页 | 左侧 Tab + 任务列表 + 任务详情/对话 | JWT |
| `/task/new` | 新建任务 | 任务创建向导或内嵌在主页 | JWT |
| `/settings` | 设置 | 设置入口页 | JWT |
| `/settings/platforms` | 平台授权 | B 站授权管理 | JWT |
| `/settings/preferences` | 偏好设置 | 主题、语言等 | JWT |
| `/settings/mapi` | MAPI 调试 | 联调用，正式版可隐藏 | JWT |
| `/settings/llm` | LLM 调试 | 联调用，正式版可隐藏 | JWT |
| `/admin` | 管理 | 仅管理员：用户、邀请码 | JWT (admin) |

---

## 5. 组件设计

### 5.1 布局组件

| 组件 | 职责 | 当前状态 |
|------|------|----------|
| `AppShell` | 整体布局：Header + 主体 + StatusBar | 有，需重设计 |
| `Header` | Logo、连接状态、账户、设置 | 有，需样式升级 |
| `LeftTab` | 左侧可伸缩 Tab（媒体账户、任务、能力栏） | 需新建，替代/扩展 Sidebar |
| `StatusBar` | 底部状态栏 | 有，需完善 |
| `MainContent` | 主操作区两栏容器 | 需明确划分 |

### 5.2 任务相关组件

| 组件 | 职责 | 当前状态 |
|------|------|----------|
| `TaskList` | 任务列表栏，展示任务卡片 | 有（在 Sidebar 内），需拆分并重设计 |
| `TaskListItem` | 单任务项：名称、状态、上次结果 | 有 TaskItem，需增强 |
| `TaskDetail` | 任务详情：展示区域 | 有，需丰富内容 |
| `TaskTimeline` | 执行时间线 | 有，需接真实数据 |
| `TaskCreate` | 新建任务 | 有，需完善流程 |
| `TaskProgress` | 进度条、状态展示 | 可集成在 TaskDetail 内 |

### 5.3 对话与输入组件

| 组件 | 职责 | 当前状态 |
|------|------|----------|
| `ChatPanel` | Agent 对话面板 | 有，需飞书风格改造 |
| `MessageBubble` | 单条消息气泡 | 有，需样式升级 |
| `ChatInput` | 飞书风格输入框 | 需新建或改造 ChatPanel 内 input |
| `ConfirmActions` | dry-run / test-run 确认按钮组 | 需新建 |

### 5.4 认证与设置组件

| 组件 | 职责 | 当前状态 |
|------|------|----------|
| `LoginForm` | 登录/注册表单 | 有，需样式升级 |
| `InviteCodeInput` | 邀请码输入（注册时） | 有 |
| `PlatformAuth` | 平台授权（B 站） | 有，需样式升级 |
| `Preferences` | 偏好设置 | 有，待完善 |

---

## 6. 数据流与状态

### 6.1 状态管理（Zustand）

| Store | 用途 | 主要字段 |
|-------|------|----------|
| `authStore` | 认证 | user, tokens, isAuthenticated |
| `taskStore` | 任务 | tasks, selectedTaskId, fetchTasks, createTask |
| `uiStore` | UI 状态 | selectedTaskId, leftTabCollapsed, connectionStatus |
| `wsStore` | WebSocket | 连接、事件（Phase 5 接入） |

### 6.2 与 Tauri / 服务端交互

- 认证：`login`, `register`, `logout`, `me`, `license`
- OAuth：`oauth_initiate`, `oauth_status`, `oauth_accounts_list`, `oauth_bind_accounts`, `oauth_delete_session`
- MAPI：`mapi_proxy`
- LLM：`llm_chat`（流式待接入）
- 任务：待 Phase 5 实现（本地 SQLite + 服务端同步）

---

## 7. 功能设计（对齐架构文档）

### 7.1 任务列表展示

- **展示字段**：任务名称、状态、上次执行结果
- **状态映射**：draft, dry_running, dry_run_passed, test_run_ready, test_run_running, test_run_passed, deployed, running, scheduled, paused, succeeded, failed
- **上次执行结果**：成功 / 失败 / 暂无（可从 task_runs 取最近一次）

### 7.2 任务详情展示区域

- 任务名称、类型（instant / scheduled）
- 当前状态、进度条（运行中时）
- 执行时间线（TaskTimeline）：
  - 步骤序号、Skill 名称、参数摘要
  - 执行开始/结束时间
  - 成功/失败、错误信息
- Agent 回复内容（markdown 渲染）
- 用户确认区块：dry-run 通过后展示「开始 Test Run」「取消」；test-run 通过后展示「部署」「取消」

### 7.3 输入区域（飞书风格）

- 多行输入，自动增高（max 约 120px）
- Enter 发送，Shift+Enter 换行
- 占位符：「输入指令或描述任务…」
- 发送按钮，支持 loading 态
- 支持流式接收 Agent 回复（Phase 5）

### 7.4 左侧 Tab 内容

- **媒体账户**：B 站卡片，已授权 / 未授权，点击跳转 `/settings/platforms`
- **任务**：对应主区任务列表，为默认视图
- **能力栏**：Skill 快捷入口（Phase 5 后实现）

---

## 8. 当前实现与待实现对照

### 8.1 已实现（联调阶段）

| 模块 | 说明 |
|------|------|
| 认证 | 登录/注册、JWT、refresh、logout |
| 首页 | AppShell + Sidebar + Header + StatusBar，展示 me/license |
| 任务列表 | Sidebar 内 TaskItem 列表，占位数据 |
| 任务详情 | TaskDetail + TaskTimeline，占位 |
| 对话 | ChatPanel + MessageBubble，占位回复 |
| 平台授权 | PlatformAuth：发起授权、列表、绑定、取消 |
| 设置 | Settings 入口、MAPI 调试、LLM 调试 |
| 路由 | /login, /, /task/new, /settings/* |

### 8.2 待实现/待改造

| 模块 | 说明 |
|------|------|
| 设计系统 | 引入设计 token（色彩、圆角、阴影、字体） |
| 左侧 Tab | 新建 LeftTab，可伸缩，含媒体账户/任务/能力栏 |
| 任务列表栏 | 从 Sidebar 拆出，独立栏，展示上次执行结果 |
| 任务详情栏 | 丰富展示区域，接入 ConfirmActions |
| 输入框 | 飞书风格 ChatInput |
| WebSocket | 连接状态、事件订阅（Phase 5） |
| 任务 CRUD | 与本地 SQLite + 服务端同步（Phase 5） |
| Agent 流式 | 流式 LLM 响应展示（Phase 5） |

---

## 9. 开发任务清单

### 9.1 设计系统

- [ ] 定义 Tailwind / CSS 变量：主色、成功/警告/危险、文字层级、圆角、阴影
- [ ] 可选：引入 shadcn/ui 或 Element Plus React 组件（按需）
- [ ] 统一按钮、输入框、卡片样式

### 9.2 布局改造

- [ ] 新建 `LeftTab`：可伸缩、Tab 切换
- [ ] 改造 `AppShell`：LeftTab + 主操作两栏 + StatusBar
- [ ] 任务列表栏与任务详情栏明确分区，支持比例/拖拽

### 9.3 任务相关 UI

- [ ] `TaskList`：独立栏，卡片式任务项，含上次执行结果
- [ ] `TaskDetail`：展示区域布局，集成 `TaskTimeline`、确认按钮
- [ ] `TaskCreate`：新建任务流程 UI（可与 Agent 对话结合）

### 9.4 对话与输入

- [ ] `ChatInput`：多行、Enter 发送、Shift+Enter 换行
- [ ] `ChatPanel`：飞书风格整体样式
- [ ] `MessageBubble`：样式升级，支持 markdown（可选）
- [ ] `ConfirmActions`：dry-run / test-run 确认区块

### 9.5 通用组件

- [ ] `Header`：样式升级，连接状态、账户下拉
- [ ] `StatusBar`：补充 License、版本信息
- [ ] `LoginForm`、`PlatformAuth`：样式升级

### 9.6 后续（Phase 5）

- [ ] WebSocket 连接、心跳、事件处理
- [ ] 任务 CRUD 与状态同步
- [ ] Agent 流式回复
- [ ] 能力栏 Skill 展示

---

## 10. 技术栈与依赖

| 类别 | 技术 | 版本 |
|------|------|------|
| 框架 | Tauri | 2.x |
| 前端 | React | 19+ |
| 构建 | Vite | 6+ |
| 样式 | Tailwind CSS | 4.x |
| 状态 | Zustand | 5+ |
| 路由 | react-router-dom | 7+ |
| 通信 | @tauri-apps/api | 2.x |

**可选增强**：
- shadcn/ui 或 Radix UI：基础组件（Button、Input、Card、Dialog 等）
- react-markdown：Agent 消息 markdown 渲染

---

## 11. 附录：飞书风格输入框参考

飞书输入框特点：
- 多行文本框，随内容增高
- 顶部可有工具栏（表情、@、附件等）
- 底部发送按钮，或 Enter 发送
- 圆角、浅色边框，聚焦时边框高亮
- 占位符灰色、轻量

实现要点：
- `<textarea>` 替代 `<input type="text">`
- `onKeyDown`: Enter 且无 Shift → 发送；Shift+Enter → 换行
- `min-height` + `max-height`，`overflow-y: auto`
- 聚焦样式：`focus:ring-2 focus:ring-primary focus:border-primary`

---

> 本文档为 Admiral 客户端样式与功能开发的指导文档，与 5thdoc、架构文档、README 保持一致。
