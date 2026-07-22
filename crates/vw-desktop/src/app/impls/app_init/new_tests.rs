use super::*;

// Tests for plan6 task 850.
const SOURCE: &str = include_str!("new.rs");

fn source_declares_symbol(name: &str) -> bool {
    let needles = [
        format!("fn {name}"),
        format!("pub fn {name}"),
        format!("struct {name}"),
        format!("pub struct {name}"),
        format!("enum {name}"),
        format!("pub enum {name}"),
        format!("type {name}"),
        format!("pub type {name}"),
        format!("const {name}"),
        format!("pub const {name}"),
        format!("static {name}"),
        format!("pub static {name}"),
        format!("impl {name}"),
    ];

    needles.iter().any(|needle| SOURCE.contains(needle))
}

#[test]
fn new_tests_keeps_planned_coverage_targets() {
    for name in ["new"] {
        assert!(source_declares_symbol(name), "expected source to declare coverage target {name}");
    }
}

#[test]
fn new_initializes_safe_empty_session_state() {
    let (app, _task) = App::new();

    assert_eq!(app.screen, Screen::Home);
    assert_eq!(app.project_path, None);
    assert_eq!(app.project_id, None);
    assert_eq!(app.active_session_id, None);
    assert!(app.chat.is_empty());
    assert!(app.chat_message_ids.is_empty());
    assert!(app.sessions.is_empty());
    assert!(!app.is_requesting);
    assert!(app.queue.is_empty());
    assert!(app.chat_auto_scroll);
    assert_eq!(app.active_tab_id.as_deref(), Some("home"));
    assert_eq!(app.open_tabs.len(), 1);
    assert_eq!(app.open_tabs[0].id, "home");
    assert_eq!(app.open_tabs[0].screen, Screen::Home);
    assert_eq!(app.open_tabs[0].project_path, None);
}

#[test]
fn new_returns_startup_work_for_runtime_initialization() {
    let (_app, task) = App::new();

    assert!(iced_runtime::task::into_stream(task.collect()).is_some());
}

#[test]
fn new_keeps_runtime_defaults_consistent() {
    let (app, _task) = App::new();

    let empty_runtime = app
        .session_runtime_states
        .get("__empty__")
        .expect("new app should seed an empty runtime state");
    assert_eq!(empty_runtime.model, app.model);
    assert_eq!(empty_runtime.auto_model, app.auto_model);
    assert_eq!(empty_runtime.acp_agent, app.acp_agent);
}

#[test]
fn new_initializes_layout_panels_and_overlay_state() {
    let (app, _task) = App::new();

    assert_eq!(app.split_ratio, 0.6);
    assert!(!app.dragging_split);
    assert_eq!(app.window_size, (1200.0, 800.0));
    assert!(app.file_manager_width >= 180.0);
    assert!(app.file_manager_width <= 600.0);
    assert!(!app.show_model_popover);
    assert!(!app.show_file_popover);
    assert!(!app.show_usage_popover);
    assert!(app.chat_context_menu_target.is_none());
    assert!(app.input_context_menu_pos.is_none());
    assert!(app.tool_detail_dialog.is_none());
}

#[test]
fn new_initializes_project_edit_and_new_session_state() {
    let (app, _task) = App::new();

    assert!(app.project_sessions.is_empty());
    assert!(app.project_sessions_loading.is_empty());
    assert!(app.project_session_load_counts.is_empty());
    assert!(app.new_session_picker_project.is_none());
    assert!(app.new_session_picker_options.is_empty());
    assert!(app.new_session_worktree_name.is_empty());
    assert!(app.new_session_confirm_delete_directory.is_none());
    assert!(app.project_edit_path.is_none());
    assert_eq!(app.project_edit_tab, crate::app::state::ProjectEditTab::General);
    assert!(app.project_edit_name.is_empty());
    assert!(app.project_edit_start_script.is_empty());
}

#[test]
fn new_initializes_notifications_and_task_board_state() {
    let (app, _task) = App::new();

    assert!(app.notifications.is_empty());
    assert!(!app.notifications_expanded);
    assert_eq!(app.next_notification_id, 0);
    assert!(app.notification_editors.is_empty());
    assert!(app.active_toast.is_none());
    assert_eq!(app.next_toast_id, 0);
    assert!(!app.show_task_board);
    assert!(!app.task_board_loading);
}

