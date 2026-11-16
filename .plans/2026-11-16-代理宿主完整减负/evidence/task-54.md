# 任务54：定时任务完整领域静态证据

首轮记录保留供比较；以下“深度恢复”更新覆盖旧的当前数量、保留原因和未来测试数。

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

## 深度恢复：真实通道路由与完整断言迁移

已读取 final.md 与 remaining-host.json 所列 owner=54 的1618行残留。第二轮迁出 announcement::deliver_announcement 原有生产分支（大小写归一化、企微分流、不可用凭据拒绝、六种配置/发送器选择和发送、未知通道拒绝，以及原WASM行为）。宿主原入口现在只注入 Host 并调用领域函数；真实 scheduler Host::deliver_announcement 及现有调用者均经过该入口，因此不是新增死实现。领域直接调用现有 channel-* 实现，不模拟HTTP发送。

agent-cron 新增原生目标通道依赖及测试配置类型依赖、tokio net/io-util 测试能力；Cargo.lock 仅补本包直接依赖。各通道清单未见 agent-cron/vw-agent 反向依赖。企微最新租户配置读取、认证会话解析及发送仍是 DeliveryHost。

迁出 shell_barrier/release_shell 和真实HTTP完整读取夹具，删除旧挂载；删除 execute_and_persist_job 宿主测试包装和无调用者 is_wecom 重导出。新测试由 host_migration_54_tests::announcement 自注册；TestHost 普通投递调用真实新生产入口，其余外部端口仍对意外调用 panic。原8条完整函数名和断言保留，只有路径/Host 注入改变，无用例删减。

- `cron::scheduler::tests::wb_schedule_feedback_persist_job_result_delivery_failure_non_best_effort_marks_error` → `agent_cron::host_migration_54_tests::announcement::wb_schedule_feedback_persist_job_result_delivery_failure_non_best_effort_marks_error`
- `cron::scheduler::tests::wb_schedule_feedback_persist_job_result_delivery_failure_best_effort_keeps_success` → `agent_cron::host_migration_54_tests::announcement::wb_schedule_feedback_persist_job_result_delivery_failure_best_effort_keeps_success`
- `cron::scheduler::tests::deliver_if_configured_handles_none_and_invalid_channel` → `agent_cron::host_migration_54_tests::announcement::deliver_if_configured_handles_none_and_invalid_channel`
- `cron::scheduler::tests::wb_schedule_feedback_rejects_each_unconfigured_known_channel` → `agent_cron::host_migration_54_tests::announcement::wb_schedule_feedback_rejects_each_unconfigured_known_channel`
- `cron::scheduler::tests::deliver_announcement_sends_configured_mattermost_thread_message` → `agent_cron::host_migration_54_tests::announcement::deliver_announcement_sends_configured_mattermost_thread_message`
- `cron::scheduler::tests::deliver_if_configured_treats_announce_mode_case_insensitively` → `agent_cron::host_migration_54_tests::announcement::deliver_if_configured_treats_announce_mode_case_insensitively`
- `cron::scheduler::tests::cron_history_scheduled_run_is_visible_until_completion` → `agent_cron::host_migration_54_tests::announcement::cron_history_scheduled_run_is_visible_until_completion`
- `cron::scheduler::tests::wb_schedule_feedback_unavailable_credentials_rejected` → `agent_cron::host_migration_54_tests::announcement::wb_schedule_feedback_unavailable_credentials_rejected`

更新宿主物理行数：2156 → 1618 → 1131，累计减少 1025 行（47.5%）。当前生产文件 310 行（含测试包装），独立测试 821 行。新生产 announcement.rs 144 行；新增领域公告测试/夹具 342 行。

当前逐文件保留：

