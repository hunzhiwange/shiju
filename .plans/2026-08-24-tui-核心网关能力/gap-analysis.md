# TUI 与 Gateway 能力缺口分析

## 审计口径

只有能从 `V3Action` 或 `commands.rs` 进入 `V3GatewayApp`/`ActiveRunner`，再调用真实 Gateway 并把结果投影回 transcript/overlay 的路径，才算活动接入。仅有 GatewayClient 方法、旧 `vw-tui` widget、view model、测试 helper 或 disabled selection 不算接入。

当前 GatewayClient 有 287 个公开异步方法；TUI 目录源码只引用约 40 个同名方法，其中还包括测试和未被命令调用的 helper。该数字说明接入面很窄，但不能直接用作功能覆盖率，因为大量接口属于管理、内部生产者或桌面宿主。

## 已形成活动闭环

- 聊天与可恢复 stream、思考/文本/tool 生命周期、取消、排队输入和 transcript。
- 权限请求、用户问题、会话 todo 的只读同步。
- 会话创建、恢复、重命名、fork、reset、compact、archive。
- 模型、推理强度、权限模式、ACP backend、delegate/full-access/subagent 会话选项。
- 文件/图片/粘贴/资源附件的提交边界。
- Skill/Plugin 目录、启停、删除和动态 `/skill:<id>` prompt 展开。
- MCP server 与 memory 的启动时只读目录。
- Workflow/Cron/Agent/Task 的启动时只读 snapshot；当前项目任务 cancel。
- `/plan <request>`、`/goal <objective>` 的 task-plan-run stream。

## 正在接入或仅有局部闭环

| 能力 | 当前真实状态 | 缺口 |
|---|---|---|
| `@` 文件搜索 | 当前工作树已有 Gateway 文件名搜索与迟到结果保护 | 尚需保留未提交改动并完成聚焦验证 |
| 工具文件 diff | 当前工作树已把 structured patch 投影到 V3 diff | 活动 `/diff` 仍是独立计划中的未开始任务 |
| 工作目录切换 | runtime 有项目解析和目录 rebind helper | 无 `/cd` 命令、无目标会话原子 bootstrap |
| Memory | 仅启动时 `memory_query_get(None,None)` | 无查询刷新、store、delete |
| MCP | 仅 `mcp_inspect` server 摘要 | 无 prompt/resource/tool 调用契约 |
| Workflow | 仅 application list | 无详情、运行、暂停数据透传和恢复 |
| Cron | 仅 jobs list | 无详情、runs、启停、删除、创建/编辑 |
| Scheduler | status/workers 只读，task 仅 cancel | 无 graph/log/artifact，其余状态控制与创建/管理 |
| Skill/Plugin | 目录、toggle、delete | 无详情、创建、安装/市场、升级/刷新 |
| 会话 todo | 自动读取并显示 | 无人工更新入口 |

## Gateway 已有契约但 TUI 完全没有活动调用链

### 适合优先接入终端

- Gateway 诊断：`doctor_get`；当前 `/status` 只是本地会话摘要。
- Hook 安全管理：catalog、audit、trust、revoke 全部未接入。
- 工作区只读浏览：目录列表、文件读取、内容搜索、Git 文件状态；当前只有在途的 `@` 文件名搜索。
- Workflow 详情/运行/恢复、Cron runs/启停/删除、Scheduler graph/log/artifact 与 canonical 控制。
- Memory 语义查询刷新、写入和删除。

### 需要独立产品面，不应直接塞入本轮命令

- Project/worktree/Git mutation：项目 CRUD、worktree 创建/删除/reset、commit/merge/任意 Git command。
- Knowledge：数据集、文档、索引、embedding、retrieve 全域。
- PTY：终端创建、resize、WebSocket 输入输出和销毁。
- Task lifecycle、Staff、worker/team、任务图替换、任务创建和内部调度事件。
- APIQL、全局/租户配置、Provider OAuth/认证、integration credential 写入。
- Skill market、Shiju account/app/skill、桌面 service/preferences/external app。
- 文件 write/move/copy/delete。聊天工具可在审批后改文件，不代表 TUI 应直接获得文件管理权限。

## Gateway 契约仍不足，TUI 不能伪接入

- Workflow resume 虽有 client 方法，但当前 Workflow SSE 丢弃 `WorkflowRunResponse.pause`，TUI 拿不到合法 form token；编排计划先补协议事件。
- MCP client 目前只有 inspect，没有稳定的 prompt/resource/tool 执行 API；不得把 `/mcp__...` 转成普通聊天文本冒充执行。
- 额外目录授权、剪贴板写入、语音和快捷键持久化缺少统一跨平台 Gateway 契约。
- Cron 创建/任意编辑参数复杂且包含投递、模型、full-access 等敏感字段，需要独立表单与权限设计。

## 休眠 UI 不是能力

`vw-tui` 中仍保留 Hooks、plugin marketplace、keymap、rate limit、usage、review、export、pets、connector 等旧 Codex 组件。活动 V3 边界只公开少量 `ChatWidget` 方法，这些模块没有 `V3Action -> CLI -> Gateway` 调用链，不能直接算 VibeWindow TUI 能力，也不应绕过 Gateway 重新启用旧 app-server 路径。

## 建议顺序

1. 先完成现有 `@` 搜索和 `/diff` 计划，稳定当前未提交基线。
2. 执行核心计划：`/cd`、`/doctor`、只读 `/files`、Hooks、Memory。
3. 执行编排计划：Workflow pause 契约、运行/恢复、Cron 历史/控制、Scheduler 详情/控制。
4. 对 Knowledge、PTY、Project/worktree/Git、全局配置与市场能力分别立项，不与聊天主链混合。
