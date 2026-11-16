# 任务55：代理调度完整领域迁移

代码完成，编译/行为未验证。按用户要求未运行任何 Cargo、构建、单元或实机测试。

## 归属与接线

实际接收库为 task-scheduler（源入口直接调用），不是 agent-scheduler-core（后者是周期闭包调度器）。本次没有生产算法迁移收益：原有生产编排已在 task-scheduler；GatewayExecutorHost 的真实实现继续调用网关流、工作树池及 AgentContextHost。领域自注册 lib.rs → host_migration_55_tests → 三个本地模块；宿主不再挂载 integration_tests/local_e2e_tests，网关规则测试也仅在领域编译。无反向 vw-agent 依赖、无 path/include 宿主源码。

新增 dev 依赖 session-info、session-ui-types 和 parking_lot，只用于原样迁入的夹具；数据库、项目存储、shell 与生命周期入口改为其既有库入口。

## 行数与保留理由

原清单宿主 1801 行（生产/文档/转发 170，测试 1631），现 654 行；净减 1147 行，63.7%。新领域测试 1202 行；生产算法新增 0 行。统计 splitlines；生产项包含文档、转发、cfg(test)导入，并非真实业务收益。

净减不足70%，以下为逐文件具体原因和继续可迁项，交收口任务核对：

- `crates/vw-agent/src/agent_scheduler/policy.rs`：2 → 2 行。既有兼容导出/模块入口，真实调用者仍经 agent_scheduler 公共路径访问；本轮未计生产迁移收益。
- `crates/vw-agent/src/agent_scheduler/state_machine.rs`：3 → 3 行。既有兼容导出/模块入口，真实调用者仍经 agent_scheduler 公共路径访问；本轮未计生产迁移收益。
- `crates/vw-agent/src/agent_scheduler/task_graph.rs`：2 → 2 行。既有兼容导出/模块入口，真实调用者仍经 agent_scheduler 公共路径访问；本轮未计生产迁移收益。
- `crates/vw-agent/src/agent_scheduler/events.rs`：3 → 3 行。既有兼容导出/模块入口，真实调用者仍经 agent_scheduler 公共路径访问；本轮未计生产迁移收益。
- `crates/vw-agent/src/agent_scheduler/lifecycle_bridge.rs`：2 → 2 行。既有兼容导出/模块入口，真实调用者仍经 agent_scheduler 公共路径访问；本轮未计生产迁移收益。
- `crates/vw-agent/src/agent_scheduler/task_board.rs`：3 → 3 行。既有兼容导出/模块入口，真实调用者仍经 agent_scheduler 公共路径访问；本轮未计生产迁移收益。
- `crates/vw-agent/src/agent_scheduler/domain.rs`：3 → 3 行。既有兼容导出/模块入口，真实调用者仍经 agent_scheduler 公共路径访问；本轮未计生产迁移收益。
- `crates/vw-agent/src/agent_scheduler/mod.rs`：33 → 23 行。既有兼容导出/模块入口，真实调用者仍经 agent_scheduler 公共路径访问；本轮未计生产迁移收益。
- `crates/vw-agent/src/agent_scheduler/artifacts.rs`：3 → 3 行。既有兼容导出/模块入口，真实调用者仍经 agent_scheduler 公共路径访问；本轮未计生产迁移收益。
- `crates/vw-agent/src/agent_scheduler/local_e2e_tests.rs`：195 → 0 行。领域测试全组迁出，旧挂载已移除。
- `crates/vw-agent/src/agent_scheduler/integration_tests.rs`：445 → 0 行。领域测试全组迁出，旧挂载已移除。
- `crates/vw-agent/src/agent_scheduler/outbox_dispatcher.rs`：2 → 2 行。既有兼容导出/模块入口，真实调用者仍经 agent_scheduler 公共路径访问；本轮未计生产迁移收益。
- `crates/vw-agent/src/agent_scheduler/runner/local.rs`：2 → 2 行。既有兼容导出/模块入口，真实调用者仍经 agent_scheduler 公共路径访问；本轮未计生产迁移收益。
- `crates/vw-agent/src/agent_scheduler/runner/workspace_claim.rs`：2 → 2 行。既有兼容导出/模块入口，真实调用者仍经 agent_scheduler 公共路径访问；本轮未计生产迁移收益。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs`：991 → 496 行。保留真实 ACP 文件授权、具体执行器上下文及 Enter/ExitWorktree 工具集成验收；详见逐用例表。继续可迁项为执行器流采集/取消测试中的领域断言，需先拆分与真实 AgentContextHost 的上下文作用域契约，不能用虚假权限端口替代。
- `crates/vw-agent/src/agent_scheduler/runner/mod.rs`：8 → 8 行。既有兼容导出/模块入口，真实调用者仍经 agent_scheduler 公共路径访问；本轮未计生产迁移收益。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor.rs`：80 → 78 行。真实网关流、工具上下文及工作树池组合端口；保留 SchedulerGatewayHost、RealGatewayChatStreamClient 和具体执行器别名。
- `crates/vw-agent/src/agent_scheduler/resources/mod.rs`：3 → 3 行。既有兼容导出/模块入口，真实调用者仍经 agent_scheduler 公共路径访问；本轮未计生产迁移收益。
- `crates/vw-agent/src/agent_scheduler/resources/worktree.rs`：3 → 3 行。既有兼容导出/模块入口，真实调用者仍经 agent_scheduler 公共路径访问；本轮未计生产迁移收益。
- `crates/vw-agent/src/agent_scheduler/scheduler/mod.rs`：2 → 2 行。既有兼容导出/模块入口，真实调用者仍经 agent_scheduler 公共路径访问；本轮未计生产迁移收益。
- `crates/vw-agent/src/agent_scheduler/adapters/mod.rs`：3 → 3 行。既有兼容导出/模块入口，真实调用者仍经 agent_scheduler 公共路径访问；本轮未计生产迁移收益。
- `crates/vw-agent/src/agent_scheduler/engine/dispatch.rs`：2 → 2 行。既有兼容导出/模块入口，真实调用者仍经 agent_scheduler 公共路径访问；本轮未计生产迁移收益。
- `crates/vw-agent/src/agent_scheduler/engine/recovery.rs`：2 → 2 行。既有兼容导出/模块入口，真实调用者仍经 agent_scheduler 公共路径访问；本轮未计生产迁移收益。
- `crates/vw-agent/src/agent_scheduler/engine/mod.rs`：5 → 5 行。既有兼容导出/模块入口，真实调用者仍经 agent_scheduler 公共路径访问；本轮未计生产迁移收益。
- `crates/vw-agent/src/agent_scheduler/store/mod.rs`：2 → 2 行。既有兼容导出/模块入口，真实调用者仍经 agent_scheduler 公共路径访问；本轮未计生产迁移收益。

