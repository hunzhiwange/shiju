# 任务19 静态证据（代码闭合，编译/行为未验证）

未运行任何 Cargo、编译、测试、实机或测试可执行文件；仅做源码与差异检查。RustCodeGraph 索引时间早于本轮，当前源码为准。

## 全文件行数及保留原因

| 原文件 | 原行数 | 现行数 | 职责/后续迁移 |
|---|---:|---:|---|
| `crates/vw-agent/src/session/child_agent/collaboration_tests.rs` | 228 | 228 | 2 条真实 AgentTool 装配验收；覆盖真实工具执行、持久化、取消、预算及父子消息循环，保留全部断言。领域状态、等待、取消与邮箱算法已由 session-engine 原有 coordinator_tests/mailbox_tests/continuation_tests 自注册覆盖，不能把本轮保留的装配验收换成模拟端口。 |
| `crates/vw-agent/src/session/child_agent/continuation_tests.rs` | 72 | 72 | 真实宿主 ChildHost/Gateway 发布提交的集成；检查发布后标题保持、四条消息与 followup。发布端口在 gateway-session，领域持久化夹具明确不替代 UI 发布；因此保留此项真实边界。 |
| `crates/vw-agent/src/session/child_agent/coordinator.rs` | 61 | 61 | Deref、new、global_child_agent_coordinator 为真实调用者保留包装；authorize_mail 绑定 NativeSessionIo；team_snapshots/send_team_message/wait_team_activity/consume_team_messages 先鉴权后调用领域入口。 |
| `crates/vw-agent/src/session/child_agent/coordinator_tests.rs` | 1 | 0 | 空注释挂载删除；无测试，删除不算覆盖迁移。 |
| `crates/vw-agent/src/session/child_agent/delivery.rs` | 12 | 9 | deliver_model_events 绑定实际 ToolUseContext 及租户检查，调用领域 child_agent_access。 |
| `crates/vw-agent/src/session/child_agent/delivery_tests.rs` | 38 | 0 | 2 条领域结果游标、父级隔离与未知会话用例迁入 session-engine。 |
| `crates/vw-agent/src/session/child_agent/entry_tests.rs` | 257 | 257 | 真实 Gateway 与 QueryEngine 两入口、原生 HTTP provider、工具工厂与消息循环验收；必须保留真实装配，不以模拟替代。 |
| `crates/vw-agent/src/session/child_agent/history.rs` | 8 | 0 | 删除未挂载且无调用的旧重导出；算法原已在领域库，无本轮生产下沉收益。 |
| `crates/vw-agent/src/session/child_agent/mailbox.rs` | 6 | 6 | 共享消息契约重导出及真实鉴权测试挂载。 |
| `crates/vw-agent/src/session/child_agent/mailbox_tests.rs` | 59 | 59 | 真实 NativeSessionIo 与 ToolUseContext、跨租户拒绝及未知会话拒绝；需要宿主端口保留。 |
| `crates/vw-agent/src/session/child_agent.rs` | 62 | 59 | ChildHost 实现绑定权限、hooks、Gateway、真实持久化和发布；泛型转发绑定具体 Host；删除空 coordinator_tests 挂载。stream_request 仍被 claude_protocol_parity_tests 直接引用。 |
| `crates/vw-agent/src/session/child_agent_tests.rs` | 704 | 251 | 迁出13条（7条纯领域与6条真实存储）测试；只保留 child_first_response_gateway_loads_workspace_agents_and_publishes 及其HTTP/配置/工作区/观测事件夹具，确认实际 Gateway 消费 AGENTS.md、发布标题、去重落盘与协议注入顺序。 |

本组宿主物理行数 1508 → 1002，净减 506（33.6%），不足70%，交收口任务依据上述逐文件原因验收，不能宣称达到该门槛。保留六条真实边界验收占据主要行数；继续可迁项为 Gateway/QueryEngine/AgentTool 的真实组合入口和 HTTP 夹具（需要它们所属领域的真实装配提供者，不能用当前 fixture 冒充）。不为凑比例删除断言或复制宿主。

领域新增：

- `crates/libs/session-engine/src/host_migration_19_tests.rs`：164行，测试/夹具。
- `crates/libs/session-engine/src/host_migration_19_tests/persistence.rs`：399行，测试/夹具。
- `crates/libs/session-engine/src/host_migration_19_tests/storage.rs`：95行，测试/夹具。

