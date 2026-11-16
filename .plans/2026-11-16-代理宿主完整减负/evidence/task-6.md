# 任务6静态迁移证据

代码完成，编译/行为未验证；禁止运行 Cargo、构建、单元测试及实机测试，均未运行。未声称性能收益。按 do-task-plans 与 rustcodegraph 技能执行；索引早于当前批次，变更源码以直接读取为准。

宿主本组 4093 → 1507 行，净减 2586（63.2%）；领域测试新增 2683 行，迁入 42 个测试。生产实现迁移 0 行；生产组合/转发保留，测试及夹具减少 2586 行（父模块减少部分为 cfg/test 注册）。原生产路由早已下沉，不能计为本轮成果。领域生产代码仅新增 cfg(test) 注册。具体逐文件行数见下。

依赖方向：vw-agent → gateway-app-providers → app-provider-services/agent-database。新增依赖均为 dev-dependencies（SDK、真实共享 Basic/租户中间件及发布适配器）；无反向 vw-agent 依赖、无复制 App/AppState、无 path/include 加载宿主。Claude 通用写路由使用现有 gateway-app-routing ProviderPublication control(None)，原 Host 包装所注入的真实适配器；Codex 显式传入相同 control。record_router 组合真实 GET/PUT/POST、basic、duplicate；不重写算法。测试名称与断言保持，正式 handler_router 鉴权仍在宿主。

领域由 src/lib.rs → host_migration_6_tests 自注册。basic/create 保持全平台，原 macOS 场景仍仅 macOS。Claude login/managed_binding 保持嵌套注册。Logs 及 Subscriber 方法仍在宿主 api_security_tests（矩阵跨 operations、Pi、Omp、Desktop 宿主适配）。Agy terminal 与受控进程用例保留，两个独立 store 场景迁入领域；原 launch.rs 挂载继续消费真实边界。

低于70%：主要保留 api_security_tests 471行（包含全矩阵真实宿主装配），agy_tests 进程/PTTY/身份授权验收；其余生产端口、薄重导出及外部挂载不可算业务下沉。本组仍可继续拆 security 领域断言并由聚合所有者提供真实装配；不得以删断言凑比例。收口任务须验收该缺口。外部 host_tests 的跨目录挂载属于下一文件组，当前父模块保持其挂载，未重复接管。

未来 macOS 命令：`python3 scripts/cargo_slot.py cargo test -p gateway-app-providers --lib host_migration_6_tests -- --test-threads=1`，预期 42 个非零测试；实际通过数/退出码未知。宿主原完整名称按下表保留项精确过滤；迁出项未来以 vw-agent --lib -- --list 确认消失，不能把零测试算通过。

## 全文件与用例映射

- `crates/vw-agent/src/gateway/api/handlers/app_providers/gemini.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/claude_desktop.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/opencode.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/pi.rs`：22 → 22 行。具体宿主装配：router/router_with_policy、PublicationControl、聚合/映射 Host 或 create/update 写入适配；保持真实端口。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/claude_login_tests.rs`：138 → 0 行。整组测试迁出，旧挂载已删除。
  - `cc04_claude_login_sdk_api_applies_bound_account_and_archives_original` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/claude_login_tests.rs`
  - `cc04_claude_login_sdk_api_denial_missing_account_and_write_failure_preserve_login` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/claude_login_tests.rs`

- `crates/vw-agent/src/gateway/api/handlers/app_providers/hermes.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/icons.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/claude.rs`：28 → 28 行。具体宿主装配：router/router_with_policy、PublicationControl、聚合/映射 Host 或 create/update 写入适配；保持真实端口。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/env_diagnose.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/grok.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/api_security_tests.rs`：471 → 471 行。用例逐条见下；保留真实鉴权/正式路由/进程边界。
  - `cc04_api_security_rejects_cross_scope_mutation` → `crates/vw-agent/src/gateway/api/handlers/app_providers/api_security_tests.rs`
  - `cc04_api_security_nested_secrets_never_reach_responses` → `crates/vw-agent/src/gateway/api/handlers/app_providers/api_security_tests.rs`
  - `cc04_api_security_reads_errors_and_progress_are_safe` → `crates/vw-agent/src/gateway/api/handlers/app_providers/api_security_tests.rs`
  - `cc04_api_security_registered_routes_deny_missing_gateway_auth_before_storage` → `crates/vw-agent/src/gateway/api/handlers/app_providers/api_security_tests.rs`

