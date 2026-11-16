# 任务14 静态迁移证据

代码完成，编译/行为未验证。本轮遵守计划禁止 Cargo、构建、单元及实机测试；未获取任何运行通过数或性能结论。

## 调用与归属
宿主路由继续调用 icons/auth/routing 等真实领域入口；此次只迁测试与消除转发，不宣称此前已经下沉的业务为生产迁移成果。gateway-agent-apps 自注册应用用例；gateway-app-routing 自注册 failover 与 record_resync 用例。两者仅依赖既有领域服务、agent-gateway，不反向依赖 vw-agent。复用任务3授权路由、任务4原生探测、任务7真实存储和 RoutingListener 生命周期夹具；没有复制 App/AppState。

## 全文件与用例映射
原用例名称、断言保留；下表函数名为各文件完整测试清单，空列表表示无本地测试函数。include 文件另列实际被加载的测试。

### `crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_remove_tests.rs`
- 335 行 → crates/libs/gateway-agent-apps/src/host_migration_14_tests/agent_apps_auth_remove_tests.rs
- 用例：`auth6_api_remove_commits_expected_scoped_result`、`auth6_api_remove_rejects_invalid_or_stale_input`、`auth6_api_remove_commits_expected_scoped_result_postgres`、`auth6_api_remove_rejects_invalid_or_stale_input_postgres`

### `crates/vw-agent/src/gateway/api/handlers/agent_apps_data/mod.rs`
- 2 行 → 删除无实现转发/空文件；父模块使用真实领域模块别名，调用路径保持
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/agent_apps_delete_tests.rs`
- 53 行 → crates/libs/gateway-agent-apps/src/host_migration_14_tests/agent_apps_delete_tests.rs
- 用例：`cc_base_icon_api_delete_sqlite_idempotent_and_scoped`、`cc_base_icon_api_delete_postgres_idempotent_and_scoped`

### `crates/vw-agent/src/gateway/api/handlers/agent_apps_diagnostics_endpoint_drafts_tests.rs`
- 148 行 → crates/libs/gateway-agent-apps/src/host_migration_4_tests/agent_apps_diagnostics_endpoint_drafts_tests.rs
- 用例：`diagnostic_endpoint_draft_reference_executes_without_save`

### `crates/vw-agent/src/gateway/api/handlers/agent_apps_error.rs`
- 2 行 → 删除无实现转发/空文件；父模块使用真实领域模块别名，调用路径保持
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/agent_apps_ha_acceptance_tests.rs`
- 210 行 → crates/libs/gateway-app-routing/src/host_migration_14_tests/agent_apps_ha_acceptance_tests.rs
- 用例：`ha_macos_headless_routing_acceptance`