#[test]
fn new_initializes_chat_caches_streaming_flags_and_todo_panel() {
    let (app, _task) = App::new();

    assert!(app.chat_render_cache.is_empty());
    assert!(app.chat_visible_text_cache.is_empty());
    assert!(app.chat_copy_hash_cache.is_empty());
    assert!(app.chat_message_expanded.is_empty());
    assert!(app.chat_message_estimated_heights.is_empty());
    assert!(app.chat_message_measured_heights.is_empty());
    assert!(app.chat_special_text_editors.is_empty());
    assert!(app.chat_markdown_blocks.is_empty());
    assert!(app.chat_markdown_block_editors.is_empty());
    assert!(app.chat_tool_text_editors.is_empty());
    assert!(app.chat_think_editors.is_empty());
    assert!(app.chat_think_expanded.is_empty());
    assert!(app.chat_think_collapsed.is_empty());
    assert!(app.chat_tool_file_expanded.is_empty());
    assert!(app.chat_tool_expanded.is_empty());
    assert!(app.chat_explore_expanded.is_empty());
    assert!(app.chat_explore_summary_animations.is_empty());
    assert!(app.chat_think_scroll_ids.is_empty());
    assert_eq!(app.chat_stream_think_msg_idx, None);
    assert_eq!(app.chat_stream_think_count, 0);
    assert_eq!(app.chat_stream_think_open_idx, None);
    assert_eq!(app.chat_context_menu_text, "");
    assert!(!app.input_context_menu_open);
    assert_eq!(app.chat_reset_menu_idx, None);
    assert_eq!(app.chat_fork_dialog_idx, None);
    assert!(app.chat_todo_expanded);
    assert_eq!(app.chat_todo_anim, 1.0);
    assert_eq!(app.chat_todo_placement, TodoPanelPlacement::ChatTopRight);
    assert_eq!(app.chat_todo_session_id, None);
    assert!(app.chat_todo_items.is_empty());
}

#[test]
fn new_initializes_search_preview_and_file_tree_state() {
    let (app, _task) = App::new();

    assert_eq!(app.search_text, "");
    assert!(!app.show_search_overlay);
    assert!(app.file_index_cache.is_empty());
    assert_eq!(app.file_index_revision, 0);
    assert!(app.file_tree_model_cache.is_empty());
    assert_eq!(app.search_panel_file_cache_query, "");
    assert_eq!(app.search_panel_file_cache_project_path, None);
    assert_eq!(app.search_panel_file_cache_revision, 0);
    assert!(app.search_panel_file_cache_results.is_empty());
    assert_eq!(app.file_search_cache_query, "");
    assert_eq!(app.file_search_cache_project_path, None);
    assert_eq!(app.file_search_cache_revision, 0);
    assert!(app.file_search_cache_entries.is_empty());
    assert!(app.file_tree_expanded_set.is_empty());
    assert!(app.file_tree_clipboard.is_none());
    assert!(app.dragging_file_paths.is_empty());
    assert_eq!(app.dragging_file_position, None);
    assert!(app.pending_drop_file_paths.is_empty());
    assert_eq!(app.pending_drop_file_position, None);
    assert!(!app.input_drop_hovered);
    assert_eq!(app.file_tree_rename_path, None);
    assert_eq!(app.file_tree_rename_value, "");
    assert_eq!(app.file_search_query, "");
    assert!(!app.show_file_search);
    assert_eq!(app.file_search_anchor, None);
    assert_eq!(app.file_search_selected_index, 0);
    assert!(app.find_results_tabs.is_empty());
    assert_eq!(app.active_find_results_tab_id, None);
    assert_eq!(app.tool_files_filter, "");
    assert_eq!(app.file_ref_hovered_index, None);
    assert!(app.preview_tabs.is_empty());
    assert_eq!(app.active_preview_path, None);
    assert!(app.project_preview_tabs.is_empty());
    assert!(app.project_preview_active_path.is_empty());
    assert_eq!(app.pending_preview_goto, None);
    assert!(app.preview_trace_back.is_empty());
    assert!(app.preview_trace_forward.is_empty());
    assert!(!app.preview_trace_navigating);
    assert_eq!(app.previous_preview_path, None);
    assert_eq!(app.focus_area, FocusArea::None);
    assert!(!app.show_preview_context_menu);
    assert_eq!(app.preview_context_target, None);
    assert_eq!(app.preview_context_menu_pos, None);
    assert_eq!(app.preview_nav_popup, None);
    assert_eq!(app.error_message, None);
}