|文件|行数|真实职责|
|---|---:|---|
|delivery.rs|71|租户配置watcher、已认证企微连接和真实webhook；薄领域入口|
|delivery_tests.rs|214|真实配置密文失败、认证socket与webhook回执|
|mod.rs|35|领域API导入及真实宿主入口/验收注册|
|scheduler.rs|204|任务池对象创建与落盘、网关流调用、Host实现、入口和剩余宿主测试包装|
|scheduler_tests.rs|68|薄宿主安全拒绝与不可用凭据契约|
|tests/mod.rs|7|原未挂载测试集合入口|
|tests/scheduler.rs|368|真实网关无密钥、任务池落盘/失败、企微实时配置/回执与历史验收|
|wecom_acceptance_tests.rs|164|真实API路由/租户/保存配置/回执跨模块验收|

仍未达70%减量；主要剩余是上述真实宿主验收，混合领域断言已继续抽出普通发送器/持久化/活动历史整组。未将真实租户配置、网关、任务池或认证会话夹具复制到领域库。收口须继续审查真实薄边界，不将本证据视为整轮门槛通过。

静态核对：8条新迁出名称唯一，其余原测试名仍存续，旧active_runs_tests挂载及execute_and_persist_job宿主包装消失，git diff --check 无空白错误。未来同一 `python3 scripts/cargo_slot.py cargo test -p agent-cron --lib host_migration_54_tests -- --test-threads=1` 预期29条，须取得非零且 failed=0；宿主按上表剩余原测试名运行。所有 Cargo、编译、构建、测试及实机检查仍明确未运行。

## 第三轮恢复：真实任务池与企微策略

重读final.md和remaining-host.json。确认app.rs的task只是task_file_store::bootstrap_task导出，因此迁移完整任务池适配器到agent-cron::task_pool；scheduler::run_agent_job现在直接调用它。删除SchedulerHost::deliver_agent_job_to_task_pool及所有实现，消除非必要Host端口。保持原Task::new(999)、spawn_blocking、prompt/model/agent/acp字段、SQLite create_task、join错误及成功文本。任务池agent/acp归一化位于execution::task_pool_agents，真实适配器消费其强类型结果。新增依赖task-file-store及测试依赖config-runtime-paths，Cargo.lock同步仅本包条目。

原两条任务池测试完整迁移，继续验证真实SQLite落盘和文件路径错误；新增trim与空白默认main断言。best_effort的Shell/Agent四组合完整迁移到wecom_policy，配置读取端口提供快照，其余校验/投递错误摘要/持久化调用真实领域实现。宿主另保留uses_saved_credentials_instead_of_launch_snapshot：启动快照有旧凭据，真实已保存配置无凭据，断言失败、历史error、Bot ID错误及旧密钥不泄漏，不用假端口冒充配置重载。

原configuration_and_reload_fail_closed纯配置循环迁移为独立领域测试；损坏密文/保存/重载/损坏JSON原断言仍留宿主。none模式用例保留原名和真实TcpListener超时零请求断言，移除只供宿主发送器使用的测试endpoint包裹，此模式不应进入发送器。删除失去调用者的宿主deliver_if_configured包装。

本轮全用例映射：
- `run_agent_job_delivers_task_pool_jobs_with_trimmed_agent_and_default_model` → `agent_cron::host_migration_54_tests::task_pool::run_agent_job_delivers_task_pool_jobs_with_trimmed_agent_and_default_model`。
- `run_agent_job_reports_task_pool_delivery_failure_without_mutating_security_result` → `agent_cron::host_migration_54_tests::task_pool::run_agent_job_reports_task_pool_delivery_failure_without_mutating_security_result`。
- `persist_job_result_delivery_failure_wecom_respects_best_effort_for_shell_and_agent` → `agent_cron::host_migration_54_tests::wecom_policy::persist_job_result_delivery_failure_wecom_respects_best_effort_for_shell_and_agent`。
- `cron_wecom_delivery_none_makes_zero_webhook_requests` → `agent_cron::host_migration_54_tests::wecom_policy::cron_wecom_delivery_none_makes_zero_webhook_requests`。
- `cron_wecom_delivery_configuration_validation_fails_closed` → `agent_cron::host_migration_54_tests::wecom_policy::cron_wecom_delivery_configuration_validation_fails_closed`。
- 原配置重载测试内纯校验循环 → wecom_policy::cron_wecom_delivery_configuration_validation_fails_closed；原真实读取/密文断言保留。
- 原best_effort宿主读取边界 → cron::scheduler::tests::persist_job_result_uses_saved_credentials_instead_of_launch_snapshot；四组合原名现属领域。
- 新领域 execution::task_pool_agents_trim_names_and_default_empty_main_agent 验证真实生产入口。

