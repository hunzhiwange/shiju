# 任务12 静态迁移证据

代码接线完成；编译/行为检查全部未运行，遵守本轮禁止 Cargo、测试、实机验证的约束。

## 全文件/全用例映射

### crates/vw-agent/src/gateway/api/handlers/session/turn_fork_tests.rs

行数 817 → 817。
- `session_fork_before_turn_keeps_only_complete_earlier_turns` → `原宿主完整模块路径不变：session_fork_before_turn_keeps_only_complete_earlier_turns`
- `session_fork_before_turn_rejects_invalid_boundaries_before_creating_targets` → `原宿主完整模块路径不变：session_fork_before_turn_rejects_invalid_boundaries_before_creating_targets`
- `session_fork_before_turn_rolls_back_staged_history_and_worktree_on_write_failure` → `原宿主完整模块路径不变：session_fork_before_turn_rolls_back_staged_history_and_worktree_on_write_failure`
- `fork_history_ui_reads_the_persisted_prefix` → `原宿主完整模块路径不变：fork_history_ui_reads_the_persisted_prefix`
- `fork_selection_locks_when_own_request_starts` → `原宿主完整模块路径不变：fork_selection_locks_when_own_request_starts`
- `worktree_environment_fork_waits_for_setup_and_rejects_failure` → `原宿主完整模块路径不变：worktree_environment_fork_waits_for_setup_and_rejects_failure`
- `fork_selection_before_start_survives_reload` → `原宿主完整模块路径不变：fork_selection_before_start_survives_reload`
保留 request/fork/history：真实 Axum 请求及路由提取验证；persist_turn：真实宿主流持久化接线；git/repository/worktree_cleanup：真实 Git 工作树生命周期。逐用例：prefix/invalid-boundary/rollback 三项同时验证真实宿主 fork 端口、绑定、回滚；UI prefix 验证热/冷 AppState 查询引擎；selection_locks 验证实际 chat_stream 接受与拒绝顺序；environment 验证工作树初始化事件；selection_reload 验证实际 agent 配置与持久绑定。尾部 fork_acp_tests 挂载归其它来源所有者，保持不变。继续可迁：前三项领域前缀断言及其真实 session/worktree 适配器，不能仅用假端口替代现有资源断言。

### crates/vw-agent/src/gateway/api/handlers/session/ui_ops.rs

行数 353 → 353。
保留 Host 的 MessageHost/UiHost 全部方法与16个 HTTP 类型转换转发：真实租户、实例、session_store、agent_binding、stream reserve 注入；已有领域生产实现不计本轮生产迁移收益。

### crates/vw-agent/src/gateway/api/handlers/session/ui_ops_tests.rs

行数 1650 → 302。
- `llm_raw_endpoint_distinguishes_turns_with_same_step_index` → `gateway_session::host_migration_12_tests::ui::llm_raw_endpoint_distinguishes_turns_with_same_step_index`
- `session_ui_summary_omits_message_payload` → `gateway_session::host_migration_12_tests::ui::session_ui_summary_omits_message_payload`
- `session_ui_summary_refreshes_a_stale_delta_before_first_response` → `gateway_session::host_migration_12_tests::ui::session_ui_summary_refreshes_a_stale_delta_before_first_response`
- `session_ui_messages_enforces_page_limit` → `gateway_session::host_migration_12_tests::ui::session_ui_messages_enforces_page_limit`
- `session_ui_messages_rejects_foreign_cursor` → `gateway_session::host_migration_12_tests::ui::session_ui_messages_rejects_foreign_cursor`
- `session_ui_message_delta_rejects_invalid_and_missing_targets` → `gateway_session::host_migration_12_tests::ui::session_ui_message_delta_rejects_invalid_and_missing_targets`
- `session_ui_message_delta_rejects_stale_version` → `gateway_session::host_migration_12_tests::ui::session_ui_message_delta_rejects_stale_version`
- `ui_handlers_are_available` → `gateway_session::host_migration_12_tests::ui::ui_handlers_are_available`
- `session_step_llm_raw_get_returns_requested_step_and_null_for_missing` → `gateway_session::host_migration_12_tests::ui::session_step_llm_raw_get_returns_requested_step_and_null_for_missing`
- `session_step_llm_raw_get_isolates_project_scope` → `gateway_session::host_migration_12_tests::ui::session_step_llm_raw_get_isolates_project_scope`
- `session_step_llm_raw_get_isolates_tenants` → `gateway_session::host_migration_12_tests::ui::session_step_llm_raw_get_isolates_tenants`
- `session_step_llm_raw_get_maps_storage_error_to_internal_error` → `gateway_session::host_migration_12_tests::ui::session_step_llm_raw_get_maps_storage_error_to_internal_error`
- `session_ui_save_propagates_session_store_failure` → `gateway_session::host_migration_12_tests::ui::session_ui_save_propagates_session_store_failure`
- `session_ui_save_leaves_pending_raw_for_runtime_commit` → `gateway_session::host_migration_12_tests::ui::session_ui_save_leaves_pending_raw_for_runtime_commit`
- `simple_answer_and_tool_rounds_commit_without_raw_duplication` → `gateway_session::host_migration_12_tests::ui::simple_answer_and_tool_rounds_commit_without_raw_duplication`
- `ui_snapshot_handlers_round_trip_scoped_sessions_and_metadata` → `原宿主完整模块路径不变：ui_snapshot_handlers_round_trip_scoped_sessions_and_metadata`
- `ui_content_save_preserves_runtime_calls_and_steps` → `gateway_session::host_migration_12_tests::ui::ui_content_save_preserves_runtime_calls_and_steps`
- `ui_snapshot_handlers_cover_archived_scope_and_errors` → `gateway_session::host_migration_12_tests::ui::ui_snapshot_handlers_cover_archived_scope_and_errors`
- `ui_snapshot_handlers_return_empty_results_for_unknown_session` → `gateway_session::host_migration_12_tests::ui::ui_snapshot_handlers_return_empty_results_for_unknown_session`
- `session_archived_put_empty_list_clears_scope` → `gateway_session::host_migration_12_tests::ui::session_archived_put_empty_list_clears_scope`
- `session_ui_get_maps_invalid_json_llm_raw_to_internal_error` → `gateway_session::host_migration_12_tests::ui::session_ui_get_maps_invalid_json_llm_raw_to_internal_error`
- `ui_snapshot_handlers_isolate_same_session_id_across_tenants` → `gateway_session::host_migration_12_tests::ui::ui_snapshot_handlers_isolate_same_session_id_across_tenants`
- `session_ui_save_returns_conflict_for_stale_content` → `gateway_session::host_migration_12_tests::ui::session_ui_save_returns_conflict_for_stale_content`
- `agent_unified_selection_summary_projects_server_owned_identity` → `gateway_session::host_migration_12_tests::ui::agent_unified_selection_summary_projects_server_owned_identity`
- `gateway_first_response_ui_save_cannot_publish_a_new_session` → `gateway_session::host_migration_12_tests::ui::gateway_first_response_ui_save_cannot_publish_a_new_session`
- `session_history_ui_save_rejects_newer_incomplete_content_with_conflict` → `gateway_session::host_migration_12_tests::ui::session_history_ui_save_rejects_newer_incomplete_content_with_conflict`
保留完整 round_trip 用例与其最小 scoped_session/save_runtime_session/agent_session、目录/header/home 夹具：同时跨真实宿主 todo 路由和 UI 存储；未拆散或删断言。继续可迁：todo 端口已有真实领域测试宿主后可整项迁移。