### `crates/vw-agent/src/gateway/api/handlers/agent_apps_high_availability.rs`
- 2 行 → 保留 2 行；宿主路径/数据库/权限/生命周期端口装配或公共兼容入口；生产实现已在领域库
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/agent_apps_high_availability_tests.rs`
- 585 行 → crates/libs/gateway-app-routing/src/host_migration_14_tests/agent_apps_high_availability_tests.rs
- 用例：`ha_api_queue_read_enforces_scope`、`ha_api_queue_write_rejects_conflicts`、`ha_api_queue_write_rejects_conflicts_enabled`、`ha_api_health_and_policy_are_scope_bound`

### `crates/vw-agent/src/gateway/api/handlers/agent_apps_rename_tests.rs`
- 83 行 → crates/libs/gateway-agent-apps/src/host_migration_14_tests/agent_apps_rename_tests.rs
- 用例：`cc_base_icon_api_rename_sqlite_boundaries_and_scope`、`cc_base_icon_api_rename_postgres_boundaries_and_scope`

### `crates/vw-agent/src/gateway/api/handlers/agent_apps_skills/mod.rs`
- 2 行 → 删除无实现转发/空文件；父模块使用真实领域模块别名，调用路径保持
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/aggregation_clients.rs`
- 2 行 → 保留 2 行；宿主路径/数据库/权限/生命周期端口装配或公共兼容入口；生产实现已在领域库
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/aggregation_clients_tests.rs`
- 2 行 → 保留 2 行；真实宿主测试装配：依赖宿主路由/全局会话/鉴权组合；后续可按独立领域端口继续拆分，当前保留原覆盖；跨目录原挂载 crates/libs/gateway-aggregation/tests/host/aggregation_clients_tests.rs 保留，未谎称自注册
- 用例：`aggregation_t44_aggregation_clients_preserves_contract`、`aggregation_t44_permissions_and_request_boundaries`、`aggregation_t44_rejects_unsafe_ack_and_preserves_external_file`、`aggregation_t44_unreadable_identity_is_unverified_and_get_never_creates_home`、`aggregation_t44_real_codex_survives_ack_and_keeps_stale_flags`、`aggregation_t44_explicit_write_grant_and_target_failures`、`aggregation_t44_acknowledgement_suppresses_only_current_native_reminder`、`aggregation_t44_snapshot_distinguishes_external_catalog_without_reading_it`

### `crates/vw-agent/src/gateway/api/handlers/aggregation_codex_settings.rs`
- 20 行 → 保留 20 行；宿主路径/数据库/权限/生命周期端口装配或公共兼容入口；生产实现已在领域库
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/aggregation_codex_settings_tests.rs`
- 2 行 → 保留 2 行；真实宿主测试装配：依赖宿主路由/全局会话/鉴权组合；后续可按独立领域端口继续拆分，当前保留原覆盖；跨目录原挂载 crates/libs/gateway-aggregation/tests/host/aggregation_codex_settings_tests.rs 保留，未谎称自注册
- 用例：`aggregation_t43_aggregation_codex_settings_preserves_contract`、`aggregation_t43_denies_missing_auth_foreign_project_and_cross_application`、`aggregation_t43_rejects_unpublished_models_and_stale_revision_without_writes`、`aggregation_t43_requires_write_and_attached_routing_permissions`、`aggregation_t43_normalizes_effort_clears_model_and_preserves_user_fields`、`aggregation_t43_reports_forced_v2_and_can_disable_classic_projection`、`aggregation_t43_invalid_payload_and_symlink_do_not_publish`、`aggregation_t43_pending_intent_uses_existing_recovery_and_blocks_api`、`aggregation_t43_membership_change_retains_intent_and_revalidates_choices`、`aggregation_t43_native_write_failure_returns_target_failed_without_saving_intent`、`aggregation_t43_reports_unavailable_and_unsupported_hosts`、`aggregation_t43_official_default_requires_verified_account_generation`

### `crates/vw-agent/src/gateway/api/handlers/aggregation_codex_settings_fixture.rs`
- 2 行 → 保留 2 行；宿主路径/数据库/权限/生命周期端口装配或公共兼容入口；生产实现已在领域库；跨目录原挂载 crates/libs/gateway-aggregation/tests/host/aggregation_codex_settings_fixture.rs 保留，未谎称自注册
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/aggregation_desktop.rs`
- 29 行 → 保留 29 行；宿主路径/数据库/权限/生命周期端口装配或公共兼容入口；生产实现已在领域库
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/aggregation_desktop_tests.rs`
- 2 行 → 保留 2 行；真实宿主测试装配：依赖宿主路由/全局会话/鉴权组合；后续可按独立领域端口继续拆分，当前保留原覆盖；跨目录原挂载 crates/libs/gateway-aggregation/tests/host/aggregation_desktop_tests.rs 保留，未谎称自注册
- 用例：`aggregation_t46_aggregation_desktop_preserves_contract`、`aggregation_t46_mapping_revisions_permissions_and_installation_are_enforced`、`aggregation_t46_mapping_retry_recovers_only_the_owned_durable_record`、`aggregation_t46_attached_candidate_requires_a_live_desktop_publication`

### `crates/vw-agent/src/gateway/api/handlers/aggregation_desktop_routing_tests.rs`
- 2 行 → 保留 2 行；真实宿主测试装配：依赖宿主路由/全局会话/鉴权组合；后续可按独立领域端口继续拆分，当前保留原覆盖；跨目录原挂载 crates/libs/gateway-aggregation/tests/host/aggregation_desktop_routing_tests.rs 保留，未谎称自注册
- 用例：`aggregation_t46_desktop_discovery_and_messages_use_one_scoped_listener`

