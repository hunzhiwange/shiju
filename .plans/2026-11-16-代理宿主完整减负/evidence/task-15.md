# 任务15：兼容入口与残留装配

代码迁移与静态接线完成；编译/行为未验证。净减未达70%，保留以下缺口供任务59验收，不能视为整轮减量达标。

当前读取来源 2886 行（任务卡规划2914行，此前其它任务已改变组合根）；现剩 1723 行，净减 1163 行，40.3%。无真实生产算法迁移收益；已有业务实现继续由 gateway-* 持有。

## 接线和依赖

agent-gateway::host_migration_15_tests 在 lib.rs 自注册，子模块 global/integrations/hermes_memory/misc，合计41条测试。使用既有 gateway_types::AppState<(),(),()>，未复制 AppState 定义。IntegrationHost 仅调用现有 api::auth 与 api::integrations，未重写算法。global 调用 gateway-global，Hermes 调用 app-access-service。没有反向依赖 vw-agent，没有宿主 path/include 加载。原 integrations/hermes 测试挂载与文件删除；global/misc 按整条函数迁移。领域环境变量测试共享本测试族异步锁及守卫，未来以单线程验证；未取得进程间/跨测试族并发证据。

## 全文件与全用例映射

### `crates/vw-agent/src/gateway/api/handlers/file_drag_integration_tests.rs`

405 → 405 行；保留真实 start/client_router 两监听器、鉴权/租户/非回环地址及磁盘无副作用验收；Server/Drop/server/server_on/Fixture/new/in_tenant 管理真实网络和项目存储。整条测试函数不可用伪路由替换。

- `file_drag_integration_both_listeners_preserve_data_and_reject_conflicts` → 原宿主模块保留：`file_drag_integration_tests::file_drag_integration_both_listeners_preserve_data_and_reject_conflicts`；原因见本文件职责

- `file_drag_integration_auth_tenant_origin_rejections_have_no_disk_effects` → 原宿主模块保留：`file_drag_integration_tests::file_drag_integration_auth_tenant_origin_rejections_have_no_disk_effects`；原因见本文件职责

- `file_drag_integration_recursive_link_failure_keeps_source_and_other_items` → 原宿主模块保留：`file_drag_integration_tests::file_drag_integration_recursive_link_failure_keeps_source_and_other_items`；原因见本文件职责

- `file_drag_integration_non_loopback_cannot_spoof_forwarded_headers` → 原宿主模块保留：`file_drag_integration_tests::file_drag_integration_non_loopback_cannot_spoof_forwarded_headers`；原因见本文件职责

- `file_drag_integration_default_tenant_cannot_be_changed_from_another_tenant` → 原宿主模块保留：`file_drag_integration_tests::file_drag_integration_default_tenant_cannot_be_changed_from_another_tenant`；原因见本文件职责

### `crates/vw-agent/src/gateway/api/handlers/git.rs`

17 → 17 行；GitHost/list_directories 将真实 with_instance/worktree 服务注入 gateway-git。

符号：`list_directories`, `router`。

### `crates/vw-agent/src/gateway/api/handlers/global.rs`

26 → 20 行；仅保留 router 重导出、具体 AppState 的 stateful_router 和宿主测试导入。

符号：`stateful_router`。

### `crates/vw-agent/src/gateway/api/handlers/global_tests.rs`

807 → 216 行；仅保留 bootstrap 源码契约与真实鉴权路由并发更新用例；StaticProvider/chat_with_system/state/EnvGuard/use_temp_config_dir 为该真实宿主用例必需。

- `router_builds_with_app_state` → `crates/libs/agent-gateway/src/host_migration_15_tests/global.rs::router_builds_with_app_state`（原名/断言保留）

- `global_config_routes_isolate_platform_and_company_updates` → `crates/libs/agent-gateway/src/host_migration_15_tests/global.rs::global_config_routes_isolate_platform_and_company_updates`（原名/断言保留）

- `local_global_config_route_writes_shared_database_only` → `crates/libs/agent-gateway/src/host_migration_15_tests/global.rs::local_global_config_route_writes_shared_database_only`（原名/断言保留）

