# 任务27实施证据（进行中，未验收）

## 授权与验证

遵守用户禁测要求，未运行任何Cargo、测试、绘制或回归。独立rustfmt仅处理修改文件；本任务路径的git diff --check无输出。全工作区diff --check发现其它任务的空白问题，未扩大修改。

## 已接线的生产归属

- settings/worktrees/environment::update → desktop-settings-workspace::environment::update：完整刷新、去重、编辑、请求代际、保存、失败保留；具名EnvironmentEffect只要求宿主选择目录、同步项目缓存和显示toast。本机目录选择仍由loopback和平台约束控制。
- settings/archived_chats::update → desktop-settings-sessions::archive_update::update：单聊天删除/恢复的确认、响应身份、代际与补偿刷新均由库处理。Deleted/Restored/ProjectLoaded/Toast由宿主处理跨域缓存、项目消息和通知。项目整组删除原有独立装配未变。
- projects::reconcile_deleted和set_worktree_enabled持有元数据名称重建与单项目patch策略；宿主保留真实路径、保存串行队列、退出等待和项目导航。
- sessions::next_after_delete只读投影下一个可导航会话；session运行时取消、权限加载、chat清理与App生命周期仍由宿主承担。
- workspace引用现有desktop-workspace::RecentProjectMeta；没有反向依赖宿主或include/path挂载宿主源码。

## 测试归属与尚未完成的范围

迁入环境页面整组、恢复流程整组、项目行布局、项目搜索/确认/toggle、managed的稳定列表/部分错误/真实绘制/HTTP分页及archive的四组HTTP用例。保留中文、双主题、draw/screenshot及网络夹具源码，未执行。

**任务尚未满足完整交付契约**：项目编辑UI整组仍在宿主，其布局/事件/帮助断言须继续迁往现有project_edit_view真实入口；managed部分纯状态断言仍混在导航契约；项目元数据与归档路由测试仍须拆开领域断言和跨域断言。未删除任务文件，不宣称完成。领域局部字段的进一步归属也须结合真实调用者继续收口。

## 基线统计

按pathlib.read_text().splitlines()，来源 3621 → 2398，减少 1223；生产候选 537 → 436，减少 101。这是当前工作区来源口径，可能含同期导航接线，不等同本提交独立减量。新增宿主替代文件0。

| 来源 | 基线 | 当前 |
|---|---:|---:|
| system_settings_projects.rs | 54 | 46 |
| system_settings_projects_tests.rs | 29 | 0 |
| system_settings_projects_ui_refresh_30_tests.rs | 931 | 931 |
| environment_tests.rs | 313 | 0 |
| managed_tests.rs | 368 | 210 |
| recovery_tests.rs | 224 | 0 |
| archived_chats.rs | 236 | 133 |
| archived_chats_tests.rs | 785 | 426 |
| projects.rs | 147 | 148 |
| projects_tests.rs | 236 | 197 |
| sessions.rs | 100 | 109 |
| sessions_tests.rs | 198 | 198 |

## 旧用例逐名映射

