# 来源复核034：agent-loop/src/tests/loop_suite/memory_context

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/libs/agent-loop/src/tests/loop_suite/memory_context.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/libs/agent-loop/src/tests/loop_suite/memory_context.rs`。
- 快照 SHA256：`95adf27ef0c03147833aa2dcc0ec5e3ed41ebc9ada223d7f8599c5ec4cd96cd7`。源码变化后重新阅读。
- 快照词法属性 23 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/agent/loop_/tests/mod.rs:60 | crates/libs/agent-loop/src/tests/loop_suite/memory_context.rs | mod memory_context | cfg(test) | agent-loop（物理所有者；生产声明见该库 src/lib.rs） | 未匹配→133 | 未发现 |

## 身份线索

- `test_scrub_credentials`
- `test_scrub_credentials_json`
- `scrub_credentials_empty_input`
- `scrub_credentials_no_sensitive_data`
- `scrub_credentials_short_values_not_redacted`
- `trim_history_preserves_system_prompt`
- `trim_history_noop_when_within_limit`
- `build_compaction_transcript_formats_roles`
- `apply_compaction_summary_replaces_old_segment`
- `autosave_memory_key_has_prefix_and_uniqueness`
- `autosave_memory_keys_preserve_multiple_turns`
- `build_context_ignores_legacy_assistant_autosave_entries`
- `trim_history_empty_history`
- `trim_history_system_only`
- `trim_history_exactly_at_limit`
- `trim_history_removes_oldest_non_system`
- `trim_history_with_no_system_prompt`
- `trim_history_preserves_role_ordering`
- `trim_history_with_only_system_prompt`
- `build_native_assistant_history_includes_reasoning_content`
- `build_native_assistant_history_omits_reasoning_content_when_none`
- `build_native_assistant_history_from_parsed_calls_includes_reasoning_content`
- `build_native_assistant_history_from_parsed_calls_omits_reasoning_content_when_none`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
