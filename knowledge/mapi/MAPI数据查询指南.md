# MAPI 数据查询指南

本文说明如何用 B站 MAPI 完成常见数据查询场景（今日消耗、分优化目标消耗、分稿件消耗等）。

## 查今日/某日消耗

- 使用**自定义报表接口**（open_api/report/v3/custom/query）。
- 必填：account_id、start_time、end_time（格式 yyyy-MM-DD）。
- **一次只能查一天**：start_time 与 end_time 必须相同；查多日需分次调用。
- 常用指标：charged_cost_milli（花费，毫分，元 = 值/10^5）、show_count（展示量）、click_count（点击量）。
- dimensions 可填 date、date_time、account_id、campaign_id、unit_id、creative_id 等。
- filters 无过滤时传空数组 []；order_by 必传，为单个对象 { field, type }，field 须在 metrics 中，type 为 "1" 升序 "2" 降序。

## 分优化目标看消耗

- 优化目标在**单元**层级（单元接口返回的 sales_mode.cpa_target 等）。
- 先可用**获取单元列表**（open_api/cpc/v3/unit/list_units）拿到单元及优化目标信息。
- 再用**自定义报表**按 unit_id 维度查消耗：dimensions 含 unit_id，metrics 含 charged_cost_milli，即可按单元/优化目标汇总。

## 分稿件看消耗

- 使用**自定义报表**，dimensions 包含 creative_id，metrics 包含 charged_cost_milli。
- 需要创意名称等信息时，可先调**获取创意列表**（open_api/cpc/v3/creative/list_creatives），再与报表数据关联。

## 计划/单元/创意列表

- 获取计划列表：open_api/cpc/v3/campaign/list_campaigns（GET，account_id 必填）。
- 获取单元列表：open_api/cpc/v3/unit/list_units（GET，account_id 必填，可选 campaign_ids、unit_ids、unit_status）。
- 获取创意列表：open_api/cpc/v3/creative/list_creatives（GET，account_id 必填，可选 campaign_ids、unit_ids、creative_ids、creative_status）。

## 注意事项

- 报表接口仅返回有消耗的数据。
- 金额建议用 _milli（毫分）字段，精度更高。
