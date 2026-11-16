# 任务19 静态证据（部分实施，未达到完整迁移验收）

未运行任何 Cargo、编译、测试、实机或测试可执行文件；仅做源码与差异检查。RustCodeGraph 索引时间早于本轮，当前源码为准。

## 全文件行数及保留原因

| 原文件 | 原行数 | 现行数 | 职责/后续迁移 |
|---|---:|---:|---|
| `crates/vw-agent/src/session/child_agent/collaboration_tests.rs` | 228 | 228 | 2 条真实 AgentTool 装配验收；依赖宿主 AgentTool 的执行器注入与工具工厂，保留完整断言，未来将领域协作断言分离。 |
| `crates/vw-agent/src/session/child_agent/continuation_tests.rs` | 72 | 72 | 真实宿主 ChildHost 的持久化、发布标题与历史集成；未来可迁至独立持久化端口夹具。 |
| `crates/vw-agent/src/session/child_agent/coordinator.rs` | 61 | 61 | Deref、new、global_child_agent_coordinator 为真实调用者保留包装；authorize_mail 绑定 NativeSessionIo；team_snapshots/send_team_message/wait_team_activity/consume_team_messages 先鉴权后调用领域入口。 |
| `crates/vw-agent/src/session/child_agent/coordinator_tests.rs` | 1 | 0 | 空注释挂载删除；无测试，删除不算覆盖迁移。 |
| `crates/vw-agent/src/session/child_agent/delivery.rs` | 12 | 9 | deliver_model_events 绑定实际 ToolUseContext 及租户检查，调用领域 child_agent_access。 |
| `crates/vw-agent/src/session/child_agent/delivery_tests.rs` | 38 | 0 | 2 条领域结果游标、父级隔离与未知会话用例迁入 session-engine。 |
| `crates/vw-agent/src/session/child_agent/entry_tests.rs` | 257 | 257 | 真实 Gateway 与 QueryEngine 两入口、原生 HTTP provider、工具工厂与消息循环验收；必须保留真实装配，不以模拟替代。 |
| `crates/vw-agent/src/session/child_agent/history.rs` | 8 | 8 | 已有领域重导出，无本轮生产下沉收益；当前无父模块挂载，后续可清理。 |
| `crates/vw-agent/src/session/child_agent/mailbox.rs` | 6 | 6 | 共享消息契约重导出及真实鉴权测试挂载。 |
| `crates/vw-agent/src/session/child_agent/mailbox_tests.rs` | 59 | 59 | 真实 NativeSessionIo 与 ToolUseContext、跨租户拒绝及未知会话拒绝；需要宿主端口保留。 |
| `crates/vw-agent/src/session/child_agent.rs` | 62 | 59 | ChildHost 实现绑定权限、hooks、Gateway、真实持久化和发布；泛型转发绑定具体 Host；删除空 coordinator_tests 挂载。stream_request 仍被 claude_protocol_parity_tests 直接引用。 |
| `crates/vw-agent/src/session/child_agent_tests.rs` | 704 | 591 | 迁出7条纯领域测试；剩余7条持久化/快照/租户/权限与真实Gateway集成，不能直接把AgentContextHost复制到库。 |

本组宿主物理行数 1508 → 1350，净减 158（10.5%），低于70%；不得作为整组完成。领域新建 host_migration_19_tests.rs：159 行（全部为测试/import），lib.rs 增加3行测试注册；本轮真实生产逻辑迁移为0，生产宿主只删除3行测试挂载，转发/端口实现保持。

## 每条用例映射（保留原名及断言）

