# 来源复核243：src/gateway/api/handlers/config_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/gateway/api/handlers/config_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/gateway/api/handlers/config_tests.rs`。
- 快照 SHA256：`7813873a71f23036ff142d10d8fd08cbc8c549f779cb12377ebf415b50c3ae80`。源码变化后重新阅读。
- 快照词法属性 43 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/gateway/api/handlers/config.rs:149 | crates/vw-agent/src/gateway/api/handlers/config_tests.rs | mod config_tests | cfg(not(target_arch="wasm32")); cfg(test) | model_types, axum, parking_lot, tempfile, vw_api_transport, config_agent_profile, config_agent | 未匹配→133 | 未发现 |

## 身份线索

- `routers_build_for_stateless_and_stateful_config_routes`
- `has_explicit_directory_detects_non_empty_query_or_header_values`
- `has_explicit_directory_ignores_non_utf8_header_value`
- `config_get_returns_serialized_instance_config`
- `config_get_stateful_returns_serialized_instance_config`
- `config_get_without_directory_restores_the_requested_tenant_after_restart`
- `config_patch_rejects_non_object_patch`
- `config_patch_updates_global_config_without_explicit_directory`
- `config_patch_without_directory_updates_only_the_requested_tenant`
- `config_patch_uses_explicit_header_for_project_override`
- `config_patch_stateful_updates_active_runtime_config_without_explicit_directory`
- `config_patch_stateful_leaves_runtime_config_when_directory_is_explicit`
- `config_patch_stateful_rejects_invalid_global_patch_without_mutating_runtime_config`
- `project_config_patch_writes_only_database_override`
- `project_config_patch_isolates_requested_tenant`
- `project_config_patch_never_persists_effective_base_fields`
- `config_providers_returns_defaults_for_available_models`
- `handle_api_config_get_requires_auth_when_pairing_is_enabled`
- `handle_api_config_get_returns_pretty_toml_with_masked_secrets`
- `handle_api_config_get_reports_gateway_skey_store_failure`
- `handle_api_config_put_rejects_unauthorized_before_parsing_body`
- `handle_api_config_put_rejects_invalid_toml`
- `handle_api_config_put_rejects_type_mismatch_after_toml_parsing`
- `handle_api_config_put_rejects_invalid_config_without_mutating_runtime_state`
- `handle_api_config_put_reports_gateway_skey_store_failure_without_mutating_runtime_state`
- `handle_api_config_put_reports_config_save_failure_without_mutating_runtime_state`
- `config_put_reloads_hooks_and_invalidates_query_engines`
- `config_put_hook_build_failure_is_atomic`
- `handle_api_config_put_hydrates_validates_saves_and_updates_runtime_config`
- `dashboard_config_skeys_are_isolated_by_tenant_scope`
- `handle_api_config_get_accepts_valid_bearer_token_when_pairing_is_enabled`
- `config_patch_stateful_rejects_invalid_project_patch_without_mutating_runtime_config`
- `manual_agent_save_updates_live_resolution_and_survives_instance_restart`
- `agent_profile_api_rejects_builtin_delete_and_backend_change`
- `agent_profile_api_persists_changes_after_reload`
- `agent_profile_api_rejects_invalid_identity_and_model_conflicts`
- `agent_profile_api_native_metadata_and_empty_rules_are_editable`
- `agent_profile_api_full_replacement_cannot_remove_saved_builtin`
- `agent_runtime_lock_patch_preserves_saved_engines_and_is_atomic`
- `agent_runtime_lock_patch_is_scoped_to_each_tenant`
- `agent_runtime_lock_put_rejects_replacement_and_implicit_switches`
- `agent_runtime_lock_put_accepts_first_save_and_locks_native`
- `worktree_settings_patch_round_trips_and_rejects_project_override`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