当前宿主978行（原2156，累计减1178，54.6%）；领域本任务测试共35条。
- `delivery.rs`：71行。
- `delivery_tests.rs`：202行。
- `mod.rs`：35行。
- `scheduler.rs`：158行。
- `scheduler_tests.rs`：68行。
- `tests/mod.rs`：7行。
- `tests/scheduler.rs`：273行。
- `wecom_acceptance_tests.rs`：164行。

保留职责：scheduler::Host接真实网关流、企微配置/认证端口和公告；run/execute_job_now为组合入口；process_due_jobs/run_agent_job/persist_job_result包装供真实宿主验收。delivery::latest_config/send_validated负责租户watcher及凭据连接；delivery_tests保留真实socket/密文/回执；wecom_acceptance_tests的state_with_config/request保留真实API和租户跨模块验收。scheduler_tests保留薄安全/不可用凭据契约。tests/scheduler剩余真实无密钥网关失败、旧启动快照覆盖和调度触发回执。未复制AppState、反向依赖或挂载宿主源码。

仍不足70%，不能靠删除真实边界断言或压缩注释达标。已落实残留清单建议的结果/best_effort矩阵，并额外迁移真实任务池调用。未来沿用 python3 scripts/cargo_slot.py cargo test -p agent-cron --lib host_migration_54_tests -- --test-threads=1，预期35条非零且failed=0；宿主按上述保留完整名称过滤并list确认迁移消失。静态核对所有SchedulerHost实现、app::task真实导出、名称唯一和旧挂载删除；git diff --check无空白错误。所有Cargo、构建、编译、测试和实机均未运行，不声称通过。

## 第四次恢复核对：清理失去调用者的旧投递入口

重新读取最新final.md及remaining-host.json：整轮当前43584行、缺口8584行，优先恢复清单已无任务54；本任务上次统计978行。再次逐符号检查未把该统计当作无法迁移的豁免。

普通通道生产迁出后，原cron::delivery::send和failure_summary重导出已无宿主调用者；本轮删除这两个过时入口，实际领域send和failure_summary及其真实调用均保留。公告通过agent-cron::announcement直接调用领域send；scheduler::persist_job_result经领域apply_result使用failure_summary，不再经过宿主转发。因此这是死转发清理，不另计生产算法迁出。

继续拆分的边界审查：ChannelConfigWatcher::new不仅是领域读取，它初始化真实GlobalDatabase/SystemConfigStore、捕获当前租户并继承测试数据库上下文；cron::latest_config是该真实组合的错误脱敏边界。WeComRuntimeScope::from_config和connected_with_credentials虽由channel-wecom持有，宿主调用仍需要当前可信租户，且认证连接与运行时注册表验收保留真实socket。send_validated的webhook测试endpoint只存在于宿主测试cfg，不能让领域库依赖宿主测试源码或把它当生产配置。

未继续搬走的具体断言：delivery_tests的持久密文损坏、配置重载及认证凭据变化；wecom_acceptance_tests的真实路由/租户保存/长输出分块回执；tests/scheduler的无密钥真实网关失败、启动旧快照被真实保存配置覆盖、手动和调度触发共享真实webhook路由；scheduler_tests的真实组合入口安全/不可用凭据拒绝。纯四组合状态矩阵、纯配置校验、零请求、普通通道及任务池测试已经自注册于agent-cron。此轮不删任何测试和断言，不创建仅为搬行数的框架。

当前宿主973行，累计自2156行减少1183行（54.9%），仍不满足单组70%建议，保留此前逐符号/逐用例映射与减量缺口。领域本任务35条未来过滤命令和真实宿主薄用例名称不变。静态rg确认旧入口无调用者，git diff --check核对本次变更；所有Cargo、构建、编译、测试和实机继续未运行，不宣称通过。
