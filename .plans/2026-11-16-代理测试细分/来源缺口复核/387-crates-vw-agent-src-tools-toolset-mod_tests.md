# 来源复核387：src/tools/toolset/mod_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/tools/toolset/mod_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/tools/toolset/mod_tests.rs`。
- 快照 SHA256：`58686561c0eb5bcaa7fa6244df4281f858580a50d2b48771d8b43dcde88573f4`。源码变化后重新阅读。
- 快照词法属性 35 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/tools/toolset/mod.rs:117 | crates/vw-agent/src/tools/toolset/mod_tests.rs | mod mod_tests | cfg(test) | runtime_types, async_trait, serde_json, config_agent_profile, config_agent | 未匹配→133 | 未发现 |

## 身份线索

- `delegate_agents_auto_load_project_skills_by_default_and_preserve_explicit_skills`
- `delegate_agents_require_explicit_skills_when_project_auto_load_is_disabled`
- `configured_subagent_opt_out_overrides_builtin_default`
- `agent_tool_parallel_limit_uses_current_agent_and_clamps_bounds`
- `runtime_context_and_scalar_parsing_cover_core_branches`
- `classify_message_and_binary_detection_cover_errors_and_content`
- `block_on_bridges_when_called_inside_tokio_runtime`
- `arc_delegating_tool_forwards_methods`
- `arc_delegating_tool_forwards_v2_contract_methods`
- `default_tools_include_native_capability_tools`
- `default_context_constructors_use_current_directory_roots`
- `default_tools_wrapper_uses_native_runtime`
- `default_tools_with_runtime_omits_batch_when_runtime_has_no_capabilities`
- `all_tools_with_runtime_respects_optional_tool_configurations`
- `all_tools_wrapper_registers_default_network_tools`
- `all_tools_with_runtime_registers_brave_search_without_ipc_tools`
- `all_tools_with_runtime_filters_delegate_agents`
- `canonical_agent_tool_registers_when_delegate_agent_exists`
- `tool_specs_for_context_registers_dynamic_mcp_tools_from_workspace_config`
- `codex_discovery_tool_factory_registers_search_for_deferred_tools`
- `codex_discovery_tool_factory_omits_search_without_deferred_tools`
- `codex_discovery_tool_factory_searches_authorized_catalog_without_remote_calls`
- `codex_discovery_tool_factory_respects_disabled_and_denied_search`
- `execution_environment_registers_agent_tool_from_loaded_config`
- `default_config_registers_agent_tool_with_builtin_subagents`
- `default_toolset_registers_thread_tools_without_delegate_agents`
- `thread_tools_visible_without_tool_search`
- `restricted_toolset_can_exclude_thread_tools`
- `workflow_and_goal_sessions_inherit_recursive_agent_tool_controls`
- `execute_tool_call_runs_dynamic_mcp_tool_through_context_registry`
- `execute_tool_from_registry_uses_supplied_tool_list`
- `register_dynamic_mcp_tools_skips_disabled_servers_and_keeps_existing_tools`
- `shell_outer_timeout_yield_does_not_cancel_process`
- `codex_plugin_compat_t57_toolset_bridge_preserves_turn_and_tenant`
- `agent_config_cleanup_delegate_fixed_selection_preserves_strict_boundary`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
