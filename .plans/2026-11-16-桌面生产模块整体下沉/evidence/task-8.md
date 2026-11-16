# 任务8：项目文件树与工作区操作静态交付

本轮未运行任何 Cargo 命令、测试、实机绘制或回归；独立 rustfmt 仅解析源码，不能证明类型检查与行为通过。

## 生产归属与真实调用链

- basic::handle → basic::handle_input；App 持有 InputState。查找消息 → FindState::handle；App 持有 FindState，不再逐字段搬出回写。所有原字段直接调用者和视图投影同步改接。
- 文件树手势 → InteractionState::handle_local/prepare_move/prepare_import；保留旧项目拒绝、冻结选择、先消费后校验、重复释放隔离。
- batch::{duplicate,paste,delete} / move_drop::submit → operations::tasks；宿主只注入原网关执行器并映射 FileEvent。原有逐项执行、不覆盖目标、部分成功列表及旧剪贴板比较保持。外部导入任务同样下沉；本机/远程网关拒绝仍由已有 gateway 实现负责。
- helpers::run_find_task / refresh_file_index_with_fallback → project_message::tasks；网关目录快照、保留空目录、本地索引回退及目录缓存参数均沿用原实现。DTO 转换与会话所属项目查找归入 data。
- open_project → desktop_navigation::panel::project_workspace::{begin_project,restore_project_preview}；导航库原本依赖 workspace，保持依赖方向。
- helpers::prepare_*ui*_task → desktop_chat_view::preparation_tasks；使用已有 desktop-session 的共享历史和 chat-ui 的真实分块实现，保持 32 条快照、stream_generation 与完成消息。chat-view 已依赖 session → workspace，禁止 workspace 反向依赖 chat-view。
- 依赖变化只将已存在 dev-dependency Tokio 的 rt 能力提供给真实生产任务；未新增包或反向宿主依赖。

## 宿主残留逐函数清单

宿主只保留项目与聊天、Git、预览、终端、看板、导航及设置的跨域效果装配、活动资源所有权和消息映射。下列清单列出每个函数，具体归属以其对应领域委派和效果调用为准。

- `crates/vw-desktop/src/app/message/project/basic.rs`：405 → 382 行。
  函数：`schedule_file_manager_refresh_tick`, `reveal_work_workspace_files`, `ensure_file_manager_changes_expanded`, `handle`。
- `crates/vw-desktop/src/app/message/project/basic_tests.rs`：875 → 818 行。
- `crates/vw-desktop/src/app/message/project/file_tree.rs`：558 → 545 行。
  函数：`handle`, `release_tree_drop`。
- `crates/vw-desktop/src/app/message/project/file_tree_batch.rs`：84 → 59 行。
  函数：`execute`, `menu_paths`, `duplicate`, `paste`, `delete`, `file_event`。
- `crates/vw-desktop/src/app/message/project/file_tree_batch_tests.rs`：258 → 211 行。
- `crates/vw-desktop/src/app/message/project/file_tree_external_drop.rs`：69 → 73 行。
  函数：`submit`, `complete`。
- `crates/vw-desktop/src/app/message/project/file_tree_external_drop_tests.rs`：202 → 202 行。
- `crates/vw-desktop/src/app/message/project/file_tree_move_drop.rs`：161 → 137 行。
  函数：`clear_drag`, `release`, `submit`, `update_tabs`, `complete`, `complete_batch`。
