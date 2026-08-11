# 任务图状态机调度第一版计划

目标：让 Agent Scheduler 依据有限任务的成功依赖图和现有状态机安全派发任务，并通过事务领取避免重复或越过依赖执行。

范围：覆盖顶层 `Finite` 任务的成功依赖边、SQLite/PostgreSQL 持久化、图快照 API、派生就绪原因、网关客户端、共享派发筛选，以及 `Queued → Assigned + Run` 的原子领取。继续使用现有显式 wake、Worker 选择、生命周期映射、Runner、Lease、Outbox 与恢复机制。

范围外：不迁移任务内部 `SubTask.batch/depends_on_batches`，不增加条件边、优先级排序、资源优化、自动终态唤醒、图形编辑 UI、旧 `dag_json` 迁移或两套 `AgentTaskStatus` 的统一重构。

假设：图节点均为同一项目作用域内已存在的 `Finite` 任务；`Queued` 表示已进入调度池但不等于已就绪；只有当前状态为 `Completed` 的前置任务满足第一版成功依赖，`Submitted`、`Failed`、`Cancelled`、`Archived` 均不满足；显式 wake 会在任务状态变化后再次触发调度。

## 设计决策

- 选择独立的 `scheduler_task_dependencies` 关系表；不复用单个任务的 `dag_json`，避免复制节点状态，也不把依赖隐藏在 `metadata_json`。
- 图成员关系与 `AgentTaskKind` 正交，但第一版显式拒绝非 `Finite` 节点；边只表达“前置任务成功完成后允许后继执行”。
- `Ready`、`WaitingDependencies`、`BlockedByDependency` 是实时派生结果，不写入任务状态，也不把临时等待改成持久 `Blocked`。
- `Failed` 可在重试回到 `Queued` 后重新参与计算；`Cancelled`、`Archived` 采取默认拒绝，不推断历史成功结果。
- 项目级 PUT 使用完整边集合原子替换；只允许修改 `Draft` 或 `Queued` 后继的入边，避免运行中任务改变执行前提。
- Graph 负责就绪性，现有状态机继续负责合法流转；领取事务负责最后一次依赖复查、无活跃 Run 检查、`Queued → Assigned` 和 Run 创建。

## 架构说明

- 新的纯领域模块计算图合法性和任务就绪原因；存储层负责作用域、外键与事务，调度引擎和本地 Runner 只消费同一判定入口。
- SQLite 使用 `IMMEDIATE` 事务；PostgreSQL 先锁定后继任务行，再检查依赖并创建 Run。图替换与领取按相同任务锁顺序串行化。
- GET 图快照返回边、任务当前状态、就绪原因及阻塞任务 ID，便于定位“Queued 但未执行”。
- 旧 `TeamDagStatus`/`ready_nodes` 和 `dag_json` 第一版不参与派发；README 必须写清边界，避免形成第二个调度权威。

## 开发策略

- 对会改变行为的代码，使用失败验证-通过验证-重构。
- 在实施步骤前规划失败验证命令和预期失败。
- 将实施步骤限制在失败测试所证明的行为范围内。
- 测试真实行为，不验证模拟对象或实现细节。
- 仓库规则禁止未获明确授权运行 `cargo test`；各任务先以测试代码导致的编译失败和聚焦 `cargo clippy` 作为红绿证据，运行时断言留待维护者授权回归。
- 最终必须运行 `cargo clippy --all-targets --all-features -- -D warnings`；不运行 `cargo fmt --all -- --check`、`cargo check --target wasm32-unknown-unknown`、`cargo test` 或 Makefile 命令。
