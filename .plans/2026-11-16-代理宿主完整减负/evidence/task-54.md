# 任务54：定时任务完整领域静态证据

代码完成，编译/行为未验证。本轮遵守公共计划，不运行 Cargo、测试、构建、实机或格式化工具。

来源共 2156 行 → 1618 行，净减 538 行（25.0%），未达到70%，须由收口验收。领域新增测试及夹具 663 行；lib.rs 注册增加3行。无新增依赖。

## 生产与依赖方向

生产调度、计划、存储、类型、活动运行及归并实现此前已属于 agent-cron，本轮不计为生产迁移收益。删除5个两行兼容文件，在 cron/mod.rs 直接导入真实领域模块；现有 cron 路径和调用者继续使用同一领域实现。依赖为 vw-agent → agent-cron → agent-config/agent-security/agent-health/session-port/channel-support，无反向依赖、include 或 path 挂载宿主。

scheduler::run/execute_job_now → agent_cron::scheduler 对应入口，注入 Host；Host 的任务池、网关流、通道动作仍接真实宿主。delivery::validate/validate_saved/send/apply_result → agent_cron::delivery；latest_config 读取真实 ChannelConfigWatcher，send_validated 使用真实认证连接和 webhook，均保留。删除无调用者 execute_job_with_retry 测试包装。

## 全文件映射与保留原因

|来源|迁移前行数|保留行数|职责/后续可迁项|
|---|---:|---:|---|
|active_runs.rs|2|0|删除纯转发，cron/mod.rs 直接引用 agent_cron 对应模块，无业务丢失。|
|active_runs_tests.rs|21|21|shell_barrier/release_shell 由真实宿主 HTTP 投递期间活动历史验收使用，仍保留。|
|consolidation/mod.rs|2|0|删除纯转发，cron/mod.rs 直接引用 agent_cron 对应模块，无业务丢失。|
|delivery.rs|71|71|删除纯转发，cron/mod.rs 直接引用 agent_cron 对应模块，无业务丢失。|
|delivery_tests.rs|214|214|fixture/tenant/save 使用真实租户配置存储、损坏密文、认证 socket 和 HTTP 回执。配置纯分支混在真实重载验收中，后续可单独拆分而不能丢失真实边界覆盖。|
|mod.rs|37|37|导入领域模块、保留对外 API/宿主 delivery 和 scheduler 接线及宿主验收注册。|
|schedule/mod.rs|2|0|删除纯转发，cron/mod.rs 直接引用 agent_cron 对应模块，无业务丢失。|
|scheduler.rs|357|348|保留 Host 的任务池落盘、网关流和通道实例装配，run/execute_job_now 真实入口及剩余边界测试包装；通道创建属于宿主端口，不伪造迁移。|
|scheduler_tests.rs|68|68|安全拒绝与不可用凭据的薄宿主契约用例保留，证明真实入口接线。|
|store.rs|2|0|删除纯转发，cron/mod.rs 直接引用 agent_cron 对应模块，无业务丢失。|
|tests/mod.rs|7|7|原有未挂载集合入口保留；有效调度测试由 scheduler.rs 直接挂载，不新增重复挂载。|
|tests/scheduler.rs|1207|688|21条领域用例迁到领域自注册模块；保留真实 provider/gateway、任务池、HTTP、凭据和回执验收。未来可把 best_effort 等混合用例的领域断言进一步拆开，但须保留原真实通道断言。|
|types/mod.rs|2|0|删除纯转发，cron/mod.rs 直接引用 agent_cron 对应模块，无业务丢失。|
|wecom_acceptance_tests.rs|164|164|StaticProvider/state_with_config/request 为真实 API 路由、租户、持久化、认证及 webhook 协议验收所需；不能复制 AppState 到领域库。|

生产/测试分项（按实际职责）：迁移前生产文件 475 行（含 scheduler 内 cfg(test) 包装），独立测试/夹具 1681 行；迁移后对应 456 / 1162 行。生产文件减少包含9行测试专用包装及10行纯转发，未宣称迁出新生产算法。

## 全用例映射

