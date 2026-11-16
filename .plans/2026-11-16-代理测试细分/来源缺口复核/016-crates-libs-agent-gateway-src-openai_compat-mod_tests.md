# 来源复核016：agent-gateway/src/openai_compat/mod_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/libs/agent-gateway/src/openai_compat/mod_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/libs/agent-gateway/src/openai_compat/mod_tests.rs`。
- 快照 SHA256：`320d59e2a06915ba90b75c8e497da045b3080cc29198a619e52e9247391e9c49`。源码变化后重新阅读。
- 快照词法属性 20 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/gateway/openai_compat/mod.rs:14 | crates/libs/agent-gateway/src/openai_compat/mod_tests.rs | mod mod_tests | cfg(not(target_arch="wasm32")); cfg(test) | agent-gateway（物理所有者；生产声明见该库 src/lib.rs） | 未匹配→133 | 未发现 |

## 身份线索

- `handle_v1_chat_completions_rejects_rate_limited_request`
- `handle_v1_chat_completions_rejects_missing_pairing_bearer`
- `handle_v1_chat_completions_rejects_invalid_json_body`
- `handle_v1_chat_completions_rejects_empty_messages`
- `handle_v1_chat_completions_passes_large_body_to_provider`
- `handle_v1_chat_completions_returns_non_streaming_completion`
- `handle_v1_chat_completions_uses_state_defaults_for_empty_model_and_temperature`
- `handle_v1_chat_completions_uses_unknown_provider_label_when_config_has_no_default`
- `handle_v1_chat_completions_returns_provider_error_without_leaking_details`
- `handle_v1_chat_completions_streams_single_chunk_when_provider_lacks_streaming`
- `handle_v1_chat_completions_streams_sanitized_fallback_error`
- `handle_v1_chat_completions_streams_native_chunks_and_done`
- `handle_v1_chat_completions_streams_native_error_chunk`
- `handle_v1_chat_completions_streams_done_after_native_error_without_success_event`
- `handle_v1_models_rejects_missing_pairing_bearer`
- `handle_v1_models_allows_request_when_pairing_is_disabled`
- `handle_v1_models_returns_configured_model_for_valid_request`
- `chunk_bytes_without_done_omits_done_marker_and_empty_fields`
- `helper_record_functions_accept_success_and_failure_events`
- `response_helpers_read_empty_body_text`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
