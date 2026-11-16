# 来源复核046：channel-runtime/src/manager/message_response_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/libs/channel-runtime/src/manager/message_response_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/libs/channel-runtime/src/manager/message_response_tests.rs`。
- 快照 SHA256：`d03a5844f17a634d887a2e5f88af58cb4866c618c4cfc2f4a7e2aa4f48e46fef`。源码变化后重新阅读。
- 快照词法属性 22 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/channels/manager/message_response.rs:33 | crates/libs/channel-runtime/src/manager/message_response_tests.rs | mod message_response_tests | cfg(test) | channel-runtime（物理所有者；生产声明见该库 src/lib.rs） | 未匹配→133 | 未发现 |

## 身份线索

- `completed_success_sends_response_and_appends_history`
- `completed_success_blocks_canary_leak_before_sending`
- `completed_success_hook_cancel_suppresses_send_and_history`
- `completed_success_hook_mutation_is_truncated_before_delivery`
- `completed_success_returns_safe_fallback_for_tool_only_output`
- `completed_success_records_tool_summary_except_for_telegram`
- `completed_success_finalizes_draft_and_falls_back_to_send_on_failure`
- `cancelled_result_cancels_existing_draft`
- `cancelled_result_without_channel_leaves_history_unchanged`
- `tool_loop_cancelled_error_cancels_existing_draft`
- `context_window_error_sends_retry_guidance`
- `tool_iteration_limit_preserves_history_with_pause_marker`
- `generic_error_records_failure_marker_and_notifies_channel`
- `vision_capability_error_rolls_back_orphan_user_turn_and_finalizes_draft`
- `timeout_records_failure_marker_and_sends_guidance`
- `timeout_finalizes_existing_draft`
- `markdown_delivery_runtime_partial_error_does_not_resend`
- `markdown_delivery_runtime_unknown_draft_error_suppresses_fallback`
- `markdown_delivery_runtime_safety_gate_runs_before_first_chunk`
- `markdown_delivery_runtime_zero_delivery_draft_preserves_safe_fallback`
- `markdown_delivery_runtime_hook_mutation_keeps_original_route`
- `channel_first_response_distinguishes_pause_commit_failure_and_redelivery`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
