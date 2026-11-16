# 来源复核115：session-engine/tests/host_suites/processor_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/libs/session-engine/tests/host_suites/processor_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/libs/session-engine/tests/host_suites/processor_tests.rs`。
- 快照 SHA256：`c4a6bdb44fc6e2e9c49aed6df6d192ac0e73121f93205c05ef3e5a9626745350`。源码变化后重新阅读。
- 快照词法属性 15 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/session/processor/mod.rs:71 | crates/libs/session-engine/tests/host_suites/processor_tests.rs | mod tests | cfg(test) | session-engine（物理所有者；生产声明见该库 src/lib.rs） | 未匹配→133 | 未发现 |

## 身份线索

- `todowrite_completed_rewritten_to_pending_when_merge_false`
- `todowrite_completed_rewritten_to_in_progress_when_merge_true`
- `sanitize_tool_input_redacts_file_write_content_for_logs`
- `sanitize_tool_input_for_ui_keeps_full_file_write_content`
- `sanitize_tool_input_for_ui_keeps_raw_apply_patch_input`
- `sanitize_tool_input_for_ui_uses_remaining_items_marker`
- `query_tool_call_detected`
- `parse_tool_at_ignores_incomplete_json_blocks`
- `parse_tool_at_ignores_invalid_json_on_same_line`
- `parse_tool_at_accepts_valid_json_on_same_line`
- `parse_tool_at_read_recovers_mention_after_malformed_block`
- `inline_tool_execution_appends_tool_context_back_to_llm_messages`
- `compact_tool_output_for_ui_keeps_file_link_and_removes_hidden_hint`
- `structured_tool_calls_are_not_executed_locally_for_acp_requests`
- `structured_tool_calls_still_execute_locally_for_non_acp_requests`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
