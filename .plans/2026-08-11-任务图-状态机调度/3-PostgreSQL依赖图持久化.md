# 任务 3: PostgreSQL 依赖图持久化

批次：【批次 3】 依赖批次 2

状态：已完成，待回归验证

验证记录：已按计划两次运行 `cargo clippy -p vw-agent --all-targets --all-features -- -D warnings`；两次均在编译本任务前被工作区无关错误阻断：`crates/vw-agent/src/session/instruction.rs` 的 `instruction_cache`、`instruction_include`、`instruction_rules` 嵌套模块缺失，以及 `session/llm/mod.rs`、`skills/mod.rs` 未使用导入和 `agent/agent/core.rs` 的 `String`→`anyhow::Error` 转换错误。未修改这些无关文件。已通过本任务新增 PostgreSQL 文件的 `rustfmt --edition 2024 --check` 与相关文件的 `git diff --check`。依赖图运行时断言和聚焦 clippy 待工作区恢复可编译后回归。

目的：为 PostgreSQL 提供与 SQLite 一致的依赖图 schema、事务替换、锁顺序和作用域语义。

来源任务：无

预计会话范围：只补齐 PostgreSQL 存储适配和相邻测试，不改变公共模型、API 或派发器；重点验证两种数据库行为等价。

## 文件

- 新建：`crates/vw-agent/src/agent_scheduler/store/postgres/dependencies.rs`
- 新建：`crates/vw-agent/src/agent_scheduler/store/postgres/dependencies_tests.rs`
- 修改：`crates/vw-agent/src/agent_scheduler/store/postgres/mod.rs`
- 修改：`crates/vw-agent/src/agent_scheduler/store/postgres/schema.rs`
- 修改：`crates/vw-agent/src/agent_scheduler/store/postgres/schema_tests.rs`

## 上下文

- 复用任务 2 的公共 Store API 和领域验证，不复制图算法。
- PostgreSQL 图替换先按任务 ID 排序锁定受影响后继行，再比较入边并替换，避免与领取事务产生反向锁顺序。
- schema 必须保持完整租户/项目作用域和两端复合外键。

## 测试计划

- 行为：PostgreSQL schema 与 Store 路由实现和 SQLite 相同的图读取、替换、拒绝与回滚语义。
- 失败验证测试：`crates/vw-agent/src/agent_scheduler/store/postgres/dependencies_tests.rs` 中新增 `postgres_task_graph_matches_sqlite_replacement_contract`，并扩展 PostgreSQL schema 断言。
- 失败验证命令：`cargo clippy -p vw-agent --all-targets --all-features -- -D warnings`
- 预期失败原因：PostgreSQL Provider 尚无依赖图方法和表定义，测试目标编译失败。
- 通过验证命令：`cargo clippy -p vw-agent --all-targets --all-features -- -D warnings`
- 模拟策略：沿用现有 PostgreSQL 测试基础设施；无可用测试数据库时只编译真实 SQL 路径，不创建伪 Store。

## 步骤

1. 编写 PostgreSQL 方法存在性、schema 和契约测试。
2. 运行失败验证命令并确认 Provider 能力缺失。
3. 添加 PostgreSQL 表、索引和模块连线。
4. 在单事务中锁定任务、验证完整图、删除旧边并插入规范排序后的新边。
5. 保证任一验证或 SQL 错误回滚完整替换。
6. 运行通过验证命令并对照 SQLite API 与错误语义。

## 验证

- 运行：`cargo clippy -p vw-agent --all-targets --all-features -- -D warnings`
- 预期：退出码 0，无 PostgreSQL 专属 warning；Provider 调用和 schema 测试目标可编译。
- 所需证据：失败编译摘要、最终退出码、复合外键与索引差异、锁顺序和事务边界审查结果。

## 完成

- PostgreSQL 与 SQLite 对外暴露同一依赖图 Store 契约。
- 无真实 PostgreSQL 回归证据时不得写“已完成”，应标记为“已完成，待回归”。
