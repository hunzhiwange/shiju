# Task 16 静态迁移证据

代码完成，编译/行为未验证。本轮遵守 plan.md，未运行任何 Cargo、单元测试、构建或实机。

## 全文件归属与行数（splitlines）

| 来源 | 修改前 | 宿主保留 | 职责/去向 |
|---|---:|---:|---|
| model_catalog_acceptance_tests.rs | 553 | 553 | 保留 SDK→真实宿主 dynamic_tools_acceptance_router→租户配置→resolver→执行流整条验收；isolated/Loopback/Fixture 是这些完整测试的进程隔离、真实监听及生命周期夹具。单条测试不可拆断执行边界。 |
| model_catalog_tests.rs | 304 | 0 | 整文件迁至 gateway-model/src/host_migration_16_tests；真实 SQLite、授权、tenant 中间件与 SDK 请求不变。 |
| model_catalog_edits_tests.rs | 247 | 0 | 整文件迁至 gateway-model/src/host_migration_16_tests；真实 SQLite、授权、tenant 中间件与 SDK 请求不变。 |
| model_discovery_tests.rs | 466 | 0 | 整文件迁至 gateway-model/src/host_migration_16_tests；真实 SQLite、授权、tenant 中间件与 SDK 请求不变。 |
| openclaw_settings.rs | 22 | 22 | router/authorization 注入真实宿主 settings 与敏感 env 授权回调。 |
| openclaw_settings_tests.rs | 64 | 64 | 保留真实 policy 授权、敏感读取与文件访问顺序边界，不能以固定允许的模拟授权替代。 |
| permission.rs | 69 | 67 | Host.resolve_directory/with_tenant_scope/with_instance/approval_manager_for_current_instance 保留真实宿主请求上下文；router 接真实 Host；测试 wrapper 仅为保留的列表/回复/路由验收。 |
| permission_tests.rs | 614 | 228 | 仅保留 Host router 构建、真实实例列表、回复及路由验收和必要 EnvGuard/manager_for_directory。映射、解析、审批回复策略、持久化已迁出。 |
| plugins.rs | 89 | 87 | Host 含真实 AppState 鉴权、pending 升级缓存及插件安装/激活能力；router/stateful_router 和保留宿主测试的适配器。无本轮新增业务迁移收益。 |
| project.rs | 22 | 22 | project_host 注入真实 session_count 与 instance_bootstrap；router 合并 worktree_management；兼容导出供当前调用者。跨目录测试中的纯路径/DTO/Git 元数据断言已迁出，存储/路由/真实启动验收保留。 |
| project_profiles.rs | 7 | 7 | router 注入真实管理员读取/创建/维护授权。 |
| provider.rs | 62 | 61 | Host 绑定 settings_provider_list、resolve_directory、with_context；router 与真实目录列表/路由验收留宿主，8条纯请求/响应及认证处理测试已迁出。 |
| pty.rs | 36 | 36 | Host 绑定真实实例上下文；router 和实例会话生命周期验收保留，4条 query 形状测试已迁出。 |
| question.rs | 16 | 16 | QuestionHost.current_tenant_scope 与 router 注入可信租户。 |
| resumable_stream.rs | 2 | 2 | 仅既有 gateway_resumable_stream 公共兼容导出，非业务实现。 |
| shiju_account.rs | 2 | 2 | 仅既有 gateway_shiju account 兼容导出，真实调用方使用。 |
| shiju_apps.rs | 2 | 2 | 仅既有 gateway_shiju apps 兼容导出，真实调用方使用。 |
| shiju_skills.rs | 90 | 90 | router/router_with_client 构造宿主上游客户端；ShijuSkillService 的12个方法及 map_error 适配真实客户端错误和 DTO，不复制领域算法。 |

18 文件基线 2667 行，当前宿主 1259 行，净减 1408 行（52.8%），未达到70%。上述逐文件说明为保留原因；继续可迁项是 acceptance 中尚与执行验收同条测试的目录断言、插件安装/升级 Host 端口对应领域测试及项目真实存储路由测试。不得按本任务结束视为整体减量达标，收口需验收。

生产：本轮未迁生产算法；原生产实现已在各领域库，本轮主要转移测试编译归属。生产入口 Host 和兼容导出保留，permission/plugins/pty 仅清理不再使用的测试 import。