## 全用例映射

原 32 条 = 领域 23 条 + 宿主 9 条，名称及断言保留；ACP 用例继续 unix/ignore 限制。

- `crates/vw-agent/src/agent_scheduler/local_e2e_tests.rs` / `task_pool_acceptance_failure_and_retry_keep_workspace_ownership` → `crates/libs/task-scheduler/src/host_migration_55_tests/local_e2e_tests.rs` / `task_pool_acceptance_failure_and_retry_keep_workspace_ownership`
- `crates/vw-agent/src/agent_scheduler/local_e2e_tests.rs` / `task_pool_acceptance_explicit_actions_and_agent_identity_are_preserved` → `crates/libs/task-scheduler/src/host_migration_55_tests/local_e2e_tests.rs` / `task_pool_acceptance_explicit_actions_and_agent_identity_are_preserved`
- `crates/vw-agent/src/agent_scheduler/integration_tests.rs` / `agent_scheduler_requires_same_scope_parent_project` → `crates/libs/task-scheduler/src/host_migration_55_tests/integration_tests.rs` / `agent_scheduler_requires_same_scope_parent_project`
- `crates/vw-agent/src/agent_scheduler/integration_tests.rs` / `task_pool_recovery_resources_wait_for_live_directory_owner` → `crates/libs/task-scheduler/src/host_migration_55_tests/integration_tests.rs` / `task_pool_recovery_resources_wait_for_live_directory_owner`
- `crates/vw-agent/src/agent_scheduler/integration_tests.rs` / `task_pool_recovery_old_lease_cannot_taint_new_owner` → `crates/libs/task-scheduler/src/host_migration_55_tests/integration_tests.rs` / `task_pool_recovery_old_lease_cannot_taint_new_owner`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_project_path_normalization_handles_platform_separators_and_edges` → `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs` / `gateway_project_path_normalization_handles_platform_separators_and_edges`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_delta_logs_trim_limit_and_truncate_output` → `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs` / `gateway_delta_logs_trim_limit_and_truncate_output`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_stream_payload_logs_cover_lifecycle_and_error_events` → `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs` / `gateway_stream_payload_logs_cover_lifecycle_and_error_events`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_stream_payload_logs_ignore_empty_delta_and_truncate_step_fields` → `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs` / `gateway_stream_payload_logs_ignore_empty_delta_and_truncate_step_fields`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `acp_background_approval_scheduler_keeps_session_isolation` → `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs` / `acp_background_approval_scheduler_keeps_session_isolation`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `acp_background_approval_scheduler_writes_only_inside_worktree` → `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `acp_background_approval_scheduler_writes_only_inside_worktree`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_stream_request_without_agent_or_acp_keeps_minimal_options` → `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs` / `gateway_stream_request_without_agent_or_acp_keeps_minimal_options`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_stream_request_preserves_reasoning_effort_without_acp` → `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs` / `gateway_stream_request_preserves_reasoning_effort_without_acp`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_stream_payload_task_log_appends_each_message_with_run_context` → `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs` / `gateway_stream_payload_task_log_appends_each_message_with_run_context`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_stream_payload_task_log_noops_when_payload_has_no_messages` → `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs` / `gateway_stream_payload_task_log_noops_when_payload_has_no_messages`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_executor_workspace_methods_fail_before_pool_io_for_unregistered_project` → `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_executor_workspace_methods_fail_before_pool_io_for_unregistered_project`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_executor_execute_collects_stream_output_and_appends_observed_logs` → `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_executor_execute_collects_stream_output_and_appends_observed_logs`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_executor_execute_propagates_stream_collection_error` → `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_executor_execute_propagates_stream_collection_error`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_project_lookup_matches_registered_worktree_and_sandbox_paths` → `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs` / `gateway_project_lookup_matches_registered_worktree_and_sandbox_paths`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_worktree_pool_inner_helpers_map_successful_pool_responses` → `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs` / `gateway_worktree_pool_inner_helpers_map_successful_pool_responses`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_worktree_pool_inner_helpers_report_unusable_pool_responses` → `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs` / `gateway_worktree_pool_inner_helpers_report_unusable_pool_responses`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_real_chat_client_is_disabled_in_unit_tests` → `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_real_chat_client_is_disabled_in_unit_tests`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_future_runner_propagates_success_and_error_without_runtime` → `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs` / `gateway_future_runner_propagates_success_and_error_without_runtime`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_future_runner_works_inside_existing_runtime` → `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs` / `gateway_future_runner_works_inside_existing_runtime`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `task_pool_workspace_non_git_uses_registered_directory` → `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs` / `task_pool_workspace_non_git_uses_registered_directory`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `task_pool_workspace_rejects_missing_or_foreign_directory` → `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs` / `task_pool_workspace_rejects_missing_or_foreign_directory`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `task_pool_agent_does_not_elevate_background_permissions` → `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs` / `task_pool_agent_does_not_elevate_background_permissions`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `task_pool_agent_runs_use_distinct_sessions` → `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs` / `task_pool_agent_runs_use_distinct_sessions`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `task_pool_recovery_gateway_passes_stop_to_stream` → `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `task_pool_recovery_gateway_passes_stop_to_stream`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `task_pool_worktree_created_only_by_explicit_tool` → `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `task_pool_worktree_created_only_by_explicit_tool`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `task_pool_worktree_non_git_rejected_without_side_effects` → `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `task_pool_worktree_non_git_rejected_without_side_effects`
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `task_pool_worktree_failed_run_preserves_changes` → `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `task_pool_worktree_failed_run_preserves_changes`