- `crates/vw-agent/src/gateway/api/handlers/app_providers/basic_tests.rs`：190 → 0 行。整组测试迁出，旧挂载已删除。
  - `cc04_ui_edit_sdk_basic_create_update_copy_preserves_executor_fields` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/basic_tests.rs`
  - `cc04_ui_edit_denied_invalid_and_foreign_scope_do_not_write` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/basic_tests.rs`
  - `cc04_ui_edit_native_metadata_keep_preserves_nested_configuration_and_rejects_legacy_secret_write` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/basic_tests.rs`

- `crates/vw-agent/src/gateway/api/handlers/app_providers/mode_state.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/mode_apply.rs`：21 → 21 行。具体宿主装配：router/router_with_policy、PublicationControl、聚合/映射 Host 或 create/update 写入适配；保持真实端口。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/desktop_mapping.rs`：39 → 39 行。具体宿主装配：router/router_with_policy、PublicationControl、聚合/映射 Host 或 create/update 写入适配；保持真实端口。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/env_restore.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/paths.rs`：4 → 4 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/create.rs`：25 → 25 行。具体宿主装配：router/router_with_policy、PublicationControl、聚合/映射 Host 或 create/update 写入适配；保持真实端口。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/preview.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/import.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/dsh.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/delete.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/update.rs`：26 → 26 行。具体宿主装配：router/router_with_policy、PublicationControl、聚合/映射 Host 或 create/update 写入适配；保持真实端口。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/catalog.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/agy.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/publication_host.rs`：14 → 14 行。具体宿主装配：router/router_with_policy、PublicationControl、聚合/映射 Host 或 create/update 写入适配；保持真实端口。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/endpoints.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/claude_managed_binding_tests.rs`：214 → 0 行。整组测试迁出，旧挂载已删除。
  - `provider_claude_managed_binding_retains_catalog_and_live_references_under_lease` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/claude_managed_binding_tests.rs`
  - `provider_claude_managed_binding_rejects_legacy_and_unknown_live_identity` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/claude_managed_binding_tests.rs`

- `crates/vw-agent/src/gateway/api/handlers/app_providers/claude_desktop_tests.rs`：276 → 0 行。整组测试迁出，旧挂载已删除。
  - `cc04_claude_desktop_sdk_api_switch_edit_reapply_and_official` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/claude_desktop_tests.rs`
  - `cc04_claude_desktop_denial_and_generic_import_leave_files_and_catalog_untouched` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/claude_desktop_tests.rs`
  - `cc04_claude_desktop_activation_and_host_scope_are_enforced` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/claude_desktop_tests.rs`
  - `cc04_claude_desktop_mapping_and_managed_rows_require_routing_without_writes` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/claude_desktop_tests.rs`

- `crates/vw-agent/src/gateway/api/handlers/app_providers/openclaw.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/universal.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/env_clean.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/codex_catalog.rs`：14 → 14 行。具体宿主装配：router/router_with_policy、PublicationControl、聚合/映射 Host 或 create/update 写入适配；保持真实端口。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/agy_tests.rs`：362 → 276 行。用例逐条见下；保留真实鉴权/正式路由/进程边界。
  - `cc04_agy_native_card_is_saved_without_live_activation` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/agy_tests.rs`
  - `cc04_agy_sdk_api_launch_uses_persistent_profile_and_denies_before_writes` → `crates/vw-agent/src/gateway/api/handlers/app_providers/agy_tests.rs`
  - `cc04_ui_agy_editor_preserves_metadata_and_latest_secret` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/agy_tests.rs`

- `crates/vw-agent/src/gateway/api/handlers/app_providers/request_options.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/codex_tests.rs`：75 → 0 行。整组测试迁出，旧挂载已删除。
  - `cc04_codex_sdk_api_profile_refusal_is_redacted_and_readonly` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/codex_tests.rs`

- `crates/vw-agent/src/gateway/api/handlers/app_providers/universal_sync.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/catalog_tests.rs`：192 → 41 行。用例逐条见下；保留真实鉴权/正式路由/进程边界。
  - `cc04_catalog_api_lists_only_authenticated_tenant_and_application` → `crates/vw-agent/src/gateway/api/handlers/app_providers/catalog_tests.rs`
  - `cc04_catalog_api_scopes_rows_and_denies_config_without_independent_grant` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/catalog_tests.rs`
  - `cc04_catalog_sdk_http_reads_native_files_without_writes` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/catalog_tests.rs`

- `crates/vw-agent/src/gateway/api/handlers/app_providers/mcode.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/editor_merge.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/omp.rs`：22 → 22 行。具体宿主装配：router/router_with_policy、PublicationControl、聚合/映射 Host 或 create/update 写入适配；保持真实端口。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/kimi.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/create_tests.rs`：206 → 0 行。整组测试迁出，旧挂载已删除。
  - `cc04_create_api_rejects_unauthorized_and_duplicates_without_mutation` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/create_tests.rs`
  - `cc04_create_sdk_api_persists_without_native_file_changes` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/create_tests.rs`
  - `cc04_create_sqlite_concurrent_duplicate_has_one_winner` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/create_tests.rs`
  - `cc04_create_api_rejects_live_and_missing_secret_without_records` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/create_tests.rs`

