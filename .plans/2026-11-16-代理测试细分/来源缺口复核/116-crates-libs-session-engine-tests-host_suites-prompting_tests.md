# 来源复核116：session-engine/tests/host_suites/prompting_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/libs/session-engine/tests/host_suites/prompting_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/libs/session-engine/tests/host_suites/prompting_tests.rs`。
- 快照 SHA256：`20de69b357488ffea9fd2309a0d95d22d49a48e6bb7f11b89ebdd1f5abe2f3d3`。源码变化后重新阅读。
- 快照词法属性 12 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/session/processor/prompting.rs:8 | crates/libs/session-engine/tests/host_suites/prompting_tests.rs | mod prompting_tests | cfg(test) | session-engine（物理所有者；生产声明见该库 src/lib.rs） | 未匹配→133 | 未发现 |

## 身份线索

- `build_prompt_with_empty_session_contains_only_system_prompt_separator`
- `build_prompt_preserves_history_order_and_role_labels`
- `build_prompt_appends_trimmed_extra_assistant_text`
- `build_prompt_ignores_blank_extra_assistant_text`
- `build_prompt_backs_up_when_tool_limit_splits_utf8_character`
- `build_prompt_keeps_tool_message_at_exact_byte_limit`
- `build_prompt_stops_before_oversized_older_history`
- `build_prompt_accepts_exact_total_budget_and_rejects_one_byte_more`
- `build_prompt_adds_missing_newline_for_message_chunks`
- `build_prompt_keeps_existing_message_newline_without_double_required_suffix`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
