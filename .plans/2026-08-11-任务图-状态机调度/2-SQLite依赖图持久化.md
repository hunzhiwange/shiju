# 任务 2: SQLite 依赖图持久化

批次：【批次 2】 依赖批次 1

状态：已完成，待回归验证

验证记录：已按计划两次运行 `cargo clippy -p vw-agent --all-targets --all-features -- -D warnings`。第一次失败验证确认本任务新增 `dependencies` 模块尚不存在；该命令随后也受到工作区无关改动阻断。实现后第二次运行，在编译本任务前因 `crates/vw-agent/src/session/instruction.rs` 引用了缺失的 `instruction_cache`、`instruction_include`、`instruction_rules` 嵌套模块而失败；未修改这些无关文件，且命令在输出错误后未自行退出，已中断。已运行 `git diff --check`（本任务已跟踪文件无空白错误），并静态审查依赖读写 SQL：所有查询、删除与插入均使用 `platform_id`、`company_id`、`project_id` 完整作用域；运行时断言待工作区恢复可编译后回归。

目的：在 SQLite 调度存储中持久化项目级任务依赖图，并原子验证和替换完整边集合。

来源任务：无

预计会话范围：只实现公共依赖存储接口与 SQLite 适配，不修改 PostgreSQL、API 或派发逻辑；围绕一个完整图替换和读取行为展开。

## 文件

- 新建：`crates/vw-agent/src/agent_scheduler/store/dependencies.rs`
- 新建：`crates/vw-agent/src/agent_scheduler/store/dependencies_tests.rs`
- 修改：`crates/vw-agent/src/agent_scheduler/store/mod.rs`
- 修改：`crates/vw-agent/src/agent_scheduler/store/schema.rs`
- 修改：`crates/vw-agent/src/agent_scheduler/store/schema_tests.rs`

## 上下文

- 新表 `scheduler_task_dependencies` 使用完整项目作用域、`predecessor_task_id`、`successor_task_id` 组成主键，并为前驱和后继查询建立索引。
- 两端都外键关联 `scheduler_tasks` 且删除级联；完整替换必须在 `TransactionBehavior::Immediate` 内完成。
- 仅当发生变化的后继仍为 `Draft` 或 `Queued` 时允许修改其入边。

## 测试计划

- 行为：SQLite 可隔离项目地替换和读取合法图，并拒绝缺失任务、跨作用域、环、非有限节点及运行中后继的入边变化。
- 失败验证测试：`crates/vw-agent/src/agent_scheduler/store/dependencies_tests.rs` 中新增 `agent_scheduler_store_replaces_scoped_task_graph_atomically`、`agent_scheduler_store_rejects_invalid_or_active_graph_mutation`。
- 失败验证命令：`cargo clippy -p vw-agent --all-targets --all-features -- -D warnings`
- 预期失败原因：测试引用尚不存在的表、模块和 Store 图操作，测试目标编译失败。
- 通过验证命令：`cargo clippy -p vw-agent --all-targets --all-features -- -D warnings`
- 模拟策略：使用测试项目和真实 SQLite 临时数据库，不模拟 SQL 行为。

## 步骤

1. 编写 schema、作用域隔离、原子替换和拒绝路径的失败验证测试。
2. 运行失败验证命令并确认依赖存储 API 缺失。
3. 添加表、外键、索引及 schema 清单断言。
4. 实现 `list_task_dependencies` 和 `replace_task_dependencies`，在事务内加载任务快照并调用任务图验证。
5. 对比原边集合，只拒绝发生入边变化且状态不允许修改的后继；失败时不得留下部分边。
6. 运行通过验证命令并审查 SQL 的完整作用域条件。

## 验证

- 运行：`cargo clippy -p vw-agent --all-targets --all-features -- -D warnings`
- 预期：退出码 0；schema 测试目标编译；所有依赖 SQL 均包含 platform/company/project 作用域。
- 所需证据：失败编译摘要、最终 clippy 退出码、表和两个索引的 schema 差异、事务失败不提交的测试断言审查。

## 完成

- SQLite Store 可原子读写规范化成功依赖图，不修改 `scheduler_tasks.dag_json`。
- 聚焦 clippy 通过；未获授权时报告依赖存储运行时断言待回归。
