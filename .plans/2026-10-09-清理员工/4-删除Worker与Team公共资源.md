# 任务 4: 删除 Worker 与 Team 公共资源

批次：【批次 4】 依赖批次 3

状态：未开始

目的：删除 Worker/Team 领域、存储、筛选、控制器、TeamDag、API 和公开 DTO，使本地执行器成为纯内部能力。

来源任务：无

预计会话范围：在本地执行不再依赖 Worker 且 Matrix 已删除后，完整移除 Worker/Team 产品面；不处理 Staff 或桌面员工界面。

## 文件

- 删除：`crates/vw-agent/src/agent_scheduler/store/workers.rs`
- 删除：`crates/vw-agent/src/agent_scheduler/store/workers_tests.rs`
- 删除：`crates/vw-agent/src/agent_scheduler/store/postgres/workers.rs`
- 删除：`crates/vw-agent/src/agent_scheduler/store/postgres/workers_tests.rs`
- 删除：`crates/vw-agent/src/agent_scheduler/store/teams.rs`
- 删除：`crates/vw-agent/src/agent_scheduler/store/teams_tests.rs`
- 删除：`crates/vw-agent/src/agent_scheduler/store/postgres/teams.rs`
- 删除：`crates/vw-agent/src/agent_scheduler/store/postgres/teams_tests.rs`
- 删除：`crates/vw-agent/src/agent_scheduler/scheduler/worker_selection.rs`
- 删除：`crates/vw-agent/src/agent_scheduler/scheduler/worker_selection_tests.rs`
- 删除：`crates/vw-agent/src/agent_scheduler/scheduler/team_dag.rs`
- 删除：`crates/vw-agent/src/agent_scheduler/scheduler/team_dag_tests.rs`
- 删除：`crates/vw-agent/src/agent_scheduler/adapters/controller.rs`
- 删除：`crates/vw-agent/src/agent_scheduler/adapters/controller_tests.rs`
- 修改：`crates/vw-agent/src/agent_scheduler/domain.rs`
- 修改：`crates/vw-agent/src/agent_scheduler/store/mod.rs`
- 修改：`crates/vw-agent/src/agent_scheduler/store/schema.rs`
- 修改：`crates/vw-agent/src/agent_scheduler/store/postgres/schema.rs`
- 修改：`crates/vw-agent/src/agent_scheduler/scheduler/mod.rs`
- 修改：`crates/vw-agent/src/gateway/api/handlers/agent_scheduler.rs`
- 修改：`crates/vw-api-types/src/agent_scheduler.rs`
- 修改：`crates/vw-gateway-client/src/client/agent_scheduler_api.rs`
- 测试：`crates/vw-agent/src/gateway/api/handlers/agent_scheduler_tests.rs`
- 测试：`crates/vw-api-types/src/agent_scheduler_tests.rs`

## 上下文

- Worker 残余表面包括状态、能力、控制器、选择评分、workers 表和网关 CRUD。
- Team 包括 teams 表、成员 DTO、TeamDag 任务类型和团队调度；普通 task graph 已能表达有限任务依赖。
- 不保留“默认 Worker”、固定 WorkerName、隐式 Team 或旧表兼容分支。
- 先用 RustCodeGraph 分析 Worker/Team 领域类型、存储方法、路由和客户端的调用影响；提交必须隔离任务开始前的工作区改动。

## 测试计划

- 行为：公开调度协议和存储不再包含 Worker/Team，项目唤醒仍能启动本地任务，普通依赖图保持可用。
- 失败验证测试：增加路由清单断言，要求 `/workers`、`/teams` 与 worker-events 路由不存在，并记录当前命中。
- 失败验证命令：`rg -n "AgentWorker|WorkerName|AgentTeam|TeamDag|list_workers|upsert_worker|list_teams|upsert_team|worker-events|worker_selection" crates/vw-agent/src/agent_scheduler crates/vw-agent/src/gateway/api/handlers/agent_scheduler.rs crates/vw-api-types/src/agent_scheduler.rs crates/vw-gateway-client/src`
- 预期失败原因：Worker/Team DTO、存储、选择器和路由仍存在。
- 通过验证命令：`cargo clippy -p vw-agent -p vw-api-types -p vw-gateway-client --all-targets --all-features -- -D warnings`
- 模拟策略：不模拟 Worker/Team；用真实本地执行器与任务图夹具证明删除后唤醒和依赖仍工作。

## 步骤

1. 按 `./skills/rustcodegraph/SKILL.md` 检查索引并分析相关类型、调用链和受影响测试。
2. 添加 Worker/Team 路由不存在且本地唤醒可用的验证。
3. 删除 Worker/Team 领域、选择器、控制器、TeamDag 和两种存储实现。
4. 删除 schema、DTO、网关路由、客户端方法和 worker lifecycle 事件。
5. 清理任务、Run、事件、日志中的剩余 Worker/Team 字段或措辞。
6. 保留普通任务依赖图并运行聚焦 clippy 与负向搜索。
7. 按 `./skills/git-commit/SKILL.md` 仅提交本任务代码、测试和删除内容。

## 验证

- 运行：`! rg -n "AgentWorker|WorkerName|AgentTeam|TeamDag|list_workers|upsert_worker|list_teams|upsert_team|worker-events|worker_selection" crates/vw-agent/src/agent_scheduler crates/vw-agent/src/gateway/api/handlers/agent_scheduler.rs crates/vw-api-types/src/agent_scheduler.rs crates/vw-gateway-client/src && cargo clippy -p vw-agent -p vw-api-types -p vw-gateway-client --all-targets --all-features -- -D warnings`
- 预期：Worker/Team 产品面无命中，clippy 退出码 0，本地项目唤醒和普通依赖图仍编译。
- 所需证据：负向搜索结果、删除文件清单、路由断言、clippy 退出码、RustCodeGraph 影响分析记录、提交短 hash 与剩余变更清单。

## 完成

完成后不得存在用户可见或可配置 Worker/Team；工作区 `slot_id` 是本地资源标识，不应改名为 Worker。
