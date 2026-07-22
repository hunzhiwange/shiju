use iced::widget::button;
use iced::{Background, Color, Element, Length, Theme};

use crate::app::message::TaskBoardMessage;
use crate::app::task::{SubTask, SubTaskStatus, Task, TaskLogEntry};
use crate::app::{App, Message};

use super::*;

const EPSILON: f32 = 0.000_01;

fn assert_close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= EPSILON, "expected {actual} to be close to {expected}");
}

fn assert_color_close(actual: Color, expected: Color) {
    assert_close(actual.r, expected.r);
    assert_close(actual.g, expected.g);
    assert_close(actual.b, expected.b);
    assert_close(actual.a, expected.a);
}

fn color_background(background: Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => color,
        other => panic!("expected color background, got {other:?}"),
    }
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

fn task_with_logs() -> Task {
    let mut task = Task { id: "task-logs".to_string(), ..Task::default() };
    task.logs = vec![
        TaskLogEntry {
            timestamp_ms: 61_000,
            status_from: None,
            status_to: None,
            message: "started".to_string(),
        },
        TaskLogEntry {
            timestamp_ms: 125_999,
            status_from: None,
            status_to: None,
            message: "finished".to_string(),
        },
    ];
    task
}

fn subtask(id: &str, content: &str, status: SubTaskStatus, created_at_ms: u64) -> SubTask {
    let mut subtask = SubTask::new(content.to_string());
    subtask.id = id.to_string();
    subtask.status = status;
    subtask.created_at_ms = created_at_ms;
    subtask
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("logs_modal_tests"));
}

#[test]
fn task_logs_modal_builds_public_and_content_entries() {
    let (mut app, _task) = App::new();
    app.task_board_new_subtask_content = "下一步".to_string();

    let mut task = task_with_logs();
    task.subtasks = vec![
        subtask("sub-1", "准备", SubTaskStatus::Pending, 0),
        subtask("sub-2", "执行", SubTaskStatus::Running, 3_600_000),
        subtask("sub-3", "验收", SubTaskStatus::Completed, 7_200_000),
    ];

    let public_modal = build_task_logs_modal(&app, &task);
    assert_eq!(public_modal.as_widget().size().width, Length::FillPortion(1));
    assert_eq!(public_modal.as_widget().size().height, Length::Fill);
    keep_element(public_modal);

    let content_modal = build_task_logs_modal_content("另一条子任务", &task);
    assert_eq!(content_modal.as_widget().size().width, Length::FillPortion(1));
    assert_eq!(content_modal.as_widget().size().height, Length::Fill);
    keep_element(content_modal);
}

#[test]
fn task_logs_modal_omits_subtask_section_when_empty() {
    let task = task_with_logs();

    let modal = build_task_logs_modal_content("", &task);

    assert_eq!(modal.as_widget().size().width, Length::FillPortion(1));
    keep_element(modal);
}

#[test]
fn subtask_section_covers_move_button_edges() {
    let mut task = Task { id: "task-subtasks".to_string(), ..Task::default() };
    task.subtasks = vec![
        subtask("first", "first item", SubTaskStatus::Pending, 0),
        subtask("middle", "middle item", SubTaskStatus::Running, 60_000),
        subtask("last", "last item", SubTaskStatus::Failed, 120_000),
    ];

    let column: Element<'_, Message> =
        build_subtasks_section(&task, iced::widget::column![]).into();

    assert_eq!(column.as_widget().children().len(), 4);
}

#[test]
fn add_subtask_section_uses_current_input_value_for_messages() {
    let task = Task { id: "task-add".to_string(), ..Task::default() };

    let column: Element<'_, Message> =
        build_add_subtask_section("新的子任务", &task, iced::widget::column![]).into();
    assert_eq!(column.as_widget().children().len(), 4);

    match update_new_subtask_content_message("typed".to_string()) {
        Message::TaskBoard(TaskBoardMessage::UpdateNewSubtaskContent(content)) => {
            assert_eq!(content, "typed");
        }
        other => panic!("unexpected message: {other:?}"),
    }

    match add_subtask_message("task-add".to_string(), "新的子任务".to_string()) {
        Message::TaskBoard(TaskBoardMessage::AddSubTask { task_id, content }) => {
            assert_eq!(task_id, "task-add");
            assert_eq!(content, "新的子任务");
        }
        other => panic!("unexpected message: {other:?}"),
    }
}

#[test]
fn logs_section_formats_empty_and_multi_line_logs() {
    let empty = Task { id: "empty".to_string(), ..Task::default() };
    assert_eq!(format_logs_content(&empty), "");

    let task = task_with_logs();
    assert_eq!(format_logs_content(&task), "1:1 - started\n2:5 - finished");

    let column: Element<'_, Message> = build_logs_section(&task, iced::widget::column![]).into();
    assert_eq!(column.as_widget().children().len(), 3);
}

#[test]
fn helper_formatters_preserve_existing_time_contracts() {
    let log = TaskLogEntry {
        timestamp_ms: 3_661_999,
        status_from: None,
        status_to: None,
        message: "one hour plus".to_string(),
    };

    assert_eq!(format_log_entry(&log), "61:1 - one hour plus");
    assert_eq!(format_subtask_created_date(0), "1970-1 0:0");
    assert_eq!(format_subtask_created_date(86_400_000 * 40 + 3_660_000), "1970-2 1:1");
}

#[test]
fn log_formatter_handles_minute_boundary_and_preserves_message_text() {
    let before_minute = TaskLogEntry {
        timestamp_ms: 59_999,
        status_from: None,
        status_to: None,
        message: "line one\nline two".to_string(),
    };
    let at_minute = TaskLogEntry { timestamp_ms: 60_000, ..before_minute.clone() };

    assert_eq!(format_log_entry(&before_minute), "0:59 - line one\nline two");
    assert_eq!(format_log_entry(&at_minute), "1:0 - line one\nline two");
}

#[test]
fn style_helpers_cover_modal_log_and_disabled_arrow_surfaces() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();

    let id_text = task_id_text_style(&theme);
    assert_eq!(id_text.color, Some(palette.background.base.text));

    let created_at = subtask_created_at_text_style(&Theme::Light);
    assert_color_close(
        created_at.color.expect("created-at color"),
        Color::from_rgb8(156, 163, 175),
    );

    let disabled_arrow = disabled_arrow_button_style(&theme, button::Status::Hovered);
    assert!(disabled_arrow.background.is_none());
    assert_close(disabled_arrow.border.radius.top_left, 4.0);
    assert_close(disabled_arrow.border.width, 0.0);
    assert_color_close(disabled_arrow.border.color, Color::TRANSPARENT);
    assert_color_close(disabled_arrow.text_color, palette.background.weak.text);

    let logs = logs_container_style(&theme);
    assert_color_close(
        color_background(logs.background),
        palette.background.weak.color.scale_alpha(0.35),
    );
    assert_close(logs.border.radius.top_left, 8.0);
    assert_close(logs.border.width, 1.0);
    assert_color_close(logs.border.color, palette.background.strong.color.scale_alpha(0.4));

    let modal = modal_panel_style(&Theme::Light);
    assert_close(modal.shadow.blur_radius, 16.0);
    assert_eq!(modal.shadow.offset, iced::Vector::new(-4.0, 0.0));
    assert_color_close(modal.shadow.color, Color::BLACK.scale_alpha(0.2));
}
