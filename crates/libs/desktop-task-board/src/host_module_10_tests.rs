//! 任务看板宿主迁移行为证据。
use crate::draft::import_prompt_template;
use crate::message::TaskBoardSettingsModalTab;
mod bulk_tests;
mod draft_adapter_tests;
mod execution_tests;
mod helpers_tests;

/// 仅持有被测领域字段，不启动桌面 App 或系统资源。
#[derive(Default)]
struct Fixture {
    task_board_worktree_pixel_office: bool,
    task_board_clear_prompt_after_create: bool,
    task_board_close_after_create: bool,
    task_board_close_after_edit: bool,
    task_board_create_modal_open: bool,
    task_board_detail_tab: crate::message::TaskBoardDetailTab,
    task_board_bulk_executor_popover: bool,
    task_board_column_has_vertical_scrollbar: HashMap<desktop_task_model::TaskStatus, bool>,
    task_board_create_submit_success: bool,
    task_board_desc_editor: iced::widget::text_editor::Content,
    task_board_drag_pending: Option<(String, desktop_task_model::TaskStatus, iced::Point)>,
    task_board_dragging: Option<(String, desktop_task_model::TaskStatus)>,
    task_board_edit_submit_success: bool,
    task_board_executor_popover: bool,
    task_board_filter_priority: Option<(u32, u32)>,
    task_board_filter_status: Option<desktop_task_model::TaskStatus>,
    task_board_import_editor: iced::widget::text_editor::Content,
    task_board_import_prompt_collapsed: bool,
    task_board_import_prompt_format: desktop_task_model::TaskImportPromptFormat,
    task_board_is_import_mode: bool,
    task_board_new_subtask_content: String,
    task_board_prompt_editor: iced::widget::text_editor::Content,
    task_board_selected_task: Option<String>,
    task_board_settings_modal_open: bool,
    task_board_settings_modal_tab: TaskBoardSettingsModalTab,
    task_board_sort_ascending: bool,
    task_board_sort_by_priority: bool,

    task_board_tasks_pending_persistence: HashSet<String>,
    task_board_loading: bool,
    task_board_next_scheduler_tick_at_ms: u64,
    task_board_next_auto_review_tick_at_ms: u64,
    task_board_next_auto_merge_tick_at_ms: u64,
    task_board_next_auto_promote_tick_at_ms: u64,
    current_line_height: f32,
    desktop_locale: config_ui::DesktopLocale,
    project_path: Option<String>,
    task_board_bulk_acp_agent: Option<String>,
    task_board_bulk_active_status: Option<desktop_task_model::TaskStatus>,
    task_board_bulk_agent: String,
    task_board_bulk_model_input: String,
    task_board_bulk_priority_input: String,
    task_board_bulk_reasoning_effort: Option<vw_api_chat_identity::ReasoningEffortDto>,
    task_board_bulk_task_split_mode: desktop_task_model::TaskSplitMode,
    task_board_context_menu: Option<(String, f32, f32)>,
    task_board_draft: desktop_task_model::TaskDraft,
    task_board_editing_task_id: Option<String>,
    task_board_executor: desktop_task_executor::executor::state::TaskExecutorState,
    task_board_last_acp_agent: Option<String>,
    task_board_last_log_flush_at_ms: u64,
    task_board_last_model: String,
    task_board_last_worktree_snapshot_at_ms: u64,
    task_board_log_cache: HashMap<String, Vec<desktop_task_model::TaskLogEntry>>,
    task_board_log_scan_cursor: usize,
    task_board_logs_auto_scroll: bool,
    task_board_logs_context_menu_open: bool,
    task_board_logs_context_menu_pos: Option<(f32, f32)>,
    task_board_logs_editor: iced::widget::text_editor::Content,
    task_board_logs_scroll_remainder: f32,
    task_board_logs_scroll_top_line: f32,
    task_board_logs_viewport_height: f32,
    task_board_manual_merge_queue: Vec<String>,
    task_board_selected_tasks: HashSet<String>,
    task_board_settings: desktop_task_model::TaskBoardSettings,
    task_board_tasks: Vec<desktop_task_model::Task>,
    task_board_timeout_scan_cursor: usize,
    task_board_viewing_logs: Option<desktop_task_model::Task>,
    task_board_worktree_action_log_rx:
        Option<std::sync::mpsc::Receiver<desktop_task_executor::TaskLogStream>>,
    task_board_worktree_action_logs: Vec<String>,
    task_board_worktree_action_logs_visible_until_ms: Option<u64>,
    task_board_worktree_desired_capacity: Option<u32>,
    task_board_worktree_maintenance_in_flight: bool,
    task_board_worktree_manual_action_kind: Option<&'static str>,
    task_board_worktree_manual_confirm_kind: Option<&'static str>,
    task_board_worktree_panel_expanded: bool,
    task_board_worktree_snapshot:
        Option<desktop_task_executor::executor::state::WorktreePoolSnapshot>,
    task_board_worktree_snapshot_loading: bool,
}
fn bulk_state(app: &mut Fixture) -> crate::update::bulk::BulkState<'_> {
    crate::update::bulk::BulkState {
        task_board_bulk_active_status: &mut app.task_board_bulk_active_status,
        task_board_bulk_acp_agent: &mut app.task_board_bulk_acp_agent,
        task_board_bulk_agent: &mut app.task_board_bulk_agent,
        task_board_bulk_model_input: &mut app.task_board_bulk_model_input,
        task_board_bulk_priority_input: &mut app.task_board_bulk_priority_input,
        task_board_bulk_reasoning_effort: &mut app.task_board_bulk_reasoning_effort,
        task_board_bulk_task_split_mode: &mut app.task_board_bulk_task_split_mode,
        task_board_context_menu: &mut app.task_board_context_menu,
        task_board_draft: &mut app.task_board_draft,
        task_board_editing_task_id: &mut app.task_board_editing_task_id,
        task_board_last_acp_agent: &mut app.task_board_last_acp_agent,
        task_board_last_model: &mut app.task_board_last_model,
        task_board_manual_merge_queue: &mut app.task_board_manual_merge_queue,
        task_board_selected_tasks: &mut app.task_board_selected_tasks,
        task_board_tasks: &mut app.task_board_tasks,
    }
}
fn worktree_logs_state(app: &mut Fixture) -> crate::helpers::worktree_logs::WorktreeActionLogs<'_> {
    crate::helpers::worktree_logs::WorktreeActionLogs {
        task_board_worktree_manual_action_kind: &mut app.task_board_worktree_manual_action_kind,
        task_board_worktree_manual_confirm_kind: &mut app.task_board_worktree_manual_confirm_kind,
        task_board_worktree_action_logs: &mut app.task_board_worktree_action_logs,
        task_board_worktree_action_logs_visible_until_ms: &mut app
            .task_board_worktree_action_logs_visible_until_ms,
        task_board_worktree_action_log_rx: &mut app.task_board_worktree_action_log_rx,
    }
}

