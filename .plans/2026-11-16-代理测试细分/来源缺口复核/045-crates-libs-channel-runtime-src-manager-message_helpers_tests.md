# 来源复核045：channel-runtime/src/manager/message_helpers_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/libs/channel-runtime/src/manager/message_helpers_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/libs/channel-runtime/src/manager/message_helpers_tests.rs`。
- 快照 SHA256：`dcaccce8af5bb8dae2ccd686f7329d99894d64a917ac836593ebf998e2362b97`。源码变化后重新阅读。
- 快照词法属性 20 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/channels/manager/message_helpers.rs:6 | crates/libs/channel-runtime/src/manager/message_helpers_tests.rs | mod message_helpers_tests | cfg(test) | channel-runtime（物理所有者；生产声明见该库 src/lib.rs） | 未匹配→133 | 未发现 |

## 身份线索

- `log_inbound_message_accepts_long_unicode_content`
- `apply_inbound_hooks_returns_original_message_without_hooks`
- `apply_inbound_hooks_applies_modified_message`
- `apply_inbound_hooks_returns_none_when_hook_cancels`
- `apply_semantic_guard_skips_commands_and_missing_runtime_config`
- `apply_semantic_guard_skips_when_runtime_config_file_is_missing`
- `apply_semantic_guard_skips_when_runtime_config_is_invalid`
- `apply_semantic_guard_returns_canary_flag_from_valid_disabled_config`
- `apply_semantic_guard_enabled_without_available_backend_allows_message`
- `block_semantic_guard_message_records_warning_in_thread`
- `notify_provider_init_failure_sends_sanitized_threaded_notice`
- `notify_provider_init_failure_without_channel_is_noop`
- `maybe_auto_save_message_records_conversation_memory_with_threaded_key`
- `maybe_auto_save_message_respects_disabled_flag_and_character_boundary`
- `has_prior_history_reports_only_non_empty_history`
- `has_prior_history_recovers_from_poisoned_lock`
- `load_prior_turns_preserves_cached_message_boundaries`
- `load_prior_turns_injects_memory_context_for_first_user_turn`
- `load_prior_turns_does_not_inject_memory_when_last_turn_is_assistant`
- `load_prior_turns_queries_memory_but_keeps_missing_history_empty`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