### `crates/vw-agent/src/gateway/api/handlers/aggregation_desktop_write.rs`
- 1 行 → 删除无实现转发/空文件；父模块使用真实领域模块别名，调用路径保持
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/aggregation_exit.rs`
- 8 行 → 保留 8 行；宿主路径/数据库/权限/生命周期端口装配或公共兼容入口；生产实现已在领域库
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/aggregation_exit_tests.rs`
- 2 行 → 保留 2 行；真实宿主测试装配：依赖宿主路由/全局会话/鉴权组合；后续可按独立领域端口继续拆分，当前保留原覆盖；跨目录原挂载 crates/libs/gateway-aggregation/tests/host/aggregation_exit_tests.rs 保留，未谎称自注册
- 用例：`aggregation_t42_aggregation_exit_preserves_contract`、`aggregation_t42_exit_restores_direct_and_replay_is_action_scoped`、`aggregation_t42_auth_and_old_revision_leave_files_unchanged`、`aggregation_t42_restore_io_failure_is_queryable_and_not_success`、`aggregation_t42_codex_and_desktop_use_native_restore_owners`、`aggregation_t42_exit_after_gateway_detach_clears_saved_intent_without_rewriting_client`、`aggregation_t42_ordinary_routing_intent_is_not_reported_as_exited`

### `crates/vw-agent/src/gateway/api/handlers/aggregation_exit_operation_tests.rs`
- 2 行 → 保留 2 行；真实宿主测试装配：依赖宿主路由/全局会话/鉴权组合；后续可按独立领域端口继续拆分，当前保留原覆盖；跨目录原挂载 crates/libs/gateway-aggregation/tests/host/aggregation_exit_operation_tests.rs 保留，未谎称自注册
- 用例：`aggregation_t42_processing_cancel_waits_for_worker_and_preserves_published_result`

### `crates/vw-agent/src/gateway/api/handlers/aggregation_members.rs`
- 15 行 → 保留 15 行；宿主路径/数据库/权限/生命周期端口装配或公共兼容入口；生产实现已在领域库
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/aggregation_members_tests.rs`
- 2 行 → 保留 2 行；真实宿主测试装配：依赖宿主路由/全局会话/鉴权组合；后续可按独立领域端口继续拆分，当前保留原覆盖；跨目录原挂载 crates/libs/gateway-aggregation/tests/host/aggregation_members_tests.rs 保留，未谎称自注册
- 用例：`aggregation_t40_aggregation_members_preserves_contract`、`aggregation_t40_write_authorization_and_scope_precede_side_effects`、`aggregation_t40_stale_revision_and_invalid_intents_preserve_state`、`aggregation_t40_active_default_removal_is_explicit_and_does_not_publish`、`aggregation_t40_target_failure_preserves_members_and_revision`、`aggregation_t40_default_adds_member_and_native_change_invalidates_revision`、`aggregation_t40_active_changes_republish_real_catalog_and_routes`、`aggregation_t40_active_changes_reject_failover_before_publication`

### `crates/vw-agent/src/gateway/api/handlers/aggregation_members_client_tests.rs`
- 2 行 → 保留 2 行；真实宿主测试装配：依赖宿主路由/全局会话/鉴权组合；后续可按独立领域端口继续拆分，当前保留原覆盖；跨目录原挂载 crates/libs/gateway-aggregation/tests/host/aggregation_members_client_tests.rs 保留，未谎称自注册
- 用例：`aggregation_t40_codex_and_desktop_save_planned_members_in_original_slot`、`aggregation_t40_codex_active_default_changes_native_selected_model`、`aggregation_t40_desktop_new_default_requires_a_live_publication`

### `crates/vw-agent/src/gateway/api/handlers/aggregation_members_ordinary_tests.rs`
- 2 行 → 保留 2 行；真实宿主测试装配：依赖宿主路由/全局会话/鉴权组合；后续可按独立领域端口继续拆分，当前保留原覆盖；跨目录原挂载 crates/libs/gateway-aggregation/tests/host/aggregation_members_ordinary_tests.rs 保留，未谎称自注册
- 用例：`aggregation_t40_ordinary_claude_default_publishes_one_target_and_preserves_failed_preimage`、`aggregation_t40_ordinary_codex_default_publishes_one_target_and_preserves_failed_preimage`、`aggregation_t40_ordinary_desktop_default_publishes_one_target_and_preserves_failed_preimage`

### `crates/vw-agent/src/gateway/api/handlers/aggregation_omo.rs`
- 10 行 → 保留 10 行；宿主路径/数据库/权限/生命周期端口装配或公共兼容入口；生产实现已在领域库
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/aggregation_omo_tests.rs`
- 2 行 → 保留 2 行；真实宿主测试装配：依赖宿主路由/全局会话/鉴权组合；后续可按独立领域端口继续拆分，当前保留原覆盖；跨目录原挂载 crates/libs/gateway-aggregation/tests/host/aggregation_omo_tests.rs 保留，未谎称自注册
- 用例：`aggregation_t47_aggregation_omo_preserves_contract`、`aggregation_t47_scope_and_permissions_precede_writes`、`aggregation_t47_stale_revision_and_variants_are_explicit_conflicts`、`aggregation_t47_profiles_and_unknown_inputs_are_rejected`、`aggregation_t47_corrupt_and_unsafe_native_inputs_fail_closed`、`aggregation_t47_read_is_observational_and_legacy_slots_are_not_claimed`、`aggregation_t47_missing_unified_write_is_explicitly_unsupported`、`aggregation_t47_pending_selection_recovers_with_the_same_native_transaction`、`aggregation_t47_unavailable_targets_are_distinct_from_invalid_configuration`、`aggregation_t47_missing_native_file_can_disable_saved_current_but_not_legacy_files`、`aggregation_t47_untracked_opposite_configuration_requires_explicit_disable`

