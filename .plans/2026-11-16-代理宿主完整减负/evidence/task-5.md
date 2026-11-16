# 任务5：网关调度测试完整归属迁移

代码完成，编译/行为未验证。遵守本轮禁止 Cargo、单元测试、构建、实机验证约束；只核对源码、挂载、依赖和差异。

## 生产入口及方向

router → router_with_gateway → gateway_agent_scheduler::router_with_gateway；生产仍使用 SchedulerGatewayHost 的真实聊天流、工具上下文及工作树池。agent_scheduler.rs 20 行未变。此前生产已经下沉，本轮生产迁移收益为 0 行，未增加领域算法副本。

原 acceptance 的 agent_scheduler_create_task/list_tasks/wake/recovery/list_events/drain_outbox 及四个 *_with_gateway 均为测试转发夹具，完整移到领域测试模块，调用原 AgentSchedulerGateway，未冒充生产迁移。

领域库 lib.rs 使用 cfg(all(test, not(wasm32))) 自注册 host_migration_5_tests → cases；不依赖 vw-agent、不使用 path/include 挂宿主源码。原 acceptance → agent_scheduler_tests 仅保留两个真实宿主用例。原领域用例的宿主挂载已删除。任务1 external-mounts 中无本文件族的额外跨目录挂载。

测试使用原真实 AgentSchedulerStore 与 agent_project/test-support 注册存储；注入原 Recording/Blocking/Acceptance 执行器，默认网关 TestHost 的聊天流按原宿主测试语义明确失败，工具上下文及真实池访问直接 panic，禁止伪造宿主能力。ContextHost 仅提供关联类型及不可达端口，无 App/AppState 或全局配置复制。原 acceptance_gateway_task 的宿主全局环境锁去除：该夹具不修改环境变量，项目使用唯一临时目录；领域未引入环境变量或 OnceLock。

## 行数与保留职责

splitlines：宿主来源 3194 → 192，净减 3002 行（94.0%）。生产 20 → 20；测试与夹具 3174 → 172。

- `agent_scheduler.rs`：20 行；真实 SchedulerGatewayHost 生产装配与测试中间件接入。
- `agent_scheduler_acceptance.rs`：15 行；默认租户测试中间件及薄验收注册。
- `agent_scheduler_tests.rs`：157 行；真实请求头租户中间件、真实工作树池运行站点生命周期；独立项目注册/Git 临时目录夹具。

领域生产文件未改变，仅增加测试及 lib.rs 注册；新测试分项：
- `cases.rs`：2753 行，测试/夹具（非生产）。
- `host.rs`：69 行，测试/夹具（非生产）。
- `mod.rs`：341 行，测试/夹具（非生产）。

## 全用例映射

静态检查原用例集合与新领域+宿主集合完全相等；完整函数体除 crate::agent_scheduler → task_scheduler 路径外相等，保留全部断言；未以测试执行证明通过。

