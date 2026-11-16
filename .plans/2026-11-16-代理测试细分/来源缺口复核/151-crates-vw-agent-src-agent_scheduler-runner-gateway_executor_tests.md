# 来源复核151：src/agent_scheduler/runner/gateway_executor_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs`。
- 快照 SHA256：`564492d703682681fd230b6c4fd113f68c5096246859a6a10000d27851c7a241`。源码变化后重新阅读。
- 快照词法属性 27 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/agent_scheduler/runner/gateway_executor.rs:80 | crates/vw-agent/src/agent_scheduler/runner/gateway_executor_tests.rs | mod gateway_executor_tests | cfg(not(target_arch="wasm32")); cfg(test) | anyhow, parking_lot, serde_json, vw_api_transport, vw_api_task, vw_api_acp_model | 未匹配→133 | 未发现 |

## 身份线索

- `gateway_project_path_normalization_handles_platform_separators_and_edges`
- `gateway_delta_logs_trim_limit_and_truncate_output`
- `gateway_stream_payload_logs_cover_lifecycle_and_error_events`
- `gateway_stream_payload_logs_ignore_empty_delta_and_truncate_step_fields`
- `acp_background_approval_scheduler_keeps_session_isolation`
- `acp_background_approval_scheduler_writes_only_inside_worktree`
- `gateway_stream_request_without_agent_or_acp_keeps_minimal_options`
- `gateway_stream_request_preserves_reasoning_effort_without_acp`
- `gateway_stream_payload_task_log_appends_each_message_with_run_context`
- `gateway_stream_payload_task_log_noops_when_payload_has_no_messages`
- `gateway_executor_workspace_methods_fail_before_pool_io_for_unregistered_project`
- `gateway_executor_execute_collects_stream_output_and_appends_observed_logs`
- `gateway_executor_execute_propagates_stream_collection_error`
- `gateway_project_lookup_matches_registered_worktree_and_sandbox_paths`
- `gateway_worktree_pool_inner_helpers_map_successful_pool_responses`
- `gateway_worktree_pool_inner_helpers_report_unusable_pool_responses`
- `gateway_real_chat_client_is_disabled_in_unit_tests`
- `gateway_future_runner_propagates_success_and_error_without_runtime`
- `gateway_future_runner_works_inside_existing_runtime`
- `task_pool_workspace_non_git_uses_registered_directory`
- `task_pool_workspace_rejects_missing_or_foreign_directory`
- `task_pool_agent_does_not_elevate_background_permissions`
- `task_pool_agent_runs_use_distinct_sessions`
- `task_pool_recovery_gateway_passes_stop_to_stream`
- `task_pool_worktree_created_only_by_explicit_tool`
- `task_pool_worktree_non_git_rejected_without_side_effects`
- `task_pool_worktree_failed_run_preserves_changes`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