生产逻辑迁移0行（算法原已下沉）；宿主生产/文档/转发原132行，现124行（删除8行无挂载重导出；另删除6行测试挂载）。原测试及其挂载1376行，现为上表现总量减124行；领域生产修改仅为 ChildHost 和 emit_boundary 各一处 trait 约束收窄，执行分支、鉴权、取消、持久化策略均不变。无新增依赖。

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
| `crates/vw-agent/src/session/child_agent_tests.rs` | `child_agent_run_creates_persistent_child_and_waits` | session-engine::host_migration_19_tests::persistence::child_agent_run_creates_persistent_child_and_waits |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `child_first_response_gateway_loads_workspace_agents_and_publishes` | 宿主原路径保留（见逐文件原因） |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `child_agent_handoff_execution_returns_last_visible_result` | session-engine::host_migration_19_tests::child_agent_handoff_execution_returns_last_visible_result |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `child_agent_handoff_strips_thinking_alias_blocks` | session-engine::host_migration_19_tests::child_agent_handoff_strips_thinking_alias_blocks |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `child_agent_handoff_plain_text_is_preserved` | session-engine::host_migration_19_tests::child_agent_handoff_plain_text_is_preserved |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `child_agent_handoff_research_returns_sources_and_persists_full_stream` | session-engine::host_migration_19_tests::persistence::child_agent_handoff_research_returns_sources_and_persists_full_stream |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `child_agent_failure_persists_child_history` | session-engine::host_migration_19_tests::persistence::child_agent_failure_persists_child_history |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `agcollab_child_snapshot_is_immutable_and_none_keeps_task_separate` | session-engine::host_migration_19_tests::persistence::agcollab_child_snapshot_is_immutable_and_none_keeps_task_separate |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `agcollab_child_rejects_cross_workspace_and_tenant` | session-engine::host_migration_19_tests::persistence::agcollab_child_rejects_cross_workspace_and_tenant |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `agcollab_stored_history_and_permissions_are_snapshotted` | session-engine::host_migration_19_tests::persistence::agcollab_stored_history_and_permissions_are_snapshotted |
| `crates/vw-agent/src/session/child_agent_tests.rs` | `agcollab_child_request_has_real_identity_and_untrusted_message_guidance` | session-engine::host_migration_19_tests::agcollab_child_request_has_real_identity_and_untrusted_message_guidance |

## 调用及依赖方向

宿主 child_agent::{create,run,run_created,...} → session_engine::child_agent::run::<AgentContextHost>；宿主 coordinator → session_engine::child_agent_access → LifecycleHost；投递 →领域 coordinator。接收库测试通过自身 lib.rs cfg(test) mod 注册，无 path/include 反载宿主、无 vw-agent 依赖、无新增依赖。原 delivery_tests 与空 coordinator_tests 文件及挂载已删除。跨目录 collaboration_tests 仍由 tools/agent_tool/mod.rs 挂载，未漏计。

## 持久化与 hook 装配分离

`ChildHost` 约束从完整 `HookHost` 收窄为 `ContextHost<HookRunner=agent_hooks::HookRunner>`；`plugin_hooks::hook_subagent::emit_boundary` 同步收窄。读过实现确认只消费 context、真实 runner.subagent 与 task-local scope，没有 MCP 操作。真实 AgentContextHost 调用与 runner 执行不变，没有增加配置或跳过鉴权。

storage::Host 使用真实 session_lifecycle::{create_next,reserve_child_agent,get,update} 与 message::{lock_history,update_message,update_part,messages}，不克隆 App/AgentContextHost；仅在外部端口构造消息记录，子会话验证、继承、快照、失败处理与 ensure_turn_history 全走生产 run。无 hook 配置返回 None；未使用的外部能力全部显式 panic。UI 发布端口明确空实现，六条迁出用例均不声称验收 UI 发布；该行为由宿主 Gateway/continuation 两条原用例保留。每条存储用例沿用领域既有子进程临时存储模式，进程启动前设置临时根目录，禁止污染并行测试的全局缓存。只编写此隔离逻辑，本次未调用测试进程。

已检查完整12文件/21用例：15条迁移、6条真实宿主边界保留；无剩余纯领域测试留在宿主。旧 delivery/coordinator 测试挂载和未挂载 history 转发已删除；session 父模块原 child_agent_tests 挂载保留用于唯一 Gateway 边界。无跨目录反向加载和 dev dependency 反向引用。

## 未来验证

`python3 scripts/cargo_slot.py cargo test -p session-engine --lib host_migration_19_tests -- --test-threads=1`：预期15条非零测试（原生目标）；未运行。

剩余宿主用例按上表原名称逐条精确过滤（每个1条）；迁出15条需未来宿主测试 list 证明旧挂载消失，不以零测试表示通过。所有编译与行为检查未运行。静态检查：git diff --check 限本次文件，确认新模块15条、原断言保留；不代表类型检查。

附加未来 hook 检查：`python3 scripts/cargo_slot.py cargo test -p plugin-hooks --lib hook_subagent::tests -- --test-threads=1`（预期2条：codex_plugin_compat_t43_rejects_invalid_input、codex_plugin_compat_t43_output_controls_follow_frozen_baseline；本轮只收窄泛型约束，未修改测试）。

宿主保留用例的完整过滤名（每条预期1条，均未运行）：

```text
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib tools::agent_tool::collaboration_tests::agcollab_end_to_end_team_workflow -- --exact --test-threads=1
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib tools::agent_tool::collaboration_tests::agcollab_end_to_end_cancel_and_budget -- --exact --test-threads=1
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib session::child_agent::coordinator::continuation_tests::child_first_response_continuation_preserves_published_title_and_history -- --exact --test-threads=1
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib session::child_agent::entry_tests::agcollab_end_to_end_gateway_and_query_engine_entries -- --exact --test-threads=1
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib session::child_agent::coordinator::mailbox::tests::agcollab_mailbox_runtime_identity_and_tenant_are_enforced -- --exact --test-threads=1
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib session::child_agent_tests::child_first_response_gateway_loads_workspace_agents_and_publishes -- --exact --test-threads=1
```

静态名称集合核对：原21条，现21条；遗漏0、新增0、重名0。该核对只是源码映射，未执行测试。
