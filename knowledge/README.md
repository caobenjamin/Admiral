# Admiral 知识库目录

本目录用于服务端 RAG 检索。将 Markdown 文件放在此目录（可含子目录，如 `mapi/`），通过 **Admin API** 执行 ingest 后即可被「数据查询」等场景的对话检索使用。

## 已包含内容

- `mapi/`：B站 MAPI 文档与数据查询指南
  - 获取计划列表.md、获取单元列表.md、获取创意列表.md、报表接口.md
  - MAPI数据查询指南.md（汇总：今日消耗、分优化目标、分稿件消耗等）

## 如何 Ingest

1. 确保服务端可访问本目录（例如在仓库根目录启动服务时，默认 `./knowledge` 即本目录）。
2. 使用具备 admin 权限的账号，调用：
   ```http
   POST /api/v1/admin/knowledge/ingest
   Content-Type: application/json

   {}
   ```
   或指定目录：
   ```json
   { "dir": "/absolute/path/to/knowledge" }
   ```
3. 未传 `dir` 时，服务端使用环境变量 `ADMIRAL_KNOWLEDGE_DIR` 或默认 `./knowledge`。
4. Ingest 会递归扫描目录下所有 `.md` 文件，按 `##` 标题分块写入 `knowledge_chunks` 表，并覆盖同源文件的旧块。

## 验证

用户提问如「账户今天花了多少钱」「分稿件的消耗分布」时，客户端会调用 `POST /api/v1/knowledge/query`，服务端从 `knowledge_chunks` 做全文检索并返回 top_k 片段，供 Agent 上下文使用。
