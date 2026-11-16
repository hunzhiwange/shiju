# 任务19：深度迁移最终静态证据

代码完成，编译/行为未验证。本轮禁止Cargo、构建与运行测试；未执行这些操作。静态差异检查不代表行为验收。

## 全文件行数

| 原文件 | 原行数 | 现行数 | 职责/后续迁移 |
|---|---:|---:|---|
| `crates/vw-agent/src/session/child_agent/collaboration_tests.rs` | 228 | 204 | 见下方当前职责 |
| `crates/vw-agent/src/session/child_agent/continuation_tests.rs` | 72 | 7 | 见下方当前职责 |
| `crates/vw-agent/src/session/child_agent/coordinator.rs` | 61 | 60 | 见下方当前职责 |
| `crates/vw-agent/src/session/child_agent/coordinator_tests.rs` | 1 | 0 | 见下方当前职责 |
| `crates/vw-agent/src/session/child_agent/delivery.rs` | 12 | 9 | 见下方当前职责 |
| `crates/vw-agent/src/session/child_agent/delivery_tests.rs` | 38 | 0 | 见下方当前职责 |
| `crates/vw-agent/src/session/child_agent/entry_tests.rs` | 257 | 98 | 见下方当前职责 |
| `crates/vw-agent/src/session/child_agent/history.rs` | 8 | 0 | 见下方当前职责 |
| `crates/vw-agent/src/session/child_agent/mailbox.rs` | 6 | 2 | 见下方当前职责 |
| `crates/vw-agent/src/session/child_agent/mailbox_tests.rs` | 59 | 0 | 见下方当前职责 |
| `crates/vw-agent/src/session/child_agent.rs` | 62 | 59 | 见下方当前职责 |
| `crates/vw-agent/src/session/child_agent_tests.rs` | 704 | 10 | 见下方当前职责 |


宿主1508 → 447行，净减1061（70.36%）；只证明物理源码减量。整轮仍需任务59重算，不修改final.md/remaining-host.json快照。

当前逐文件职责：

- collaboration_tests.rs：保留真实AgentTool工厂、并发与取消预算、通信/后续回合接线断言；两处持久化父会话夹具移gateway-session::child_acceptance::create_collaboration_parent。
- continuation_tests.rs：仅7行绑定实际AgentContextHost与NativeSessionIo；完整标题发布及4条历史断言移gateway-session::child_acceptance同名泛型验收。
- coordinator.rs：真实上下文与NativeSessionIo装配、Deref/new/global/team_snapshots/wait保留；send/consume鉴权和操作顺序本轮下沉child_agent_access，宿主两处直接调用。
- coordinator_tests.rs：空注释文件及挂载已删除，无新增覆盖收益。
- delivery.rs：实际上下文到领域租户投递入口的薄适配；delivery_tests.rs整2条领域游标/父隔离用例已迁。
- entry_tests.rs：只留配置、实际Gateway/QueryEngine入口调用与结果检查；HttpMode、ParentModel、HTTP脚本及父会话夹具已移gateway-session::child_acceptance。
- history.rs：未挂载重导出删除，无生产算法收益。
- mailbox.rs：仅共享契约重导出；mailbox_tests.rs完整身份/租户断言移领域access模块，消费真实生产send/consume入口和真实会话存储，旧挂载删除。
- child_agent.rs：保留实际存储、权限、hooks、Gateway端口与生产/compat调用需要的类型绑定。
- child_agent_tests.rs：仅保留环境锁和三种实际宿主类型调用；HTTP/配置/AGENTS.md/观测事件及去重发布断言已移gateway-session验收。

## 生产入口、注册与依赖

新增session-engine::child_agent_access::{send_team_message,consume_team_messages}共17行，真实调用方为宿主coordinator对应方法。先authorize_mail加载真实源/目标身份并检查租户，再enqueue_mail/take_mail；状态权限与拒绝顺序不变。ChildHost与emit_boundary上一提交已收窄为实际需要的ContextHost<HookRunner=HookRunner>，没有MCP调用、权限检查或hook行为删除。

16条领域用例由session-engine自身host_migration_19_tests注册（9条纯领域、6条持久化、1条access）。5条真实宿主组合验收保留同名测试。gateway-session在已有test-support特性下提供child_acceptance模块；它是实际宿主验收函数与模型脚本，不能声称是额外自注册测试，也不声称它们退出宿主编译图。宿主stream::Host仅从pub(super)改为pub(crate)，让同crate薄测试指定原真实端口。无path/include反载宿主，无vw-agent反向依赖，无App复制。

gateway-session既有test-support增加4个实际使用的可选依赖：agent-env、agent-config、model-types、async-trait。它们只服务真实配置隔离和Provider脚本；Cargo.lock仅提交该包4项新增引用，保留他人变更。无新增生产配置或重依赖。

存储fixture使用真实session_lifecycle和message存储；不模拟UI发布。发布/标题验收调用真实C端口，原断言完整保留。子进程临时存储隔离沿用已有领域模式，本轮未运行测试子进程。物理归属、测试实例化归属与编译耗时分别说明，不宣称编译收益。

## 每条用例映射（保留原名及断言）