### `crates/vw-agent/src/gateway/api/handlers/aggregation_omp.rs`
- 2 行 → 删除无实现转发/空文件；父模块使用真实领域模块别名，调用路径保持
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/aggregation_omp_tests.rs`
- 2 行 → 保留 2 行；真实宿主测试装配：依赖宿主路由/全局会话/鉴权组合；后续可按独立领域端口继续拆分，当前保留原覆盖；跨目录原挂载 crates/libs/gateway-aggregation/tests/host/aggregation_omp_tests.rs 保留，未谎称自注册
- 用例：`aggregation_t49_aggregation_omp_preserves_contract`、`aggregation_t49_writes_only_selected_profile_and_target_entry`、`aggregation_t49_denies_missing_grants_and_foreign_scope`、`aggregation_t49_conflicts_and_invalid_yaml_leave_native_bytes_untouched`、`aggregation_t49_unclaimed_slot_symlinks_and_unknown_fields_are_rejected`、`aggregation_t49_pending_intent_is_not_recovered_by_read_or_write`

### `crates/vw-agent/src/gateway/api/handlers/aggregation_pi.rs`
- 2 行 → 删除无实现转发/空文件；父模块使用真实领域模块别名，调用路径保持
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/aggregation_pi_tests.rs`
- 2 行 → 保留 2 行；真实宿主测试装配：依赖宿主路由/全局会话/鉴权组合；后续可按独立领域端口继续拆分，当前保留原覆盖；跨目录原挂载 crates/libs/gateway-aggregation/tests/host/aggregation_pi_tests.rs 保留，未谎称自注册
- 用例：`aggregation_t48_aggregation_pi_preserves_contract`、`aggregation_t48_rejects_unauthorized_and_cross_project_access`、`aggregation_t48_revision_and_strict_entry_semantics`、`aggregation_t48_unknown_fields_and_legacy_scope_bypass_are_rejected`、`aggregation_t48_preserves_other_providers_and_native_secrets`、`aggregation_t48_rejects_unbound_and_unsafe_native_targets`、`aggregation_t48_pending_transaction_is_not_recovered_by_directory_api`、`aggregation_t48_host_directory_override_controls_native_target`、`aggregation_t48_unavailable_host_and_wrong_tenant_fail_without_changes`、`aggregation_t48_legacy_native_operations_cannot_bypass_project_binding`

