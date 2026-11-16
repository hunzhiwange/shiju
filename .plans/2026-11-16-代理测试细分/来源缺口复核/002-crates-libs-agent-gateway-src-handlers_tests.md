# 来源复核002：agent-gateway/src/handlers_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/libs/agent-gateway/src/handlers_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/libs/agent-gateway/src/handlers_tests.rs`。
- 快照 SHA256：`99af80772c78aa046109f83db7ddbea0feec68b519b91b5fd5612f6c343b78be`。源码变化后重新阅读。
- 快照词法属性 36 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/gateway/handlers.rs:129 | crates/libs/agent-gateway/src/handlers_tests.rs | mod handlers_tests | cfg(not(target_arch="wasm32")); cfg(test) | agent-gateway（物理所有者；生产声明见该库 src/lib.rs） | 未匹配→133 | 未发现 |

## 身份线索

- `process_channel_messages_sends_fallback_with_thread_when_gateway_chat_fails`
- `process_channel_messages_with_chat_sends_successful_reply`
- `send_successful_channel_reply_sanitizes_and_omits_thread_when_disabled`
- `send_successful_channel_reply_logs_send_errors_without_panicking`
- `whatsapp_signature_accepts_valid_hmac`
- `whatsapp_signature_rejects_wrong_signature`
- `whatsapp_signature_rejects_missing_prefix`
- `whatsapp_signature_rejects_valid_hex_with_wrong_length`
- `whatsapp_verify_query_deserializes_fields`
- `whatsapp_verify_returns_not_found_when_unconfigured`
- `whatsapp_verify_echoes_challenge_for_matching_token`
- `whatsapp_verify_rejects_bad_or_incomplete_verification`
- `whatsapp_verify_requires_subscribe_mode_and_token`
- `whatsapp_message_validates_configuration_signature_json_and_empty_payloads`
- `whatsapp_message_valid_signature_does_not_hide_invalid_json`
- `whatsapp_message_rejects_present_invalid_signature`
- `whatsapp_message_acknowledges_nonempty_payload_after_local_send_failure`
- `linq_webhook_handles_configuration_signature_json_and_empty_messages`
- `linq_webhook_accepts_valid_signature_before_empty_message_ack`
- `linq_webhook_rejects_present_invalid_signature`
- `linq_webhook_acknowledges_unactionable_message_received_event`
- `wati_verify_query_deserializes_challenge_and_defaults_missing_value`
- `wati_verify_and_webhook_cover_error_and_empty_paths`
- `wati_verify_echoes_empty_challenge`
- `wati_webhook_processes_messages_and_handles_provider_errors`
- `nextcloud_talk_webhook_handles_security_and_empty_payloads`
- `nextcloud_talk_webhook_accepts_valid_signature_and_processes_messages`
- `nextcloud_talk_valid_signature_does_not_hide_invalid_json`
- `nextcloud_talk_webhook_rejects_present_invalid_signature`
- `qq_webhook_handles_configuration_validation_and_empty_events`
- `qq_webhook_accepts_matching_app_id_with_whitespace`
- `qq_webhook_matching_app_id_does_not_hide_invalid_json`
- `channel_reload_saved_credentials_update_existing_webhook_router`
- `channel_command_cancel_webhook_uses_common_stop_and_help`
- `channel_command_cancel_qq_webhook_stops_previous_message_id`
- `channel_command_cancel_webhook_failed_send_preserves_state`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