use crate::helpers::execution::*;
use desktop_task_model::{Task, TaskStatus};
fn reset_bulk_operation_inputs(app: &mut Fixture) {
    bulk_state(app).reset_inputs(desktop_task_model::TASK_AGENT_MAIN);
}
fn deactivate_bulk_selection_mode(app: &mut Fixture) {
    bulk_state(app).deactivate();
}
fn manual_merge_running(app: &Fixture) -> bool {
    crate::bulk::manual_merge_running(&app.task_board_tasks, &app.task_board_executor.running_tasks)
}
fn pop_next_manual_merge_task(app: &mut Fixture) -> Option<Task> {
    crate::bulk::pop_next_manual_merge_task(
        &app.task_board_tasks,
        &app.task_board_executor.running_tasks,
        &mut app.task_board_manual_merge_queue,
    )
}
fn start_manual_merge_queue_task(app: &mut Fixture) -> crate::bulk::ManualMergeEffect {
    crate::bulk::prepare_manual_merge(
        &mut app.task_board_tasks,
        &app.task_board_executor.running_tasks,
        &mut app.task_board_manual_merge_queue,
        app.project_path.is_some(),
    )
}
fn apply_execution_timeouts(
    app: &mut Fixture,
    started: web_time::Instant,
) -> (Vec<Task>, Vec<iced::Task<crate::message::TaskBoardMessage>>) {
    crate::helpers::execution::apply_execution_timeouts_and_finish(
        &mut app.task_board_tasks,
        &app.task_board_settings,
        &mut app.task_board_executor,
        &[],
        app.project_path.as_deref(),
        &mut app.task_board_timeout_scan_cursor,
        started,
    )
}
fn pick_next_code_review_task(app: &Fixture, excludes: &[String]) -> Option<String> {
    crate::helpers::execution::pick_next_code_review_task(
        &app.task_board_tasks,
        &app.task_board_settings,
        excludes,
    )
}
fn push_worktree_action_log(app: &mut Fixture, line: String) {
    crate::helpers::worktree_logs::push_worktree_action_log(&mut worktree_logs_state(app), line);
}
fn clear_worktree_action_logs(app: &mut Fixture) {
    crate::helpers::worktree_logs::clear_worktree_action_logs(&mut worktree_logs_state(app));
}
fn finish_worktree_action_logs(app: &mut Fixture) {
    crate::helpers::worktree_logs::finish_worktree_action_logs(&mut worktree_logs_state(app));
}
fn poll_worktree_action_logs(app: &mut Fixture) {
    crate::helpers::worktree_logs::poll_worktree_action_logs(&mut worktree_logs_state(app));
}
fn worktree_snapshot_refresh_interval_ms(app: &Fixture) -> u64 {
    crate::update::worktree::refresh_interval(app.task_board_worktree_panel_expanded)
}
fn maybe_schedule_worktree_snapshot_refresh(app: &mut Fixture, force: bool) -> bool {
    crate::update::worktree::SnapshotState {
        snapshot: &mut app.task_board_worktree_snapshot,
        loading: &mut app.task_board_worktree_snapshot_loading,
        last_refresh: &mut app.task_board_last_worktree_snapshot_at_ms,
    }
    .request(
        app.project_path.as_deref(),
        app.task_board_worktree_panel_expanded,
        force,
        runtime_time::now_ms(),
    )
}
fn schedule_worktree_pool_maintenance(app: &mut Fixture, _: &str, _: usize) -> bool {
    crate::update::worktree::begin_maintenance(
        &mut app.task_board_worktree_maintenance_in_flight,
        app.task_board_worktree_manual_action_kind,
        app.task_board_worktree_desired_capacity,
        app.task_board_settings.max_concurrent,
    )
    .is_some()
}
fn worktree_snapshot_from_gateway(
    dto: vw_api_task::TaskPoolWorktreeSnapshotResponse,
) -> desktop_task_executor::executor::state::WorktreePoolSnapshot {
    desktop_task_executor::executor::state::WorktreePoolSnapshot::from_gateway_response(dto)
}