## 全用例映射

### crates/vw-agent/src/gateway/api/handlers/model_catalog_acceptance_tests.rs

- `catalog_macos_acceptance_persists_and_enforces_identity` → `crates/vw-agent/src/gateway/api/handlers/model_catalog_acceptance_tests.rs`（真实宿主边界保留）
- `catalog_macos_acceptance_scopes_preset_selection_and_price` → `crates/vw-agent/src/gateway/api/handlers/model_catalog_acceptance_tests.rs`（真实宿主边界保留）
- `catalog_macos_acceptance_rejects_stale_or_unauthorized_fill` → `crates/vw-agent/src/gateway/api/handlers/model_catalog_acceptance_tests.rs`（真实宿主边界保留）
- `catalog_macos_acceptance_cancel_keeps_saved_catalog` → `crates/vw-agent/src/gateway/api/handlers/model_catalog_acceptance_tests.rs`（真实宿主边界保留）
### crates/vw-agent/src/gateway/api/handlers/model_catalog_tests.rs

- `catalog_api_read_denies_other_host` → `crates/libs/gateway-model/src/host_migration_16_tests/model_catalog_tests.rs`（领域自注册）
- `catalog_api_read_denies_other_project_or_tenant` → `crates/libs/gateway-model/src/host_migration_16_tests/model_catalog_tests.rs`（领域自注册）
- `catalog_api_read_searches_alias_and_pages_saved_models` → `crates/libs/gateway-model/src/host_migration_16_tests/model_catalog_tests.rs`（领域自注册）
- `catalog_api_read_rejects_unbounded_or_invalid_parameters` → `crates/libs/gateway-model/src/host_migration_16_tests/model_catalog_tests.rs`（领域自注册）
- `catalog_api_read_lists_presets_and_templates_without_auth_material` → `crates/libs/gateway-model/src/host_migration_16_tests/model_catalog_tests.rs`（领域自注册）
- `catalog_api_read_resolves_only_the_exact_model_deployment` → `crates/libs/gateway-model/src/host_migration_16_tests/model_catalog_tests.rs`（领域自注册）
### crates/vw-agent/src/gateway/api/handlers/model_catalog_edits_tests.rs

- `catalog_api_edits_conflict_keeps_persisted_model` → `crates/libs/gateway-model/src/host_migration_16_tests/model_catalog_edits_tests.rs`（领域自注册）
- `catalog_api_edits_concurrent_edits_have_one_winner` → `crates/libs/gateway-model/src/host_migration_16_tests/model_catalog_edits_tests.rs`（领域自注册）
- `catalog_api_edits_denies_missing_credentials_and_read_only_writes` → `crates/libs/gateway-model/src/host_migration_16_tests/model_catalog_edits_tests.rs`（领域自注册）
- `catalog_api_edits_denies_other_hosts_projects_and_tenants` → `crates/libs/gateway-model/src/host_migration_16_tests/model_catalog_edits_tests.rs`（领域自注册）
- `catalog_api_edits_persists_alias_and_default_across_gateway_reopen` → `crates/libs/gateway-model/src/host_migration_16_tests/model_catalog_edits_tests.rs`（领域自注册）
- `catalog_api_edits_rejects_default_from_another_deployment_atomically` → `crates/libs/gateway-model/src/host_migration_16_tests/model_catalog_edits_tests.rs`（领域自注册）
- `catalog_api_edits_fill_preview_is_read_only_and_commit_preserves_manual_values` → `crates/libs/gateway-model/src/host_migration_16_tests/model_catalog_edits_tests.rs`（领域自注册）
- `catalog_api_edits_fill_rejects_stale_revision_and_other_endpoint` → `crates/libs/gateway-model/src/host_migration_16_tests/model_catalog_edits_tests.rs`（领域自注册）
### crates/vw-agent/src/gateway/api/handlers/model_discovery_tests.rs

