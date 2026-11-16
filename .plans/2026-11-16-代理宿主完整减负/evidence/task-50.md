# 任务50 工作树领域迁移证据

代码与注册已完成；编译/行为未验证。按计划禁止运行任何 Cargo、构建、测试或实机命令；仅源码/差异核对。

## 生产归属与接线

生产迁出：0 行、0 个符号。生命周期算法原已在 worktree-core，不重复计作本轮收益。实际生产方向：vw-agent::worktree wrappers → initialize_host → worktree_core::{create,remove,reset,...}；WorktreeHost 将 ui_store/status/ProcessTree 注入既有 Host。领域库不反向依赖 vw-agent，不通过 path/include 加载宿主源码。

领域 lib.rs 自注册 host_migration_50_tests，内部直接引用 crate::worktree::{native,creation_policy,...}，不增加公开测试 API。新增 agent-project/agent-database test-support dev 依赖以保留真实夹具。恢复子进程 --exact 已更新到新编译归属。

## 文件与行数（splitlines）

|原宿主文件|前|后|职责/继续可迁项|
|---|---:|---:|---|
|worktree_native_tests.rs|545|72|保留三条 run_start_scripts 用例，运行真实项目/extra 命令和 ProcessTree；project_info/unique_project_id/failing_shell_command 为其最小夹具。|
|worktree_ops_tests.rs|312|251|保留 remove/reset 所需真实当前租户及 fork 注册、外租户拒绝、主目录保护、真实删除/重置；in_project/GitWorktreeFixture/canonical_string/run/assert_missing_project_context 随调用保留。继续可迁：显式租户 remove_for_tenant 的纯领域断言可后续拆分，当前保留对公开宿主 wrapper 的验收。|
|host_adapter.rs|47|47|WorktreeHost::with_tenant_scope/current_tenant_scope/read_snapshot/spawn、ProcessGuard for ProcessTree、INITIALIZED/initialize_host、busy_sessions 均为真实 ui_store/status/进程边界；无生产迁出收益。|
|cleanup_flow_tests.rs|666|666|逐用例保留真实 WorktreeHost 的会话快照、HTTP 确认与状态路由、运行时/脱离进程/绑定租约及 HostCleanup 生命周期；不能以空快照 mock 冒充安全验收。isolated、refresh、git、Fixture::new/checkout/call、request、LeaseProbeProvider、legacy_turn 随实际调用保留。继续可迁：将纯 refresh/management 场景用显式快照端口拆分，保留路由断言；本轮未新增测试专用生产端口。|
|cleanup_tests.rs|431|208|两条保留用例分别覆盖真实 Busy/Retry→busy_sessions 映射及真实 SessionStore 数据库重复会话、pending、跨租户目录和损坏数据拒绝；scope、Fixture、git、session 随其调用保留。继续可迁：首条观察断言可进一步拆分到 session-info 输入测试，Busy/Retry 映射仍须宿主覆盖。|
|mod.rs|40|40|公共模块与类型重导出、宿主适配器及薄测试挂载，无领域算法。|
|environment_tests.rs|338|338|逐用例覆盖实际 ProcessTree spawn/析构、取消和超时杀后代、ui_store 租户、会话创建拒绝及事件顺序；project/git/fixture/isolated_tenant 为实际进程与会话接线夹具。继续可迁：pending 和状态断言可进一步抽成不运行命令的领域测试；其余须保留真实终止守卫验收。|
|tests.rs|13|13|top_level_reexports_worktree_types 验证宿主公共路径兼容，保留 13 行薄契约。|
|recovery_restore_tests.rs|326|0|整组恢复测试迁入领域，原文件和挂载删除；fixture/git/deleted 随用例迁入。|
|worktree.rs|72|68|create/create_fork_checkout/remove_fork_checkout/retry_environment/remove/reset 在调用 worktree_core 前安装真实 Host；WASM initialize_host 和公共 re-export 保持兼容。删除 recovery_restore_tests 旧挂载，其余挂载仅剩实际宿主测试。|
|management_e2e_tests.rs|262|262|两条是实际 Axum 路由、会话持久化、pin、确认删除和子进程重启还原的完整端到端链；isolated/git/Fixture::new/checkout/call 保留。继续可迁：领域原子行为已由迁入恢复/策略测试覆盖，整链不能替换成领域 mock。|
|creation_policy_tests.rs|173|88|仅保留 normal_and_fork_share_tenant_settings，覆盖 ui_store 当前租户与普通/fork 创建宿主装配；git、fixture、commit 为此构造真实仓库。继续可迁：无独立纯策略断言残留。|