### `crates/vw-agent/src/gateway/api/handlers/aggregation_publish.rs`
- 11 行 → 保留 11 行；宿主路径/数据库/权限/生命周期端口装配或公共兼容入口；生产实现已在领域库
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/aggregation_publish_tests.rs`
- 2 行 → 保留 2 行；真实宿主测试装配：依赖宿主路由/全局会话/鉴权组合；后续可按独立领域端口继续拆分，当前保留原覆盖；跨目录原挂载 crates/libs/gateway-aggregation/tests/host/aggregation_publish_tests.rs 保留，未谎称自注册
- 用例：`aggregation_t41_aggregation_publish_preserves_contract`、`aggregation_t41_authorization_revision_and_signed_intent_precede_side_effects`、`aggregation_t41_initial_publication_reuses_catalog_transaction_and_live_route`、`aggregation_t41_target_failure_preserves_original_state_and_native_bytes`

### `crates/vw-agent/src/gateway/api/handlers/aggregation_publish_client_tests.rs`
- 2 行 → 保留 2 行；真实宿主测试装配：依赖宿主路由/全局会话/鉴权组合；后续可按独立领域端口继续拆分，当前保留原覆盖；跨目录原挂载 crates/libs/gateway-aggregation/tests/host/aggregation_publish_client_tests.rs 保留，未谎称自注册
- 用例：`aggregation_t41_codex_initial_publication_honors_confirmed_catalog_takeover`、`aggregation_t41_desktop_initial_publication_requires_installation_and_writes_original_profile`

### `crates/vw-agent/src/gateway/api/handlers/aggregation_publish_failure_tests.rs`
- 2 行 → 保留 2 行；真实宿主测试装配：依赖宿主路由/全局会话/鉴权组合；后续可按独立领域端口继续拆分，当前保留原覆盖；跨目录原挂载 crates/libs/gateway-aggregation/tests/host/aggregation_publish_failure_tests.rs 保留，未谎称自注册
- 用例：`aggregation_t41_unavailable_controller_does_not_claim_publication`、`aggregation_t41_native_io_failure_reports_target_failed_without_a_false_commit`

### `crates/vw-agent/src/gateway/api/handlers/aggregation_publish_operation_tests.rs`
- 2 行 → 保留 2 行；真实宿主测试装配：依赖宿主路由/全局会话/鉴权组合；后续可按独立领域端口继续拆分，当前保留原覆盖；跨目录原挂载 crates/libs/gateway-aggregation/tests/host/aggregation_publish_operation_tests.rs 保留，未谎称自注册
- 用例：`aggregation_t41_signed_preview_expiry_and_subject_are_enforced`、`aggregation_t41_cancellation_respects_original_publication_boundary`、`aggregation_t41_concurrent_request_payload_changes_return_request_conflict`

### `crates/vw-agent/src/gateway/api/handlers/aggregation_read.rs`
- 10 行 → 保留 10 行；宿主路径/数据库/权限/生命周期端口装配或公共兼容入口；生产实现已在领域库
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/aggregation_read_tests.rs`
- 2 行 → 保留 2 行；真实宿主测试装配：依赖宿主路由/全局会话/鉴权组合；后续可按独立领域端口继续拆分，当前保留原覆盖；跨目录原挂载 crates/libs/gateway-aggregation/tests/host/aggregation_read_tests.rs 保留，未谎称自注册
- 用例：`aggregation_t39_aggregation_read_preserves_contract`、`aggregation_t39_denies_missing_credentials_host_and_real_foreign_project`、`aggregation_t39_unclaimed_state_and_pending_are_conflicts_without_recovery`、`aggregation_t39_preview_blocks_invalid_default_and_failover_without_issuing_token`、`aggregation_t39_revision_changes_with_native_inputs_and_rejects_unsafe_paths`、`aggregation_t39_rejects_unknown_preview_fields_and_non_codex_catalog_policy`、`aggregation_t39_codex_preview_uses_saved_profile_without_publishing_catalog`、`aggregation_t39_foreign_tenant_state_is_forbidden_before_preview`、`aggregation_t39_corrupt_native_configuration_cannot_receive_preview_token`、`aggregation_t39_desktop_preview_uses_native_mapping_without_creating_profile`