### crates/vw-agent/src/gateway/api/handlers/session.rs

行数 135 → 135。
保留 router 全部路由装配及 session_tenant_scope 请求级中间件和其它所有者测试挂载；均为真实宿主边界，无领域算法。

### crates/vw-agent/src/gateway/api/handlers/session_tests.rs

行数 362 → 362。
- `session_router_can_be_constructed` → `原宿主完整模块路径不变：session_router_can_be_constructed`
- `session_router_registers_expected_paths_and_methods` → `原宿主完整模块路径不变：session_router_registers_expected_paths_and_methods`
- `session_router_rejects_unknown_paths` → `原宿主完整模块路径不变：session_router_rejects_unknown_paths`
- `session_tenant_scope_forwards_each_request_scope` → `原宿主完整模块路径不变：session_tenant_scope_forwards_each_request_scope`
- `session_tenant_scope_rejects_requests_without_extension` → `原宿主完整模块路径不变：session_tenant_scope_rejects_requests_without_extension`
- `persist_stream_chat_turn_creates_user_and_assistant_messages` → `原宿主完整模块路径不变：persist_stream_chat_turn_creates_user_and_assistant_messages`
- `persist_stream_chat_turn_reuses_preallocated_message_ids` → `原宿主完整模块路径不变：persist_stream_chat_turn_reuses_preallocated_message_ids`
- `persist_stream_chat_turn_stores_extra_assistant_patch_parts` → `原宿主完整模块路径不变：persist_stream_chat_turn_stores_extra_assistant_patch_parts`
保留 StaticProvider/state/test_app/test_app_for_tenant/response_status 与租户探针：证明真实 Router<AppState>、Extension 提取、请求隔离；三条 persist_stream_chat_turn 用例验证宿主具体 stream 端口的用户/助手消息、预分配 ID、Patch 存储接线，断言未复制到假端口。继续可迁：领域 stream adapter 测试宿主具备真实存储端口后，可迁三条持久化用例。

## 减量与依赖
宿主合计 3317 → 1969，净减 1348 行（40.6%），低于70%；逐文件原因和继续可迁项见上，交收口任务审查，未宣称门槛达成。
生产实现迁移 0 行；本轮是领域测试归属纠正。ui_ops 生产链仍为 router → 宿主 ui_ops → gateway_session::ui_ops::<Host> → 原有存储端口。领域不依赖 vw-agent，也不通过 path/include 加载宿主源码。新测试通过 lib.rs 自注册，旧 ui_ops_tests 挂载只承接保留用例。
测试 adapter 仅实现既有 MessageHost/UiHost，复用真实 agent_project/session_store/binding_store，不复制 AppState。新增 dev 依赖 agent-paths/config-runtime-paths/session-paths；Cargo.lock 的 gateway-session 节点同步登记。
UiHost::reserve 使用最小 Info 夹具注入真实 reserve_pending_session；settings 读取真实 binding_store。原 step.step 私有访问改为序列化后按 ChatSessionStep 解码，保留原业务字段断言且不暴露生产私有字段。
- 新领域测试/夹具 `crates/libs/gateway-session/src/host_migration_12_tests.rs`：196 行，生产 0 行。
- 新领域测试/夹具 `crates/libs/gateway-session/src/host_migration_12_tests/ui.rs`：1472 行，生产 0 行。

## 未来验证
`python3 scripts/cargo_slot.py cargo test -p gateway-session --lib host_migration_12_tests -- --test-threads=1`，预期 25 条非零测试。
宿主保留用例使用上表完整名称：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib <name> -- --test-threads=1`，每条预期1；迁出25条应在未来宿主 --list 输出中消失（不以零测试作为通过）。
静态 diff whitespace 检查已执行；Cargo check/test/list/fmt、编译、行为及渲染检查全部未运行；不宣称性能或构建收益。