宿主 3225 → 2053 行，净减 1172 行（36.3%），**不足70%，交收口按逐文件原因验收，不宣称减量达标**。宿主生产/文档/装配原159行，现155行（仅删除4行测试挂载）；其余为测试与夹具。领域新增测试/夹具/注册 1269 行，新增生产0行。继续可迁项见上表。

## 全用例映射

|原完整测试名|新位置或保留原因|
|---|---|
|`worktree::worktree::recovery_restore_tests::worktree_recovery_restore_round_trips_staged_and_unstaged_changes`|host_migration_50_tests::recovery_restore_tests::worktree_recovery_restore_round_trips_staged_and_unstaged_changes|
|`worktree::worktree::recovery_restore_tests::worktree_recovery_restore_rejects_other_tenant`|host_migration_50_tests::recovery_restore_tests::worktree_recovery_restore_rejects_other_tenant|
|`worktree::worktree::recovery_restore_tests::worktree_recovery_restore_handles_path_conflict`|host_migration_50_tests::recovery_restore_tests::worktree_recovery_restore_handles_path_conflict|
|`worktree::worktree::recovery_restore_tests::worktree_recovery_restore_failure_can_retry_and_preserves_snapshot`|host_migration_50_tests::recovery_restore_tests::worktree_recovery_restore_failure_can_retry_and_preserves_snapshot|
|`worktree::worktree::recovery_restore_tests::worktree_recovery_restore_survives_process_restart`|host_migration_50_tests::recovery_restore_tests::worktree_recovery_restore_survives_process_restart|
|`worktree::worktree::recovery_restore_tests::worktree_recovery_restore_rejects_foreign_repository_and_busy_operation`|host_migration_50_tests::recovery_restore_tests::worktree_recovery_restore_rejects_foreign_repository_and_busy_operation|
|`worktree::worktree::recovery_restore_tests::worktree_recovery_restore_rejects_unsafe_tree_before_writing`|host_migration_50_tests::recovery_restore_tests::worktree_recovery_restore_rejects_unsafe_tree_before_writing|
|`worktree::worktree::recovery_restore_tests::worktree_recovery_restore_registration_failure_rolls_back_only_new_resources`|host_migration_50_tests::recovery_restore_tests::worktree_recovery_restore_registration_failure_rolls_back_only_new_resources|
|`worktree::worktree::recovery_restore_tests::worktree_recovery_restore_does_not_interpret_branch_names_as_options`|host_migration_50_tests::recovery_restore_tests::worktree_recovery_restore_does_not_interpret_branch_names_as_options|
|`worktree::worktree::creation_policy_tests::worktree_creation_policy_uses_configured_root`|host_migration_50_tests::creation_policy_tests::worktree_creation_policy_uses_configured_root|
|`worktree::worktree::creation_policy_tests::worktree_creation_policy_fetches_only_when_enabled`|host_migration_50_tests::creation_policy_tests::worktree_creation_policy_fetches_only_when_enabled|
|`worktree::worktree::creation_policy_tests::worktree_creation_policy_fetch_failure_does_not_create_checkout`|host_migration_50_tests::creation_policy_tests::worktree_creation_policy_fetch_failure_does_not_create_checkout|
|`worktree::worktree::creation_policy_tests::worktree_creation_policy_rejects_symlinks`|host_migration_50_tests::creation_policy_tests::worktree_creation_policy_rejects_symlinks|
|`worktree::worktree::creation_policy_tests::worktree_creation_policy_normal_and_fork_share_tenant_settings`|原名保留；仅保留 normal_and_fork_share_tenant_settings，覆盖 ui_store 当前租户与普通/fork 创建宿主装配；git、fixture、commit 为此构造真实仓库。继续可迁：无独立纯策略断言残留。|
|`worktree::worktree::worktree_native_tests::parse_worktree_list_preserves_paths_and_branches`|host_migration_50_tests::worktree_native_tests::parse_worktree_list_preserves_paths_and_branches|
|`worktree::worktree::worktree_native_tests::parse_worktree_list_ignores_orphan_branches_and_unknown_records`|host_migration_50_tests::worktree_native_tests::parse_worktree_list_ignores_orphan_branches_and_unknown_records|
|`worktree::worktree::worktree_native_tests::cmd_result_prefers_stderr_then_stdout_for_error_text`|host_migration_50_tests::worktree_native_tests::cmd_result_prefers_stderr_then_stdout_for_error_text|
|`worktree::worktree::worktree_native_tests::cmd_result_ignores_whitespace_only_streams_for_error_text`|host_migration_50_tests::worktree_native_tests::cmd_result_ignores_whitespace_only_streams_for_error_text|
|`worktree::worktree::worktree_native_tests::path_exists_reports_temp_directory`|host_migration_50_tests::worktree_native_tests::path_exists_reports_temp_directory|
|`worktree::worktree::worktree_native_tests::path_exists_reports_regular_file`|host_migration_50_tests::worktree_native_tests::path_exists_reports_regular_file|
|`worktree::worktree::worktree_native_tests::canonical_absolutizes_missing_relative_paths`|host_migration_50_tests::worktree_native_tests::canonical_absolutizes_missing_relative_paths|
|`worktree::worktree::worktree_native_tests::canonical_resolves_existing_paths`|host_migration_50_tests::worktree_native_tests::canonical_resolves_existing_paths|
|`worktree::worktree::worktree_native_tests::find_entry_matches_canonical_target_path`|host_migration_50_tests::worktree_native_tests::find_entry_matches_canonical_target_path|
|`worktree::worktree::worktree_native_tests::find_entry_returns_none_for_unlisted_path`|host_migration_50_tests::worktree_native_tests::find_entry_returns_none_for_unlisted_path|
|`worktree::worktree::worktree_native_tests::run_git_and_resolve_reset_target_work_in_temp_repo`|host_migration_50_tests::worktree_native_tests::run_git_and_resolve_reset_target_work_in_temp_repo|
|`worktree::worktree::worktree_native_tests::candidate_uses_requested_base_when_directory_and_branch_are_available`|host_migration_50_tests::worktree_native_tests::candidate_uses_requested_base_when_directory_and_branch_are_available|
|`worktree::worktree::worktree_native_tests::candidate_skips_existing_directory_for_requested_base`|host_migration_50_tests::worktree_native_tests::candidate_skips_existing_directory_for_requested_base|
|`worktree::worktree::worktree_native_tests::candidate_skips_existing_branch_for_requested_base`|host_migration_50_tests::worktree_native_tests::candidate_skips_existing_branch_for_requested_base|
|`worktree::worktree::worktree_native_tests::candidate_generates_random_name_without_base`|host_migration_50_tests::worktree_native_tests::candidate_generates_random_name_without_base|
|`worktree::worktree::worktree_native_tests::run_start_scripts_runs_extra_when_project_lookup_is_unavailable`|原名保留；保留三条 run_start_scripts 用例，运行真实项目/extra 命令和 ProcessTree；project_info/unique_project_id/failing_shell_command 为其最小夹具。|
|`worktree::worktree::worktree_native_tests::run_start_scripts_runs_project_start_before_extra_script`|原名保留；保留三条 run_start_scripts 用例，运行真实项目/extra 命令和 ProcessTree；project_info/unique_project_id/failing_shell_command 为其最小夹具。|
|`worktree::worktree::worktree_native_tests::run_start_scripts_stops_when_project_start_fails`|原名保留；保留三条 run_start_scripts 用例，运行真实项目/extra 命令和 ProcessTree；project_info/unique_project_id/failing_shell_command 为其最小夹具。|
|`worktree::worktree::worktree_native_tests::resolve_reset_target_parses_valid_slash_ref_as_remote_branch`|host_migration_50_tests::worktree_native_tests::resolve_reset_target_parses_valid_slash_ref_as_remote_branch|
|`worktree::worktree::worktree_native_tests::resolve_reset_target_uses_main_for_blank_base_ref`|host_migration_50_tests::worktree_native_tests::resolve_reset_target_uses_main_for_blank_base_ref|
|`worktree::worktree::worktree_native_tests::resolve_reset_target_falls_back_to_master_without_remotes`|host_migration_50_tests::worktree_native_tests::resolve_reset_target_falls_back_to_master_without_remotes|
|`worktree::worktree::worktree_native_tests::resolve_reset_target_prefers_origin_remote_head`|host_migration_50_tests::worktree_native_tests::resolve_reset_target_prefers_origin_remote_head|
|`worktree::worktree::worktree_native_tests::resolve_reset_target_uses_single_non_origin_remote_head`|host_migration_50_tests::worktree_native_tests::resolve_reset_target_uses_single_non_origin_remote_head|
|`worktree::worktree::worktree_native_tests::resolve_reset_target_prefers_upstream_among_multiple_non_origin_remotes`|host_migration_50_tests::worktree_native_tests::resolve_reset_target_prefers_upstream_among_multiple_non_origin_remotes|
|`worktree::worktree::worktree_native_tests::resolve_reset_target_uses_local_main_when_multiple_remotes_have_no_preferred_name`|host_migration_50_tests::worktree_native_tests::resolve_reset_target_uses_local_main_when_multiple_remotes_have_no_preferred_name|
|`worktree::worktree::worktree_native_tests::resolve_reset_target_falls_back_to_local_main_when_remote_head_is_missing`|host_migration_50_tests::worktree_native_tests::resolve_reset_target_falls_back_to_local_main_when_remote_head_is_missing|
|`worktree::worktree::worktree_native_tests::resolve_reset_target_rejects_repository_without_default_branch`|host_migration_50_tests::worktree_native_tests::resolve_reset_target_rejects_repository_without_default_branch|
|`worktree::worktree::worktree_native_tests::resolve_reset_target_reports_non_repository_error`|host_migration_50_tests::worktree_native_tests::resolve_reset_target_reports_non_repository_error|
|`worktree::worktree::worktree_native_tests::info_is_cloneable_for_native_callers`|host_migration_50_tests::worktree_native_tests::info_is_cloneable_for_native_callers|
|`worktree::worktree::worktree_native_tests::worktree_root_uses_shared_task_worktrees_directory`|host_migration_50_tests::worktree_native_tests::worktree_root_uses_shared_task_worktrees_directory|
|`worktree::worktree::worktree_native_tests::worktree_root_rejects_blank_project_worktree`|host_migration_50_tests::worktree_native_tests::worktree_root_rejects_blank_project_worktree|
|`worktree::worktree::worktree_ops_tests::operation_inputs_deserialize_aliases_and_defaults`|host_migration_50_tests::worktree_ops_tests::operation_inputs_deserialize_aliases_and_defaults|
|`worktree::worktree::worktree_ops_tests::operation_inputs_serialize_camel_case_fields`|host_migration_50_tests::worktree_ops_tests::operation_inputs_serialize_camel_case_fields|
|`worktree::worktree::worktree_ops_tests::create_without_project_context_returns_missing_project_before_creating`|host_migration_50_tests::worktree_ops_tests::create_without_project_context_returns_missing_project_before_creating|
|`worktree::worktree::worktree_ops_tests::remove_without_project_context_preserves_requested_directory`|原名保留；保留 remove/reset 所需真实当前租户及 fork 注册、外租户拒绝、主目录保护、真实删除/重置；in_project/GitWorktreeFixture/canonical_string/run/assert_missing_project_context 随调用保留。继续可迁：显式租户 remove_for_tenant 的纯领域断言可后续拆分，当前保留对公开宿主 wrapper 的验收。|
|`worktree::worktree::worktree_ops_tests::list_directories_without_project_context_returns_missing_project`|host_migration_50_tests::worktree_ops_tests::list_directories_without_project_context_returns_missing_project|
|`worktree::worktree::worktree_ops_tests::reset_without_project_context_preserves_requested_directory`|原名保留；保留 remove/reset 所需真实当前租户及 fork 注册、外租户拒绝、主目录保护、真实删除/重置；in_project/GitWorktreeFixture/canonical_string/run/assert_missing_project_context 随调用保留。继续可迁：显式租户 remove_for_tenant 的纯领域断言可后续拆分，当前保留对公开宿主 wrapper 的验收。|
|`worktree::worktree::worktree_ops_tests::list_directories_returns_primary_and_linked_worktrees`|原名保留；保留 remove/reset 所需真实当前租户及 fork 注册、外租户拒绝、主目录保护、真实删除/重置；in_project/GitWorktreeFixture/canonical_string/run/assert_missing_project_context 随调用保留。继续可迁：显式租户 remove_for_tenant 的纯领域断言可后续拆分，当前保留对公开宿主 wrapper 的验收。|
|`worktree::worktree::worktree_ops_tests::reset_rejects_primary_workspace_without_modifying_it`|原名保留；保留 remove/reset 所需真实当前租户及 fork 注册、外租户拒绝、主目录保护、真实删除/重置；in_project/GitWorktreeFixture/canonical_string/run/assert_missing_project_context 随调用保留。继续可迁：显式租户 remove_for_tenant 的纯领域断言可后续拆分，当前保留对公开宿主 wrapper 的验收。|
|`worktree::worktree::worktree_ops_tests::reset_restores_tracked_files_and_removes_untracked_files`|原名保留；保留 remove/reset 所需真实当前租户及 fork 注册、外租户拒绝、主目录保护、真实删除/重置；in_project/GitWorktreeFixture/canonical_string/run/assert_missing_project_context 随调用保留。继续可迁：显式租户 remove_for_tenant 的纯领域断言可后续拆分，当前保留对公开宿主 wrapper 的验收。|
|`worktree::worktree::worktree_ops_tests::remove_registered_worktree_deletes_directory_and_branch`|原名保留；保留 remove/reset 所需真实当前租户及 fork 注册、外租户拒绝、主目录保护、真实删除/重置；in_project/GitWorktreeFixture/canonical_string/run/assert_missing_project_context 随调用保留。继续可迁：显式租户 remove_for_tenant 的纯领域断言可后续拆分，当前保留对公开宿主 wrapper 的验收。|
|`worktree::worktree::worktree_ops_tests::worktree_registry_remove_rejects_unregistered_directory_and_primary`|原名保留；保留 remove/reset 所需真实当前租户及 fork 注册、外租户拒绝、主目录保护、真实删除/重置；in_project/GitWorktreeFixture/canonical_string/run/assert_missing_project_context 随调用保留。继续可迁：显式租户 remove_for_tenant 的纯领域断言可后续拆分，当前保留对公开宿主 wrapper 的验收。|
|`worktree::worktree::worktree_ops_tests::worktree_registry_fork_creation_registers_and_legacy_remove_checks_owner`|原名保留；保留 remove/reset 所需真实当前租户及 fork 注册、外租户拒绝、主目录保护、真实删除/重置；in_project/GitWorktreeFixture/canonical_string/run/assert_missing_project_context 随调用保留。继续可迁：显式租户 remove_for_tenant 的纯领域断言可后续拆分，当前保留对公开宿主 wrapper 的验收。|
|`worktree::worktree::cleanup_tests::worktree_cleanup_keeps_pinned_busy_and_pool_worktrees`|host_migration_50_tests::cleanup_tests::worktree_cleanup_keeps_pinned_busy_and_pool_worktrees|
|`worktree::worktree::cleanup_tests::worktree_cleanup_orders_by_last_use`|host_migration_50_tests::cleanup_tests::worktree_cleanup_orders_by_last_use|
|`worktree::worktree::cleanup_tests::worktree_cleanup_fails_closed_when_protection_is_unknown`|host_migration_50_tests::cleanup_tests::worktree_cleanup_fails_closed_when_protection_is_unknown|
|`worktree::worktree::cleanup_tests::worktree_cleanup_all_associated_sessions_protect_and_latest_title_wins`|原名保留；两条保留用例分别覆盖真实 Busy/Retry→busy_sessions 映射及真实 SessionStore 数据库重复会话、pending、跨租户目录和损坏数据拒绝；scope、Fixture、git、session 随其调用保留。继续可迁：首条观察断言可进一步拆分到 session-info 输入测试，Busy/Retry 映射仍须宿主覆盖。|
|`worktree::worktree::cleanup_tests::worktree_cleanup_initialization_pool_external_and_unknown_are_protected`|host_migration_50_tests::cleanup_tests::worktree_cleanup_initialization_pool_external_and_unknown_are_protected|
|`worktree::worktree::cleanup_tests::worktree_cleanup_shared_repository_fails_closed_across_tenants`|host_migration_50_tests::cleanup_tests::worktree_cleanup_shared_repository_fails_closed_across_tenants|
|`worktree::worktree::cleanup_tests::worktree_cleanup_session_store_preserves_pending_duplicates_and_errors`|原名保留；两条保留用例分别覆盖真实 Busy/Retry→busy_sessions 映射及真实 SessionStore 数据库重复会话、pending、跨租户目录和损坏数据拒绝；scope、Fixture、git、session 随其调用保留。继续可迁：首条观察断言可进一步拆分到 session-info 输入测试，Busy/Retry 映射仍须宿主覆盖。|
|`worktree::worktree::cleanup_tests::worktree_cleanup_record_use_is_monotonic_and_identity_checked`|host_migration_50_tests::cleanup_tests::worktree_cleanup_record_use_is_monotonic_and_identity_checked|
|`worktree::worktree::environment_tests::worktree_environment_runs_in_checkout_before_ready`|原名保留；逐用例覆盖实际 ProcessTree spawn/析构、取消和超时杀后代、ui_store 租户、会话创建拒绝及事件顺序；project/git/fixture/isolated_tenant 为实际进程与会话接线夹具。继续可迁：pending 和状态断言可进一步抽成不运行命令的领域测试；其余须保留真实终止守卫验收。|
|`worktree::worktree::environment_tests::worktree_environment_failure_prevents_ready`|原名保留；逐用例覆盖实际 ProcessTree spawn/析构、取消和超时杀后代、ui_store 租户、会话创建拒绝及事件顺序；project/git/fixture/isolated_tenant 为实际进程与会话接线夹具。继续可迁：pending 和状态断言可进一步抽成不运行命令的领域测试；其余须保留真实终止守卫验收。|
|`worktree::worktree::environment_tests::worktree_environment_retry_does_not_rerun_successful_setup`|原名保留；逐用例覆盖实际 ProcessTree spawn/析构、取消和超时杀后代、ui_store 租户、会话创建拒绝及事件顺序；project/git/fixture/isolated_tenant 为实际进程与会话接线夹具。继续可迁：pending 和状态断言可进一步抽成不运行命令的领域测试；其余须保留真实终止守卫验收。|
|`worktree::worktree::environment_tests::worktree_environment_timeout_kills_descendants`|原名保留；逐用例覆盖实际 ProcessTree spawn/析构、取消和超时杀后代、ui_store 租户、会话创建拒绝及事件顺序；project/git/fixture/isolated_tenant 为实际进程与会话接线夹具。继续可迁：pending 和状态断言可进一步抽成不运行命令的领域测试；其余须保留真实终止守卫验收。|
|`worktree::worktree::environment_tests::worktree_environment_cancellation_records_failure_and_excludes_duplicates`|原名保留；逐用例覆盖实际 ProcessTree spawn/析构、取消和超时杀后代、ui_store 租户、会话创建拒绝及事件顺序；project/git/fixture/isolated_tenant 为实际进程与会话接线夹具。继续可迁：pending 和状态断言可进一步抽成不运行命令的领域测试；其余须保留真实终止守卫验收。|
|`worktree::worktree::environment_tests::worktree_environment_extra_failure_is_reported`|原名保留；逐用例覆盖实际 ProcessTree spawn/析构、取消和超时杀后代、ui_store 租户、会话创建拒绝及事件顺序；project/git/fixture/isolated_tenant 为实际进程与会话接线夹具。继续可迁：pending 和状态断言可进一步抽成不运行命令的领域测试；其余须保留真实终止守卫验收。|
|`worktree::worktree::environment_tests::worktree_environment_creation_and_fork_publish_ready_after_setup`|原名保留；逐用例覆盖实际 ProcessTree spawn/析构、取消和超时杀后代、ui_store 租户、会话创建拒绝及事件顺序；project/git/fixture/isolated_tenant 为实际进程与会话接线夹具。继续可迁：pending 和状态断言可进一步抽成不运行命令的领域测试；其余须保留真实终止守卫验收。|
|`worktree::worktree::environment_tests::worktree_environment_creation_failure_can_retry_without_resetting_checkout`|原名保留；逐用例覆盖实际 ProcessTree spawn/析构、取消和超时杀后代、ui_store 租户、会话创建拒绝及事件顺序；project/git/fixture/isolated_tenant 为实际进程与会话接线夹具。继续可迁：pending 和状态断言可进一步抽成不运行命令的领域测试；其余须保留真实终止守卫验收。|
|`worktree::worktree::environment_tests::worktree_environment_pending_cannot_be_retried_or_opened`|原名保留；逐用例覆盖实际 ProcessTree spawn/析构、取消和超时杀后代、ui_store 租户、会话创建拒绝及事件顺序；project/git/fixture/isolated_tenant 为实际进程与会话接线夹具。继续可迁：pending 和状态断言可进一步抽成不运行命令的领域测试；其余须保留真实终止守卫验收。|
|`worktree::worktree::cleanup_flow_tests::worktree_cleanup_flow_waits_for_confirmation`|原名保留；逐用例保留真实 WorktreeHost 的会话快照、HTTP 确认与状态路由、运行时/脱离进程/绑定租约及 HostCleanup 生命周期；不能以空快照 mock 冒充安全验收。isolated、refresh、git、Fixture::new/checkout/call、request、LeaseProbeProvider、legacy_turn 随实际调用保留。继续可迁：将纯 refresh/management 场景用显式快照端口拆分，保留路由断言；本轮未新增测试专用生产端口。|
|`worktree::worktree::cleanup_flow_tests::worktree_cleanup_flow_rechecks_newly_busy_checkout`|原名保留；逐用例保留真实 WorktreeHost 的会话快照、HTTP 确认与状态路由、运行时/脱离进程/绑定租约及 HostCleanup 生命周期；不能以空快照 mock 冒充安全验收。isolated、refresh、git、Fixture::new/checkout/call、request、LeaseProbeProvider、legacy_turn 随实际调用保留。继续可迁：将纯 refresh/management 场景用显式快照端口拆分，保留路由断言；本轮未新增测试专用生产端口。|
|`worktree::worktree::cleanup_flow_tests::worktree_cleanup_flow_restore_is_visible`|原名保留；逐用例保留真实 WorktreeHost 的会话快照、HTTP 确认与状态路由、运行时/脱离进程/绑定租约及 HostCleanup 生命周期；不能以空快照 mock 冒充安全验收。isolated、refresh、git、Fixture::new/checkout/call、request、LeaseProbeProvider、legacy_turn 随实际调用保留。继续可迁：将纯 refresh/management 场景用显式快照端口拆分，保留路由断言；本轮未新增测试专用生产端口。|
|`worktree::worktree::cleanup_flow_tests::worktree_cleanup_flow_automatic_partial_failure_is_recoverable`|原名保留；逐用例保留真实 WorktreeHost 的会话快照、HTTP 确认与状态路由、运行时/脱离进程/绑定租约及 HostCleanup 生命周期；不能以空快照 mock 冒充安全验收。isolated、refresh、git、Fixture::new/checkout/call、request、LeaseProbeProvider、legacy_turn 随实际调用保留。继续可迁：将纯 refresh/management 场景用显式快照端口拆分，保留路由断言；本轮未新增测试专用生产端口。|
|`worktree::worktree::cleanup_flow_tests::worktree_cleanup_flow_snapshot_failure_keeps_checkout_and_can_retry`|原名保留；逐用例保留真实 WorktreeHost 的会话快照、HTTP 确认与状态路由、运行时/脱离进程/绑定租约及 HostCleanup 生命周期；不能以空快照 mock 冒充安全验收。isolated、refresh、git、Fixture::new/checkout/call、request、LeaseProbeProvider、legacy_turn 随实际调用保留。继续可迁：将纯 refresh/management 场景用显式快照端口拆分，保留路由断言；本轮未新增测试专用生产端口。|
|`worktree::worktree::cleanup_flow_tests::worktree_cleanup_flow_rejects_settings_changes_and_stale_candidates`|原名保留；逐用例保留真实 WorktreeHost 的会话快照、HTTP 确认与状态路由、运行时/脱离进程/绑定租约及 HostCleanup 生命周期；不能以空快照 mock 冒充安全验收。isolated、refresh、git、Fixture::new/checkout/call、request、LeaseProbeProvider、legacy_turn 随实际调用保留。继续可迁：将纯 refresh/management 场景用显式快照端口拆分，保留路由断言；本轮未新增测试专用生产端口。|
|`worktree::worktree::cleanup_flow_tests::worktree_cleanup_flow_runtime_and_detached_shell_hold_leases`|原名保留；逐用例保留真实 WorktreeHost 的会话快照、HTTP 确认与状态路由、运行时/脱离进程/绑定租约及 HostCleanup 生命周期；不能以空快照 mock 冒充安全验收。isolated、refresh、git、Fixture::new/checkout/call、request、LeaseProbeProvider、legacy_turn 随实际调用保留。继续可迁：将纯 refresh/management 场景用显式快照端口拆分，保留路由断言；本轮未新增测试专用生产端口。|
|`worktree::worktree::cleanup_flow_tests::worktree_cleanup_flow_session_creation_cannot_cross_removal`|原名保留；逐用例保留真实 WorktreeHost 的会话快照、HTTP 确认与状态路由、运行时/脱离进程/绑定租约及 HostCleanup 生命周期；不能以空快照 mock 冒充安全验收。isolated、refresh、git、Fixture::new/checkout/call、request、LeaseProbeProvider、legacy_turn 随实际调用保留。继续可迁：将纯 refresh/management 场景用显式快照端口拆分，保留路由断言；本轮未新增测试专用生产端口。|
|`worktree::worktree::cleanup_flow_tests::worktree_cleanup_flow_host_refreshes_deduplicates_and_stops`|原名保留；逐用例保留真实 WorktreeHost 的会话快照、HTTP 确认与状态路由、运行时/脱离进程/绑定租约及 HostCleanup 生命周期；不能以空快照 mock 冒充安全验收。isolated、refresh、git、Fixture::new/checkout/call、request、LeaseProbeProvider、legacy_turn 随实际调用保留。继续可迁：将纯 refresh/management 场景用显式快照端口拆分，保留路由断言；本轮未新增测试专用生产端口。|
|`worktree::worktree::cleanup_flow_tests::worktree_cleanup_flow_reconciles_interrupted_execution`|原名保留；逐用例保留真实 WorktreeHost 的会话快照、HTTP 确认与状态路由、运行时/脱离进程/绑定租约及 HostCleanup 生命周期；不能以空快照 mock 冒充安全验收。isolated、refresh、git、Fixture::new/checkout/call、request、LeaseProbeProvider、legacy_turn 随实际调用保留。继续可迁：将纯 refresh/management 场景用显式快照端口拆分，保留路由断言；本轮未新增测试专用生产端口。|
|`worktree::worktree::cleanup_flow_tests::worktree_cleanup_flow_cancelled_host_does_not_start_deletion`|原名保留；逐用例保留真实 WorktreeHost 的会话快照、HTTP 确认与状态路由、运行时/脱离进程/绑定租约及 HostCleanup 生命周期；不能以空快照 mock 冒充安全验收。isolated、refresh、git、Fixture::new/checkout/call、request、LeaseProbeProvider、legacy_turn 随实际调用保留。继续可迁：将纯 refresh/management 场景用显式快照端口拆分，保留路由断言；本轮未新增测试专用生产端口。|
|`worktree::worktree::cleanup_flow_tests::worktree_cleanup_flow_legacy_execution_cannot_cross_removal`|原名保留；逐用例保留真实 WorktreeHost 的会话快照、HTTP 确认与状态路由、运行时/脱离进程/绑定租约及 HostCleanup 生命周期；不能以空快照 mock 冒充安全验收。isolated、refresh、git、Fixture::new/checkout/call、request、LeaseProbeProvider、legacy_turn 随实际调用保留。继续可迁：将纯 refresh/management 场景用显式快照端口拆分，保留路由断言；本轮未新增测试专用生产端口。|
|`worktree::worktree::cleanup_flow_tests::worktree_cleanup_flow_binding_retains_lease_until_cleared`|原名保留；逐用例保留真实 WorktreeHost 的会话快照、HTTP 确认与状态路由、运行时/脱离进程/绑定租约及 HostCleanup 生命周期；不能以空快照 mock 冒充安全验收。isolated、refresh、git、Fixture::new/checkout/call、request、LeaseProbeProvider、legacy_turn 随实际调用保留。继续可迁：将纯 refresh/management 场景用显式快照端口拆分，保留路由断言；本轮未新增测试专用生产端口。|
|`worktree::worktree::cleanup_flow_tests::worktree_cleanup_flow_binding_rejects_removal_without_changing_context`|原名保留；逐用例保留真实 WorktreeHost 的会话快照、HTTP 确认与状态路由、运行时/脱离进程/绑定租约及 HostCleanup 生命周期；不能以空快照 mock 冒充安全验收。isolated、refresh、git、Fixture::new/checkout/call、request、LeaseProbeProvider、legacy_turn 随实际调用保留。继续可迁：将纯 refresh/management 场景用显式快照端口拆分，保留路由断言；本轮未新增测试专用生产端口。|
|`worktree::worktree::cleanup_flow_tests::worktree_cleanup_flow_status_is_queryable_during_removal`|原名保留；逐用例保留真实 WorktreeHost 的会话快照、HTTP 确认与状态路由、运行时/脱离进程/绑定租约及 HostCleanup 生命周期；不能以空快照 mock 冒充安全验收。isolated、refresh、git、Fixture::new/checkout/call、request、LeaseProbeProvider、legacy_turn 随实际调用保留。继续可迁：将纯 refresh/management 场景用显式快照端口拆分，保留路由断言；本轮未新增测试专用生产端口。|
|`worktree::worktree::management_e2e_tests::worktree_management_e2e_create_chat_delete_restore`|原名保留；两条是实际 Axum 路由、会话持久化、pin、确认删除和子进程重启还原的完整端到端链；isolated/git/Fixture::new/checkout/call 保留。继续可迁：领域原子行为已由迁入恢复/策略测试覆盖，整链不能替换成领域 mock。|
|`worktree::worktree::management_e2e_tests::worktree_management_e2e_restart_restore`|原名保留；两条是实际 Axum 路由、会话持久化、pin、确认删除和子进程重启还原的完整端到端链；isolated/git/Fixture::new/checkout/call 保留。继续可迁：领域原子行为已由迁入恢复/策略测试覆盖，整链不能替换成领域 mock。|
|`worktree::tests::top_level_reexports_worktree_types`|原名保留；top_level_reexports_worktree_types 验证宿主公共路径兼容，保留 13 行薄契约。|

## 未来验证

静态逐函数比对：迁移前后所有 92 条测试函数断言一致；只删除领域测试 initialize_host 调用和更新恢复子进程精确路径。领域迁入 51 条，宿主保留 41 条（其中 restart_restore 由父测试显式 --ignored 调用）。未运行不得据此宣称通过。

`python3 scripts/cargo_slot.py cargo test -p worktree-core --lib host_migration_50_tests -- --test-threads=1`：Unix预期 51 条非零；非Unix缺少 symlink 策略用例。

`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib worktree:: -- --list`：核对上表所有领域旧名消失、宿主原名存在；随后逐个以表中宿主完整名加 --exact 定向验证（ignored restart_restore 使用父用例提供的环境运行），不可用零测试充当成功。

任务1证据未列出额外工作树跨目录挂载；本组旧挂载已按实际剩余宿主用例核对。未获取编译、行为、耗时或物理进程验证证据。
