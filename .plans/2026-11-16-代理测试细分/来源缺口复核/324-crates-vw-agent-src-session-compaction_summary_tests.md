# 来源复核324：src/session/compaction_summary_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/session/compaction_summary_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/session/compaction_summary_tests.rs`。
- 快照 SHA256：`f02256e49259d745d8ac2164a545ca203e31063f853ce6c349c183220bd21815`。源码变化后重新阅读。
- 快照词法属性 14 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/session/compaction_summary.rs:79 | crates/vw-agent/src/session/compaction_summary_tests.rs | mod tests | cfg(test) | vw-agent 宿主模块（生产父声明见左列） | 未匹配→133 | 未发现 |

## 身份线索

- `compaction_single_request_preserves_structured_history`
- `compaction_overflow_trims_request_copy_and_keeps_tool_pairs`
- `compaction_reuses_cancellation_and_retry`
- `compaction_preflight_bounds_large_tool_output_without_mutating_source`
- `compaction_range_overflow_recovers_single_large_message`
- `compaction_overflow_retries_are_bounded_and_preserve_source`
- `compaction_impossible_budget_preserves_system_and_skips_model`
- `compaction_budget_reserves_output_and_accepts_unset_input_limit`
- `compaction_equal_context_and_output_uses_bounded_request_allocation`
- `compaction_unknown_window_recovers_from_provider_overflow_without_changing_history`
- `compaction_empty_output_is_normalized_without_replaying_partial_streams`
- `compaction_empty_error_translation_preserves_metadata_and_other_failures`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