fn flush_running_task_logs(app: &mut Fixture, started: web_time::Instant) {
    let mut logs = crate::helpers::logs::TaskLogsState {
        task_board_logs_editor: &mut app.task_board_logs_editor,
        task_board_logs_auto_scroll: &mut app.task_board_logs_auto_scroll,
        task_board_logs_scroll_top_line: &mut app.task_board_logs_scroll_top_line,
        task_board_logs_scroll_remainder: &mut app.task_board_logs_scroll_remainder,
        task_board_logs_viewport_height: &mut app.task_board_logs_viewport_height,
        current_line_height: &mut app.current_line_height,
        desktop_locale: &mut app.desktop_locale,
        task_board_log_cache: &mut app.task_board_log_cache,
        task_board_tasks: &mut app.task_board_tasks,
        task_board_viewing_logs: &mut app.task_board_viewing_logs,
        task_board_logs_context_menu_open: &mut app.task_board_logs_context_menu_open,
        task_board_logs_context_menu_pos: &mut app.task_board_logs_context_menu_pos,
    };
    crate::helpers::execution::flush_running_task_logs(
        &mut logs,
        &mut app.task_board_executor,
        &mut app.task_board_log_scan_cursor,
        started,
        |_| {},
    );
}
fn flush_running_task_logs_throttled(app: &mut Fixture, force: bool, started: web_time::Instant) {
    let mut logs = crate::helpers::logs::TaskLogsState {
        task_board_logs_editor: &mut app.task_board_logs_editor,
        task_board_logs_auto_scroll: &mut app.task_board_logs_auto_scroll,
        task_board_logs_scroll_top_line: &mut app.task_board_logs_scroll_top_line,
        task_board_logs_scroll_remainder: &mut app.task_board_logs_scroll_remainder,
        task_board_logs_viewport_height: &mut app.task_board_logs_viewport_height,
        current_line_height: &mut app.current_line_height,
        desktop_locale: &mut app.desktop_locale,
        task_board_log_cache: &mut app.task_board_log_cache,
        task_board_tasks: &mut app.task_board_tasks,
        task_board_viewing_logs: &mut app.task_board_viewing_logs,
        task_board_logs_context_menu_open: &mut app.task_board_logs_context_menu_open,
        task_board_logs_context_menu_pos: &mut app.task_board_logs_context_menu_pos,
    };
    crate::helpers::execution::flush_running_task_logs_throttled(
        &mut logs,
        &mut app.task_board_executor,
        &mut app.task_board_log_scan_cursor,
        &mut app.task_board_last_log_flush_at_ms,
        force,
        started,
        |_| {},
    );
}