#[test]
fn new_initializes_settings_panels_and_gateway_drafts_idle() {
    let (app, _task) = App::new();

    assert_eq!(app.settings_tab, SettingsTab::Sessions);
    assert_eq!(app.system_settings_query, "");
    assert_eq!(app.system_settings_help_tab, None);
    assert_eq!(app.provider_settings.save_error, None);
    assert_eq!(app.model_settings.save_error, None);
    assert!(!app.embedding_routes_settings.save_success);
    assert_eq!(app.query_classification_settings.save_error, None);
    assert!(!app.cron_settings.jobs_loading);
    assert!(app.cron_settings.jobs.is_empty());
    assert!(app.cron_settings.selected_job_ids.is_empty());
    assert_eq!(app.cron_settings.editing_job_id, None);
    assert_eq!(app.cron_settings.runs_modal_job_id, None);
    assert!(!app.cron_settings.runs_modal_loading);
    assert!(app.cron_settings.runs_modal.is_empty());
    assert_eq!(app.scheduler_settings.save_error, None);
    assert_eq!(app.hooks_settings.save_error, None);
    assert_eq!(app.runtime_settings.save_error, None);
    assert_eq!(app.skills_settings.query, "");
    assert!(!app.skills_settings.loading);
    assert!(app.skills_settings.catalog.is_empty());
    assert!(app.skills_settings.plugin_catalog.is_empty());
    assert_eq!(app.skills_settings.selected_plugin_id, None);
    assert_eq!(app.skills_settings.selected_skill_id, None);
    assert_eq!(app.web_search_settings.save_error, None);
    assert_eq!(app.browser_settings.save_error, None);
    assert_eq!(app.gateway_settings.active_tab, crate::app::state::GatewaySettingsTab::Config);
    assert_eq!(app.gateway_settings.new_skey_name_input, "");
    assert_eq!(app.gateway_settings.new_skey_expires_at_input, "");
    assert_eq!(app.gateway_settings.service_action_running, None);
    assert_eq!(app.gateway_settings.service_action_output, None);
    assert_eq!(app.gateway_client_settings.pending_remove_server_id, None);
    assert!(app.gateway_client_settings.health.is_empty());
    assert_eq!(app.agents_settings.new_agent_key_input, "");
    assert!(!app.agents_settings.loading);
    assert_eq!(app.agents_settings.selected_agent, crate::app::state::MAIN_AGENT_KEY);
    assert_eq!(app.coordination_settings.save_error, None);
}

#[test]
fn new_initializes_git_terminal_and_window_controls() {
    let (app, _task) = App::new();

    assert!(app.expanded_hunks.is_empty());
    assert!(app.context_expansions.is_empty());
    assert_eq!(app.git_commit_message, "");
    assert_eq!(app.git_commit_type, Some(ConventionalCommitType::Feat));
    assert_eq!(app.git_commit_scope, "");
    assert_eq!(app.git_commit_description, "");
    assert!(!app.git_commit_in_progress);
    assert!(!app.show_git_commit_help_modal);
    assert!(!app.show_git_filter_help_modal);
    assert!(app.staged_files_selected.is_empty());
    assert!(app.staged_hunks_selected.is_empty());
    assert!(app.staged_lines_selected.is_empty());
    assert!(app.staged_old_lines_selected.is_empty());
    assert!(app.git_diff_selected_lines.is_empty());
    assert!(app.git_diff_drag_range.is_none());
    assert!(app.git_diff_selected_range.is_none());
    assert!(!app.git_diff_dragging);
    assert!(app.git_diff_comment_draft.is_none());
    assert!(app.git_diff_context_menu.is_none());
    assert!(app.git_diff_file_menu.is_none());
    assert!(!app.show_git_copy_modal);
    assert!(app.git_copy_modal_use_color);
    assert!(!app.show_git_custom_diff_modal);
    assert!(!app.git_custom_diff_hide_inputs);
    assert_eq!(app.git_custom_diff_title, "");
    assert!(!app.show_git_filter_options);
    assert_eq!(app.git_filter_query, "");
    assert!(!app.git_filter_included);
    assert!(!app.git_filter_excluded);
    assert!(!app.git_filter_new);
    assert!(!app.git_filter_modified);
    assert!(!app.git_filter_deleted);
    assert!(app.git_changed_files.is_empty());
    assert!(!app.git_changed_files_loading);
    assert_eq!(app.git_changed_files_repo_path, None);
    assert!(app.git_diff_contents.is_empty());
    assert!(app.terminals_by_project.is_empty());
    assert!(app.show_diff);
    assert_eq!(app.diff_theme, DiffTheme::GitHub);
    assert_eq!(app.window_position, (0.0, 0.0));
    assert!(app.main_window_id.is_none());
    assert!(app.task_pet_window_id.is_none());
}

