# 来源复核159：src/channels/format_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/channels/format_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/channels/format_tests.rs`。
- 快照 SHA256：`3b9274fa8406a3a7eaeb92e17b8ddf875cfc3d9993824b01cb480cf27f09aeea`。源码变化后重新阅读。
- 快照词法属性 39 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/channels/format.rs:83 | crates/vw-agent/src/channels/format_tests.rs | mod format_tests | cfg(test) | vw-agent 宿主模块（生产父声明见左列） | 未匹配→133 | 未发现 |

## 身份线索

- `strip_tool_call_tags_removes_closed_tool_blocks`
- `strip_tool_call_tags_handles_aliases_unclosed_json_and_sentinel`
- `split_internal_progress_delta_detects_known_prefix`
- `split_internal_progress_delta_hides_ws_event_payloads_and_preserves_normal_text`
- `channel_delivery_instructions_are_known_for_chat_channels`
- `channel_delivery_instructions_include_channel_specific_requirements`
- `should_expose_internal_tool_details_respects_positive_and_negative_intents`
- `extract_tool_context_summary_collects_unique_tool_names_after_start_index`
- `tool_json_payload_detection_requires_known_tool_and_arguments`
- `tool_result_payload_requires_prior_tool_call_and_result_shape`
- `sanitize_tool_json_value_strips_calls_and_keeps_message_content`
- `isolated_tool_json_artifacts_are_removed_without_touching_inline_json`
- `sanitize_channel_response_removes_known_tool_json_and_tags`
- `sanitize_channel_response_keeps_only_visible_answer_from_execution_transcript`
- `sanitize_channel_response_rejects_execution_transcript_without_final_answer`
- `line_isolated_json_segment_detects_whitespace_boundaries`
- `strip_tool_call_tags_removes_all_closed_tag_variants`
- `strip_tool_call_tags_handles_block_boundaries_and_extra_blank_lines`
- `strip_tool_call_tags_keeps_empty_line_after_removed_block`
- `strip_tool_call_tags_handles_unclosed_json_arrays_and_leading_close_tags`
- `strip_tool_call_tags_uses_earliest_tool_marker`
- `strip_tool_call_tags_preserves_messages_without_tags`
- `should_expose_internal_tool_details_covers_verb_and_no_match_paths`
- `should_expose_internal_tool_details_handles_english_verb_and_negation_hints`
- `extract_tool_context_summary_handles_tag_variants_and_malformed_results`
- `extract_tool_context_summary_ignores_native_json_without_calls`
- `extract_tool_context_summary_deduplicates_native_and_result_names`
- `is_tool_call_payload_accepts_fallback_names_and_rejects_bad_shapes`
- `is_tool_result_payload_rejects_objects_without_result`
- `sanitize_tool_json_value_handles_arrays_and_non_objects`
- `sanitize_tool_json_value_handles_tool_call_messages_without_content`
- `strip_isolated_tool_json_artifacts_replaces_message_objects_and_normalizes_crlf`
- `strip_isolated_tool_json_artifacts_preserves_invalid_and_unrelated_json`
- `strip_isolated_tool_json_artifacts_removes_tool_arrays_and_following_results`
- `strip_isolated_tool_json_artifacts_keeps_early_results_until_a_tool_call_is_seen`
- `strip_isolated_tool_json_artifacts_preserves_plain_text_without_json`
- `sanitize_channel_response_preserves_clean_response_without_tools`
- `sanitize_channel_response_matches_tool_names_case_insensitively`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