### `crates/vw-agent/src/gateway/api/handlers/aggregation_read_snapshot.rs`
- 2 行 → 删除无实现转发/空文件；父模块使用真实领域模块别名，调用路径保持
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/aggregation_restart.rs`
- 10 行 → 保留 10 行；宿主路径/数据库/权限/生命周期端口装配或公共兼容入口；生产实现已在领域库
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/aggregation_resync.rs`
- 2 行 → 保留 2 行；宿主路径/数据库/权限/生命周期端口装配或公共兼容入口；生产实现已在领域库
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/aggregation_resync_tests.rs`
- 459 行 → crates/libs/gateway-app-routing/src/host_migration_14_tests/aggregation_resync_tests.rs
- 用例：`aggregation_t16_claude_record_edit_publishes_real_files_and_listener`、`aggregation_t16_unavailable_listener_and_detached_intent_preserve_native_config`、`aggregation_t16_default_and_member_order_use_same_transaction`、`aggregation_t16_project_write_grant_is_checked_before_record_or_file_changes`、`aggregation_t16_database_failure_keeps_durable_intent_and_revokes_partial_route`、`aggregation_t16_detached_catalog_edit_does_not_create_client_directory`、`aggregation_t16_legacy_catalog_save_keeps_gateway_and_provider_credentials_separate`

### `crates/vw-agent/src/gateway/api/handlers/app_providers_tests.rs`
- 223 行 → 保留 223 行；真实宿主测试装配：依赖宿主路由/全局会话/鉴权组合；后续可按独立领域端口继续拆分，当前保留原覆盖
- 用例：`cc_apps_provider_launch_isolation_api_write_read_auth_and_redaction`、`cc_apps_provider_launch_isolation_sdk_http_sqlite_projection_process_roundtrip`

### `crates/vw-agent/src/gateway/api/handlers/auth.rs`
- 3 行 → 保留 3 行；宿主路径/数据库/权限/生命周期端口装配或公共兼容入口；生产实现已在领域库
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/backup_inventory.rs`
- 4 行 → 保留 4 行；宿主路径/数据库/权限/生命周期端口装配或公共兼容入口；生产实现已在领域库
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/channel_qr.rs`
- 6 行 → 保留 6 行；宿主路径/数据库/权限/生命周期端口装配或公共兼容入口；生产实现已在领域库
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/channel_qr_tests.rs`
- 87 行 → 保留 87 行；真实宿主测试装配：依赖宿主路由/全局会话/鉴权组合；后续可按独立领域端口继续拆分，当前保留原覆盖
- 用例：`five_channel_qr_credentials_match_runtime_config`

### `crates/vw-agent/src/gateway/api/handlers/computer_control.rs`
- 2 行 → 保留 2 行；宿主路径/数据库/权限/生命周期端口装配或公共兼容入口；生产实现已在领域库
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/desktop.rs`
- 2 行 → 保留 2 行；宿主路径/数据库/权限/生命周期端口装配或公共兼容入口；生产实现已在领域库
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/desktop_skills.rs`
- 2 行 → 保留 2 行；宿主路径/数据库/权限/生命周期端口装配或公共兼容入口；生产实现已在领域库
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/device_sync_config.rs`
- 2 行 → 保留 2 行；宿主路径/数据库/权限/生命周期端口装配或公共兼容入口；生产实现已在领域库
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/device_sync_download.rs`
- 1 行 → 删除无实现转发/空文件；父模块使用真实领域模块别名，调用路径保持
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/device_sync_jobs.rs`
- 7 行 → 保留 7 行；宿主路径/数据库/权限/生命周期端口装配或公共兼容入口；生产实现已在领域库
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/device_sync_probe.rs`
- 2 行 → 保留 2 行；宿主路径/数据库/权限/生命周期端口装配或公共兼容入口；生产实现已在领域库
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/device_sync_projection.rs`
- 3 行 → 保留 3 行；宿主路径/数据库/权限/生命周期端口装配或公共兼容入口；生产实现已在领域库
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/device_sync_tasks.rs`
- 2 行 → 保留 2 行；宿主路径/数据库/权限/生命周期端口装配或公共兼容入口；生产实现已在领域库
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/dynamic_tools.rs`
- 50 行 → 保留 50 行；宿主路径/数据库/权限/生命周期端口装配或公共兼容入口；生产实现已在领域库
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/dynamic_tools_tests.rs`
- 448 行 → 保留 448 行；真实宿主测试装配：依赖宿主路由/全局会话/鉴权组合；后续可按独立领域端口继续拆分，当前保留原覆盖
- 用例：`dynamic_tools_gateway_cross_tenant_registration_is_denied`、`dynamic_tools_gateway_scope_and_host_updates_are_denied`、`dynamic_tools_gateway_replacement_is_atomic_and_policy_cannot_be_elevated`、`dynamic_tools_gateway_disconnect_cancels_pending_and_expires_binding`、`dynamic_tools_gateway_http_host_receives_real_call_and_responds_once`、`dynamic_tools_gateway_production_chat_emits_tool_events_and_keeps_runtime_tools`、`dynamic_tools_gateway_scope_validation_does_not_interrupt_active_history`