| 原文件 | 用例 | 归属 |
|---|---|---|
| `crates/vw-agent/src/session/child_agent/collaboration_tests.rs` | `agcollab_end_to_end_team_workflow` | 宿主原路径保留（见逐文件原因） |
| `crates/vw-agent/src/session/child_agent/collaboration_tests.rs` | `agcollab_end_to_end_cancel_and_budget` | 宿主原路径保留（见逐文件原因） |
| `crates/vw-agent/src/session/child_agent/continuation_tests.rs` | `child_first_response_continuation_preserves_published_title_and_history` | 宿主原路径保留（见逐文件原因） |
| `crates/vw-agent/src/session/child_agent/delivery_tests.rs` | `agcollab_delivery_results_are_once_per_turn_and_parent_scoped` | session-engine::host_migration_19_tests::agcollab_delivery_results_are_once_per_turn_and_parent_scoped |
| `crates/vw-agent/src/session/child_agent/delivery_tests.rs` | `agcollab_delivery_unknown_session_has_no_team_data` | session-engine::host_migration_19_tests::agcollab_delivery_unknown_session_has_no_team_data |
| `crates/vw-agent/src/session/child_agent/entry_tests.rs` | `agcollab_end_to_end_gateway_and_query_engine_entries` | 宿主原路径保留（见逐文件原因） |
| `crates/vw-agent/src/session/child_agent/mailbox_tests.rs` | `agcollab_mailbox_runtime_identity_and_tenant_are_enforced` | 宿主原路径保留（见逐文件原因） |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `child_agent_stream_request_keeps_complete_model_reference` | session-engine::host_migration_19_tests::child_agent_stream_request_keeps_complete_model_reference |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `child_agent_stream_request_registers_configured_tools_and_preserves_skill_policy` | session-engine::host_migration_19_tests::child_agent_stream_request_registers_configured_tools_and_preserves_skill_policy |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `session_protocol_child_agent_uses_same_contract` | session-engine::host_migration_19_tests::session_protocol_child_agent_uses_same_contract |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `child_agent_run_creates_persistent_child_and_waits` | 宿主原路径保留（见逐文件原因） |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `child_first_response_gateway_loads_workspace_agents_and_publishes` | 宿主原路径保留（见逐文件原因） |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `child_agent_handoff_execution_returns_last_visible_result` | session-engine::host_migration_19_tests::child_agent_handoff_execution_returns_last_visible_result |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `child_agent_handoff_strips_thinking_alias_blocks` | session-engine::host_migration_19_tests::child_agent_handoff_strips_thinking_alias_blocks |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `child_agent_handoff_plain_text_is_preserved` | session-engine::host_migration_19_tests::child_agent_handoff_plain_text_is_preserved |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `child_agent_handoff_research_returns_sources_and_persists_full_stream` | 宿主原路径保留（见逐文件原因） |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `child_agent_failure_persists_child_history` | 宿主原路径保留（见逐文件原因） |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `agcollab_child_snapshot_is_immutable_and_none_keeps_task_separate` | 宿主原路径保留（见逐文件原因） |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `agcollab_child_rejects_cross_workspace_and_tenant` | 宿主原路径保留（见逐文件原因） |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `agcollab_stored_history_and_permissions_are_snapshotted` | 宿主原路径保留（见逐文件原因） |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `agcollab_child_request_has_real_identity_and_untrusted_message_guidance` | session-engine::host_migration_19_tests::agcollab_child_request_has_real_identity_and_untrusted_message_guidance |

## 调用及依赖方向

宿主 child_agent::{create,run,run_created,...} → session_engine::child_agent::run::<AgentContextHost>；宿主 coordinator → session_engine::child_agent_access → LifecycleHost；投递 →领域 coordinator。接收库测试通过自身 lib.rs cfg(test) mod 注册，无 path/include 反载宿主、无 vw-agent 依赖、无新增依赖。原 delivery_tests 与空 coordinator_tests 文件及挂载已删除。跨目录 collaboration_tests 仍由 tools/agent_tool/mod.rs 挂载，未漏计。

## 未闭合项

持久化测试需要 ChildHost: HookHost: McpToolHost: ContextHost 具体端口，session-engine 当前没有可复用的实现。下一步需把真实持久化端口与 hook/工具上下文装配分离，再迁移完整持久化测试族；保留真实权限、Gateway 与工具工厂边界断言。此项尚未实施，不能因已迁9条而删任务。完整源族迁移、70%减量与宿主薄化均未达到。

## 未来验证

`python3 scripts/cargo_slot.py cargo test -p session-engine --lib host_migration_19_tests -- --test-threads=1`：预期9条非零测试；未运行。

剩余宿主用例按上表原名称逐条精确过滤（每个1条）；迁出9条需未来宿主测试 list 证明旧挂载消失，不以零测试表示通过。所有编译与行为检查未运行。静态检查：git diff --check 限本次文件，确认新模块9条、原断言保留；不代表类型检查。
