# 来源复核322：src/session/codex_alignment_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/session/codex_alignment_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/session/codex_alignment_tests.rs`。
- 快照 SHA256：`dab03ee58410bf9d5edd6aa18509a5475f09a5b2e2cac7ed07d619da01999d76`。源码变化后重新阅读。
- 快照词法属性 5 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/session/mod.rs:16 | crates/vw-agent/src/session/codex_alignment_tests.rs | mod codex_alignment_tests | cfg(all(test,not(target_arch="wasm32"))) | serde_json | 未匹配→133 | 未发现 |

## 身份线索

- `codex_alignment_skill_search_mcp_compaction_and_new_turn_preserve_boundaries`
- `codex_alignment_explicit_tools_search_disabled_and_disable_tools_keep_authorization`
- `codex_alignment_strict_skills_and_last_round_do_not_expand_capabilities`
- `codex_alignment_same_name_replacement_and_disabled_sources_cannot_replay_old_body`
- `codex_alignment_new_session_does_not_inherit_skill_or_tool_exposure`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