- `crates/vw-agent/src/gateway/api/handlers/app_providers/snippets.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/presentation.rs`：8 → 8 行。具体宿主装配：router/router_with_policy、PublicationControl、聚合/映射 Host 或 create/update 写入适配；保持真实端口。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/operations.rs`：23 → 23 行。具体宿主装配：router/router_with_policy、PublicationControl、聚合/映射 Host 或 create/update 写入适配；保持真实端口。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/codex_catalog_tests.rs`：515 → 0 行。整组测试迁出，旧挂载已删除。
  - `cc04_codex_catalog_sdk_api_commits_saved_directory_and_scoped_pointer` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/codex_catalog_tests.rs`
  - `cc04_codex_catalog_sdk_api_denial_and_profile_failure_keep_native_files` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/codex_catalog_tests.rs`
  - `cc04_codex_catalog_saved_revisions_are_deployment_and_tenant_scoped` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/codex_catalog_tests.rs`
  - `cc04_codex_catalog_sdk_api_auth_stash_preservation_and_failure` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/codex_catalog_tests.rs`
  - `cc04_codex_catalog_sdk_api_recovers_marked_catalog_and_pointer_before_replanning` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/codex_catalog_tests.rs`
  - `cc04_codex_catalog_missing_write_grant_and_managed_rows_are_rejected` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/codex_catalog_tests.rs`
  - `cc04_ui_codex_sdk_edits_mapping_and_preserves_inactive_fields_through_native_transaction` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/codex_catalog_tests.rs`
  - `auth6_remove_codex_reference_is_private_and_global_switch_is_rejected` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/codex_catalog_tests.rs`

- `crates/vw-agent/src/gateway/api/handlers/app_providers/claude_tests.rs`：633 → 30 行。用例逐条见下；保留真实鉴权/正式路由/进程边界。
  - `cc04_ui_paths_saved_directory_is_used_by_native_activation_and_import` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/claude_tests.rs`
  - `cc04_claude_sdk_api_switches_four_variants_and_cleans_credentials` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/claude_tests.rs`
  - `cc04_claude_current_edit_reapply_import_and_inactive_edit_share_adapter` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/claude_tests.rs`
  - `cc04_claude_denied_and_invalid_requests_leave_database_and_files_untouched` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/claude_tests.rs`
  - `cc04_claude_sync_projects_existing_universal_owner_and_applies_current_child` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/claude_tests.rs`
  - `cc04_claude_live_failure_reports_saved_catalog_and_reapply_repairs_it` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/claude_tests.rs`
  - `cc04_claude_legacy_path_first_activation_and_symlink_rejection` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/claude_tests.rs`
  - `cc04_claude_registered_route_requires_gateway_authentication` → `crates/vw-agent/src/gateway/api/handlers/app_providers/claude_tests.rs`
  - `cc04_claude_first_activation_creates_missing_config_and_catalog_defaults_do_not` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/claude_tests.rs`
  - `cc04_claude_committed_exclusives_survive_catalog_edits_until_switch_cleans_them` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/claude_tests.rs`
  - `cc04_ui_claude_sdk_round_trip_projects_key_fields_on_the_authorized_host` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/claude_tests.rs`
  - `cc04_ui_claude_sdk_validation_and_permission_failures_do_not_write` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/claude_tests.rs`

- `crates/vw-agent/src/gateway/api/handlers/app_providers/delete_tests.rs`：191 → 72 行。用例逐条见下；保留真实鉴权/正式路由/进程边界。
  - `cc04_delete_sdk_http_rejects_members_and_deletes_only_unused_rows` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/delete_tests.rs`
  - `cc04_delete_api_permission_scope_and_unknown_config_denials_preserve_rows` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/delete_tests.rs`
  - `cc04_delete_store_preserves_other_tenants_and_applications` → `crates/libs/gateway-app-providers/src/host_migration_6_tests/delete_tests.rs`
  - `cc04_delete_is_registered_in_formal_handler_router` → `crates/vw-agent/src/gateway/api/handlers/app_providers/delete_tests.rs`

- `crates/vw-agent/src/gateway/api/handlers/app_providers/basic.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers/duplicate.rs`：2 → 2 行。薄重导出：仍有父 router、相邻宿主测试调用；非本轮生产迁移收益。


- `crates/vw-agent/src/gateway/api/handlers/app_providers.rs`：330 → 317 行。生产组合 router/save/PublicationInput/publication_store/sync_saved_provider；保留发布端口与其他任务测试挂载。