- `catalog_api_discovery_cancel_prevents_result_publication` → `crates/libs/gateway-model/src/host_migration_16_tests/model_discovery_tests.rs`（领域自注册）
- `catalog_api_discovery_http_completes_without_saving_models` → `crates/libs/gateway-model/src/host_migration_16_tests/model_discovery_tests.rs`（领域自注册）
- `catalog_api_discovery_http_uses_the_selected_native_codex_provider` → `crates/libs/gateway-model/src/host_migration_16_tests/model_discovery_tests.rs`（领域自注册）
- `catalog_api_discovery_keeps_failure_details_private` → `crates/libs/gateway-model/src/host_migration_16_tests/model_discovery_tests.rs`（领域自注册）
- `catalog_api_discovery_task_id_does_not_grant_ownership` → `crates/libs/gateway-model/src/host_migration_16_tests/model_discovery_tests.rs`（领域自注册）
- `catalog_api_discovery_rejects_unbound_input_before_network` → `crates/libs/gateway-model/src/host_migration_16_tests/model_discovery_tests.rs`（领域自注册）
- `catalog_api_discovery_rejects_commands_and_mismatched_methods` → `crates/libs/gateway-model/src/host_migration_16_tests/model_discovery_tests.rs`（领域自注册）
- `catalog_api_discovery_opencode_preserves_qualified_model_ids` → `crates/libs/gateway-model/src/host_migration_16_tests/model_discovery_tests.rs`（领域自注册）
- `catalog_api_discovery_agy_uses_saved_profile_and_native_display_name` → `crates/libs/gateway-model/src/host_migration_16_tests/model_discovery_tests.rs`（领域自注册）
- `catalog_api_discovery_cancel_waits_for_cli_exit` → `crates/libs/gateway-model/src/host_migration_16_tests/model_discovery_tests.rs`（领域自注册）
### crates/vw-agent/src/gateway/api/handlers/openclaw_settings_tests.rs

- `openclaw_api_roundtrips_defaults_denies_before_io` → `crates/vw-agent/src/gateway/api/handlers/openclaw_settings_tests.rs`（真实宿主边界保留）
- `openclaw_api_protects_sensitive_env` → `crates/vw-agent/src/gateway/api/handlers/openclaw_settings_tests.rs`（真实宿主边界保留）
### crates/vw-agent/src/gateway/api/handlers/permission_tests.rs

- `acp_permission_request_omits_expiration` → `crates/libs/gateway-permission/src/host_migration_16_tests.rs`（领域自注册）
- `router_builds_with_app_state` → `crates/vw-agent/src/gateway/api/handlers/permission_tests.rs`（真实宿主边界保留）
- `permission_reply_request_deserializes_reply_field` → `crates/libs/gateway-permission/src/host_migration_16_tests.rs`（领域自注册）
- `permission_reply_request_deserializes_all_reply_variants_and_message` → `crates/libs/gateway-permission/src/host_migration_16_tests.rs`（领域自注册）
- `permission_reply_request_rejects_missing_or_unknown_reply` → `crates/libs/gateway-permission/src/host_migration_16_tests.rs`（领域自注册）
- `permission_request_from_pending_maps_metadata_and_tool_info` → `crates/libs/gateway-permission/src/host_migration_16_tests.rs`（领域自注册）
- `permission_request_from_pending_skips_blank_optional_fields` → `crates/libs/gateway-permission/src/host_migration_16_tests.rs`（领域自注册）
- `permission_request_from_pending_requires_both_non_blank_tool_identifiers` → `crates/libs/gateway-permission/src/host_migration_16_tests.rs`（领域自注册）
- `permission_request_from_pending_preserves_non_object_arguments` → `crates/libs/gateway-permission/src/host_migration_16_tests.rs`（领域自注册）
- `permission_list_returns_empty_for_instance_without_pending_requests` → `crates/vw-agent/src/gateway/api/handlers/permission_tests.rs`（真实宿主边界保留）
- `permission_list_returns_pending_requests_for_resolved_directory` → `crates/vw-agent/src/gateway/api/handlers/permission_tests.rs`（真实宿主边界保留）
- `map_pending_approval_error_preserves_public_status_and_message` → `crates/libs/gateway-permission/src/host_migration_16_tests.rs`（领域自注册）
- `reply_pending_request_returns_false_for_missing_request` → `crates/libs/gateway-permission/src/host_migration_16_tests.rs`（领域自注册）
- `reply_pending_request_once_confirms_and_records_yes_resolution` → `crates/libs/gateway-permission/src/host_migration_16_tests.rs`（领域自注册）
- `reply_pending_request_always_confirms_grants_and_records_yes_resolution` → `crates/libs/gateway-permission/src/host_migration_16_tests.rs`（领域自注册）
- `reply_pending_request_always_applies_even_when_persist_fails` → `crates/libs/gateway-permission/src/host_migration_16_tests.rs`（领域自注册）
- `reply_pending_request_reject_removes_request_and_records_no_resolution` → `crates/libs/gateway-permission/src/host_migration_16_tests.rs`（领域自注册）
- `permission_reply_once_applies_pending_request` → `crates/vw-agent/src/gateway/api/handlers/permission_tests.rs`（真实宿主边界保留）
- `permission_reply_missing_request_returns_not_found` → `crates/vw-agent/src/gateway/api/handlers/permission_tests.rs`（真实宿主边界保留）
- `permission_router_lists_and_rejects_missing_request` → `crates/vw-agent/src/gateway/api/handlers/permission_tests.rs`（真实宿主边界保留）
- `persist_non_cli_approval_adds_auto_approve_and_removes_always_ask` → `crates/libs/gateway-permission/src/host_migration_16_tests.rs`（领域自注册）
- `persist_non_cli_approval_adds_tool_without_changing_unrelated_always_ask_entries` → `crates/libs/gateway-permission/src/host_migration_16_tests.rs`（领域自注册）
- `persist_non_cli_approval_removes_always_ask_when_tool_is_already_approved` → `crates/libs/gateway-permission/src/host_migration_16_tests.rs`（领域自注册）
- `persist_non_cli_approval_noops_when_tool_is_already_persisted` → `crates/libs/gateway-permission/src/host_migration_16_tests.rs`（领域自注册）
### crates/libs/gateway-provider/src/provider_tests.rs