### `crates/vw-agent/src/gateway/api/handlers/external_sessions/mod.rs`
- 2 行 → 保留 2 行；宿主路径/数据库/权限/生命周期端口装配或公共兼容入口；生产实现已在领域库
- 用例：无

### `crates/vw-agent/src/gateway/api/handlers/file.rs`
- 39 行 → 保留 39 行；宿主路径/数据库/权限/生命周期端口装配或公共兼容入口；生产实现已在领域库
- 用例：无

## 行数与验证登记
来源基线 61 文件 2926 行；保留 1039 行，净减 1887 行（64.49%）。这是来源物理源码口径；父模块最小注册补丁另外计入 diff。迁出的真实生产逻辑为 0 行；删除的纯转发不作为生产迁移收益。
迁入的测试/夹具行数：
- `crates/libs/gateway-agent-apps/src/host_migration_14_tests/agent_apps_auth_remove_tests.rs`：335 行，4 条测试。
- `crates/libs/gateway-agent-apps/src/host_migration_14_tests/agent_apps_delete_tests.rs`：54 行，2 条测试。
- `crates/libs/gateway-agent-apps/src/host_migration_4_tests/agent_apps_diagnostics_endpoint_drafts_tests.rs`：148 行，1 条测试。
- `crates/libs/gateway-app-routing/src/host_migration_14_tests/agent_apps_ha_acceptance_tests.rs`：210 行，1 条测试。
- `crates/libs/gateway-app-routing/src/host_migration_14_tests/agent_apps_high_availability_tests.rs`：585 行，4 条测试。
- `crates/libs/gateway-agent-apps/src/host_migration_14_tests/agent_apps_rename_tests.rs`：84 行，2 条测试。
- `crates/libs/gateway-app-routing/src/host_migration_14_tests/aggregation_resync_tests.rs`：460 行，7 条测试。

净减不足70%时的原因与可迁项均已逐文件列在上表：主要为 dynamic_tools 的真实 SessionHost/配置恢复组合、app_providers 的 SDK/进程投影验收及 aggregation include 的宿主装配。不能删除这些断言凑比例；收口须继续审核这些具体端口。

未来命令（本次全部未运行）：
```sh
python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib host_migration_14_tests -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib host_migration_4_tests::agent_apps_diagnostics_endpoint_drafts_tests -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p gateway-app-routing --lib host_migration_14_tests -- --test-threads=1
```
预期 macOS 应用任务14非零数：8；诊断草稿：1；路由任务14：12。Postgres 用例需要现有环境。其它平台按 cfg 排除对应原生场景。
宿主保留测试使用上表每条完整模块/函数名逐一过滤；未来还须通过宿主 `--list` 确认迁出条目消失，不能把零测试当成功。include 测试仍属于宿主目标，尚未计入领域自注册收益。
静态核对：迁出模块取消原挂载；领域模块自注册；补充 gateway-app-routing 定向测试直接依赖；保留原断言及真实数据库/文件/HTTP/路由监听器。构建、行为、安全运行验收全部未运行。

