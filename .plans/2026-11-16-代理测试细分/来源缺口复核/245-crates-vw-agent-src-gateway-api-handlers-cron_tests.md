# 来源复核245：src/gateway/api/handlers/cron_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/gateway/api/handlers/cron_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/gateway/api/handlers/cron_tests.rs`。
- 快照 SHA256：`e1871ee04f7acde2d99661f5a1f429f86e57aa4e4cd3e02002fbf9df54003332`。源码变化后重新阅读。
- 快照词法属性 23 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/gateway/api/handlers/cron.rs:46 | crates/vw-agent/src/gateway/api/handlers/cron_tests.rs | mod cron_tests | cfg(not(target_arch="wasm32")); cfg(test) | model_types, axum, chrono, tokio, tower | 未匹配→133 | 未发现 |

## 身份线索

- `cron_json_serializes_schedule_variants_and_delivery_fields`
- `cron_json_serializes_absent_optional_job_and_run_fields_as_null`
- `cron_add_schedule_infers_and_validates_kinds`
- `cron_add_schedule_reports_missing_values_and_accepts_localized_kinds`
- `cron_update_schedule_supports_partial_updates_and_localized_kinds`
- `cron_update_schedule_rejects_invalid_at_and_every_values`
- `cron_add_job_type_delivery_patch_and_log_fields_normalize_input`
- `cron_add_delivery_and_patch_apply_safe_defaults_for_blank_values`
- `cron_add_job_type_defaults_from_prompt_and_command`
- `cron_update_schedule_treats_blank_kind_as_default_and_requires_matching_value`
- `cron_handlers_reject_requests_when_pairing_is_required`
- `cron_mutating_handlers_reject_requests_before_writing_when_pairing_is_required`
- `cron_handlers_manage_shell_job_lifecycle`
- `cron_add_handler_covers_agent_and_bad_request_branches`
- `cron_update_and_delete_handlers_report_errors`
- `cron_update_handler_reports_missing_job_after_valid_patch`
- `cron_history_endpoint_requires_auth`
- `cron_history_endpoint_merges_active_and_finished`
- `cron_history_endpoint_reports_finished_window_and_workspace_isolation`
- `cron_history_endpoint_respects_retention_and_cascade_deletion`
- `cron_wecom_delivery_add_rejects_invalid_target_without_creating_job`
- `cron_wecom_delivery_add_requires_configuration_but_none_does_not`
- `cron_wecom_delivery_api_roundtrip_and_partial_edit_validate_merged_target`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
