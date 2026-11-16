# 来源复核134：src/agent/loop_/core/tool_loop_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/agent/loop_/core/tool_loop_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/agent/loop_/core/tool_loop_tests.rs`。
- 快照 SHA256：`a1f7c07e2d6cb602b7a240bbc698e52e4f9a05eb73a35fc82a039fb8a9cc2de1`。源码变化后重新阅读。
- 快照词法属性 12 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/agent/loop_/core/tool_loop.rs:21 | crates/vw-agent/src/agent/loop_/core/tool_loop_tests.rs | mod tool_loop_tests | cfg(test) | model_types, async_trait, tokio_util | 未匹配→133 | 未发现 |

## 身份线索

- `completion_detection_is_reexported_for_tool_loop`
- `run_tool_call_loop_returns_final_text_and_updates_history_when_max_iterations_is_zero`
- `run_tool_call_loop_exits_before_provider_call_when_cancelled`
- `run_tool_call_loop_reports_exhausted_after_last_tool_iteration`
- `run_tool_call_loop_retries_empty_final_response_until_text_arrives`
- `run_tool_call_loop_reports_exhausted_when_all_final_responses_are_empty`
- `run_tool_call_loop_omits_excluded_tools_from_provider_request`
- `run_tool_call_loop_retries_once_when_completion_claim_lacks_tool_call`
- `run_tool_call_loop_retries_once_when_tool_payload_cannot_be_parsed`
- `run_tool_call_loop_errors_when_completion_claim_repeats_without_tool_call`
- `agcollab_channel_integrates_child_results`
- `agcollab_channel_cancellation_and_budget_reject_unfinished_children`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