- `crates/vw-desktop/src/app/message/project/file_tree_move_drop_http_tests.rs`：239 → 239 行。
- `crates/vw-desktop/src/app/message/project/file_tree_move_drop_tests.rs`：149 → 137 行。
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs`：801 → 742 行。
- `crates/vw-desktop/src/app/message/project/find.rs`：76 → 100 行。
  函数：`handle`。
- `crates/vw-desktop/src/app/message/project/find_tests.rs`：630 → 423 行。
- `crates/vw-desktop/src/app/message/project/helpers.rs`：681 → 638 行。
  函数：`collect_local_attachments`, `append_local_attachments`, `refresh_file_index`, `refresh_file_index_with_fallback`, `find_session_project_path`, `now_ms`, `gateway_client`, `resolve_gateway_file_context`, `load_gateway_recent_projects`, `resolve_gateway_project`, `list_gateway_project_sessions`, `gateway_session_project_path`, `load_project_info_task`, `load_project_worktree_picker_options`, `create_gateway_session_worktree`, `delete_gateway_worktree`, `reset_gateway_worktree`, `gateway_import_local_paths`, `gateway_move_path`, `gateway_copy_path`, `gateway_duplicate_path`, `gateway_delete_path`, `run_find_task`, `load_prepared_session_open_from_ui_store_task`, `load_prepared_session_open_from_gateway_task`, `prepare_gateway_session_open`, `load_session_history_page_task`, `ui_event`, `prepare_session_ui_task`, `prepare_stream_message_ui_task`, `prepare_session_ui_chunks_task`。
- `crates/vw-desktop/src/app/message/project/helpers_tests.rs`：299 → 208 行。
- `crates/vw-desktop/src/app/message/project/mod.rs`：58 → 58 行。
  函数：`update`。
- `crates/vw-desktop/src/app/message/project/mod_tests.rs`：30 → 30 行。
- `crates/vw-desktop/src/app/message/project/session.rs`：38 → 38 行。
  函数：`handle`。
- `crates/vw-desktop/src/app/message/project/session_tests.rs`：80 → 80 行。
- `crates/vw-desktop/src/app/message/project/tests.rs`：89 → 89 行。
- `crates/vw-desktop/src/app/projects.rs`：469 → 438 行。
  函数：`save_recent_projects_background`, `save_recent_projects_meta_background`, `load_recent_projects`, `save_recent_projects`, `push_recent_project`, `ensure_recent_project`, `load_recent_projects_meta`, `save_recent_projects_meta`, `load_project_branches`, `project_settings_refresh_task`, `switch_project_terminal`, `reset_task_board_project_state`, `open_project_and_index`, `open_first_recent_project_or_home`, `switch_work_space_preserving_chat`, `open_project`, `refresh_branches`。

## 断言映射

同名测试保留原状态、错误、边界与结果断言；领域测试直接使用领域状态或临时目录，不构造 App。宿主保留实际跨域消息契约；前轮 desktop_split_02/03/26 领域断言沿用现有注册，没有复制。UI 分块断言因真实依赖方向归入 chat-view。

- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `test_module_is_wired` → `crates/libs/desktop-workspace/src/desktop_split_19_tests/main_tests.rs`, `crates/libs/desktop-workspace/src/host_module_7_tests/worktree_tests.rs`, `crates/libs/desktop-workspace/src/project_message/common_tests.rs`, `crates/libs/desktop-workspace/src/views/project/handles_tests.rs`, `crates/libs/desktop-workspace/src/views/project/utils_tests.rs`, `crates/libs/desktop-workspace/src/views/project/layout/chrome_tests.rs`, `crates/libs/desktop-workspace/src/views/project/layout/bottom_tests.rs`, `crates/libs/desktop-workspace/src/views/project/layout/sessions_panel_tests.rs`, `crates/libs/desktop-workspace/src/views/project/layout/drag_tests.rs`, `crates/libs/desktop-workspace/src/views/project/layout/chat_tests.rs`, `crates/libs/desktop-workspace/src/views/project/components/sessions_panel_tests.rs`, `crates/libs/desktop-workspace/src/views/project/components/new_session_tests.rs`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `recent_project` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `project_path_changed_updates_input` → `crates/libs/desktop-workspace/src/host_module_8_tests/basic_tests.rs`, `crates/libs/desktop-workspace/src/host_module_8_tests/basic_tests.rs`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `open_project_pressed_uses_non_empty_input_path` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `recent_projects_loaded_success_replaces_all_recent_project_views` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `recent_projects_loaded_error_preserves_existing_projects` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `file_url_changed_and_add_file_pressed_append_file_once` → `crates/libs/desktop-workspace/src/host_module_8_tests/basic_tests.rs`, `crates/libs/desktop-workspace/src/host_module_8_tests/basic_tests.rs`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `add_file_pressed_ignores_empty_input` → `crates/libs/desktop-workspace/src/host_module_8_tests/basic_tests.rs`, `crates/libs/desktop-workspace/src/host_module_8_tests/basic_tests.rs`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `attachment_files_picked_appends_paths` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `attachment_files_picked_none_preserves_existing_files` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `remove_attached_file_removes_matching_path` → `crates/libs/desktop-workspace/src/host_module_8_tests/basic_tests.rs`, `crates/libs/desktop-workspace/src/host_module_8_tests/basic_tests.rs`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `remove_attached_file_removes_every_duplicate_and_preserves_other_paths` → `crates/libs/desktop-workspace/src/host_module_8_tests/basic_tests.rs`, `crates/libs/desktop-workspace/src/host_module_8_tests/basic_tests.rs`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `file_index_loaded_without_project_does_not_expose_cached_files` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `file_index_loaded_for_project_sets_cached_files_without_changing_project` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `file_index_loaded_requests_refresh_when_project_exists_and_index_is_stale` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `file_index_ready_sets_index_and_stops_refreshing` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `file_index_ready_without_active_refresh_only_sets_index` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `toggle_tree_dir_toggles_expanded_state` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `file_manager_show_changes_true_clears_active_preview_and_expands_changed_dirs` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `file_manager_show_changes_true_in_work_mode_reveals_workspace_files` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `file_manager_show_changes_false_preserves_preview_and_resets_find_selection` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `file_manager_refresh_changes_ignores_refresh_while_git_files_are_loading` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `file_manager_refresh_changes_ignores_duplicate_refresh` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `file_manager_refresh_changes_marks_refreshing_when_idle` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `file_manager_refresh_file_tree_ignores_duplicate_refresh` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `file_manager_refresh_file_tree_marks_refreshing_when_idle` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `file_manager_refresh_animation_tick_stops_when_all_work_is_idle` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `file_manager_refresh_animation_tick_wraps_frame_while_work_is_active` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `open_changed_file_without_project_returns_none_task_only` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `open_changed_file_in_work_mode_reveals_workspace_changes_and_focuses_file` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `open_changed_file_expands_collapsed_change_dirs_when_already_in_changes_view` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `file_tree_add_to_chat_rejects_path_outside_project_without_mutating_search` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `file_tree_add_to_chat_without_project_preserves_editor_and_search_state` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `file_tree_add_to_chat_inserts_relative_mention_and_resets_search_state` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `start_deferred_tasks_rejects_stale_project_path` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `unrelated_project_message_is_not_claimed_by_basic_handler` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `empty_directory_snapshot` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `file_tree_empty_directory_snapshot_refresh_updates_tree_and_search_without_attachments` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `file_tree_empty_directory_old_project_result_only_updates_its_cache` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `file_tree_empty_directory_directory_only_snapshot_is_cached_with_empty_file_index` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `file_tree_empty_directory_old_local_load_does_not_pollute_current_project` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/basic_tests.rs` / `project_file_preferences_update_before_gateway_reply` → `crates/vw-desktop/src/app/message/project/basic_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_batch_tests.rs` / `selected_context_preserves_multi_selection_and_single_actions_use_first_visible_item` → `crates/vw-desktop/src/app/message/project/file_tree_batch_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_batch_tests.rs` / `right_clicking_unselected_item_replaces_selection` → `crates/libs/desktop-workspace/src/host_module_8_tests/batch_tests.rs`, `crates/libs/desktop-workspace/src/host_module_8_tests/batch_tests.rs`
- `crates/vw-desktop/src/app/message/project/file_tree_batch_tests.rs` / `batch_delete_confirmation_captures_all_paths_and_cancel_preserves_files` → `crates/vw-desktop/src/app/message/project/file_tree_batch_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_batch_tests.rs` / `copy_cut_and_chat_actions_include_all_selected_items` → `crates/vw-desktop/src/app/message/project/file_tree_batch_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_batch_tests.rs` / `partial_cut_completion_keeps_failed_sources_and_does_not_clear_new_clipboard` → `crates/libs/desktop-workspace/src/host_module_8_tests/batch_tests.rs`, `crates/libs/desktop-workspace/src/host_module_8_tests/batch_tests.rs`
- `crates/vw-desktop/src/app/message/project/file_tree_batch_tests.rs` / `partial_delete_completion_only_clears_successful_selection_in_original_project` → `crates/libs/desktop-workspace/src/host_module_8_tests/batch_tests.rs`, `crates/libs/desktop-workspace/src/host_module_8_tests/batch_tests.rs`
- `crates/vw-desktop/src/app/message/project/file_tree_batch_tests.rs` / `batch_delete_http_attempts_every_root_once_and_reports_partial_failure` → `crates/vw-desktop/src/app/message/project/file_tree_batch_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_external_drop_tests.rs` / `file_tree_external_drop_remote_rejected_before_project_resolution` → `crates/vw-desktop/src/app/message/project/file_tree_external_drop_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_external_drop_tests.rs` / `file_tree_external_drop_tasks_keep_each_source_and_project_after_switch_and_partial_failure` → `crates/vw-desktop/src/app/message/project/file_tree_external_drop_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_move_drop_http_tests.rs` / `file_tree_move_drop_http_counts_requests_for_both_release_orders_and_conflict` → `crates/vw-desktop/src/app/message/project/file_tree_move_drop_http_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_move_drop_tests.rs` / `selected_drag_freezes_all_items_and_unselected_drag_replaces_selection` → `crates/libs/desktop-workspace/src/host_module_8_tests/interaction_tests.rs`, `crates/libs/desktop-workspace/src/host_module_8_tests/interaction_tests.rs`
- `crates/vw-desktop/src/app/message/project/file_tree_move_drop_tests.rs` / `selected_drag_chat_delivery_contains_every_item_for_both_release_orders` → `crates/vw-desktop/src/app/message/project/file_tree_move_drop_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_move_drop_tests.rs` / `selected_drag_folder_release_is_consumed_once_and_cancel_clears_entire_gesture` → `crates/vw-desktop/src/app/message/project/file_tree_move_drop_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_move_drop_tests.rs` / `selected_drag_rejects_target_inside_any_selected_directory` → `crates/vw-desktop/src/app/message/project/file_tree_move_drop_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_move_drop_tests.rs` / `selected_drag_partial_completion_preserves_dirty_tabs_and_failed_selection` → `crates/vw-desktop/src/app/message/project/file_tree_move_drop_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `test_module_is_wired` → `crates/libs/desktop-workspace/src/desktop_split_19_tests/main_tests.rs`, `crates/libs/desktop-workspace/src/host_module_7_tests/worktree_tests.rs`, `crates/libs/desktop-workspace/src/project_message/common_tests.rs`, `crates/libs/desktop-workspace/src/views/project/handles_tests.rs`, `crates/libs/desktop-workspace/src/views/project/utils_tests.rs`, `crates/libs/desktop-workspace/src/views/project/layout/chrome_tests.rs`, `crates/libs/desktop-workspace/src/views/project/layout/bottom_tests.rs`, `crates/libs/desktop-workspace/src/views/project/layout/sessions_panel_tests.rs`, `crates/libs/desktop-workspace/src/views/project/layout/drag_tests.rs`, `crates/libs/desktop-workspace/src/views/project/layout/chat_tests.rs`, `crates/libs/desktop-workspace/src/views/project/components/sessions_panel_tests.rs`, `crates/libs/desktop-workspace/src/views/project/components/new_session_tests.rs`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `file_tree_right_click_and_close_manage_menu_state` → `crates/libs/desktop-workspace/src/host_module_8_tests/interaction_tests.rs`, `crates/libs/desktop-workspace/src/host_module_8_tests/interaction_tests.rs`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `rename_changed_and_cancel_manage_rename_state` → `crates/libs/desktop-workspace/src/host_module_8_tests/interaction_tests.rs`, `crates/libs/desktop-workspace/src/host_module_8_tests/interaction_tests.rs`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `drag_start_and_end_transfer_pending_drop_state` → `crates/libs/desktop-workspace/src/host_module_8_tests/interaction_tests.rs`, `crates/libs/desktop-workspace/src/host_module_8_tests/interaction_tests.rs`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `rename_save_rejects_each_invalid_name_shape` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `rename_save_without_target_or_project_is_safely_ignored` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `rename_save_with_project_queues_operation_and_clears_editor_state` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `rename_unchanged_name_closes_editor_without_moving_file` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `rename_completed_updates_preview_paths_on_success` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `rename_completed_failure_preserves_paths_and_reports_error` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `file_tree_action_without_menu_path_is_ignored` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `delete_action_keeps_file_until_confirmed` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `delete_confirmation_removes_directory_subtree_before_gateway_completes` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `delete_confirmation_keeps_file_after_cancel_or_project_change` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `copy_and_cut_actions_update_file_tree_clipboard` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `find_in_folder_action_creates_active_find_tab_and_open_tab` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `rename_action_primes_rename_fields` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `open_and_copy_path_actions_clear_menu_and_return_tasks` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `relative_copy_delete_and_add_to_chat_handle_missing_project_root` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `paste_without_clipboard_or_project_does_not_mutate_clipboard` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `unrelated_project_message_is_not_handled` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `paste_completed_cut_success_clears_clipboard` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `paste_completed_copy_success_preserves_clipboard` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `paste_and_delete_failures_show_error_toasts` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `file_tree_move_drop_release_consumes_tree_source_once` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `file_tree_move_drop_completion_preserves_dirty_content_and_directory_boundaries` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `file_tree_move_drop_failure_and_old_project_leave_current_editor_and_drag_untouched` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `file_tree_move_drop_rejects_invalid_targets_and_consumes_release` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `file_tree_move_drop_unicode_conflict_preserves_unsaved_editor_and_active_path` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `file_tree_move_drop_target_release_uses_current_source_and_is_consumed_once` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `modified_tree_clicks_select_files_and_directories_without_opening_them` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/file_tree_tests.rs` / `tree_selection_rejects_old_project_and_invisible_rows` → `crates/vw-desktop/src/app/message/project/file_tree_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/find_tests.rs` / `test_module_is_wired` → `crates/libs/desktop-workspace/src/desktop_split_19_tests/main_tests.rs`, `crates/libs/desktop-workspace/src/host_module_7_tests/worktree_tests.rs`, `crates/libs/desktop-workspace/src/project_message/common_tests.rs`, `crates/libs/desktop-workspace/src/views/project/handles_tests.rs`, `crates/libs/desktop-workspace/src/views/project/utils_tests.rs`, `crates/libs/desktop-workspace/src/views/project/layout/chrome_tests.rs`, `crates/libs/desktop-workspace/src/views/project/layout/bottom_tests.rs`, `crates/libs/desktop-workspace/src/views/project/layout/sessions_panel_tests.rs`, `crates/libs/desktop-workspace/src/views/project/layout/drag_tests.rs`, `crates/libs/desktop-workspace/src/views/project/layout/chat_tests.rs`, `crates/libs/desktop-workspace/src/views/project/components/sessions_panel_tests.rs`, `crates/libs/desktop-workspace/src/views/project/components/new_session_tests.rs`
- `crates/vw-desktop/src/app/message/project/find_tests.rs` / `find_in_project_without_project_path_does_not_create_tabs` → `crates/libs/desktop-workspace/src/host_module_8_tests/find_tests.rs`, `crates/libs/desktop-workspace/src/host_module_8_tests/find_tests.rs`
- `crates/vw-desktop/src/app/message/project/find_tests.rs` / `find_in_project_creates_active_find_tab_and_open_tab` → `crates/vw-desktop/src/app/message/project/find_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/find_tests.rs` / `find_scope_changed_resolves_project_relative_input_and_rejects_escape` → `crates/libs/desktop-workspace/src/host_module_8_tests/find_tests.rs`, `crates/libs/desktop-workspace/src/host_module_8_tests/find_tests.rs`
- `crates/vw-desktop/src/app/message/project/find_tests.rs` / `find_completed_updates_existing_tab_without_replacing_matches_on_error` → `crates/libs/desktop-workspace/src/host_module_8_tests/find_tests.rs`, `crates/libs/desktop-workspace/src/host_module_8_tests/find_tests.rs`
- `crates/vw-desktop/src/app/message/project/find_tests.rs` / `find_completed_creates_missing_tab_with_matches` → `crates/libs/desktop-workspace/src/host_module_8_tests/find_tests.rs`, `crates/libs/desktop-workspace/src/host_module_8_tests/find_tests.rs`
- `crates/vw-desktop/src/app/message/project/find_tests.rs` / `find_completed_success_replaces_existing_matches_and_clears_running` → `crates/libs/desktop-workspace/src/host_module_8_tests/find_tests.rs`, `crates/libs/desktop-workspace/src/host_module_8_tests/find_tests.rs`
- `crates/vw-desktop/src/app/message/project/find_tests.rs` / `find_completed_reuses_existing_open_tab_without_duplication` → `crates/vw-desktop/src/app/message/project/find_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/find_tests.rs` / `tab_selected_empty_clears_active_find_tab` → `crates/libs/desktop-workspace/src/host_module_8_tests/find_tests.rs`, `crates/libs/desktop-workspace/src/host_module_8_tests/find_tests.rs`
- `crates/vw-desktop/src/app/message/project/find_tests.rs` / `match_selected_prepares_zero_based_editor_target_and_opens_file` → `crates/vw-desktop/src/app/message/project/find_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/find_tests.rs` / `match_selected_clamps_invalid_one_based_position_to_editor_origin` → `crates/vw-desktop/src/app/message/project/find_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/find_tests.rs` / `tab_selected_existing_open_tab_keeps_the_active_workspace` → `crates/vw-desktop/src/app/message/project/find_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/find_tests.rs` / `tab_selected_without_open_tab_preserves_active_open_tab` → `crates/vw-desktop/src/app/message/project/find_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/find_tests.rs` / `tab_selected_while_search_is_running_keeps_the_active_find_tab` → `crates/libs/desktop-workspace/src/host_module_8_tests/find_tests.rs`, `crates/libs/desktop-workspace/src/host_module_8_tests/find_tests.rs`
- `crates/vw-desktop/src/app/message/project/find_tests.rs` / `tab_closed_removes_find_and_open_tabs_and_selects_last` → `crates/vw-desktop/src/app/message/project/find_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/find_tests.rs` / `tab_closed_nonactive_tab_preserves_active_selections` → `crates/vw-desktop/src/app/message/project/find_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/find_tests.rs` / `tab_closed_without_open_tab_selects_last_remaining_find_tab` → `crates/vw-desktop/src/app/message/project/find_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/find_tests.rs` / `tab_closed_last_active_find_tab_clears_active_find_and_falls_back_to_last_open_tab` → `crates/vw-desktop/src/app/message/project/find_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/find_tests.rs` / `find_tab_right_click_records_menu_target_and_close_dismisses_it` → `crates/libs/desktop-workspace/src/host_module_8_tests/find_tests.rs`, `crates/libs/desktop-workspace/src/host_module_8_tests/find_tests.rs`
- `crates/vw-desktop/src/app/message/project/find_tests.rs` / `find_tab_menu_close_current_syncs_active_find_and_top_level_tabs` → `crates/vw-desktop/src/app/message/project/find_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/find_tests.rs` / `find_tab_menu_close_left_syncs_active_find_and_top_level_tabs` → `crates/vw-desktop/src/app/message/project/find_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/find_tests.rs` / `find_tab_menu_close_right_syncs_active_find_and_top_level_tabs` → `crates/vw-desktop/src/app/message/project/find_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/find_tests.rs` / `find_tab_menu_close_all_clears_all_find_state_and_top_level_tabs` → `crates/vw-desktop/src/app/message/project/find_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/find_tests.rs` / `unrelated_project_message_is_not_handled` → `crates/vw-desktop/src/app/message/project/find_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/helpers_tests.rs` / `append_local_attachments_adds_only_new_valid_files` → `crates/vw-desktop/src/app/message/project/helpers_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/helpers_tests.rs` / `find_session_project_path_falls_back_to_active_project` → `crates/libs/desktop-workspace/src/host_module_8_tests/helpers_tests.rs`, `crates/libs/desktop-workspace/src/host_module_8_tests/helpers_tests.rs`
- `crates/vw-desktop/src/app/message/project/helpers_tests.rs` / `refresh_file_index_without_project_path_produces_no_messages` → `crates/vw-desktop/src/app/message/project/helpers_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/helpers_tests.rs` / `map_loaded_project_info_drops_blank_current_branch` → `crates/libs/desktop-workspace/src/host_module_8_tests/helpers_tests.rs`, `crates/libs/desktop-workspace/src/host_module_8_tests/helpers_tests.rs`
- `crates/vw-desktop/src/app/message/project/helpers_tests.rs` / `map_loaded_project_info_keeps_non_blank_current_branch` → `crates/libs/desktop-workspace/src/host_module_8_tests/helpers_tests.rs`, `crates/libs/desktop-workspace/src/host_module_8_tests/helpers_tests.rs`
- `crates/vw-desktop/src/app/message/project/helpers_tests.rs` / `prepare_session_ui_tasks_return_none_for_empty_work_units` → `crates/vw-desktop/src/app/message/project/helpers_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/helpers_tests.rs` / `chat_ui_chunk_snapshots_requested_32_only` → `crates/libs/desktop-chat-view/src/preparation_tasks_tests.rs`
- `crates/vw-desktop/src/app/message/project/helpers_tests.rs` / `prepare_session_ui_does_not_clone_full_history` → `crates/libs/desktop-chat-view/src/preparation_tasks_tests.rs`
- `crates/vw-desktop/src/app/message/project/helpers_tests.rs` / `prepare_stream_message_ui_task_prepares_only_target` → `crates/vw-desktop/src/app/message/project/helpers_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/helpers_tests.rs` / `file_tree_empty_directory_async_refresh_retains_originating_project` → `crates/vw-desktop/src/app/message/project/helpers_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/helpers_tests.rs` / `run_find_task_maps_request_and_localized_error_to_completion` → `crates/vw-desktop/src/app/message/project/helpers_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/mod_tests.rs` / `test_module_is_wired` → `crates/libs/desktop-workspace/src/desktop_split_19_tests/main_tests.rs`, `crates/libs/desktop-workspace/src/host_module_7_tests/worktree_tests.rs`, `crates/libs/desktop-workspace/src/project_message/common_tests.rs`, `crates/libs/desktop-workspace/src/views/project/handles_tests.rs`, `crates/libs/desktop-workspace/src/views/project/utils_tests.rs`, `crates/libs/desktop-workspace/src/views/project/layout/chrome_tests.rs`, `crates/libs/desktop-workspace/src/views/project/layout/bottom_tests.rs`, `crates/libs/desktop-workspace/src/views/project/layout/sessions_panel_tests.rs`, `crates/libs/desktop-workspace/src/views/project/layout/drag_tests.rs`, `crates/libs/desktop-workspace/src/views/project/layout/chat_tests.rs`, `crates/libs/desktop-workspace/src/views/project/components/sessions_panel_tests.rs`, `crates/libs/desktop-workspace/src/views/project/components/new_session_tests.rs`
- `crates/vw-desktop/src/app/message/project/mod_tests.rs` / `update_returns_no_task_for_unhandled_start_deferred_tasks_message` → `crates/vw-desktop/src/app/message/project/mod_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/session_tests.rs` / `test_module_is_wired` → `crates/libs/desktop-workspace/src/desktop_split_19_tests/main_tests.rs`, `crates/libs/desktop-workspace/src/host_module_7_tests/worktree_tests.rs`, `crates/libs/desktop-workspace/src/project_message/common_tests.rs`, `crates/libs/desktop-workspace/src/views/project/handles_tests.rs`, `crates/libs/desktop-workspace/src/views/project/utils_tests.rs`, `crates/libs/desktop-workspace/src/views/project/layout/chrome_tests.rs`, `crates/libs/desktop-workspace/src/views/project/layout/bottom_tests.rs`, `crates/libs/desktop-workspace/src/views/project/layout/sessions_panel_tests.rs`, `crates/libs/desktop-workspace/src/views/project/layout/drag_tests.rs`, `crates/libs/desktop-workspace/src/views/project/layout/chat_tests.rs`, `crates/libs/desktop-workspace/src/views/project/components/sessions_panel_tests.rs`, `crates/libs/desktop-workspace/src/views/project/components/new_session_tests.rs`
- `crates/vw-desktop/src/app/message/project/session_tests.rs` / `handle_returns_none_for_non_session_message` → `crates/vw-desktop/src/app/message/project/session_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/session_tests.rs` / `handle_dispatches_recent_overlay_close_to_open_module` → `crates/vw-desktop/src/app/message/project/session_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/session_tests.rs` / `handle_dispatches_project_tools_toggle_to_edit_module` → `crates/vw-desktop/src/app/message/project/session_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/session_tests.rs` / `handle_dispatches_project_create_session_picker_close_to_worktree_module` → `crates/vw-desktop/src/app/message/project/session_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/session_tests.rs` / `handle_dispatches_session_right_click_to_lifecycle_module` → `crates/vw-desktop/src/app/message/project/session_tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/tests.rs` / `update_dispatches_basic_message` → `crates/vw-desktop/src/app/message/project/tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/tests.rs` / `update_dispatches_find_message` → `crates/vw-desktop/src/app/message/project/tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/tests.rs` / `update_dispatches_file_tree_message` → `crates/vw-desktop/src/app/message/project/tests.rs（宿主消息/跨域契约）`
- `crates/vw-desktop/src/app/message/project/tests.rs` / `update_dispatches_session_message` → `crates/vw-desktop/src/app/message/project/tests.rs（宿主消息/跨域契约）`

## 统计与静态核对

- pathlib splitlines 基线口径：21 个独占来源 6251 → 5647 行，净减少 604 行；生产候选 2599 → 2468 行，净减少 131 行。本任务无新增宿主替代文件；新增领域文件单独计入库，不计宿主减量。格式化展开计入真实现状，不以测试减少冒充生产减少。
- 已逐个解析本任务修改的 Rust 源码，核对新模块注册、cfg、宿主字段初始化和直接调用者；领域没有 App、宿主 path/include 或循环依赖。
- git diff --check 仅对本任务路径执行。其他任务的未提交/已暂存改动不纳入本提交。
- 未验证风险：禁止编译和运行，因此类型兼容、实际异步调度、HTTP 请求数量、LSP/窗口/字体绘制未获得运行证据。
- 以后只有用户另行授权才执行：`python3 scripts/cargo_slot.py cargo test -p desktop-workspace --lib host_module_8_tests -- --test-threads=1`；chat-view 分块过滤 `preparation_tasks::tests`；宿主目标检查按公共规则合并执行。本轮未运行这些命令。