保留原函数名；迁移位置保留对应断言。恢复测试中宿主的draft/new-session未变化断言改为领域不发Open效果，宿主真实导航契约仍由managed导航用例验证；项目行使用共享help控件的局部装配。其余宿主残留断言尚需继续逐项收口。
- `worktree_row_builds_fill_width_help_label_and_control_layout` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/projects_view.rs
- `ui_refresh_30_layout_and_interaction` → crates/vw-desktop/src/app/components/system_settings_projects_ui_refresh_30_tests.rs（宿主残留）
- `ui_refresh_30_preserves_values_and_disabled_states` → crates/vw-desktop/src/app/components/system_settings_projects_ui_refresh_30_tests.rs（宿主残留）
- `settings_tabs_peripheral_appearance_project` → crates/vw-desktop/src/app/components/system_settings_projects_ui_refresh_30_tests.rs（宿主残留）
- `help_rollout_21_project_settings_render_contextual_help` → crates/vw-desktop/src/app/components/system_settings_projects_ui_refresh_30_tests.rs（宿主残留）
- `help_rollout_21_renders_contextual_help` → crates/vw-desktop/src/app/components/system_settings_projects_ui_refresh_30_tests.rs（宿主残留）
- `worktree_environment_ui_saves_setup_for_selected_project` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/environment.rs
- `worktree_environment_ui_cancel_preserves_command` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/environment.rs
- `worktree_environment_ui_save_does_not_execute` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/environment.rs
- `worktree_environment_ui_clear_failure_retry_and_stale_response` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/environment.rs
- `worktree_environment_ui_rejects_remote_picker_and_deduplicates_additions` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/environment.rs
- `worktree_environment_ui_project_switch_discards_pending_save` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/environment.rs
- `worktree_environment_ui_renders_chinese_editor_in_both_themes` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/environment.rs
- `worktree_environment_ui_paginates_and_sends_only_project_patch` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/environment.rs
- `worktree_environment_ui_retry_blocks_chat_until_refreshed_ready` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/environment.rs
- `worktree_managed_ui_opens_chat_in_selected_existing_checkout` → crates/vw-desktop/src/app/components/system_settings_worktrees/managed_tests.rs（宿主残留）
- `worktree_managed_ui_switches_project_before_draft` → crates/vw-desktop/src/app/components/system_settings_worktrees/managed_tests.rs（宿主残留）
- `worktree_managed_ui_refresh_keeps_groups_stable` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/managed.rs
- `worktree_managed_ui_deduplicates_and_preserves_session_on_failure` → crates/vw-desktop/src/app/components/system_settings_worktrees/managed_tests.rs（宿主残留）
- `worktree_managed_ui_rejects_stale_refresh_and_navigation` → crates/vw-desktop/src/app/components/system_settings_worktrees/managed_tests.rs（宿主残留）
- `worktree_managed_ui_preflight_is_scoped_and_never_deletes` → crates/vw-desktop/src/app/components/system_settings_worktrees/managed_tests.rs（宿主残留）
- `worktree_managed_ui_partial_error_retains_rows_and_collapsed_groups` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/managed.rs
- `worktree_managed_ui_renders_long_chinese_and_errors_in_both_themes` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/managed.rs
- `worktree_managed_ui_paginates_all_projects_and_keeps_partial_failure` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/managed.rs
- `worktree_managed_ui_returns_to_project_screen_before_showing_draft` → crates/vw-desktop/src/app/components/system_settings_worktrees/managed_tests.rs（宿主残留）
- `worktree_cleanup_flow_cross_project_response_is_ignored` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/recovery.rs
- `worktree_cleanup_flow_old_epoch_response_is_ignored` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/recovery.rs
- `worktree_cleanup_flow_item_failure_requires_new_preflight` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/recovery.rs
- `worktree_cleanup_flow_restores_and_refreshes` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/recovery.rs
- `worktree_cleanup_flow_delete_success_refreshes_and_records_snapshot` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/recovery.rs
- `worktree_cleanup_flow_load_failure_does_not_report_empty_success` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/recovery.rs
- `worktree_cleanup_flow_restore_failure_can_retry_persisted_snapshot` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/recovery.rs
- `worktree_cleanup_flow_cancel_rejects_late_preflight` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/recovery.rs
- `worktree_cleanup_flow_preflight_cannot_cross_project` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/recovery.rs
- `worktree_cleanup_flow_delete_revokes_inflight_snapshot_list` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/recovery.rs
- `archived_chats_restore_http_persists_only_target_and_retries_failure` → crates/libs/desktop-settings-sessions/src/host_module_27_tests.rs
- `archived_chats_restore_routes_sync_caches_without_navigation_and_rejects_late_results` → crates/vw-desktop/src/app/message/settings/archived_chats_tests.rs（宿主残留）
- `archived_chats_browse_task_reads_all_projects_over_http` → crates/libs/desktop-settings-sessions/src/host_module_27_tests.rs
- `archived_chats_browse_settings_routes_and_gateway_switch` → crates/vw-desktop/src/app/message/settings/archived_chats_tests.rs（宿主残留）
- `archived_chats_delete_http_failure_retains_confirmation_then_retry_deletes_tree` → crates/libs/desktop-settings-sessions/src/host_module_27_tests.rs
- `archived_chats_delete_routes_cleanup_only_after_success` → crates/vw-desktop/src/app/message/settings/archived_chats_tests.rs（宿主残留）
- `archived_chats_project_delete_rechecks_archive_before_restricted_http_delete` → crates/libs/desktop-settings-sessions/src/host_module_27_tests.rs
- `search_updates_query_and_dismisses_previous_row_actions` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/project_events.rs
- `delete_request_and_cancel_only_change_confirmation_state` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/project_events.rs
- `delete_request_allows_current_project` → crates/vw-desktop/src/app/message/settings/projects_tests.rs（宿主残留）
- `delete_confirmed_current_project_selects_first_remaining_project` → crates/vw-desktop/src/app/message/settings/projects_tests.rs（宿主残留）
- `delete_confirmed_removes_associated_state_and_rebuilds_display_names` → crates/vw-desktop/src/app/message/settings/projects_tests.rs（宿主残留）
- `worktree_toggles_patch_one_project_in_click_order` → crates/vw-desktop/src/app/message/settings/projects_tests.rs（宿主残留）
- `delete_confirmed_handles_missing_edit_for_existing_project` → crates/vw-desktop/src/app/message/settings/projects_tests.rs（宿主残留）
- `delete_confirmed_rejects_out_of_bounds_index_but_closes_confirmation` → crates/vw-desktop/src/app/message/settings/projects_tests.rs（宿主残留）
- `worktree_toggle_inserts_enabled_entry_and_removes_disabled_entry` → crates/libs/desktop-settings-workspace/src/host_module_27_tests/project_events.rs
- `unrelated_settings_message_leaves_project_state_unchanged` → crates/vw-desktop/src/app/message/settings/projects_tests.rs（宿主残留）
- `session_delete_active_only_session_clears_conversation_and_usage` → crates/vw-desktop/src/app/message/settings/sessions_tests.rs（宿主残留）
- `session_delete_active_session_switches_to_first_remaining_and_syncs_preferences` → crates/vw-desktop/src/app/message/settings/sessions_tests.rs（宿主残留）
- `session_delete_inactive_session_preserves_active_context` → crates/vw-desktop/src/app/message/settings/sessions_tests.rs（宿主残留）
- `session_delete_missing_session_removes_orphan_runtime_without_changing_context` → crates/vw-desktop/src/app/message/settings/sessions_tests.rs（宿主残留）
- `session_delete_missing_active_session_switches_to_first_available_session` → crates/vw-desktop/src/app/message/settings/sessions_tests.rs（宿主残留）
- `session_copy_leaves_local_session_state_unchanged_until_task_completes` → crates/vw-desktop/src/app/message/settings/sessions_tests.rs（宿主残留）
- `unrelated_settings_message_is_ignored` → crates/vw-desktop/src/app/message/settings/sessions_tests.rs（宿主残留）

## 后续验证（仅用户另行授权）

- python3 scripts/cargo_slot.py cargo test -p desktop-settings-workspace --lib host_module_27_tests -- --test-threads=1
- python3 scripts/cargo_slot.py cargo test -p desktop-settings-sessions --lib host_module_27_tests -- --test-threads=1
- python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib

当前不宣称类型检查、绘制和行为通过；新增依赖锁文件仅文本同步。