- `router_builds_with_unit_state` → `crates/libs/gateway-provider/src/provider_tests.rs`（真实宿主边界保留）
- `oauth_request_bodies_deserialize` → `crates/libs/gateway-provider/src/host_migration_16_tests.rs`（领域自注册）
- `oauth_callback_request_allows_missing_or_null_code` → `crates/libs/gateway-provider/src/host_migration_16_tests.rs`（领域自注册）
- `oauth_requests_reject_missing_or_non_numeric_method` → `crates/libs/gateway-provider/src/host_migration_16_tests.rs`（领域自注册）
- `provider_list_response_serializes_public_contract` → `crates/libs/gateway-provider/src/host_migration_16_tests.rs`（领域自注册）
- `provider_auth_methods_delegates_to_auth_module` → `crates/libs/gateway-provider/src/host_migration_16_tests.rs`（领域自注册）
- `provider_list_returns_defaults_and_sorted_connections` → `crates/libs/gateway-provider/src/provider_tests.rs`（真实宿主边界保留）
- `oauth_authorize_maps_empty_authorization` → `crates/libs/gateway-provider/src/host_migration_16_tests.rs`（领域自注册）
- `oauth_authorize_accepts_maximum_method_index_as_empty_authorization` → `crates/libs/gateway-provider/src/host_migration_16_tests.rs`（领域自注册）
- `oauth_callback_maps_unsupported_to_bad_request` → `crates/libs/gateway-provider/src/host_migration_16_tests.rs`（领域自注册）
- `router_auth_methods_returns_empty_json_object` → `crates/libs/gateway-provider/src/provider_tests.rs`（真实宿主边界保留）
- `router_oauth_authorize_returns_null_authorization` → `crates/libs/gateway-provider/src/provider_tests.rs`（真实宿主边界保留）
- `router_oauth_callback_accepts_absent_code_and_maps_provider_error` → `crates/libs/gateway-provider/src/provider_tests.rs`（真实宿主边界保留）
- `router_oauth_routes_reject_invalid_json_shapes` → `crates/libs/gateway-provider/src/provider_tests.rs`（真实宿主边界保留）
- `router_rejects_wrong_method_and_unknown_oauth_action` → `crates/libs/gateway-provider/src/provider_tests.rs`（真实宿主边界保留）
### crates/libs/gateway-pty/src/pty_tests.rs