- `global_config_api_projects_shared_skeys_and_ignores_conflicting_legacy_sqlite` → `crates/libs/agent-gateway/src/host_migration_15_tests/global.rs::global_config_api_projects_shared_skeys_and_ignores_conflicting_legacy_sqlite`（原名/断言保留）

- `gateway_bootstrap_loads_default_tenant_skeys_from_shared_store_without_legacy_fallback` → 原宿主模块保留：`global_tests::gateway_bootstrap_loads_default_tenant_skeys_from_shared_store_without_legacy_fallback`；原因见本文件职责

- `global_health_reports_ok_status` → `crates/libs/agent-gateway/src/host_migration_15_tests/global.rs::global_health_reports_ok_status`（原名/断言保留）

- `global_config_get_serializes_current_config` → `crates/libs/agent-gateway/src/host_migration_15_tests/global.rs::global_config_get_serializes_current_config`（原名/断言保留）

- `global_config_get_maps_unusable_config_directory_to_bad_request` → `crates/libs/agent-gateway/src/host_migration_15_tests/global.rs::global_config_get_maps_unusable_config_directory_to_bad_request`（原名/断言保留）

- `config_response_maps_secret_encryption_failure_to_internal_error` → `crates/libs/agent-gateway/src/host_migration_15_tests/global.rs::config_response_maps_secret_encryption_failure_to_internal_error`（原名/断言保留）

- `global_config_acp_get_returns_merged_agent_specs` → `crates/libs/agent-gateway/src/host_migration_15_tests/global.rs::global_config_acp_get_returns_merged_agent_specs`（原名/断言保留）

- `global_config_acp_get_trims_and_filters_user_overrides` → `crates/libs/agent-gateway/src/host_migration_15_tests/global.rs::global_config_acp_get_trims_and_filters_user_overrides`（原名/断言保留）

- `global_config_patch_accepts_object_patch` → `crates/libs/agent-gateway/src/host_migration_15_tests/global.rs::global_config_patch_accepts_object_patch`（原名/断言保留）

- `global_config_patch_preserves_mcp_servers` → `crates/libs/agent-gateway/src/host_migration_15_tests/global.rs::global_config_patch_preserves_mcp_servers`（原名/断言保留）

- `global_config_patch_rejects_non_object_patch` → `crates/libs/agent-gateway/src/host_migration_15_tests/global.rs::global_config_patch_rejects_non_object_patch`（原名/断言保留）

- `global_config_patch_stateful_updates_cached_config_and_pairing` → `crates/libs/agent-gateway/src/host_migration_15_tests/global.rs::global_config_patch_stateful_updates_cached_config_and_pairing`（原名/断言保留）

- `global_config_patch_stateful_rejects_invalid_patch_without_runtime_update` → `crates/libs/agent-gateway/src/host_migration_15_tests/global.rs::global_config_patch_stateful_rejects_invalid_patch_without_runtime_update`（原名/断言保留）

- `global_config_patch_stateful_concurrent_fields_preserve_both_values` → 原宿主模块保留：`global_tests::global_config_patch_stateful_concurrent_fields_preserve_both_values`；原因见本文件职责

- `global_dispose_returns_true_and_publishes_global_event` → `crates/libs/agent-gateway/src/host_migration_15_tests/global.rs::global_dispose_returns_true_and_publishes_global_event`（原名/断言保留）

- `global_event_sse_response_uses_event_stream_content_type` → `crates/libs/agent-gateway/src/host_migration_15_tests/global.rs::global_event_sse_response_uses_event_stream_content_type`（原名/断言保留）

- `global_event_sse_streams_connected_and_published_events` → `crates/libs/agent-gateway/src/host_migration_15_tests/global.rs::global_event_sse_streams_connected_and_published_events`（原名/断言保留）

- `global_event_sse_preserves_published_event_directory` → `crates/libs/agent-gateway/src/host_migration_15_tests/global.rs::global_event_sse_preserves_published_event_directory`（原名/断言保留）

- `global_event_sse_streams_heartbeat_and_stops_after_receiver_closes` → `crates/libs/agent-gateway/src/host_migration_15_tests/global.rs::global_event_sse_streams_heartbeat_and_stops_after_receiver_closes`（原名/断言保留）

