# 任务 3: SessionStore 原子提交

批次：【批次 2】 依赖批次 1

状态：未开始

目的：在 SQLite 与 PostgreSQL 中原子提交会话快照、步骤引用、turn 公共 raw 和 round 差量。

来源任务：无

预计会话范围：只修改 SessionStore 的表结构、事务写入与 raw 重建读取，不改处理器、Gateway 事件或桌面状态。

## 文件

- 修改：`crates/vw-agent/src/session/store.rs`
- 修改：`crates/vw-agent/src/session/store/postgres.rs`
- 测试：`crates/vw-agent/src/session/store_tests.rs`
- 测试：`crates/vw-agent/src/session/store/postgres_tests.rs`

## 上下文

- 新建 `session_llm_raw_turns`：每个 `(scope, session_id, turn_id)` 一条公共上下文。
- 新建 `session_llm_raw_rounds`：每个 `(scope, session_id, turn_id, step_index)` 一条输入差量和输出。
- 新增 `commit_session_ui_snapshot`，接收会话快照、待提交 raw 工件和预期完成步骤键；一个事务内替换 UI messages/steps 并 upsert turn/round，缺少对应 raw 时拒绝提交。
- `load_session_step_llm_raw` 改为使用 `turn_id + step_index` 重建完整旧展示形状；停止向旧表写新数据，不迁移旧表。

## 测试计划

- 行为：raw round 插入失败或预期步骤缺 raw 时，会话与 raw 均不产生部分更新；成功时二者同时可见。
- 失败验证测试：新增 `session_ui_commit_rolls_back_snapshot_when_raw_insert_fails`、`session_ui_commit_rejects_finished_step_without_raw`、`session_ui_commit_reconstructs_shared_turn_raw`。
- 失败验证命令：`cargo test -p vw-agent session_ui_commit --lib`
- 预期失败原因：当前 `save_session_ui_snapshot` 与 `save_session_step_llm_raw` 使用不同连接和事务。
- 通过验证命令：`cargo test -p vw-agent session_ui_commit --lib && cargo test -p vw-agent session_store --lib`
- 模拟策略：SQLite 使用真实事务和触发器制造 raw 插入失败；PostgreSQL 路径至少由真实 SQL 构造测试或现有后端测试覆盖，不伪造成功。

## 步骤

1. 先添加成功提交、缺 raw 拒绝和数据库中途失败回滚测试。
2. 运行失败验证命令并确认现有独立写入不能满足。
3. 添加两张规范化 raw 表、作用域外键和查询索引。
4. 实现 SQLite `Immediate` 事务和 PostgreSQL transaction 的同语义提交。
5. 实现按稳定 turn 键读取并调用任务 2 的重建器；列表索引也从新表生成。
6. 运行聚焦测试、相邻 SessionStore 测试与 clippy。

## 验证

- 运行：`cargo test -p vw-agent session_ui_commit --lib`
- 运行：`cargo test -p vw-agent session_store --lib`
- 运行：`cargo clippy --all-targets --all-features -- -D warnings`
- 预期：故障注入后旧会话保持不变且无孤立 raw；成功后会话和 raw 同时可读；两种后端编译语义一致。
- 所需证据：失败验证、回滚后的数据库断言、成功重建断言、测试与 clippy 退出码 0。

## 完成

确认两种数据库实现、原子提交 API、稳定键读取与回滚证据齐全后才可标记完成。