- `router_builds_with_app_state` → `crates/libs/gateway-pty/src/pty_tests.rs`（真实宿主边界保留）
- `pty_connect_query_deserializes_directory_and_cursor` → `crates/libs/gateway-pty/src/host_migration_16_tests.rs`（领域自注册）
- `pty_connect_query_allows_missing_optional_fields` → `crates/libs/gateway-pty/src/host_migration_16_tests.rs`（领域自注册）
- `pty_connect_query_rejects_non_integer_cursor` → `crates/libs/gateway-pty/src/host_migration_16_tests.rs`（领域自注册）
- `pty_connect_query_preserves_cursor_boundaries_and_blank_directory` → `crates/libs/gateway-pty/src/host_migration_16_tests.rs`（领域自注册）
- `pty_list_returns_empty_for_new_instance` → `crates/libs/gateway-pty/src/pty_tests.rs`（真实宿主边界保留）
- `pty_create_rejects_blank_cwd_without_starting_process` → `crates/libs/gateway-pty/src/pty_tests.rs`（真实宿主边界保留）
- `pty_get_missing_session_returns_not_found` → `crates/libs/gateway-pty/src/pty_tests.rs`（真实宿主边界保留）
- `pty_update_missing_session_returns_not_found` → `crates/libs/gateway-pty/src/pty_tests.rs`（真实宿主边界保留）
- `pty_remove_missing_session_is_idempotent` → `crates/libs/gateway-pty/src/pty_tests.rs`（真实宿主边界保留）
### crates/libs/gateway-plugins/src/plugins_tests.rs

- `plugins_handler_lists_installed_plugins` → `crates/libs/gateway-plugins/src/plugins_tests.rs`（真实宿主边界保留）
- `plugin_detail_rejects_blank_and_missing_ids` → `crates/libs/gateway-plugins/src/plugins_tests.rs`（真实宿主边界保留）
- `plugin_mutation_handlers_reject_blank_ids_before_loading_config` → `crates/libs/gateway-plugins/src/plugins_tests.rs`（真实宿主边界保留）
- `plugin_text_helpers_trim_values_and_fall_back_for_empty_text` → `crates/libs/gateway-plugins/src/host_migration_16_tests.rs`（领域自注册）
- `codex_plugin_compat_t06_gateway_retains_exact_marketplace_ids` → `crates/libs/gateway-plugins/src/plugins_tests.rs`（真实宿主边界保留）
### crates/libs/gateway-project/src/project_tests.rs