|原 acceptance::agent_scheduler_tests 名称|新归属|
|---|---|
|`agent_scheduler_routes_require_header_tenant_scope`|宿主原完整路径保留|
|`agent_scheduler_sse_uses_extension_tenant_scope_and_isolates_projects`|`gateway_agent_scheduler::host_migration_5_tests::cases::agent_scheduler_sse_uses_extension_tenant_scope_and_isolates_projects`|
|`agent_scheduler_api_creates_task_and_wakes_dispatch_engine`|`gateway_agent_scheduler::host_migration_5_tests::cases::agent_scheduler_api_creates_task_and_wakes_dispatch_engine`|
|`agent_scheduler_gateway_command_runs_task`|`gateway_agent_scheduler::host_migration_5_tests::cases::agent_scheduler_gateway_command_runs_task`|
|`task_pool_acceptance_non_git_runs_through_gateway_without_git`|`gateway_agent_scheduler::host_migration_5_tests::cases::task_pool_acceptance_non_git_runs_through_gateway_without_git`|
|`task_pool_acceptance_git_task_has_zero_implicit_actions`|`gateway_agent_scheduler::host_migration_5_tests::cases::task_pool_acceptance_git_task_has_zero_implicit_actions`|
|`agent_scheduler_gateway_local_command_respects_max_concurrent`|`gateway_agent_scheduler::host_migration_5_tests::cases::agent_scheduler_gateway_local_command_respects_max_concurrent`|
|`agent_scheduler_project_wake_skips_lifecycle_ineligible_queued_task`|`gateway_agent_scheduler::host_migration_5_tests::cases::agent_scheduler_project_wake_skips_lifecycle_ineligible_queued_task`|
|`project_local_wake_uses_graph_readiness`|`gateway_agent_scheduler::host_migration_5_tests::cases::project_local_wake_uses_graph_readiness`|
|`agent_scheduler_project_wake_ignores_autonomous_staff_assignment`|`gateway_agent_scheduler::host_migration_5_tests::cases::agent_scheduler_project_wake_ignores_autonomous_staff_assignment`|
|`project_wake_does_not_apply_staff_plan`|`gateway_agent_scheduler::host_migration_5_tests::cases::project_wake_does_not_apply_staff_plan`|
|`agent_scheduler_project_wake_updates_task_board_projection_from_initial_pending_map`|`gateway_agent_scheduler::host_migration_5_tests::cases::agent_scheduler_project_wake_updates_task_board_projection_from_initial_pending_map`|
|`agent_scheduler_project_wake_respects_local_max_concurrent`|`gateway_agent_scheduler::host_migration_5_tests::cases::agent_scheduler_project_wake_respects_local_max_concurrent`|
|`agent_scheduler_gateway_policy_wake_leaves_pool_when_auto_promote_disabled`|`gateway_agent_scheduler::host_migration_5_tests::cases::agent_scheduler_gateway_policy_wake_leaves_pool_when_auto_promote_disabled`|
|`agent_scheduler_gateway_policy_wake_promotes_and_dispatches_pool_task`|`gateway_agent_scheduler::host_migration_5_tests::cases::agent_scheduler_gateway_policy_wake_promotes_and_dispatches_pool_task`|
|`agent_scheduler_gateway_policy_leaves_review_ready_task_submitted`|`gateway_agent_scheduler::host_migration_5_tests::cases::agent_scheduler_gateway_policy_leaves_review_ready_task_submitted`|
|`agent_scheduler_task_board_uses_task_board_status_key_metadata`|`gateway_agent_scheduler::host_migration_5_tests::cases::agent_scheduler_task_board_uses_task_board_status_key_metadata`|
|`agent_scheduler_gateway_replaces_and_reads_scoped_task_graph`|`gateway_agent_scheduler::host_migration_5_tests::cases::agent_scheduler_gateway_replaces_and_reads_scoped_task_graph`|
|`agent_scheduler_gateway_rejects_invalid_graph_without_partial_write`|`gateway_agent_scheduler::host_migration_5_tests::cases::agent_scheduler_gateway_rejects_invalid_graph_without_partial_write`|
|`agent_scheduler_gateway_task_board_updates_moves_and_soft_deletes_task`|`gateway_agent_scheduler::host_migration_5_tests::cases::agent_scheduler_gateway_task_board_updates_moves_and_soft_deletes_task`|
|`agent_scheduler_gateway_task_board_bulk_archives_and_deletes_tasks`|`gateway_agent_scheduler::host_migration_5_tests::cases::agent_scheduler_gateway_task_board_bulk_archives_and_deletes_tasks`|
|`agent_scheduler_gateway_legacy_import_is_idempotent_and_persists_workflow`|`gateway_agent_scheduler::host_migration_5_tests::cases::agent_scheduler_gateway_legacy_import_is_idempotent_and_persists_workflow`|
|`agent_scheduler_gateway_local_command_returns_before_execution_finishes`|`gateway_agent_scheduler::host_migration_5_tests::cases::agent_scheduler_gateway_local_command_returns_before_execution_finishes`|
|`agent_scheduler_gateway_start_is_idempotent_for_running_task`|`gateway_agent_scheduler::host_migration_5_tests::cases::agent_scheduler_gateway_start_is_idempotent_for_running_task`|
|`task_pool_capacity_gateway_runtime_projects_logical_station`|宿主原完整路径保留|
|`gateway_local_agent_stream_request_forwards_execution_selection`|`gateway_agent_scheduler::host_migration_5_tests::cases::gateway_local_agent_stream_request_forwards_execution_selection`|
|`gateway_local_agent_stream_payloads_append_task_logs`|`gateway_agent_scheduler::host_migration_5_tests::cases::gateway_local_agent_stream_payloads_append_task_logs`|
|`scheduler_command_and_metadata_helpers_cover_edge_cases`|`gateway_agent_scheduler::host_migration_5_tests::cases::scheduler_command_and_metadata_helpers_cover_edge_cases`|
|`scheduler_task_board_command_payload_errors_are_reported_before_store_mutation`|`gateway_agent_scheduler::host_migration_5_tests::cases::scheduler_task_board_command_payload_errors_are_reported_before_store_mutation`|
|`scheduler_unscoped_handlers_reject_invalid_or_missing_context`|`gateway_agent_scheduler::host_migration_5_tests::cases::scheduler_unscoped_handlers_reject_invalid_or_missing_context`|
|`scheduler_wake_concurrency_clamps_to_supported_bounds`|`gateway_agent_scheduler::host_migration_5_tests::cases::scheduler_wake_concurrency_clamps_to_supported_bounds`|
|`scheduler_status_helpers_distinguish_idempotent_and_slot_states`|`gateway_agent_scheduler::host_migration_5_tests::cases::scheduler_status_helpers_distinguish_idempotent_and_slot_states`|
|`scheduler_workspace_context_accepts_nested_and_legacy_metadata`|`gateway_agent_scheduler::host_migration_5_tests::cases::scheduler_workspace_context_accepts_nested_and_legacy_metadata`|
|`scheduler_metadata_fields_trim_strings_and_reject_numeric_overflow`|`gateway_agent_scheduler::host_migration_5_tests::cases::scheduler_metadata_fields_trim_strings_and_reject_numeric_overflow`|
|`scheduler_event_error_prefers_payload_and_requires_error_context_for_message`|`gateway_agent_scheduler::host_migration_5_tests::cases::scheduler_event_error_prefers_payload_and_requires_error_context_for_message`|
|`scheduler_time_and_merge_helpers_preserve_safety_boundaries`|`gateway_agent_scheduler::host_migration_5_tests::cases::scheduler_time_and_merge_helpers_preserve_safety_boundaries`|
|`task_pool_schedule_status_named_merge_does_not_execute_git`|`gateway_agent_scheduler::host_migration_5_tests::cases::task_pool_schedule_status_named_merge_does_not_execute_git`|