以下每条保持原测试名和断言；领域仅把显式 Host 参数接到拒绝意外外部调用的 TestHost。该端口不模拟鉴权/存储，领域使用真实 SecurityPolicy、临时目录及 SQLite；健康检查使用唯一组件名。环境变量守卫留在宿主，领域用例无环境变更。

- `crates/vw-agent/src/cron/delivery_tests.rs::cron_wecom_delivery_configuration_and_reload_fail_closed` → `原宿主 crates/vw-agent/src/cron/delivery_tests.rs::cron_wecom_delivery_configuration_and_reload_fail_closed`
- `crates/vw-agent/src/cron/delivery_tests.rs::cron_wecom_delivery_sends_on_authenticated_socket_and_rejects_old_credentials` → `原宿主 crates/vw-agent/src/cron/delivery_tests.rs::cron_wecom_delivery_sends_on_authenticated_socket_and_rejects_old_credentials`
- `crates/vw-agent/src/cron/delivery_tests.rs::cron_wecom_delivery_webhook_routes_and_checks_receipt` → `原宿主 crates/vw-agent/src/cron/delivery_tests.rs::cron_wecom_delivery_webhook_routes_and_checks_receipt`
- `crates/vw-agent/src/cron/scheduler_tests.rs::execute_job_now_preserves_security_rejection` → `原宿主 crates/vw-agent/src/cron/scheduler_tests.rs::execute_job_now_preserves_security_rejection`
- `crates/vw-agent/src/cron/scheduler_tests.rs::unavailable_channel_credentials_prevent_announcement_delivery` → `原宿主 crates/vw-agent/src/cron/scheduler_tests.rs::unavailable_channel_credentials_prevent_announcement_delivery`
- `crates/vw-agent/src/cron/tests/scheduler.rs::run_job_command_uses_job_project_path_as_working_directory` → `agent_cron::host_migration_54_tests::execution::run_job_command_uses_job_project_path_as_working_directory`
- `crates/vw-agent/src/cron/tests/scheduler.rs::execute_job_with_retry_recovers_after_first_failure` → `agent_cron::host_migration_54_tests::execution::execute_job_with_retry_recovers_after_first_failure`
- `crates/vw-agent/src/cron/tests/scheduler.rs::execute_job_with_retry_exhausts_attempts` → `agent_cron::host_migration_54_tests::execution::execute_job_with_retry_exhausts_attempts`
- `crates/vw-agent/src/cron/tests/scheduler.rs::run_agent_job_returns_error_without_provider_key` → `原宿主 crates/vw-agent/src/cron/tests/scheduler.rs::run_agent_job_returns_error_without_provider_key`
- `crates/vw-agent/src/cron/tests/scheduler.rs::wb_schedule_feedback_security_rejects_readonly_agent` → `agent_cron::host_migration_54_tests::execution::wb_schedule_feedback_security_rejects_readonly_agent`
- `crates/vw-agent/src/cron/tests/scheduler.rs::run_agent_job_blocks_rate_limited` → `agent_cron::host_migration_54_tests::execution::run_agent_job_blocks_rate_limited`
- `crates/vw-agent/src/cron/tests/scheduler.rs::full_access_agent_job_bypasses_readonly_security_gate` → `原宿主 crates/vw-agent/src/cron/tests/scheduler.rs::full_access_agent_job_bypasses_readonly_security_gate`
- `crates/vw-agent/src/cron/tests/scheduler.rs::run_agent_job_delivers_task_pool_jobs_with_trimmed_agent_and_default_model` → `原宿主 crates/vw-agent/src/cron/tests/scheduler.rs::run_agent_job_delivers_task_pool_jobs_with_trimmed_agent_and_default_model`
- `crates/vw-agent/src/cron/tests/scheduler.rs::run_agent_job_reports_task_pool_delivery_failure_without_mutating_security_result` → `原宿主 crates/vw-agent/src/cron/tests/scheduler.rs::run_agent_job_reports_task_pool_delivery_failure_without_mutating_security_result`
- `crates/vw-agent/src/cron/tests/scheduler.rs::process_due_jobs_marks_component_ok_even_when_idle` → `agent_cron::host_migration_54_tests::execution::process_due_jobs_marks_component_ok_even_when_idle`
- `crates/vw-agent/src/cron/tests/scheduler.rs::process_due_jobs_failure_does_not_mark_component_unhealthy` → `agent_cron::host_migration_54_tests::execution::process_due_jobs_failure_does_not_mark_component_unhealthy`
- `crates/vw-agent/src/cron/tests/scheduler.rs::persist_job_result_records_run_and_reschedules_shell_job` → `agent_cron::host_migration_54_tests::execution::persist_job_result_records_run_and_reschedules_shell_job`
- `crates/vw-agent/src/cron/tests/scheduler.rs::persist_job_result_success_deletes_one_shot` → `agent_cron::host_migration_54_tests::execution::persist_job_result_success_deletes_one_shot`
- `crates/vw-agent/src/cron/tests/scheduler.rs::persist_job_result_failure_disables_one_shot` → `agent_cron::host_migration_54_tests::execution::persist_job_result_failure_disables_one_shot`
- `crates/vw-agent/src/cron/tests/scheduler.rs::persist_job_result_success_deletes_one_shot_shell_job` → `agent_cron::host_migration_54_tests::execution::persist_job_result_success_deletes_one_shot_shell_job`
- `crates/vw-agent/src/cron/tests/scheduler.rs::persist_job_result_failure_disables_one_shot_shell_job` → `agent_cron::host_migration_54_tests::persistence::persist_job_result_failure_disables_one_shot_shell_job`
- `crates/vw-agent/src/cron/tests/scheduler.rs::wb_schedule_feedback_persist_job_result_delivery_failure_non_best_effort_marks_error` → `原宿主 crates/vw-agent/src/cron/tests/scheduler.rs::wb_schedule_feedback_persist_job_result_delivery_failure_non_best_effort_marks_error`
- `crates/vw-agent/src/cron/tests/scheduler.rs::wb_schedule_feedback_persist_job_result_delivery_failure_best_effort_keeps_success` → `原宿主 crates/vw-agent/src/cron/tests/scheduler.rs::wb_schedule_feedback_persist_job_result_delivery_failure_best_effort_keeps_success`
- `crates/vw-agent/src/cron/tests/scheduler.rs::persist_job_result_at_schedule_without_delete_after_run_is_not_deleted` → `agent_cron::host_migration_54_tests::persistence::persist_job_result_at_schedule_without_delete_after_run_is_not_deleted`
- `crates/vw-agent/src/cron/tests/scheduler.rs::deliver_if_configured_handles_none_and_invalid_channel` → `原宿主 crates/vw-agent/src/cron/tests/scheduler.rs::deliver_if_configured_handles_none_and_invalid_channel`
- `crates/vw-agent/src/cron/tests/scheduler.rs::wb_schedule_feedback_rejects_each_unconfigured_known_channel` → `原宿主 crates/vw-agent/src/cron/tests/scheduler.rs::wb_schedule_feedback_rejects_each_unconfigured_known_channel`
- `crates/vw-agent/src/cron/tests/scheduler.rs::deliver_announcement_sends_configured_mattermost_thread_message` → `原宿主 crates/vw-agent/src/cron/tests/scheduler.rs::deliver_announcement_sends_configured_mattermost_thread_message`
- `crates/vw-agent/src/cron/tests/scheduler.rs::execute_and_persist_job_disables_invalid_shell_job` → `agent_cron::host_migration_54_tests::persistence::execute_and_persist_job_disables_invalid_shell_job`
- `crates/vw-agent/src/cron/tests/scheduler.rs::deliver_if_configured_requires_channel_and_target_for_announce` → `agent_cron::host_migration_54_tests::persistence::deliver_if_configured_requires_channel_and_target_for_announce`
- `crates/vw-agent/src/cron/tests/scheduler.rs::deliver_if_configured_treats_announce_mode_case_insensitively` → `原宿主 crates/vw-agent/src/cron/tests/scheduler.rs::deliver_if_configured_treats_announce_mode_case_insensitively`
- `crates/vw-agent/src/cron/tests/scheduler.rs::run_agent_job_rejects_missing_delegate_agent_before_provider_call` → `agent_cron::host_migration_54_tests::persistence::run_agent_job_rejects_missing_delegate_agent_before_provider_call`
- `crates/vw-agent/src/cron/tests/scheduler.rs::run_agent_job_rejects_disabled_delegate_agent_before_provider_call` → `agent_cron::host_migration_54_tests::persistence::run_agent_job_rejects_disabled_delegate_agent_before_provider_call`
- `crates/vw-agent/src/cron/tests/scheduler.rs::wb_schedule_window_blocks_expired_manual_run` → `agent_cron::host_migration_54_tests::persistence::wb_schedule_window_blocks_expired_manual_run`
- `crates/vw-agent/src/cron/tests/scheduler.rs::wb_schedule_window_blocks_future_manual_run` → `agent_cron::host_migration_54_tests::persistence::wb_schedule_window_blocks_future_manual_run`
- `crates/vw-agent/src/cron/tests/scheduler.rs::wb_schedule_window_queued_expiration_never_starts_shell` → `agent_cron::host_migration_54_tests::persistence::wb_schedule_window_queued_expiration_never_starts_shell`
- `crates/vw-agent/src/cron/tests/scheduler.rs::cron_history_scheduled_run_is_visible_until_completion` → `原宿主 crates/vw-agent/src/cron/tests/scheduler.rs::cron_history_scheduled_run_is_visible_until_completion`
- `crates/vw-agent/src/cron/tests/scheduler.rs::wb_schedule_feedback_unavailable_credentials_rejected` → `原宿主 crates/vw-agent/src/cron/tests/scheduler.rs::wb_schedule_feedback_unavailable_credentials_rejected`
- `crates/vw-agent/src/cron/tests/scheduler.rs::wb_schedule_acceptance_manual_cron_run_preserves_next_run` → `agent_cron::host_migration_54_tests::persistence::wb_schedule_acceptance_manual_cron_run_preserves_next_run`
- `crates/vw-agent/src/cron/tests/scheduler.rs::cron_wecom_delivery_none_makes_zero_webhook_requests` → `原宿主 crates/vw-agent/src/cron/tests/scheduler.rs::cron_wecom_delivery_none_makes_zero_webhook_requests`
- `crates/vw-agent/src/cron/tests/scheduler.rs::cron_wecom_delivery_manual_and_due_shell_trigger_share_route_and_receipts` → `原宿主 crates/vw-agent/src/cron/tests/scheduler.rs::cron_wecom_delivery_manual_and_due_shell_trigger_share_route_and_receipts`
- `crates/vw-agent/src/cron/tests/scheduler.rs::persist_job_result_delivery_failure_wecom_respects_best_effort_for_shell_and_agent` → `原宿主 crates/vw-agent/src/cron/tests/scheduler.rs::persist_job_result_delivery_failure_wecom_respects_best_effort_for_shell_and_agent`
- `crates/vw-agent/src/cron/wecom_acceptance_tests.rs::wecom_delivery_acceptance_saved_api_jobs_reload_and_receipts` → `原宿主 crates/vw-agent/src/cron/wecom_acceptance_tests.rs::wecom_delivery_acceptance_saved_api_jobs_reload_and_receipts`

无其他来源隐藏测试；active_runs/schedule/types/store/consolidation 两行重导出均无本地用例。任务1既有领域测试不重复搬迁、不计本轮收益。

## 静态与未来验证

已静态核对21条迁移名称唯一、其余原用例仍在宿主、无领域反向宿主引用；git diff --check 无空白错误。所有编译、行为、测试 list、实机检查均未运行，不宣称通过或编译加速。

未来执行：
- `python3 scripts/cargo_slot.py cargo test -p agent-cron --lib host_migration_54_tests -- --test-threads=1`，预期21条，必须 failed=0。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib cron:: -- --list`，确认上述21条原用例消失，保留清单中的真实边界用例非零。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib cron:: -- --test-threads=1`，按上述保留完整名称逐项核对，不将零测试当通过。

不足70%的原因逐文件见上表：保留的大文件主要是真实认证、通道、网关和任务池边界。继续可迁项是混合 best_effort/配置校验/活动快照用例的纯领域分支，须另行拆分且保持真实边界断言；本轮未为行数删除断言。
