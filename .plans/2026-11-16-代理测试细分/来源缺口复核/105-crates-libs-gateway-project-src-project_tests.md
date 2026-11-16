# 来源复核105：gateway-project/src/project_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/libs/gateway-project/src/project_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/libs/gateway-project/src/project_tests.rs`。
- 快照 SHA256：`4d30df4cf8054a4f64d8333e0b6924caf95e6f0a7b4f299c71b35f9b75665165`。源码变化后重新阅读。
- 快照词法属性 48 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/gateway/api/handlers/project.rs:22 | crates/libs/gateway-project/src/project_tests.rs | mod project_tests | cfg(not(target_arch="wasm32")); cfg(test) | gateway-project（物理所有者；生产声明见该库 src/lib.rs） | 未匹配→133 | 未发现 |

## 身份线索

- `router_builds_with_app_state`
- `project_routes_require_explicit_tenant_scope`
- `normalize_path_removes_current_directory_prefix`
- `normalize_path_handles_empty_root_and_repeated_edge_separators`
- `worktree_id_round_trips_directory`
- `directory_from_worktree_id_rejects_invalid_values`
- `timestamp_ms_clamps_to_i64_range`
- `project_context_directory_prefers_worktree_then_sandbox`
- `project_scope_rejects_blank_project_id`
- `project_owns_directory_normalizes_worktree_and_sandbox_paths`
- `map_project_uses_sandbox_directory_and_path_name_fallback`
- `map_project_uses_explicit_non_blank_name`
- `map_project_ignores_blank_explicit_name`
- `map_project_falls_back_to_directory_when_no_name_component_exists`
- `map_worktree_derives_name_id_and_timestamps`
- `current_branch_and_change_records_are_empty_for_non_repo`
- `current_branch_reads_committed_repository_branch`
- `collect_project_change_records_empty_for_unborn_git_repo`
- `collect_project_change_records_groups_modified_files`
- `collect_project_change_records_includes_nested_untracked_files`
- `map_project_reports_git_branch_and_dirty_state`
- `gateway_git_metadata_works_without_git_executable`
- `map_worktree_uses_default_name_for_blank_directory`
- `load_project_filters_missing_sandboxes_and_maps_missing_project`
- `project_get_and_update_round_trip_storage`
- `project_get_missing_project_returns_not_found`
- `project_routes_do_not_cross_tenant_for_same_project_id`
- `project_update_missing_project_returns_bad_request`
- `project_list_filters_sorts_limits_and_cursors`
- `project_list_handles_missing_cursor_blank_query_and_status_miss`
- `project_list_uses_name_directory_and_id_tie_breakers`
- `project_list_searches_directory_case_insensitively`
- `project_change_records_requires_directory`
- `project_change_records_rejects_blank_and_unowned_directory`
- `project_resolve_and_change_records_handle_plain_directory`
- `project_resolve_without_create_does_not_persist_missing_directory`
- `project_resolve_without_create_returns_registered_project`
- `project_change_records_uses_stored_sandbox_match`
- `project_worktrees_return_empty_for_non_git_project`
- `project_worktrees_list_git_project_primary_worktree`
- `project_store_rejects_project_without_directory`
- `project_worktree_create_rejects_non_git_project`
- `worktree_get_rejects_invalid_id_before_project_lookup`
- `worktree_get_maps_stored_project_directory`
- `worktree_handlers_reject_directory_outside_project`
- `worktree_delete_and_reset_reject_invalid_ids_before_project_lookup`
- `worktree_delete_and_reset_reject_non_git_project_after_lookup`
- `worktree_environment_ui_project_command_roundtrip_and_clear`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