#[test]
fn new_initializes_task_pet_and_task_board_executor_contracts() {
    let (app, _task) = App::new();

    assert_eq!(app.task_pet_position, iced::Point::new(620.0, 96.0));
    assert!(app.task_pet_collapsed);
    assert_eq!(app.task_pet_expand_progress, 0.0);
    assert_eq!(app.task_pet_expand_target, None);
    assert!(!app.task_pet_dragging);
    assert_eq!(app.task_pet_drag_anchor, None);
    assert_eq!(app.task_pet_drag_start, iced::Point::ORIGIN);
    assert_eq!(app.task_pet_drag_direction, 1);
    assert_eq!(app.task_pet_walk_until_ms, 0);
    assert_eq!(app.task_pet_avatar_kind, crate::app::state::TaskPetAvatarKind::Robot);
    assert!(!app.task_pet_robot_hovered);
    assert!(app.task_pet_items.is_empty());
    assert_eq!(app.task_pet_hovered_request_id, None);
    assert_eq!(app.task_pet_reply_request_id, None);
    assert_eq!(app.task_pet_reply_input, "");
    assert!(app.task_pet_dismissed_request_ids.is_empty());

    assert!(app.task_board_executor.can_start_more());
    assert!(app.task_board_executor.running_tasks.is_empty());
    assert_eq!(app.task_board_executor.max_concurrent, 3);
    assert_eq!(app.task_board_executor.simulation_delay_ms, 2000);
    assert!(app.task_board_executor.log_receivers.is_empty());
    assert!(!app.task_board_executor_running);
    assert_eq!(
        app.task_board_next_scheduler_tick_at_ms + 59_000,
        app.task_board_next_refresh_at_ms
    );
    assert_eq!(
        app.task_board_next_auto_merge_tick_at_ms + 20_000,
        app.task_board_next_auto_review_tick_at_ms
    );
    assert_eq!(
        app.task_board_next_auto_review_tick_at_ms,
        app.task_board_next_auto_promote_tick_at_ms
    );
    assert_eq!(app.task_board_worktree_desired_capacity, None);
    assert_eq!(app.task_board_matrix_event_cursor, 0);
    assert!(app.task_board_processed_matrix_event_ids.is_empty());
    assert_eq!(app.task_board_last_log_flush_at_ms, 0);
    assert_eq!(app.task_board_log_scan_cursor, 0);
    assert_eq!(app.task_board_timeout_scan_cursor, 0);
    assert_eq!(app.task_board_schedule_scan_cursor, 0);
}