- `agent_unified_selection_install_status_uses_gateway_executable_without_download` → `crates/libs/agent-gateway/src/host_migration_15_tests/global.rs::agent_unified_selection_install_status_uses_gateway_executable_without_download`（原名/断言保留）

- `global_acp_defaults_route_returns_builtin_launch_fields` → `crates/libs/agent-gateway/src/host_migration_15_tests/global.rs::global_acp_defaults_route_returns_builtin_launch_fields`（原名/断言保留）

### `crates/vw-agent/src/gateway/api/handlers/hermes_dashboard.rs`

2 → 2 行；现存 router 直接导出。

符号：。

### `crates/vw-agent/src/gateway/api/handlers/hermes_memory.rs`

15 → 11 行；router 注入宿主授权回调并转换 Host；领域权限测试已迁出。

符号：`router`。

### `crates/vw-agent/src/gateway/api/handlers/hermes_memory_tests.rs`

61 → 0 行；整组领域测试已迁入自注册模块，原文件删除。

- `hermes_api_isolates_memory_denies_without_project_grant` → `crates/libs/agent-gateway/src/host_migration_15_tests/hermes_memory.rs::hermes_api_isolates_memory_denies_without_project_grant`（原名/断言保留）

- `hermes_api_isolates_memory_tenant_host_and_native_files` → `crates/libs/agent-gateway/src/host_migration_15_tests/hermes_memory.rs::hermes_api_isolates_memory_tenant_host_and_native_files`（原名/断言保留）

### `crates/vw-agent/src/gateway/api/handlers/hooks.rs`

22 → 22 行；router 和具体 HookRuntime trait 实现保留真实 runtime reload/audit 端口；include 测试仍需本机 hook runtime，后续可拆纯脱敏断言。

符号：`router`, `build_candidate`, `replace`, `audit_store`。

### `crates/vw-agent/src/gateway/api/handlers/instance.rs`

40 → 40 行；Host 绑定实际实例初始化与审批缓存释放；跨目录测试仍有审批生命周期契约，纯路径/序列化/分支查询测试仍可继续下沉。

符号：`resolve_directory`, `with_instance`, `clear_approval_manager_for_directory`, `router`。

### `crates/vw-agent/src/gateway/api/handlers/integrations.rs`

46 → 42 行；IntegrationsHost 将真实 AppState 配置及 auth/settings 服务接到现有领域处理器；三个公开 handler 维持路由函数签名。

符号：`config`, `require_auth`, `build_integration_settings_payload`, `config_revision`, `apply_integration_credentials_update`, `handle_api_integrations`, `handle_api_integrations_settings`, `handle_api_integration_credentials_put`。

### `crates/vw-agent/src/gateway/api/handlers/integrations_tests.rs`

468 → 0 行；整组领域测试已迁入自注册模块，原文件删除。

- `exported_handlers_are_available` → `crates/libs/agent-gateway/src/host_migration_15_tests/integrations.rs::exported_handlers_are_available`（原名/断言保留）

- `integrations_requires_auth_when_pairing_enabled` → `crates/libs/agent-gateway/src/host_migration_15_tests/integrations.rs::integrations_requires_auth_when_pairing_enabled`（原名/断言保留）

- `integration_settings_requires_auth_when_pairing_enabled` → `crates/libs/agent-gateway/src/host_migration_15_tests/integrations.rs::integration_settings_requires_auth_when_pairing_enabled`（原名/断言保留）

- `integrations_lists_registry_entries_with_status` → `crates/libs/agent-gateway/src/host_migration_15_tests/integrations.rs::integrations_lists_registry_entries_with_status`（原名/断言保留）

- `integration_settings_include_revision_and_active_provider` → `crates/libs/agent-gateway/src/host_migration_15_tests/integrations.rs::integration_settings_include_revision_and_active_provider`（原名/断言保留）

- `integration_credentials_put_requires_auth_before_processing_body` → `crates/libs/agent-gateway/src/host_migration_15_tests/integrations.rs::integration_credentials_put_requires_auth_before_processing_body`（原名/断言保留）

