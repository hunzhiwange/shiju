# 来源复核054：channel-whatsapp/src/mod_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/libs/channel-whatsapp/src/mod_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/libs/channel-whatsapp/src/mod_tests.rs`。
- 快照 SHA256：`2c52ba070d7cdf10ae7cce42ffc284dc0caaf65932a9a81ef173e56fd8414f35`。源码变化后重新阅读。
- 快照词法属性 62 个，不是当前 cfg 展开后的有效测试数。

| crates/libs/channel-whatsapp/src/lib.rs:272 | crates/libs/channel-whatsapp/src/mod_tests.rs | mod mod_tests | cfg(not(target_arch="wasm32")); cfg(test); cfg(test) | channel-whatsapp（物理所有者；生产声明见该库 src/lib.rs） | 未匹配→133 | channel-whatsapp |

## 身份线索

- `whatsapp_channel_name`
- `whatsapp_verify_token`
- `whatsapp_ensure_https_accepts_https_urls`
- `whatsapp_listen_runs_until_cancelled`
- `whatsapp_send_and_health_check_fail_with_unreachable_proxy`
- `whatsapp_health_check_reports_false_for_graph_error_response`
- `whatsapp_number_allowed_exact`
- `whatsapp_number_allowed_wildcard`
- `whatsapp_number_denied_empty`
- `whatsapp_parse_empty_payload`
- `whatsapp_parse_valid_text_message`
- `whatsapp_parse_unauthorized_number`
- `whatsapp_parse_non_text_message_skipped`
- `whatsapp_parse_multiple_messages`
- `whatsapp_parse_normalizes_phone_with_plus`
- `whatsapp_parse_preserves_phone_already_with_plus`
- `whatsapp_empty_text_skipped`
- `whatsapp_parse_missing_entry_array`
- `whatsapp_parse_entry_not_array`
- `whatsapp_parse_missing_changes_array`
- `whatsapp_parse_changes_not_array`
- `whatsapp_parse_missing_value`
- `whatsapp_parse_missing_messages_array`
- `whatsapp_parse_messages_not_array`
- `whatsapp_parse_missing_from_field`
- `whatsapp_parse_missing_text_body`
- `whatsapp_parse_missing_text_object_is_treated_as_non_text`
- `whatsapp_parse_null_text_body`
- `whatsapp_parse_invalid_timestamp_uses_current`
- `whatsapp_parse_missing_timestamp_uses_current`
- `whatsapp_parse_multiple_entries`
- `whatsapp_parse_multiple_changes`
- `whatsapp_parse_status_update_ignored`
- `whatsapp_parse_audio_message_skipped`
- `whatsapp_parse_video_message_skipped`
- `whatsapp_parse_document_message_skipped`
- `whatsapp_parse_sticker_message_skipped`
- `whatsapp_parse_location_message_skipped`
- `whatsapp_parse_contacts_message_skipped`
- `whatsapp_parse_reaction_message_skipped`
- `whatsapp_parse_mixed_authorized_unauthorized`
- `whatsapp_parse_unicode_message`
- `whatsapp_parse_very_long_message`
- `whatsapp_parse_whitespace_only_message_skipped`
- `whatsapp_number_allowed_multiple_numbers`
- `whatsapp_number_allowed_case_sensitive`
- `whatsapp_parse_phone_already_has_plus`
- `whatsapp_channel_fields_stored_correctly`
- `whatsapp_parse_empty_messages_array`
- `whatsapp_parse_empty_entry_array`
- `whatsapp_parse_empty_changes_array`
- `whatsapp_parse_newlines_preserved`
- `whatsapp_parse_special_characters`
- `whatsapp_parse_from_must_be_string`
- `whatsapp_parse_text_body_must_be_string`
- `whatsapp_parse_numeric_timestamp_uses_current`
- `whatsapp_parse_max_u64_timestamp`
- `whatsapp_parse_message_metadata_fields`
- `whatsapp_parse_null_value_skips_change`
- `whatsapp_parse_null_text_object_skips_message`
- `whatsapp_send_reports_graph_error_response`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