## 全符号/夹具映射

- `crates/vw-agent/src/agent_scheduler/local_e2e_tests.rs` / `RecordingExecutor`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/local_e2e_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/local_e2e_tests.rs` / `prepare_git_workspace`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/local_e2e_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/local_e2e_tests.rs` / `finite_task`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`, `crates/libs/task-scheduler/src/host_migration_55_tests/local_e2e_tests.rs`, `crates/libs/task-scheduler/src/host_migration_55_tests/integration_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/local_e2e_tests.rs` / `AcceptanceExecutor`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/local_e2e_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/local_e2e_tests.rs` / `task_pool_acceptance_failure_and_retry_keep_workspace_ownership`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/local_e2e_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/local_e2e_tests.rs` / `task_pool_acceptance_explicit_actions_and_agent_identity_are_preserved`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/local_e2e_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/integration_tests.rs` / `GLOBAL_SCOPED_TABLES`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/integration_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/integration_tests.rs` / `GLOBAL_APPLICATION_TABLES`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/integration_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/integration_tests.rs` / `agent_scheduler_requires_same_scope_parent_project`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/integration_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/integration_tests.rs` / `assert_scheduler_requires_same_scope_parent_project`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/integration_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/integration_tests.rs` / `isolation_session`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/integration_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/integration_tests.rs` / `isolation_agent_session`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/integration_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/integration_tests.rs` / `append_lifecycle_event`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/integration_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/integration_tests.rs` / `finite_task`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`, `crates/libs/task-scheduler/src/host_migration_55_tests/local_e2e_tests.rs`, `crates/libs/task-scheduler/src/host_migration_55_tests/integration_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/integration_tests.rs` / `assert_run_persisted`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/integration_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/integration_tests.rs` / `assert_outbox_delivered`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/integration_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/integration_tests.rs` / `isolation_project_info`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/integration_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/integration_tests.rs` / `isolation_task`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/integration_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/integration_tests.rs` / `assert_project_local_databases_absent`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/integration_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/integration_tests.rs` / `assert_global_database_permissions`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/integration_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/integration_tests.rs` / `assert_global_database_permissions`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/integration_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/integration_tests.rs` / `TestClock`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/integration_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/integration_tests.rs` / `task_pool_recovery_resources_wait_for_live_directory_owner`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/integration_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/integration_tests.rs` / `task_pool_recovery_old_lease_cannot_taint_new_owner`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/integration_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_project_path_normalization_handles_platform_separators_and_edges`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_delta_logs_trim_limit_and_truncate_output`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_stream_payload_logs_cover_lifecycle_and_error_events`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_stream_payload_logs_ignore_empty_delta_and_truncate_step_fields`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `acp_background_approval_scheduler_keeps_session_isolation`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `acp_background_approval_scheduler_writes_only_inside_worktree`：保留宿主真实边界或其专用夹具。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_stream_request_without_agent_or_acp_keeps_minimal_options`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_stream_request_preserves_reasoning_effort_without_acp`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_stream_payload_task_log_appends_each_message_with_run_context`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_stream_payload_task_log_noops_when_payload_has_no_messages`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_executor_workspace_methods_fail_before_pool_io_for_unregistered_project`：保留宿主真实边界或其专用夹具。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_executor_execute_collects_stream_output_and_appends_observed_logs`：保留宿主真实边界或其专用夹具。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_executor_execute_propagates_stream_collection_error`：保留宿主真实边界或其专用夹具。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_project_lookup_matches_registered_worktree_and_sandbox_paths`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_worktree_pool_inner_helpers_map_successful_pool_responses`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_worktree_pool_inner_helpers_report_unusable_pool_responses`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_real_chat_client_is_disabled_in_unit_tests`：保留宿主真实边界或其专用夹具。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_future_runner_propagates_success_and_error_without_runtime`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `gateway_future_runner_works_inside_existing_runtime`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `execution_request`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `workspace_request`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `release_request`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `recycle_request`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `project_info`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `finite_task`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`, `crates/libs/task-scheduler/src/host_migration_55_tests/local_e2e_tests.rs`, `crates/libs/task-scheduler/src/host_migration_55_tests/integration_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `FakeChatStream`：保留宿主真实边界或其专用夹具。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `pop_stream_result`：保留宿主真实边界或其专用夹具。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `FakeWorktreePool`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `pop_pool_result`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `task_pool_workspace_non_git_uses_registered_directory`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `task_pool_workspace_rejects_missing_or_foreign_directory`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `task_pool_agent_does_not_elevate_background_permissions`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `task_pool_agent_runs_use_distinct_sessions`：迁至 `crates/libs/task-scheduler/src/host_migration_55_tests/gateway_tests.rs`。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `task_pool_recovery_gateway_passes_stop_to_stream`：保留宿主真实边界或其专用夹具。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `exercise_task_pool_worktree`：保留宿主真实边界或其专用夹具。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `task_pool_worktree_created_only_by_explicit_tool`：保留宿主真实边界或其专用夹具。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `task_pool_worktree_non_git_rejected_without_side_effects`：保留宿主真实边界或其专用夹具。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` / `task_pool_worktree_failed_run_preserves_changes`：保留宿主真实边界或其专用夹具。
- `crates/vw-agent/src/agent_scheduler/runner/gateway_executor.rs` / `RealGatewayChatStreamClient`：保留宿主真实边界或其专用夹具。

