# Admiral 现代交互改版方案 v2

> 日期：2026-03-01
> 范围：仅界面与交互重构，不改功能、不改接口、不改任务状态机
> 适用版本：`crates/admiral-client/ui`

## 1. 目标与边界

### 1.1 改版目标

1. 让用户在首屏 3 秒内识别当前任务状态、下一步动作、系统连接状态。
2. 将长对话与工具输出从“纯文本堆叠”升级为“摘要优先、细节按需展开”。
3. 在不改变任何业务流程的前提下，显著提升视觉层级、可读性和专业感。

### 1.2 不变边界

1. 路由与功能：`/`、`/task/new`、`/settings/*`、`/login` 行为保持。
2. 任务生命周期保持：`draft -> dry_run_passed -> test_run_passed -> deployed` 等状态语义不变。
3. Tauri 命令与数据流保持：`task_*`、`agent_chat`、`oauth_*`、`auth_*` 不变。
4. 后端 API 与数据库结构不调整。

## 2. 信息架构与布局

## 2.1 全局布局

延续现有结构：`Header + LeftTab + (TaskList | TaskDetail/Chat) + StatusBar`。

1. `Header`：品牌 + 连接状态 + 账号菜单 + 设置入口。
2. `LeftTab`：媒体账户 / 任务 / 能力栏（预留） + 折叠控制。
3. `TaskList`：搜索、筛选、任务项列表、快速新建。
4. `TaskDetail/Chat`：
- 任务总览卡（状态、类型、最近更新时间）
- 执行时间线
- 确认动作条（dry-run/test-run）
- Agent 对话流
- 固定底部输入区
5. `StatusBar`：连接、License、版本。

## 2.2 视觉层级

1. 背景层：浅色渐变与轻纹理，避免“纯白文档感”。
2. 容器层：卡片分三级（基础/抬升/浮层）。
3. 交互层：主色仅用于可操作元素与关键状态。
4. 状态层：成功/警告/危险严格语义化，不做装饰色。

## 3. 设计系统（Token）

## 3.1 字体

1. 中文：`"PingFang SC", "Noto Sans SC", sans-serif`
2. 英文数字：`"IBM Plex Sans", "Segoe UI", sans-serif`
3. 代码：`"JetBrains Mono", "SFMono-Regular", monospace`

## 3.2 色彩

1. 主色：`#0F6CBD`
2. 主色悬浮：`#0B5CA4`
3. 成功：`#0E9F6E`
4. 警告：`#D97706`
5. 危险：`#DC2626`
6. 背景基底：`#F3F7FB`
7. 卡片：`#FFFFFF`
8. 描边：`#D7E0EA`
9. 主文本：`#102A43`
10. 次文本：`#486581`

## 3.3 尺寸与圆角

1. 页面间距：`16/20/24`
2. 卡片圆角：`12`
3. 控件圆角：`10`
4. 标签圆角：`999`

## 3.4 动效参数

1. 过渡：`160ms/220ms`，曲线 `ease-out`
2. 页面进入：`translateY(6px) + opacity`
3. 列表渐进：`stagger 40ms`
4. 运行态节点：`pulse 1.8s`

## 4. 页面级改造规格

## 4.1 首页 `/`

### 4.1.1 任务列表栏

1. 顶部加入搜索框（本地过滤，不改后端）。
2. 每项展示：任务名、状态标签、上次更新时间、最近结果。
3. 选中项强调：色条 + 高亮底 + 阴影。

### 4.1.2 任务详情栏

1. 顶部“任务总览条”：任务名、状态胶囊、类型、更新时间。
2. 进度条保留，但加入运行条纹与状态文案。
3. 时间线改卡片化节点，空态语义化。
4. 确认动作改为“Action Bar”风格，固定视觉焦点。

### 4.1.3 对话区

1. 消息分层：`user`、`assistant`、`system`、`tool`。
2. `assistant` 长文默认折叠，支持“展开/收起”。
3. `tool` 默认摘要，支持展开查看原始结果。
4. 输入框支持快捷指令 chips（仅填充发送框，不改接口）。

## 4.2 Header

1. Logo 增强为品牌徽标块。
2. 连接状态改为 badge，颜色语义化。
3. 账号菜单与设置按钮统一 hover/focus 反馈。

## 4.3 LeftTab

1. 媒体账户项展示授权状态 badge。
2. 折叠态图标按钮统一尺寸与 tooltip。
3. 当前项使用左侧高亮条 + 背景 + 文字色三重反馈。

## 4.4 StatusBar

1. 背景改轻渐变，弱化分割线突兀感。
2. 在线状态与 License 信息卡片化。

## 5. 组件状态矩阵

1. Button：default/hover/active/disabled/focus-visible。
2. TaskItem：default/hover/selected/running/failed。
3. MessageBubble：user/assistant/system/tool + expanded/collapsed。
4. ConfirmActions：dry_run_passed/test_run_passed + disabled/loading。
5. ChatInput：idle/focus/typing/sending/disabled。

## 6. 可访问性与可用性

1. 所有关键按钮具备 `aria-label`。
2. 颜色对比满足 WCAG AA（文本对比 >= 4.5）。
3. `prefers-reduced-motion` 下禁用复杂动画。
4. 键盘行为保持：`Enter` 发送，`Shift+Enter` 换行。

## 7. 开发映射（本次代码改造）

1. 全局 Token 与视觉层：`ui/src/styles/globals.css`
2. 框架布局：`layout/AppShell.tsx`、`layout/Header.tsx`、`layout/LeftTab.tsx`、`layout/MainContent.tsx`、`layout/StatusBar.tsx`
3. 任务域：`task/TaskList.tsx`、`task/TaskDetail.tsx`、`task/TaskTimeline.tsx`、`task/ConfirmActions.tsx`
4. 对话域：`chat/MessageBubble.tsx`、`chat/ChatInput.tsx`
5. 首页组合：`pages/HomePage.tsx`

## 8. 验收标准

1. 功能回归：任务创建、选择、DryRun/TestRun/Deploy、Agent 对话、平台授权入口可用。
2. 视觉验收：首屏信息层级清晰，关键动作可见。
3. 性能验收：滚动与输入无明显卡顿，动画无眩晕。
4. 可维护性：减少内联样式，通用样式进入 `globals.css`。