- `integration_credentials_put_rejects_stale_revision` → `crates/libs/agent-gateway/src/host_migration_15_tests/integrations.rs::integration_credentials_put_rejects_stale_revision`（原名/断言保留）

- `integration_credentials_put_maps_unknown_integration_to_not_found` → `crates/libs/agent-gateway/src/host_migration_15_tests/integrations.rs::integration_credentials_put_maps_unknown_integration_to_not_found`（原名/断言保留）

- `integration_credentials_put_maps_unsupported_api_url_to_internal_server` → `crates/libs/agent-gateway/src/host_migration_15_tests/integrations.rs::integration_credentials_put_maps_unsupported_api_url_to_internal_server`（原名/断言保留）

- `integration_credentials_put_maps_unknown_field_to_bad_request` → `crates/libs/agent-gateway/src/host_migration_15_tests/integrations.rs::integration_credentials_put_maps_unknown_field_to_bad_request`（原名/断言保留）

- `integration_credentials_put_maps_invalid_update_to_bad_request` → `crates/libs/agent-gateway/src/host_migration_15_tests/integrations.rs::integration_credentials_put_maps_invalid_update_to_bad_request`（原名/断言保留）

- `integration_credentials_put_reports_unchanged_when_revision_is_same` → `crates/libs/agent-gateway/src/host_migration_15_tests/integrations.rs::integration_credentials_put_reports_unchanged_when_revision_is_same`（原名/断言保留）

- `integration_credentials_put_saves_and_updates_state` → `crates/libs/agent-gateway/src/host_migration_15_tests/integrations.rs::integration_credentials_put_saves_and_updates_state`（原名/断言保留）

- `integration_credentials_put_accepts_valid_bearer_token` → `crates/libs/agent-gateway/src/host_migration_15_tests/integrations.rs::integration_credentials_put_accepts_valid_bearer_token`（原名/断言保留）

- `integration_credentials_put_reports_config_persistence_failure` → `crates/libs/agent-gateway/src/host_migration_15_tests/integrations.rs::integration_credentials_put_reports_config_persistence_failure`（原名/断言保留）

### `crates/vw-agent/src/gateway/api/handlers/knowledge.rs`

3 → 3 行；公共 router/knowledge_store/knowledge_db_path 兼容导出仍被路由使用。

符号：。

### `crates/vw-agent/src/gateway/api/handlers/mcp.rs`

14 → 14 行；具体 AppState/McpHost handler 装配；跨目录 inspect 用例还可在独立迁移中拆开认证与实际工具宿主端口。

符号：`handle_api_mcp_inspect`, `handle_api_mcp_app_call`。

### `crates/vw-agent/src/gateway/api/handlers/mcp_host.rs`

46 → 46 行；Host/ProbeHost 负责 session 租户、MCP app execution、诊断 job、ACP 挂载读取；probe_error 显式映射实际宿主错误。

符号：`inspect`, `current_tenant`, `call`, `probe_error`, `start_details`, `start_checks`, `read_mounts`。

### `crates/vw-agent/src/gateway/api/handlers/mcp_management_probe.rs`

17 → 17 行；submit 绑定具体 DiagnosticJobs/Host。

符号：`submit`。

### `crates/vw-agent/src/gateway/api/handlers/mcp_management_read.rs`

13 → 13 行；router 组合实际 probe/tools/acp 端点与 body limit；PolicyFile/McpHttpError 为调用者使用。

符号：`router`。

### `crates/vw-agent/src/gateway/api/handlers/mcp_management_resync.rs`

3 → 3 行；Job/Jobs/sync_saved_bindings 保留宿主其它处理器的直接引用。

符号：。

### `crates/vw-agent/src/gateway/api/handlers/mcp_management_tools.rs`

25 → 25 行；submit/read_acp 注入实际 ProbeHost。

符号：`submit`, `read_acp`。

### `crates/vw-agent/src/gateway/api/handlers/memory.rs`

38 → 38 行；MemoryAuth 与三个具体 AppState handler 保留真实认证装配。

