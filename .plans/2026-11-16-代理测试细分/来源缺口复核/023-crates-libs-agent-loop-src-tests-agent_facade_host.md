# 来源复核023：agent-loop/src/tests/agent_facade_host

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/libs/agent-loop/src/tests/agent_facade_host.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/libs/agent-loop/src/tests/agent_facade_host.rs`。
- 快照 SHA256：`db6998de1fcd16ed7056b76dec7dd2f879dbf6e99d4536b864f60f683c38ea3c`。源码变化后重新阅读。
- 快照词法属性 22 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/agent/mod.rs:41 | crates/libs/agent-loop/src/tests/agent_facade_host.rs | mod mod_tests | cfg(test) | agent-loop（物理所有者；生产声明见该库 src/lib.rs） | 未匹配→133 | agent-loop |

## 身份线索

- `info_serialization_flattens_and_round_trips_definition`
- `primary_visible_excludes_hidden_and_subagents`
- `choose_default_agent_validates_configured_default`
- `choose_default_agent_uses_configured_then_builtin_priority`
- `choose_default_agent_falls_back_alphabetically_or_errors`
- `normalize_agent_mode_accepts_known_aliases`
- `merge_json_value_deep_merges_objects_and_replaces_scalars`
- `load_agent_configs_merges_configured_agents_with_builtins`
- `load_agent_configs_discards_malformed_configured_map`
- `load_agent_configs_uses_builtins_when_agents_key_is_missing`
- `resolve_model_ref_handles_direct_split_and_missing_values`
- `resolve_model_ref_trims_components_and_prefers_explicit_provider`
- `build_resolved_agent_applies_runtime_defaults`
- `public_delegate_agents_resolve_dedicated_role_prompts`
- `build_resolved_agent_orders_permission_layers`
- `build_resolved_agent_allows_selected_skills`
- `build_resolved_agent_keeps_global_skill_deny_authoritative`
- `build_resolved_agent_preserves_custom_builtin_prompts`
- `build_resolved_agent_keeps_explicit_tool_output_deny`
- `load_state_skips_disabled_agents_and_preserves_default_name`
- `prompt_and_truncate_glob_are_non_empty`
- `agent_native_defaults_skill_grants_preserve_user_denial_and_fixed_empty`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
