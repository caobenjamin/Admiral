# Admiral v1.0 完整需求与工程实现文档（MVP可直接进入编码）

> 项目代号：Admiral  
> 类型：本地执行型广告自动化 Agent 控制系统  
> 用途：直接供 AI 编码与工程实现  
> 本文档为 v1 唯一权威需求来源

---

# 0. 项目目标（第一性原理）

Admiral 是一个服务于广告代理公司优化师的本地 Agent 控制系统。

核心目标：

让广告优化师可以通过自然语言 + 任务系统自动执行广告操作与监控，
并通过一个智能执行引擎统一调度。

Admiral 本质定位：

> 广告操作的智能指挥系统（Control System for AdOps）

---

# 1. 本期版本范围（MVP冻结）

## 本期只做

- 桌面客户端（macOS + Windows）
- 中央服务端
- 仅支持 Bilibili 广告平台
- 仅付费邀请码模式
- 单用户单设备
- 单执行节点
- 必须在线使用

## 本期不做

- 微信小程序
- 微信服务号
- 免费版
- 多设备同步
- 多广告平台
- 团队协作

---

# 2. 系统总体架构

Client(Admiral) <-> Server <-> Bilibili MAPI

Server职责：
- 登录认证
- 邀请码与使用期限
- OAuth2授权与Token管理
- MAPI代理调用
- LLM代理
- 日志存储
- WebSocket中继

Client职责：
- 任务创建与执行
- 本地日志与状态
- Skill执行引擎
- UI界面

---

# 3. 架构原则

## 单设备原则
同一账号只允许一个客户端在线，新设备登录自动踢旧设备。

## 单执行节点原则
所有任务仅在本地执行，服务端只做中继与API代理。

## 安全原则
客户端不得保存任何广告平台token。
所有MAPI调用必须经服务端代理。

## 在线强制原则
客户端断网>5分钟禁止执行任务。

---

# 4. 技术选型

客户端：
- Rust
- Tauri
- TypeScript
- SQLite

服务端：
- Rust
- MySQL
- WebSocket

---

# 5. 登录与邀请码

仅支持微信扫码登录。

首次登录必须输入邀请码。

邀请码绑定有效期：
3~30天可配置。

服务端可手动延长用户有效期。

客户端每10分钟校验license。

---

# 6. 设备绑定与在线机制

登录成功后生成device_id。

服务端记录active_device。

新设备登录踢旧设备。

客户端必须保持WebSocket连接。

断开>5分钟进入offline状态。

offline状态禁止任务执行。

---

# 7. B站OAuth授权

客户端请求授权链接：
POST /oauth/bili/link

浏览器完成授权。

服务端callback获取token并存储。

token仅存在服务端。

客户端通过 websocket 收到授权成功通知。

---

# 8. MAPI Gateway

统一调用入口：

POST /mapi/execute

body:
skill_id
account_id
payload

服务端负责：
- token注入
- 请求发送
- 重试
- 日志记录

客户端禁止直连广告API。

---

# 9. 任务系统

## 任务类型
- 手动任务
- 定时任务（MVP可延后）

## 状态机
Draft
DryRunReady
DryRunPassed
TestRunRunning
TestRunPassed
Deployed
Running
Paused
Succeeded
Failed

---

# 10. dry-run 与 test-run

dry-run：
仅做逻辑校验与风险提示。

test-run：
真实调用API但仅1个对象。

成功后才能deploy。

---

# 11. Skill系统

三类skill：

1. 语义解析
2. 通用工具
3. B站MAPI能力（核心）

统一调用：
POST /mapi/execute

---

# 12. 日志体系

本地日志为唯一真实源。

服务端仅保存摘要。

同步方向：
client -> server 单向。

---

# 13. 本地SQLite表

tasks
task_runs
logs
config

---

# 14. 服务端MySQL表

users
devices
invites
entitlements
bili_tokens
task_meta
log_summary

---

# 15. 开发顺序

Phase1
登录+邀请码+license+ws

Phase2
B站OAuth+token

Phase3
任务系统+skill

Phase4
日志同步

Phase5
首个自动化任务闭环

---

# 16. AI编码规则

1. 所有token仅服务端存储
2. 所有写操作必须test-run
3. 所有任务必须可dry-run
4. 所有日志必须本地完整
5. 所有请求必须带request_id

---

# Admiral v1.0 需求冻结
此文档作为唯一开发依据