mod settings_tests;
mod update_tests;
use crate::helpers::settings::now_ms;
use std::collections::{HashMap, HashSet};
fn fixture_pair() -> (Fixture, ()) {
    (Fixture::default(), ())
}
fn projection_state(app: &mut Fixture) -> crate::update::projection::BoardProjection<'_> {
    crate::update::projection::BoardProjection {
        logs: crate::helpers::logs::TaskLogsState {
            task_board_logs_editor: &mut app.task_board_logs_editor,
            task_board_logs_auto_scroll: &mut app.task_board_logs_auto_scroll,
            task_board_logs_scroll_top_line: &mut app.task_board_logs_scroll_top_line,
            task_board_logs_scroll_remainder: &mut app.task_board_logs_scroll_remainder,
            task_board_logs_viewport_height: &mut app.task_board_logs_viewport_height,
            current_line_height: &mut app.current_line_height,
            desktop_locale: &mut app.desktop_locale,
            task_board_log_cache: &mut app.task_board_log_cache,
            task_board_tasks: &mut app.task_board_tasks,
            task_board_viewing_logs: &mut app.task_board_viewing_logs,
            task_board_logs_context_menu_open: &mut app.task_board_logs_context_menu_open,
            task_board_logs_context_menu_pos: &mut app.task_board_logs_context_menu_pos,
        },
        executor: &mut app.task_board_executor,
        pending: &mut app.task_board_tasks_pending_persistence,
        selected: &mut app.task_board_selected_tasks,
        loading: &mut app.task_board_loading,
        settings: &mut app.task_board_settings,
    }
}

fn local_state(app: &mut Fixture) -> crate::update::local::LocalState<'_> {
    crate::update::local::LocalState {
        worktree_panel_expanded: &mut app.task_board_worktree_panel_expanded,
        worktree_pixel_office: &mut app.task_board_worktree_pixel_office,
        project_path: app.project_path.as_deref(),
        clear_prompt_after_create: &mut app.task_board_clear_prompt_after_create,
        close_after_create: &mut app.task_board_close_after_create,
        close_after_edit: &mut app.task_board_close_after_edit,
        settings: &app.task_board_settings,
        last_model: &mut app.task_board_last_model,
        create_modal_open: &mut app.task_board_create_modal_open,
        tasks: &mut app.task_board_tasks,
        logs_auto_scroll: &mut app.task_board_logs_auto_scroll,
        editing_task_id: &mut app.task_board_editing_task_id,
        detail_tab: &mut app.task_board_detail_tab,
        bulk_acp_agent: &mut app.task_board_bulk_acp_agent,
        bulk_agent: &mut app.task_board_bulk_agent,
        bulk_executor_popover: &mut app.task_board_bulk_executor_popover,
        bulk_priority_input: &mut app.task_board_bulk_priority_input,
        bulk_reasoning_effort: &mut app.task_board_bulk_reasoning_effort,
        bulk_task_split_mode: &mut app.task_board_bulk_task_split_mode,
        column_has_vertical_scrollbar: &mut app.task_board_column_has_vertical_scrollbar,
        context_menu: &mut app.task_board_context_menu,
        create_submit_success: &mut app.task_board_create_submit_success,
        desc_editor: &mut app.task_board_desc_editor,
        draft: &mut app.task_board_draft,
        drag_pending: &mut app.task_board_drag_pending,
        dragging: &mut app.task_board_dragging,
        edit_submit_success: &mut app.task_board_edit_submit_success,
        executor_popover: &mut app.task_board_executor_popover,
        filter_priority: &mut app.task_board_filter_priority,
        filter_status: &mut app.task_board_filter_status,
        import_editor: &mut app.task_board_import_editor,
        import_prompt_collapsed: &mut app.task_board_import_prompt_collapsed,
        import_prompt_format: &mut app.task_board_import_prompt_format,
        is_import_mode: &mut app.task_board_is_import_mode,
        last_acp_agent: &mut app.task_board_last_acp_agent,
        logs_context_menu_open: &mut app.task_board_logs_context_menu_open,
        logs_context_menu_pos: &mut app.task_board_logs_context_menu_pos,
        new_subtask_content: &mut app.task_board_new_subtask_content,
        prompt_editor: &mut app.task_board_prompt_editor,
        selected_task: &mut app.task_board_selected_task,
        selected_tasks: &mut app.task_board_selected_tasks,
        settings_modal_open: &mut app.task_board_settings_modal_open,
        settings_modal_tab: &mut app.task_board_settings_modal_tab,
        sort_ascending: &mut app.task_board_sort_ascending,
        sort_by_priority: &mut app.task_board_sort_by_priority,
        worktree_manual_confirm_kind: &mut app.task_board_worktree_manual_confirm_kind,
    }
}

mod local_update_tests;