符号：`require_auth`, `handle_api_memory_list`, `handle_api_memory_store`, `handle_api_memory_delete`。

### `crates/vw-agent/src/gateway/api/handlers/misc.rs`

53 → 53 行；MiscHost 绑定真实 agent/skill 目录、实例初始化、tenant task scope；测试辅助函数只给剩余真实宿主目录测试使用。

符号：`resolve_directory`, `with_instance`, `with_tenant_scope`, `commands`, `agents`, `skills`, `get_current`, `get_global`, `update_global`, `router`, `command_list`, `agent_list`, `agent_copy`, `skill_list`。

### `crates/vw-agent/src/gateway/api/handlers/misc_tests.rs`

304 → 280 行；保留真实目录 metadata、损坏旧项目文件容忍、disabled agent catalog、跨租户持久化 copy；StoredProjectCleanup/query_for/route_status/ProfileEnvGuard 管理真实存储和环境。纯 501 和 SkillInfo 序列化已迁出；路由方法和目录元数据断言可继续分离，但本轮不复制 DefinitionHost 或伪造目录。

- `router_builds_with_unit_state` → 原宿主模块保留：`misc_tests::router_builds_with_unit_state`；原因见本文件职责

- `router_dispatches_all_get_routes` → 原宿主模块保留：`misc_tests::router_dispatches_all_get_routes`；原因见本文件职责

- `router_rejects_non_get_methods_and_unknown_paths` → 原宿主模块保留：`misc_tests::router_rejects_non_get_methods_and_unknown_paths`；原因见本文件职责

- `not_implemented_returns_501` → `crates/libs/agent-gateway/src/host_migration_15_tests/misc.rs::not_implemented_returns_501`（原名/断言保留）

- `command_list_returns_named_command_metadata` → 原宿主模块保留：`misc_tests::command_list_returns_named_command_metadata`；原因见本文件职责

- `agent_list_returns_keyed_agent_metadata` → 原宿主模块保留：`misc_tests::agent_list_returns_keyed_agent_metadata`；原因见本文件职责

- `skill_list_returns_named_and_located_skill_metadata` → 原宿主模块保留：`misc_tests::skill_list_returns_named_and_located_skill_metadata`；原因见本文件职责

- `skill_info_serializes_all_public_response_fields` → `crates/libs/agent-gateway/src/host_migration_15_tests/misc.rs::skill_info_serializes_all_public_response_fields`（原名/断言保留）

- `list_handlers_ignore_legacy_invalid_project_json` → 原宿主模块保留：`misc_tests::list_handlers_ignore_legacy_invalid_project_json`；原因见本文件职责

- `agent_profile_api_catalog_includes_disabled_only_when_requested` → 原宿主模块保留：`misc_tests::agent_profile_api_catalog_includes_disabled_only_when_requested`；原因见本文件职责

- `agent_profile_api_copy_has_unique_custom_identity_and_survives_reload` → 原宿主模块保留：`misc_tests::agent_profile_api_copy_has_unique_custom_identity_and_survives_reload`；原因见本文件职责

- `agent_config_cleanup_catalog_copy_is_global_and_tenant_isolated` → 原宿主模块保留：`misc_tests::agent_config_cleanup_catalog_copy_is_global_and_tenant_isolated`；原因见本文件职责

### `crates/vw-agent/src/gateway/api/handlers/mod.rs`

445 → 440 行；跨领域模块/路由重导出及各所有者测试注册，是宿主组合根；本任务仅核对、不改写其它任务挂载。

符号：。

### `crates/vw-agent/src/gateway/api/handlers/model_catalog.rs`

16 → 16 行；router/router_with_policy 注入实际授权、provider home 和 macOS discovery 端口。

符号：`router`, `router_with_policy`。

## 来源直接跨目录挂载

- `crates/vw-agent/src/gateway/api/handlers/hooks.rs` → `crates/libs/gateway-hooks/src/hooks_tests.rs`：保持原挂载；其真实宿主端口归属未因物理文件在 libs 改变。

  - `hook_trust_rejects_stale_or_unknown_hash`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `hook_trust_mutation_reloads_runtime`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `hook_audit_response_omits_sensitive_payloads`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

