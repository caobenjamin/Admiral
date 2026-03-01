# 「按优化目标维度的花费」调用链说明

## 目标

用户说「按优化目标维度的花费」时：先确认账户 → 拉单元列表拿优化目标 → 再查报表/花费。全程不出现「account_id required for MAPI」、不重复确认、不卡住。

---

## 1. 聊天路径（主路径）

### 1.1 工具列表与路由

- LLM 可见工具来自 `skill_registry.build_llm_tools()`，其中账户列表工具名为 **`bili_account_list_local`**。
- 解析工具名时（`resolve_tool_name_to_skill_id`）：
  - `bili_account_list_local` 或 **`bili_account_list`** 都会解析到 **`bili.account.list`**，只走本地，不进入 MAPI。

### 1.2 执行分支

- 执行器 `execute()` 中：若 `skill.id == "bili.account.list"`，**一律**走 `execute_bili_account_list_local`（请求服务端 `GET /api/v1/oauth/bilibili/bindings`，即 `ad_account_bindings`），**不**走 `execute_mapi`。
- 因此不会出现「account_id required for MAPI」。

### 1.3 预期对话流

| 步骤 | LLM 行为 | 工具/执行 | 结果 |
|------|----------|-----------|------|
| 1 | 发现用户未给 account_id | 调用 `bili_account_list_local`（无参数） | 返回 `{ records: [{ account_id, account_name }], total }` |
| 2 | 0 条 → 提示「请先授权并同步账户」；1 条 → 用该 account_id；多条 → 让用户选 | - | 得到 account_id |
| 3 | 拉单元列表（拿优化目标等） | 调用 `bili_adgroup_list`，params 带 `account_id` | 单元列表（含单元维度信息） |
| 4 | 按单元/优化目标查花费 | 调用 `bili_report_query`，params 带 `account_id`、`dimensions`（如 unit_id）、`metrics`（含 charged_cost_milli）等 | 报表数据 |

### 1.4 Profile 约束

- 未提供 account_id 时：**必须先**调用 `bili_account_list_local`，不得先调用任何需要 account_id 的 MAPI 工具（报表、计划列表、单元列表等）。

---

## 2. 任务路径（带 plan 的执行）

### 2.1 account_id 来源

- 执行器对每一步：`account_id = step.params["account_id"].or(task.account_ids.first())`。
- 即：要么本步 params 里带 account_id，要么用任务级 `task.account_ids`（通常来自创建/编辑任务时用户选择或规划时写入）。

### 2.2 若计划里第一步是「获取账户列表」

- 第一步：`bili.account.list`，无 params → 走本地列表，返回绑定账户。
- **执行器增强**：若第一步是 `bili.account.list` 且成功且返回**恰好 1 条**记录，则将该 `account_id` 记为 `resolved_account_id`，后续步在 `step.params` 和 `task.account_ids` 都无 account_id 时，会使用该值。这样「先拉账户列表再拉单元/报表」的 plan 无需事先填 account_id 即可跑通。
- 若绑定账户为 0 条或多条，仍依赖计划/任务里事先带好 account_id（或由 UI/LLM 在规划时写入）。

---

## 3. 涉及 Skill 一览

| skill_id | 类型 | 说明 | account_id |
|----------|------|------|------------|
| `bili.account.list` | tools | 本地绑定账户列表（ad_account_bindings） | 不需要 |
| `bili.adgroup.list` | mapi | 单元列表（含单元维度，可含优化目标等信息） | 必填 |
| `bili.report.query` | mapi | 自定义报表（维度含 unit_id 等） | 必填 |

---

## 4. 小结

- **聊天路径**：通过「账户列表只走本地 + 工具名别名 + 执行器 bili.account.list 强制走本地」，已保证先拿账户再走 MAPI，不会因账户列表报 MAPI 错误或重复确认。
- **任务路径**：依赖计划/任务里事先带好 account_id（或后续增强「首步为账户列表时自动注入」），当前逻辑下不会因「账户列表」这一步误走 MAPI。