## 全夹具/符号映射

- `agent_scheduler_acceptance.rs::test_default_tenant_scope` → 领域原名；宿主薄验收也保留必要同名夹具。
- `agent_scheduler_acceptance.rs::agent_scheduler_create_task` → 领域原名。
- `agent_scheduler_acceptance.rs::agent_scheduler_list_tasks` → 领域原名。
- `agent_scheduler_acceptance.rs::agent_scheduler_wake` → 领域原名。
- `agent_scheduler_acceptance.rs::agent_scheduler_recovery` → 领域原名。
- `agent_scheduler_acceptance.rs::agent_scheduler_list_events` → 领域原名。
- `agent_scheduler_acceptance.rs::agent_scheduler_drain_outbox` → 领域原名。
- `agent_scheduler_acceptance.rs::agent_scheduler_create_task_with_gateway` → 领域原名。
- `agent_scheduler_acceptance.rs::agent_scheduler_wake_with_gateway` → 领域原名。
- `agent_scheduler_acceptance.rs::agent_scheduler_list_events_with_gateway` → 领域原名。
- `agent_scheduler_acceptance.rs::agent_scheduler_drain_outbox_with_gateway` → 领域原名。
- `agent_scheduler_acceptance.rs::FakeAgentSchedulerGateway` → 领域原名。
- `agent_scheduler_acceptance.rs::FakeAgentSchedulerState` → 领域原名。
- `agent_scheduler_acceptance.rs::FakeTaskRecord` → 领域原名。
- `agent_scheduler_acceptance.rs::FakeOutboxRecord` → 领域原名。
- `agent_scheduler_tests.rs::AcceptanceGatewayExecutor` → 领域原名。
- `agent_scheduler_tests.rs::acceptance_gateway_task` → 领域原名。
- `agent_scheduler_tests.rs::default_context` → 领域原名。
- `agent_scheduler_tests.rs::create_task_request` → 领域原名。
- `agent_scheduler_tests.rs::RecordingLocalExecutor` → 领域原名。
- `agent_scheduler_tests.rs::BlockingLocalExecutor` → 领域原名。
- `agent_scheduler_tests.rs::test_local_workspace` → 领域原名。
- `agent_scheduler_tests.rs::response_json` → 领域原名。
- `agent_scheduler_tests.rs::get_json` → 领域原名。
- `agent_scheduler_tests.rs::post_json` → 领域原名。
- `agent_scheduler_tests.rs::wait_for_log` → 领域原名。
- `agent_scheduler_tests.rs::wait_for_store_log` → 领域原名。
- `agent_scheduler_tests.rs::wait_for_store_artifact` → 领域原名。
- `agent_scheduler_tests.rs::encode_path_segment` → 领域原名。
- `agent_scheduler_tests.rs::task_graph_edge` → 领域原名。
- `agent_scheduler_tests.rs::task_graph_task` → 领域原名。
- `agent_scheduler_tests.rs::assert_task_graph_readiness` → 领域原名。
- `agent_scheduler_tests.rs::scheduler_project_id` → 领域原名；宿主薄验收也保留必要同名夹具。
- `agent_scheduler_tests.rs::init_merge_test_repo` → 领域原名；宿主薄验收也保留必要同名夹具。
- `agent_scheduler_tests.rs::register_merge_test_project` → 领域原名；宿主薄验收也保留必要同名夹具。
- `agent_scheduler_tests.rs::cleanup_merge_test_project` → 领域原名；宿主薄验收也保留必要同名夹具。
- `agent_scheduler_tests.rs::submitted_merge_task` → 领域原名。
- `agent_scheduler_tests.rs::post_merge_command` → 领域原名。
- `agent_scheduler_tests.rs::post_json_response` → 领域原名。
- `agent_scheduler_tests.rs::put_json_response` → 领域原名。
- `agent_scheduler_tests.rs::workflow_status_with_transitions` → 领域原名。
- `agent_scheduler_tests.rs::workflow_transition` → 领域原名。
- `agent_scheduler_tests.rs::run_test_git` → 领域原名；宿主薄验收也保留必要同名夹具。

## 未来验证

- `python3 scripts/cargo_slot.py cargo test -p gateway-agent-scheduler --lib host_migration_5_tests -- --test-threads=1`：静态预期 35 个，必须 ran > 0、failed=0。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib gateway::api::handlers::agent_scheduler::acceptance::agent_scheduler_tests -- --test-threads=1`：静态预期 2 个，仅表中保留两项。
- 同宿主过滤追加 `--list`，确认迁出的 35 项从宿主测试注册消失；不以零用例冒充通过。
- 当前所有 Cargo（含 list/check/fmt）、测试、实机、编译检查均未运行，未取得实际测试数或耗时收益。Cargo.lock 仅手工补入已有工作区依赖名，未来 Cargo 检验锁文件。