- `router_builds_with_app_state` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `project_routes_require_explicit_tenant_scope` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `normalize_path_removes_current_directory_prefix` → `crates/libs/gateway-project/src/host_migration_16_tests.rs`（领域自注册）
- `normalize_path_handles_empty_root_and_repeated_edge_separators` → `crates/libs/gateway-project/src/host_migration_16_tests.rs`（领域自注册）
- `worktree_id_round_trips_directory` → `crates/libs/gateway-project/src/host_migration_16_tests.rs`（领域自注册）
- `directory_from_worktree_id_rejects_invalid_values` → `crates/libs/gateway-project/src/host_migration_16_tests.rs`（领域自注册）
- `timestamp_ms_clamps_to_i64_range` → `crates/libs/gateway-project/src/host_migration_16_tests.rs`（领域自注册）
- `project_context_directory_prefers_worktree_then_sandbox` → `crates/libs/gateway-project/src/host_migration_16_tests.rs`（领域自注册）
- `project_scope_rejects_blank_project_id` → `crates/libs/gateway-project/src/host_migration_16_tests.rs`（领域自注册）
- `project_owns_directory_normalizes_worktree_and_sandbox_paths` → `crates/libs/gateway-project/src/host_migration_16_tests.rs`（领域自注册）
- `map_project_uses_sandbox_directory_and_path_name_fallback` → `crates/libs/gateway-project/src/host_migration_16_tests.rs`（领域自注册）
- `map_project_uses_explicit_non_blank_name` → `crates/libs/gateway-project/src/host_migration_16_tests.rs`（领域自注册）
- `map_project_ignores_blank_explicit_name` → `crates/libs/gateway-project/src/host_migration_16_tests.rs`（领域自注册）
- `map_project_falls_back_to_directory_when_no_name_component_exists` → `crates/libs/gateway-project/src/host_migration_16_tests.rs`（领域自注册）
- `map_worktree_derives_name_id_and_timestamps` → `crates/libs/gateway-project/src/host_migration_16_tests.rs`（领域自注册）
- `current_branch_and_change_records_are_empty_for_non_repo` → `crates/libs/gateway-project/src/host_migration_16_tests.rs`（领域自注册）
- `current_branch_reads_committed_repository_branch` → `crates/libs/gateway-project/src/host_migration_16_tests.rs`（领域自注册）
- `collect_project_change_records_empty_for_unborn_git_repo` → `crates/libs/gateway-project/src/host_migration_16_tests.rs`（领域自注册）
- `collect_project_change_records_groups_modified_files` → `crates/libs/gateway-project/src/host_migration_16_tests.rs`（领域自注册）
- `collect_project_change_records_includes_nested_untracked_files` → `crates/libs/gateway-project/src/host_migration_16_tests.rs`（领域自注册）
- `map_project_reports_git_branch_and_dirty_state` → `crates/libs/gateway-project/src/host_migration_16_tests.rs`（领域自注册）
- `gateway_git_metadata_works_without_git_executable` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `map_worktree_uses_default_name_for_blank_directory` → `crates/libs/gateway-project/src/host_migration_16_tests.rs`（领域自注册）
- `load_project_filters_missing_sandboxes_and_maps_missing_project` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `project_get_and_update_round_trip_storage` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `project_get_missing_project_returns_not_found` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `project_routes_do_not_cross_tenant_for_same_project_id` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `project_update_missing_project_returns_bad_request` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `project_list_filters_sorts_limits_and_cursors` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `project_list_handles_missing_cursor_blank_query_and_status_miss` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `project_list_uses_name_directory_and_id_tie_breakers` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `project_list_searches_directory_case_insensitively` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `project_change_records_requires_directory` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `project_change_records_rejects_blank_and_unowned_directory` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `project_resolve_and_change_records_handle_plain_directory` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `project_resolve_without_create_does_not_persist_missing_directory` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `project_resolve_without_create_returns_registered_project` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `project_change_records_uses_stored_sandbox_match` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `project_worktrees_return_empty_for_non_git_project` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `project_worktrees_list_git_project_primary_worktree` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `project_store_rejects_project_without_directory` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `project_worktree_create_rejects_non_git_project` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `worktree_get_rejects_invalid_id_before_project_lookup` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `worktree_get_maps_stored_project_directory` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `worktree_handlers_reject_directory_outside_project` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `worktree_delete_and_reset_reject_invalid_ids_before_project_lookup` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `worktree_delete_and_reset_reject_non_git_project_after_lookup` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）
- `worktree_environment_ui_project_command_roundtrip_and_clear` → `crates/libs/gateway-project/src/project_tests.rs`（真实宿主边界保留）

## 接线、隔离与依赖

- gateway-model 自注册 host_migration_16_tests，macOS 条件与原来一致；三个源文件及两个原父 mod 已删除，edits 由迁后的 catalog_tests 自注册。采用 gateway-agent-apps::host_context 真实授权和 agent-gateway::middleware 真实 tenant 中间件；dev依赖无 vw-agent，无 path/include 宿主源码。
- gateway-permission 自注册18条领域测试；原宿主仅6条上下文边界。真实 agent-approval/config 持久化与权限失败断言保留；本模块环境测试串行锁；未来统一 --test-threads=1，避免其他模块环境变量竞争。
- gateway-provider 自注册8条；gateway-pty自注册4条；gateway-plugins自注册1条；gateway-project自注册20条，原跨目录文件仅剩宿主验收及仍被使用的夹具。
- project DTO 测试的 session_count 是明确的宿主端口，注入0；bootstrap 若误调用直接 panic。文件系统与 Git 元数据仍用真实临时目录/libgit2。此处不宣称验证真实宿主会话数量；宿主测试仍保留真实接线。
- RustCodeGraph 已检查索引；计划指出索引早于上轮迁移，实际源码与直接引用按 rg/读取复核。
- 并行会话提交已包含本轮 model 源文件删除及父模块清理；本提交补齐接收库，未覆盖其它会话改动。

## 未来限定验证（均未运行）