#[test]
fn new_initializes_task_board_editing_import_and_worktree_ui_state() {
    let (app, _task) = App::new();

    assert_eq!(app.task_board_view_mode, crate::app::state::TaskBoardViewMode::Kanban);
    assert!(app.task_board_tasks.is_empty());
    assert_eq!(app.task_board_staff_config_workflow_id, None);
    assert_eq!(app.task_board_staff_config_load_error, None);
    assert!(app.task_board_staff_capability_catalog.is_empty());
    assert_eq!(app.task_board_staff_capability_catalog_workflow_id, None);
    assert!(app.task_board_staff_capability_draft.is_empty());
    assert!(!app.task_board_staff_capability_catalog_loading);
    assert!(!app.task_board_staff_capability_saving);
    assert_eq!(app.task_board_staff_capability_catalog_error, None);
    assert!(app.task_board_staff_events.is_empty());
    assert_eq!(app.task_board_staff_feed_tab, crate::app::state::TaskBoardStaffFeedTab::Tasks);
    assert!(app.task_board_staff_task_status_filter.is_empty());
    assert_eq!(app.task_board_staff_command_input, "");
    assert_eq!(app.task_board_staff_filter_member_id, None);
    assert!(!app.task_board_staff_assignment_open);
    assert!(app.task_board_staff_assignment_draft.is_none());
    assert!(app.task_board_staff_workflow_binding_draft.is_none());
    assert!(app.task_board_staff_stage_action_draft.is_none());
    assert!(!app.task_board_staff_stage_action_reset_confirm);
    assert!(app.task_board_staff_workflow_options.is_empty());
    assert_eq!(app.task_board_staff_workflow_options_error, None);
    assert!(app.task_board_staff_assignment_tabs.is_empty());
    assert_eq!(
        app.task_board_staff_orchestration_tab,
        crate::app::state::TaskBoardStaffOrchestrationTab::Dispatch
    );
    assert!(!app.task_board_create_modal_open);
    assert!(!app.task_board_settings_modal_open);
    assert_eq!(
        app.task_board_settings_modal_tab,
        crate::app::state::TaskBoardSettingsModalTab::Refresh
    );
    assert!(!app.task_board_is_import_mode);
    assert_eq!(app.task_board_import_editor.text(), "");
    assert_eq!(app.task_board_import_prompt_format, crate::app::task::TaskImportPromptFormat::Json);
    assert!(app.task_board_import_prompt_collapsed);
    assert!(app.task_board_column_has_vertical_scrollbar.is_empty());
    assert_eq!(app.task_board_last_model, "auto");
    assert!(app.task_board_selected_tasks.is_empty());
    assert_eq!(app.task_board_bulk_active_status, None);
    assert_eq!(app.task_board_bulk_priority_input, "999");
    assert_eq!(app.task_board_bulk_model_input, "auto");
    assert_eq!(app.task_board_bulk_agent, crate::app::task::TASK_AGENT_MAIN);
    assert_eq!(app.task_board_bulk_task_split_mode, crate::app::task::TaskSplitMode::FollowSystem);
    assert!(app.task_board_manual_merge_queue.is_empty());
    assert_eq!(app.task_board_selected_task, None);
    assert!(app.task_board_viewing_logs.is_none());
    assert_eq!(app.task_board_detail_tab, crate::app::state::TaskBoardDetailTab::Basic);
    assert!(app.task_board_log_cache.is_empty());
    assert_eq!(app.task_board_editing_task_id, None);
    assert!(app.task_board_dragging.is_none());
    assert!(app.task_board_drag_pending.is_none());
    assert_eq!(app.task_board_filter_status, None);
    assert_eq!(app.task_board_filter_priority, None);
    assert!(app.task_board_sort_by_priority);
    assert!(!app.task_board_sort_ascending);
    assert!(!app.task_board_executor_popover);
    assert!(!app.task_board_bulk_executor_popover);
    assert!(!app.task_board_bulk_model_popover);
    assert!(!app.task_board_worktree_maintenance_in_flight);
    assert!(!app.task_board_worktree_snapshot_loading);
    assert_eq!(app.task_board_worktree_manual_action_kind, None);
    assert_eq!(app.task_board_worktree_manual_confirm_kind, None);
    assert!(app.task_board_worktree_action_logs.is_empty());
    assert_eq!(app.task_board_worktree_action_logs_visible_until_ms, None);
    assert!(app.task_board_worktree_snapshot.is_none());
    assert_eq!(app.task_board_last_worktree_snapshot_at_ms, 0);
    assert!(!app.task_board_worktree_panel_expanded);
    assert_eq!(app.task_board_context_menu, None);
    assert_eq!(app.task_board_new_subtask_content, "");
    assert_eq!(app.task_board_desc_editor.text(), "");
    assert_eq!(app.task_board_prompt_editor.text(), "");
    assert_eq!(app.task_board_logs_editor.text(), "");
    assert!(app.task_board_logs_auto_scroll);
    assert!(!app.task_board_logs_context_menu_open);
    assert_eq!(app.task_board_logs_context_menu_pos, None);
    assert_eq!(app.task_board_logs_scroll_top_line, 0.0);
    assert_eq!(app.task_board_logs_scroll_remainder, 0.0);
    assert_eq!(app.task_board_logs_viewport_height, 0.0);
    assert!(!app.task_board_model_popover);
    assert!(app.task_board_clear_prompt_after_create);
    assert!(app.task_board_close_after_create);
    assert!(app.task_board_close_after_edit);
    assert!(!app.task_board_create_submit_success);
    assert!(!app.task_board_edit_submit_success);
}