- `crates/vw-agent/src/gateway/api/handlers/instance.rs` → `crates/libs/gateway-instance/src/instance_tests.rs`：保持原挂载；其真实宿主端口归属未因物理文件在 libs 改变。

  - `router_builds_with_app_state`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `response_structs_serialize_stable_api_shapes`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `router_path_get_prefers_query_directory_over_header`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `router_vcs_get_uses_directory_header_when_query_missing`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `router_instance_dispose_accepts_post_only`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `path_get_reports_global_and_instance_paths`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `path_get_uses_directory_header_when_query_missing`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `branch_for_worktree_returns_empty_for_blank_worktree`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `branch_for_worktree_returns_empty_when_directory_is_missing`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `vcs_get_returns_empty_branch_for_non_git_directory`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `vcs_get_reports_current_git_branch`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `instance_event_sse_builds_event_stream_response`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `instance_event_sse_sends_connected_event`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `instance_event_sse_emits_initial_and_periodic_heartbeat_events`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `instance_event_sse_filters_events_by_instance_directory`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `instance_event_sse_forwards_events_without_directory_scope`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `instance_event_sse_stops_when_receiver_is_dropped`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `instance_dispose_returns_true_and_clears_cached_instance`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `instance_dispose_clears_directory_approval_manager`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `instance_dispose_publishes_disposed_event_for_directory`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

- `crates/vw-agent/src/gateway/api/handlers/mcp.rs` → `crates/libs/gateway-mcp/src/host_tests/mcp_tests.rs`：保持原挂载；其真实宿主端口归属未因物理文件在 libs 改变。

  - `handle_api_mcp_inspect_returns_disabled_server_metadata`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `handle_api_mcp_inspect_rejects_missing_bearer_when_pairing_required`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `handle_api_mcp_inspect_maps_invalid_workspace_config_to_bad_request`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `codex_plugin_compat_t30_gateway_requires_authentication`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

- `crates/vw-agent/src/gateway/api/handlers/mod.rs` → `crates/libs/gateway-mcp/src/host_tests/mcp_management_create_tests.rs`：保持原挂载；其真实宿主端口归属未因物理文件在 libs 改变。

  - `cc_mcp_t33_create_validates_definition_and_persists_only_in_authorized_scope`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

- `crates/vw-agent/src/gateway/api/handlers/mod.rs` → `crates/libs/gateway-mcp/src/host_tests/mcp_management_delete_tests.rs`：保持原挂载；其真实宿主端口归属未因物理文件在 libs 改变。

  - `cc_mcp_t35_delete_revokes_each_target_and_retries_only_failures`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

- `crates/vw-agent/src/gateway/api/handlers/mod.rs` → `crates/libs/gateway-mcp/src/host_tests/mcp_management_read_tests.rs`：保持原挂载；其真实宿主端口归属未因物理文件在 libs 改变。

  - `cc_mcp_t32_gateway_reads_are_authorized_redacted_and_offline`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

- `crates/vw-agent/src/gateway/api/handlers/mod.rs` → `crates/libs/gateway-mcp/src/host_tests/mcp_management_update_tests.rs`：保持原挂载；其真实宿主端口归属未因物理文件在 libs 改变。

  - `cc_mcp_t34_update_checks_revision_and_never_claims_native_sync`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

- `crates/vw-agent/src/gateway/api/handlers/mod.rs` → `crates/libs/gateway-voice/src/host_tests/voice_handoff_session_tests.rs`：保持原挂载；其真实宿主端口归属未因物理文件在 libs 改变。

  - `voice_task_12_existing_session_duplicate_uses_real_producer_once`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `voice_task_12_existing_session_desktop_approval_rejection_prevents_write`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

- `crates/vw-agent/src/gateway/api/handlers/mod.rs` → `crates/libs/gateway-mcp/src/host_tests/mcp_management_bindings_tests.rs`：保持原挂载；其真实宿主端口归属未因物理文件在 libs 改变。

  - `cc_mcp_t36_bind_and_bulk_apply_real_native_results`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