| 原文件 | 用例 | 归属 |
|---|---|---|
| `crates/vw-agent/src/session/child_agent/collaboration_tests.rs` | `agcollab_end_to_end_team_workflow` | 宿主原路径保留（见逐文件原因） |
| `crates/vw-agent/src/session/child_agent/collaboration_tests.rs` | `agcollab_end_to_end_cancel_and_budget` | 宿主原路径保留（见逐文件原因） |
| `crates/vw-agent/src/session/child_agent/continuation_tests.rs` | `child_first_response_continuation_preserves_published_title_and_history` | gateway-session::child_acceptance::child_first_response_continuation_preserves_published_title_and_history；原同名宿主测试只绑定真实类型 |
| `crates/vw-agent/src/session/child_agent/delivery_tests.rs` | `agcollab_delivery_results_are_once_per_turn_and_parent_scoped` | session-engine::host_migration_19_tests::agcollab_delivery_results_are_once_per_turn_and_parent_scoped |
| `crates/vw-agent/src/session/child_agent/delivery_tests.rs` | `agcollab_delivery_unknown_session_has_no_team_data` | session-engine::host_migration_19_tests::agcollab_delivery_unknown_session_has_no_team_data |
| `crates/vw-agent/src/session/child_agent/entry_tests.rs` | `agcollab_end_to_end_gateway_and_query_engine_entries` | 宿主原路径保留（见逐文件原因） |
| `crates/vw-agent/src/session/child_agent/mailbox_tests.rs` | `agcollab_mailbox_runtime_identity_and_tenant_are_enforced` | session-engine::host_migration_19_tests::access::agcollab_mailbox_runtime_identity_and_tenant_are_enforced |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `child_agent_stream_request_keeps_complete_model_reference` | session-engine::host_migration_19_tests::child_agent_stream_request_keeps_complete_model_reference |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `child_agent_stream_request_registers_configured_tools_and_preserves_skill_policy` | session-engine::host_migration_19_tests::child_agent_stream_request_registers_configured_tools_and_preserves_skill_policy |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `session_protocol_child_agent_uses_same_contract` | session-engine::host_migration_19_tests::session_protocol_child_agent_uses_same_contract |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `child_agent_run_creates_persistent_child_and_waits` | session-engine::host_migration_19_tests::persistence::child_agent_run_creates_persistent_child_and_waits |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `child_first_response_gateway_loads_workspace_agents_and_publishes` | gateway-session::child_acceptance::child_first_response_gateway_loads_workspace_agents_and_publishes；原同名宿主测试只绑定真实类型 |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `child_agent_handoff_execution_returns_last_visible_result` | session-engine::host_migration_19_tests::child_agent_handoff_execution_returns_last_visible_result |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `child_agent_handoff_strips_thinking_alias_blocks` | session-engine::host_migration_19_tests::child_agent_handoff_strips_thinking_alias_blocks |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `child_agent_handoff_plain_text_is_preserved` | session-engine::host_migration_19_tests::child_agent_handoff_plain_text_is_preserved |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `child_agent_handoff_research_returns_sources_and_persists_full_stream` | session-engine::host_migration_19_tests::persistence::child_agent_handoff_research_returns_sources_and_persists_full_stream |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `child_agent_failure_persists_child_history` | session-engine::host_migration_19_tests::persistence::child_agent_failure_persists_child_history |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `agcollab_child_snapshot_is_immutable_and_none_keeps_task_separate` | session-engine::host_migration_19_tests::persistence::agcollab_child_snapshot_is_immutable_and_none_keeps_task_separate |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `agcollab_child_rejects_cross_workspace_and_tenant` | session-engine::host_migration_19_tests::persistence::agcollab_child_rejects_cross_workspace_and_tenant |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `agcollab_stored_history_and_permissions_are_snapshotted` | session-engine::host_migration_19_tests::persistence::agcollab_stored_history_and_permissions_are_snapshotted |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `agcollab_child_request_has_real_identity_and_untrusted_message_guidance` | session-engine::host_migration_19_tests::agcollab_child_request_has_real_identity_and_untrusted_message_guidance |


## 当前领域文件行数

- `crates/libs/session-engine/src/child_agent_access.rs`：32行。
- `crates/libs/session-engine/src/host_migration_19_tests.rs`：167行。
- `crates/libs/session-engine/src/host_migration_19_tests/access.rs`：77行。
- `crates/libs/session-engine/src/host_migration_19_tests/persistence.rs`：399行。
- `crates/libs/session-engine/src/host_migration_19_tests/storage.rs`：95行。
- `crates/libs/gateway-session/src/child_acceptance.rs`：487行。

## 未来验证（均未运行）

- `python3 scripts/cargo_slot.py cargo test -p session-engine --lib host_migration_19_tests -- --test-threads=1`：原生目标预期16条。
- `python3 scripts/cargo_slot.py cargo test -p plugin-hooks --lib hook_subagent::tests -- --test-threads=1`：预期2条。
- 宿主保留5条：tools::agent_tool::collaboration_tests两条，session::child_agent::coordinator::continuation_tests一条，session::child_agent::entry_tests一条，session::child_agent_tests一条；按映射表原函数名完整过滤，每条预期1条。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib -- --list`：未来证明16条迁出旧名称消失；零测试不能称通过。

静态核对：所有来源逐项映射、旧挂载移除、新生产调用、特性注册、依赖方向及限定文件git diff --check。未执行类型检查或测试，不记录通过数。仍保留的447行按上方具体职责，不以隐藏/删除断言凑比例。

静态名称集合：21原用例 = 16领域 + 5宿主，遗漏0。可选TOML解析尝试未执行成功：本机python3没有tomllib；未安装依赖，未将其记为通过。