## 未来验证

`python3 scripts/cargo_slot.py cargo test -p task-scheduler --lib host_migration_55_tests -- --test-threads=1`，静态预期 23 条非零测试。

`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent_scheduler::runner::gateway_executor::gateway_executor_tests -- --test-threads=1`，静态预期9条（其中1条unix且ignored）。逐条真实完整名称为该前缀加上表中保留用例名称。

未来宿主 --list 应不再出现旧 integration_tests/local_e2e_tests 和上表已迁出网关规则测试；零测试不能作为通过证据。本次仅核对原用例一一唯一归属、模块注册、依赖方向及 git diff --check；所有编译、行为、性能、实机与 PostgreSQL 检查均未运行。

## 专项复审追加（覆盖上述旧行数及“执行器契约尚待拆分”的结论）

将执行器领域契约及脚本流夹具继续迁至 `crates/libs/task-scheduler/src/gateway_executor_contract_tests.rs`（257行），使用已有 `test-support` 特性注册。四条原宿主测试保留原完整名称，函数体仅传入真实 `SchedulerGatewayHost` 调用领域契约；不是新增模拟宿主，没有复制 App 状态或权限系统。领域契约实际执行 `GatewayLocalAgentExecutor<H>`，生产入口及权限语义保持原样。

逐符号最终映射：
- `gateway_executor_workspace_methods_fail_before_pool_io_for_unregistered_project` → 同名领域泛型契约，宿主原测试薄调用。
- `gateway_executor_execute_collects_stream_output_and_appends_observed_logs` → 同名领域泛型契约，宿主原测试薄调用。
- `gateway_executor_execute_propagates_stream_collection_error` → 同名领域泛型契约，宿主原测试薄调用。
- `task_pool_recovery_gateway_passes_stop_to_stream` → 同名领域泛型契约（含 CancelStream），宿主原测试薄调用。
- `execution_request`、`finite_task` → 领域测试支持夹具，宿主真实 ACP/工具集成测试复用，不复制。
- `FakeChatStream`、`pop_stream_result` → 领域私有脚本流夹具，仅替代流传输端口。改用 std Mutex 并显式处理锁中毒，未增加生产依赖。

