# 来源复核186：src/cron/tests/scheduler

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/cron/tests/scheduler.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/cron/tests/scheduler.rs`。
- 快照 SHA256：`ef5c14833365391a9211f71de321f4a50135610083639638b60f315dea6d4e27`。源码变化后重新阅读。
- 快照词法属性 36 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/cron/scheduler.rs:337 | crates/vw-agent/src/cron/tests/scheduler.rs | mod tests | cfg(test) | chrono, tempfile, tokio, config_agent, config_channel_types | 未匹配→133 | 未发现 |

## 身份线索

- `run_job_command_uses_job_project_path_as_working_directory`
- `execute_job_with_retry_recovers_after_first_failure`
- `execute_job_with_retry_exhausts_attempts`
- `run_agent_job_returns_error_without_provider_key`
- `wb_schedule_feedback_security_rejects_readonly_agent`
- `run_agent_job_blocks_rate_limited`
- `full_access_agent_job_bypasses_readonly_security_gate`
- `run_agent_job_delivers_task_pool_jobs_with_trimmed_agent_and_default_model`
- `run_agent_job_reports_task_pool_delivery_failure_without_mutating_security_result`
- `process_due_jobs_marks_component_ok_even_when_idle`
- `process_due_jobs_failure_does_not_mark_component_unhealthy`
- `persist_job_result_records_run_and_reschedules_shell_job`
- `persist_job_result_success_deletes_one_shot`
- `persist_job_result_failure_disables_one_shot`
- `persist_job_result_success_deletes_one_shot_shell_job`
- `persist_job_result_failure_disables_one_shot_shell_job`
- `wb_schedule_feedback_persist_job_result_delivery_failure_non_best_effort_marks_error`
- `wb_schedule_feedback_persist_job_result_delivery_failure_best_effort_keeps_success`
- `persist_job_result_at_schedule_without_delete_after_run_is_not_deleted`
- `deliver_if_configured_handles_none_and_invalid_channel`
- `wb_schedule_feedback_rejects_each_unconfigured_known_channel`
- `deliver_announcement_sends_configured_mattermost_thread_message`
- `execute_and_persist_job_disables_invalid_shell_job`
- `deliver_if_configured_requires_channel_and_target_for_announce`
- `deliver_if_configured_treats_announce_mode_case_insensitively`
- `run_agent_job_rejects_missing_delegate_agent_before_provider_call`
- `run_agent_job_rejects_disabled_delegate_agent_before_provider_call`
- `wb_schedule_window_blocks_expired_manual_run`
- `wb_schedule_window_blocks_future_manual_run`
- `wb_schedule_window_queued_expiration_never_starts_shell`
- `cron_history_scheduled_run_is_visible_until_completion`
- `wb_schedule_feedback_unavailable_credentials_rejected`
- `wb_schedule_acceptance_manual_cron_run_preserves_next_run`
- `cron_wecom_delivery_none_makes_zero_webhook_requests`
- `cron_wecom_delivery_manual_and_due_shell_trigger_share_route_and_receipts`
- `persist_job_result_delivery_failure_wecom_respects_best_effort_for_shell_and_agent`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
