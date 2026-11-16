# 来源复核022：agent-loop/src/query_engine_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/libs/agent-loop/src/query_engine_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/libs/agent-loop/src/query_engine_tests.rs`。
- 快照 SHA256：`b0807651d6c39bc4d1fd343008977b1a618a9c17357fc46734b193105161e12b`。源码变化后重新阅读。
- 快照词法属性 26 个，不是当前 cfg 展开后的有效测试数。

| crates/libs/agent-loop/src/query_engine_test_binding.rs:4 | crates/libs/agent-loop/src/query_engine_tests.rs | mod query_engine_tests | cfg(test) | agent-loop（物理所有者；生产声明见该库 src/lib.rs） | 未匹配→133 | 未发现 |

## 身份线索

- `codex_discovery_core_query_engine_loads_and_resets_tools_between_user_turns`
- `agent_and_query_engine_use_ephemeral_query_engine_context_each_turn`
- `agent_and_query_engine_preserve_history_with_ephemeral_context_each_turn`
- `query_engine_usage_totals_billable_tokens`
- `query_engine_usage_total_saturates_on_overflow`
- `recording_observer_accumulates_successful_llm_usage_and_forwards_calls`
- `recording_observer_records_success_with_missing_usage_as_zero`
- `recording_observer_saturates_oversized_token_counts`
- `session_state_counts_non_system_messages_and_remaining_budget`
- `snapshot_replace_and_restore_round_trip_history_turns_and_usage`
- `submit_message_success_enriches_history_and_records_usage`
- `submit_message_without_memory_context_uses_timestamp_prefix`
- `first_turn_compaction_allows_threshold_but_rejects_overflow`
- `submit_message_timestamp_uses_configured_timezone`
- `new_rejects_invalid_programmatic_timezone`
- `submit_message_success_preserves_history_after_turn`
- `submit_message_error_rolls_back_history_and_turn_count`
- `govern_history_does_not_apply_message_count_trimming`
- `govern_history_preserves_history_after_a_response`
- `govern_history_never_uses_local_summary_fallback`
- `build_tool_descriptions_tracks_optional_tools`
- `build_tool_descriptions_includes_browser_open_when_enabled`
- `agcollab_channel_request_contains_only_enabled_team_runtime`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