最终原清单宿主1801 → 421行，净减1380行（76.6%），本轮额外减233行。宿主测试496 → 263行；其余原清单文件行数沿用上表。领域原自注册测试1202行，新增test-support契约/夹具257行，均非生产算法收益。宿主当前保留生产/文档/转发158行及测试263行。

保留宿主逐项职责：四个泛型契约薄调用证明真实 SchedulerGatewayHost 接线；`gateway_real_chat_client_is_disabled_in_unit_tests` 证明实际网关端口测试隔离；`acp_background_approval_scheduler_writes_only_inside_worktree` 必须跨真实配置、ACP协议及文件授权边界；三个 `task_pool_worktree_*` 用例及 `exercise_task_pool_worktree` 验证真实 Enter/ExitWorktree 工具、项目实例作用域及权限/删除预检边界。继续抽离这些真实工具调用需要其所有者契约支持，本轮不新建测试专用泛化端口或复制权限状态。

静态核对：四条契约仍使用同一真实执行器、同一原断言；原宿主文件及迁出契约的 assert/assert_eq/assert_ne 总数保持一致（44）；9条宿主测试注册名称不变。新增模块只由 test-support 注册，无 vw-agent 反向依赖。未来仍执行上文23条领域自注册测试及9条宿主测试（1条unix/ignored）；后者实际调用四条新领域契约，不能用库过滤零测试替代其验证。新增模块在普通生产依赖构建中不注册。

本次所有 Cargo、构建、单元、实机测试仍未运行；仅源码、断言数量/完整用例映射、注册与差异静态核对。
