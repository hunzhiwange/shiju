//! 子任务区域的测试，覆盖编辑态与草稿态列表、动作按钮和日志面板。
//! fixture 包含空列表、混合状态及无效时间戳，验证展示回退与禁用分支。
use crate::app::task::{SubTask, Task};
use crate::app::{App, Message};
use iced::advanced::widget::Tree;
use iced::widget::text_editor;
use iced::{Element, Size};

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

fn child_count(element: Element<'_, Message>) -> usize {
    Tree::new(element.as_widget()).children.len()
}

fn task_with_mixed_subtasks() -> Task {
    let mut task = Task::new(8);
    task.id = "task-subtasks".to_string();

    let pending = SubTask::new("pending".to_string());

    let mut running = SubTask::new("running".to_string());
    running.start_execution_at(1_700_000_000_000);

    let mut completed = SubTask::new("completed".to_string());
    completed.start_execution_at(1_700_000_010_000);
    completed.mark_completed_at(1_700_000_015_000);

    let mut failed = SubTask::new("failed".to_string());
    failed.start_execution_at(1_700_000_020_000);
    failed.mark_failed_at(1_700_000_028_000);

    task.subtasks = vec![pending, running, completed, failed];
    task
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("subtask_editor_tests"));
}

#[test]
fn subtask_scrollbar_constants_match_panel_contract() {
    assert_eq!(super::SUBTASK_SCROLLBAR_WIDTH, 4.0);
    assert!(super::SUBTASK_LIST_MAX_HEIGHT > 0.0);
    assert!(super::SUBTASK_SCROLLBAR_GUTTER >= super::SUBTASK_SCROLLBAR_WIDTH);
}

#[test]
fn format_subtask_started_at_formats_epoch_and_invalid_values() {
    assert_eq!(super::format_subtask_started_at(0), "1970-01-01 00:00:00");
    assert_eq!(super::format_subtask_started_at(u64::MAX), "-");
}

#[test]
fn format_subtask_started_at_converts_nonzero_milliseconds_to_utc_seconds() {
    assert_eq!(super::format_subtask_started_at(1_700_000_000_123), "2023-11-14 22:13:20");
}

#[test]
fn edit_mode_subtasks_build_empty_and_mixed_states() {
    let mut app = test_app();
    app.task_board_new_subtask_content = "新增子任务".to_string();

    let empty_task = Task::default();
    let (empty_list, empty_add_section) = super::build_edit_mode_subtasks(&app, &empty_task);
    assert_eq!(child_count(empty_list), 1);
    assert_eq!(child_count(empty_add_section), 3);

    let mixed_task = task_with_mixed_subtasks();
    let (subtasks_view, add_section) = super::build_edit_mode_subtasks(&app, &mixed_task);
    assert_eq!(child_count(subtasks_view), 1);
    assert_eq!(child_count(add_section), 3);
}

#[test]
fn draft_mode_subtasks_build_empty_and_populated_lists() {
    let mut app = test_app();

    assert_eq!(child_count(super::build_draft_mode_subtasks(&app)), 5);

    app.task_board_draft.subtasks = vec!["起草需求".to_string(), "补充测试".to_string()];
    assert_eq!(child_count(super::build_draft_mode_subtasks(&app)), 5);

    app.task_board_draft.subtasks.clear();
    assert_eq!(child_count(super::build_draft_mode_subtasks(&app)), 5);
}

#[test]
fn subtask_action_buttons_cover_enabled_disabled_and_draft_paths() {
    let task = task_with_mixed_subtasks();
    let total = task.subtasks.len();

    keep_element(super::build_up_button(0, &task, true));
    keep_element(super::build_up_button(1, &task, true));
    keep_element(super::build_up_button(1, &task, false));

    keep_element(super::build_down_button(0, total, &task, true));
    keep_element(super::build_down_button(total - 1, total, &task, true));
    keep_element(super::build_down_button(0, total, &task, false));

    keep_element(super::build_delete_button(&task, task.subtasks[0].id.as_str(), true));

    keep_element(super::build_draft_up_button(0));
    keep_element(super::build_draft_up_button(1));
    keep_element(super::build_draft_down_button(0, 2));
    keep_element(super::build_draft_down_button(1, 2));
    keep_element(super::build_draft_delete_button(1));
}

#[test]
fn task_logs_build_root_and_panel_for_context_menu_variants() {
    let mut app = test_app();
    app.current_line_height = 20.0;
    app.task_board_logs_editor = text_editor::Content::with_text(
        "[12:00:00] first line\n/tmp/demo.txt\nhttps://example.com",
    );

    std::hint::black_box(super::build_task_logs(&app));
    assert!(child_count(super::build_task_logs_panel(&app, Size::new(960.0, 420.0))) > 0);

    app.task_board_logs_context_menu_open = true;
    app.task_board_logs_context_menu_pos = Some((24.0, 32.0));
    app.task_board_logs_scroll_top_line = 1.0;
    assert!(child_count(super::build_task_logs_panel(&app, Size::new(640.0, 260.0))) > 0);
}