- `crates/vw-agent/src/gateway/api/handlers/mod.rs` → `crates/libs/gateway-aggregation/tests/host/aggregation_resync_codex_tests.rs`：保持原挂载；其真实宿主端口归属未因物理文件在 libs 改变。

  - `aggregation_t16_codex_catalog_pointer_record_and_native_responses_settle_together`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

- `crates/vw-agent/src/gateway/api/handlers/mod.rs` → `crates/libs/gateway-mcp/src/host_tests/mcp_management_import_tests.rs`：保持原挂载；其真实宿主端口归属未因物理文件在 libs 改变。

  - `cc_mcp_t37_import_preserves_partial_results_and_authorization`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

- `crates/vw-agent/src/gateway/api/handlers/mod.rs` → `crates/libs/gateway-mcp/src/host_tests/mcp_management_resync_tests.rs`：保持原挂载；其真实宿主端口归属未因物理文件在 libs 改变。

  - `cc_mcp_t38_resync_repairs_native_and_reuses_owned_operations`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

- `crates/vw-agent/src/gateway/api/handlers/mod.rs` → `crates/libs/gateway-mcp/src/host_tests/mcp_management_probe_tests.rs`：保持原挂载；其真实宿主端口归属未因物理文件在 libs 改变。

  - `cc_mcp_t39_checks_reuse_scoped_jobs_and_cancel_only_the_owned_batch`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

- `crates/vw-agent/src/gateway/api/handlers/mod.rs` → `crates/libs/gateway-aggregation/tests/host/aggregation_restart_tests.rs`：保持原挂载；其真实宿主端口归属未因物理文件在 libs 改变。

  - `aggregation_t45_aggregation_restart_preserves_contract`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `aggregation_t45_not_running_replays_and_independent_permissions`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `aggregation_t45_request_boundaries_reject_arbitrary_process_control`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `aggregation_t45_invalid_daemon_record_reports_failure_without_side_effects`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `aggregation_t45_real_unmanaged_codex_is_never_restarted`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

- `crates/vw-agent/src/gateway/api/handlers/mod.rs` → `crates/libs/gateway-mcp/src/host_tests/mcp_management_tools_tests.rs`：保持原挂载；其真实宿主端口归属未因物理文件在 libs 改变。

  - `cc_mcp_t50_acp_http_reads_only_explicitly_granted_scoped_sessions`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

  - `cc_mcp_t50_detail_routes_do_not_shadow_existing_catalog_record_ids`：保留原注册（后续纯领域拆分项，未冒充领域测试）。

## 行数分类与未来验证

来源生产/装配文件的 cfg(test) 导入和挂载属于测试支持，不计业务迁移；global_tests/integrations_tests/hermes_memory_tests/misc_tests/file_drag 均为测试及夹具。

- 领域新增 `hermes_memory.rs`：61 测试/夹具行，2 条用例；生产逻辑0行。

- 领域新增 `global.rs`：700 测试/夹具行，22 条用例；生产逻辑0行。

- 领域新增 `misc.rs`：29 测试/夹具行，2 条用例；生产逻辑0行。

- 领域新增 `integrations.rs`：507 测试/夹具行，15 条用例；生产逻辑0行。

生产/装配（包含原有cfg(test)支持，不是纯生产逻辑）：841 → 822 行。

独立测试/夹具：2045 → 901 行。

未来：`python3 scripts/cargo_slot.py cargo test -p agent-gateway --lib host_migration_15_tests -- --test-threads=1`，预期41条（Unix），非Unix39条。宿主按上列保留模块完整名逐一过滤：global_tests 2、misc_tests 10、file_drag_integration_tests 5（Unix），跨目录用例按上列真实名称分别验证。迁出名称未来以 vw-agent --lib -- --list 确认消失，零测试不算通过。

当前仅进行了源码、注册、依赖方向与 diff whitespace 静态检查；Cargo/list/fmt/check/test、构建、实机、行为检查全部未运行，遵循计划禁止执行约束。不声称编译收益。Cargo.lock 仅手工登记现存包的测试依赖，未来须由 Cargo 验证。

静态名称集合核对：原58条 = 领域41条 + 宿主17条，未删除或重复场景。
