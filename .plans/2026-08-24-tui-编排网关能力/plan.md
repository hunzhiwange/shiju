# TUI 编排网关能力补齐计划

目标：把 TUI 当前只读的 Workflow、Cron 与 Scheduler 面板升级为有归属校验、可观察、可安全控制的终端编排入口。

范围：Workflow 详情、流式运行、暂停事件与人工恢复；Cron 执行历史、启停和删除；Scheduler 任务详情、日志、产物与状态控制；更新能力矩阵。

范围外：Workflow CRUD、Cron 新建/任意编辑、Scheduler 任务图替换、任务创建、worker/team/staff 管理、worktree merge、远程 worker 事件与内部 outbox。

假设：当前 `/workflows`、`/schedule`、`/agents`、`/tasks` 已加载 canonical snapshot；Gateway client 已有相应查询与 mutation API；Workflow SSE 当前未透传 `WorkflowRunResponse.pause`，需先补协议事件。

## 设计决策

- 先从当前 Gateway 快照选择稳定 ID，再允许运行或控制；不接受未加载对象的自由构造 ID。
- Workflow 使用独立 active stream kind 和归属 ID，不伪装为普通聊天；暂停表单经 typed question overlay 恢复。
- Cron 首批只做历史、启停、删除；复杂创建/编辑需要独立表单设计，不用松散命令参数代替。
- Scheduler 只控制当前项目 snapshot 中、canonical 状态允许的任务；内部生产者和 Staff API 不进入 TUI。

## 架构说明

- 编排 DTO 在 `runtime/orchestration.rs` 及新增窄模块中转换，UI model 不直接依赖传输句柄。
- 复用 ActiveRunner 后台消息、generation、confirm overlay 和 transcript；单来源失败不能清空其它来源。
- 所有破坏性操作由 Gateway 成功响应后提交并刷新，不做乐观状态猜测。

## 开发策略

- 对会改变行为的代码，使用失败验证-通过验证-重构。
- 在实施步骤前规划失败验证命令和预期失败。
- 将实施步骤限制在失败测试所证明的行为范围内。
- 测试真实行为，不验证模拟对象或实现细节。
- 仓库禁止主动执行 `cargo test`；用全量 clippy 编译新增测试并在交接中注明未运行测试。