## 深度恢复（收口后续请求）

读取 final.md 与 remaining-host.json：原任务残留 1039 行。此次继续迁移真实行为，并非仅移动存放路径。

- app_providers_tests.rs 原 223 行、两条完整用例 → gateway-app-providers/src/host_migration_14_tests.rs。两条原名称与全部原断言保留：`cc_apps_provider_launch_isolation_api_write_read_auth_and_redaction`、`cc_apps_provider_launch_isolation_sdk_http_sqlite_projection_process_roundtrip`。直接注册在所属 crate 的 lib.rs。原 handler_router 改为 crate::router，鉴权与 tenant 中间件来自 agent-gateway；真实 PairingGuard 来自 gateway-pairing。数据库、SDK HTTP、真实脚本进程、私有目录清理与非本机平台拒绝断言保持；无 mock。新增 gateway-pairing 仅为 dev 依赖，锁文件对应边补齐。
- 宿主 app_providers_tests.rs 仅留 `app_provider_host_route_requires_basic_auth`：使用真实 handler_router，验证真实组合入口基本鉴权、缺租户拒绝；不重复 provider 数据与投影断言。
- channel_qr_tests.rs 原 87 行、`five_channel_qr_credentials_match_runtime_config` → channel-runtime/src/host_migration_14_tests.rs，领域 lib.rs 自注册。真实 collect_configured_channels 使用显式 TenantScope（原宿主适配器隐式提供），Config/LarkReceiveMode 使用真实所有者。保留五通道排序断言及原二维码配置转换夹具，原 channel_qr.rs 挂载删除。
- 未迁生产算法：provider/通道实现此前已在领域库，本轮迁入完整领域测试和夹具，不计作新增生产迁移收益。宿主路由仍调用这些真实 crate router/collect 入口，无反向宿主依赖与 path/include 宿主源码。

动态工具剩余的具体边界：`dynamic_tools.rs::SessionDynamicToolsHost` 绑定 NativeSessionIo 的 scope 验证、宿主 HostTools 客户端/搜索工具构造与上下文、SessionHost 注册鉴权和事件流。`dynamic_tools_tests` 前五条通过真实宿主 SessionHost 验证租户、目录、binding token、替换原子性与 SSE/真实工具调用；第六条通过宿主 scripted 模型和 Gateway chat observer 检验模型请求工具覆盖及原生产事件；第七条检验宿主消息持久化与正在执行的工具历史。直接迁往 gateway-dynamic-tools 会缺少 NativeSessionIo 与 HostTools，复制它们会扩大范围到会话/工具所有者；保留这些宿主端口验收，不以 mock 代替。后续若其所有者提供领域原生适配器，可继续拆解；这不是整个文件无需审查的声明。

其它保留装配逐符号：aggregation_codex_settings::SnapshotHost/router 与 desktop::read_form/save_mapping 注入具体 snapshot 及数据库；members/omo/read::router 捕获真实 home 数据库；exit/publish/restart::router 注入唯一 provider operation registry；device_sync_jobs::router 注入真实 receive projection；backup_inventory::router 注入宿主权限；file::FileHost::bootstrap 调用真实 instance_bootstrap。纯 re-export 不算领域生产逻辑。原 include 宿主聚合套件仍保留旧挂载，此轮未宣称它们已迁移或改变运行归属。

未来（本轮未运行）：
```sh
python3 scripts/cargo_slot.py cargo test -p gateway-app-providers --lib host_migration_14_tests -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p channel-runtime --lib host_migration_14_tests -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib app_provider_host_route_requires_basic_auth -- --test-threads=1
```
预期分别 2、1、1 条非零测试；数据库/平台依赖同原测试。静态核对直接 import、父模块注册、原挂载删除与 diff 空白；未运行 Cargo、构建、测试及实机验收，不声称通过。

恢复后原61文件物理残留 743 行；相对2926行基线净减 2183 行（74.61%）。新增薄宿主用例计入该数；领域新增行数见 git diff。