- `python3 scripts/cargo_slot.py cargo test -p gateway-model --lib host_migration_16_tests -- --test-threads=1`，macOS预期 24 条，必须非零且 failed=0。
- `python3 scripts/cargo_slot.py cargo test -p gateway-permission --lib host_migration_16_tests -- --test-threads=1`，macOS预期 18 条，必须非零且 failed=0。
- `python3 scripts/cargo_slot.py cargo test -p gateway-provider --lib host_migration_16_tests -- --test-threads=1`，macOS预期 8 条，必须非零且 failed=0。
- `python3 scripts/cargo_slot.py cargo test -p gateway-pty --lib host_migration_16_tests -- --test-threads=1`，macOS预期 4 条，必须非零且 failed=0。
- `python3 scripts/cargo_slot.py cargo test -p gateway-plugins --lib host_migration_16_tests -- --test-threads=1`，macOS预期 1 条，必须非零且 failed=0。
- `python3 scripts/cargo_slot.py cargo test -p gateway-project --lib host_migration_16_tests -- --test-threads=1`，macOS预期 20 条，必须非零且 failed=0。
- 宿主保留测试按上文完整模块/函数名限定运行；对迁出名称执行真实宿主 --list，确认消失（不能以零测试当通过）。
- 编译、行为、耗时与构建收益均未验证；依赖变更后未来 Cargo 需更新锁文件。

## 分项统计

- gateway-model 新增测试/夹具 1030 行；生产算法新增0行，模块注册另计。
- gateway-permission 新增测试/夹具 437 行；生产算法新增0行，模块注册另计。
- gateway-provider 新增测试/夹具 106 行；生产算法新增0行，模块注册另计。
- gateway-pty 新增测试/夹具 43 行；生产算法新增0行，模块注册另计。
- gateway-plugins 新增测试/夹具 6 行；生产算法新增0行，模块注册另计。
- gateway-project 新增测试/夹具 306 行；生产算法新增0行，模块注册另计。

## Second pass after final host audit (supersedes retained mappings above)

Current original-source host lines: 727; baseline 2667; reduction 1940 (72.7%). The shiju_skills reduction belongs to another session. No Cargo, build, test or executable was run.

OpenClaw: production authorization() moved to gateway-openclaw-settings/src/authorization.rs. Host calls authorized_router, retaining the same real app-access-service authorization and credential_root projection. Both original tests now self-register there; no mocked policy or filesystem.

Permission: all six remaining tests moved to agent-gateway::host_migration_16_tests. Fixture binds real session-store tenant scope, agent-project instance context and approval_state cache. It intentionally omits application bootstrap (not covered by these assertions). Host retains the concrete trait implementation and new router_builds_with_host_ports boundary. Old permission_tests module and two test wrappers removed.

Catalog acceptance: three whole tests moved to gateway-model::host_migration_16_tests::catalog_acceptance_tests, preserving original assertions for preset/price isolation, stale/unauthorized fill, cancellation and saved catalog. Fixture uses the real domain router, real authorization and tenant middleware, SQLite, loopback upstream and process-isolated models cache. Chat stream/config/resolver fixture code is excluded. Host retains only catalog_macos_acceptance_persists_and_enforces_identity and its necessary isolated/Loopback/Fixture/send lifecycle: this one case explicitly verifies SDK to resolver to actual chat execution and cannot be replaced with a simulated domain port.


### agent-gateway

- `router_builds_with_app_state` -> `crates/libs/agent-gateway/src/host_migration_16_tests.rs`
- `permission_list_returns_empty_for_instance_without_pending_requests` -> `crates/libs/agent-gateway/src/host_migration_16_tests.rs`
- `permission_list_returns_pending_requests_for_resolved_directory` -> `crates/libs/agent-gateway/src/host_migration_16_tests.rs`
- `permission_reply_once_applies_pending_request` -> `crates/libs/agent-gateway/src/host_migration_16_tests.rs`
- `permission_reply_missing_request_returns_not_found` -> `crates/libs/agent-gateway/src/host_migration_16_tests.rs`
- `permission_router_lists_and_rejects_missing_request` -> `crates/libs/agent-gateway/src/host_migration_16_tests.rs`
Future: `python3 scripts/cargo_slot.py cargo test -p agent-gateway --lib host_migration_16_tests -- --test-threads=1`; expected 6 cases for this filter (macOS). Not run.

### gateway-openclaw-settings

- `openclaw_api_roundtrips_defaults_denies_before_io` -> `crates/libs/gateway-openclaw-settings/src/host_migration_16_tests.rs`
- `openclaw_api_protects_sensitive_env` -> `crates/libs/gateway-openclaw-settings/src/host_migration_16_tests.rs`
Future: `python3 scripts/cargo_slot.py cargo test -p gateway-openclaw-settings --lib host_migration_16_tests -- --test-threads=1`; expected 2 cases for this filter (macOS). Not run.

### gateway-model

- `catalog_macos_acceptance_scopes_preset_selection_and_price` -> `crates/libs/gateway-model/src/host_migration_16_tests/catalog_acceptance_tests.rs`
- `catalog_macos_acceptance_rejects_stale_or_unauthorized_fill` -> `crates/libs/gateway-model/src/host_migration_16_tests/catalog_acceptance_tests.rs`
- `catalog_macos_acceptance_cancel_keeps_saved_catalog` -> `crates/libs/gateway-model/src/host_migration_16_tests/catalog_acceptance_tests.rs`
Future: `python3 scripts/cargo_slot.py cargo test -p gateway-model --lib host_migration_16_tests::catalog_acceptance_tests -- --test-threads=1`; expected 3 cases for this filter (macOS). Not run.

### Current retained original files (splitlines)

- `model_catalog_acceptance_tests.rs`: 443 lines.
- `model_catalog_tests.rs`: 0 lines.
- `model_catalog_edits_tests.rs`: 0 lines.
- `model_discovery_tests.rs`: 0 lines.
- `openclaw_settings.rs`: 2 lines.
- `openclaw_settings_tests.rs`: 0 lines.
- `permission.rs`: 45 lines.
- `permission_tests.rs`: 0 lines.
- `plugins.rs`: 87 lines.
- `project.rs`: 22 lines.
- `project_profiles.rs`: 7 lines.
- `provider.rs`: 61 lines.
- `pty.rs`: 36 lines.
- `question.rs`: 16 lines.
- `resumable_stream.rs`: 2 lines.
- `shiju_account.rs`: 2 lines.
- `shiju_apps.rs`: 2 lines.
- `shiju_skills.rs`: 2 lines.

New test dependencies are declared in the receiving manifests. No reverse vw-agent dependency, host path/include, fake auth or duplicated AppState. Cargo.lock resolution remains future authorized Cargo work. All compilation, runtime behavior, build-time and performance claims remain unverified.

## Task59 requested deletion/commit closure

Read latest evidence/task-59-recovery-priorities.json, owner 16 priority 3. Scope is exclusively committing the three already-deleted source files; no restoration or additional migration. The current HEAD still tracks these files while the worktree deletes them. The domain replacements were already committed in 98aa2a2bd.

Static full-file comparison against current HEAD, rewriting only the original host imports to existing domain crate imports:

| Deleted source | Existing receiver under gateway-model/src/host_migration_16_tests | Lines | Tests | Comparison |
|---|---|---:|---:|---|
| model_catalog_tests.rs | model_catalog_tests.rs | 304 | 6 | Entire file identical after import rewrite |
| model_catalog_edits_tests.rs | model_catalog_edits_tests.rs | 247 | 8 | Entire file identical, no rewrite needed |
| model_discovery_tests.rs | model_discovery_tests.rs | 466 | 10 | Entire file identical after import rewrite |

This includes every fixture, function, failure/cancellation assertion and test body, not merely matching names. Total 1017 stale host lines removed from Git ownership; these bodies were already absent from the working tree, so this closure does not claim 1017 new physical line savings.

Registration checked: gateway-model/src/lib.rs gates host_migration_16_tests with test+macOS; that module registers model_catalog_tests and model_discovery_tests; catalog_tests registers edits via its local path attribute. Host handlers/mod.rs has no registration for these three modules. The receiving manifest declares SDK, transport, real tenant middleware, policy, temporary-directory and async dependencies. The existing actual catalog client/gateway authentication acceptance remains in the host.

Future precise validation: `python3 scripts/cargo_slot.py cargo test -p gateway-model --lib host_migration_16_tests::model_catalog_tests -- --test-threads=1` expects 14 cases including edits; `python3 scripts/cargo_slot.py cargo test -p gateway-model --lib host_migration_16_tests::model_discovery_tests -- --test-threads=1` expects 10 cases on macOS. Neither was run. No Cargo, build or test execution performed. Only source comparison, registration/dependency inspection and diff whitespace checks were performed. Task16 numbered file remains deleted and is included in this closure where tracked; unrelated deletions and staged work are excluded.
