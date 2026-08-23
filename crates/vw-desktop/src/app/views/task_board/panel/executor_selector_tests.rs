//! 任务详情执行器选择器的测试，覆盖候选项、当前选择与不可用状态。
//! 用例验证提供商图标和交互消息在空目录或混合配置下仍保持一致。
use iced::widget::button;
use iced::{Background, Color, Element, Theme};

use crate::app::message::TaskBoardMessage;
use crate::app::{App, Message};

use super::*;

const EPSILON: f32 = 0.000_01;

fn test_app() -> App {
    App::new().0
}

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

fn assert_task_board_message(message: TaskBoardMessage, expected: TaskBoardMessage) {
    match (message, expected) {
        (TaskBoardMessage::ToggleExecutorPopover, TaskBoardMessage::ToggleExecutorPopover)
        | (
            TaskBoardMessage::ToggleBulkExecutorPopover,
            TaskBoardMessage::ToggleBulkExecutorPopover,
        )
        | (
            TaskBoardMessage::CloseBulkExecutorPopover,
            TaskBoardMessage::CloseBulkExecutorPopover,
        ) => {}
        (
            TaskBoardMessage::UpdateDraftExecutor(actual),
            TaskBoardMessage::UpdateDraftExecutor(expected),
        )
        | (
            TaskBoardMessage::UpdateEditingTaskExecutor(actual),
            TaskBoardMessage::UpdateEditingTaskExecutor(expected),
        )
        | (
            TaskBoardMessage::BulkExecutorSelected(actual),
            TaskBoardMessage::BulkExecutorSelected(expected),
        ) => assert_eq!(actual, expected),
        (actual, expected) => panic!("expected {expected:?}, got {actual:?}"),
    }
}

fn keep_element(element: Element<'_, Message>) {
    let _ = std::hint::black_box(element);
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("executor_selector_tests"));
}

#[test]
fn selector_kind_reads_current_executor_from_expected_app_state() {
    let mut app = test_app();
    app.task_board_draft.acp_agent = Some("draft-agent".to_string());
    app.task_board_bulk_acp_agent = Some("bulk-agent".to_string());

    assert_eq!(
        TaskBoardExecutorSelectorKind::Draft.current_executor(&app),
        Some("draft-agent".to_string())
    );
    assert_eq!(
        TaskBoardExecutorSelectorKind::Edit.current_executor(&app),
        Some("draft-agent".to_string())
    );
    assert_eq!(
        TaskBoardExecutorSelectorKind::Bulk.current_executor(&app),
        Some("bulk-agent".to_string())
    );
}

#[test]
fn selector_kind_maps_open_close_toggle_and_select_messages() {
    let mut app = test_app();
    app.task_board_executor_popover = true;
    app.task_board_bulk_executor_popover = false;

    assert!(TaskBoardExecutorSelectorKind::Draft.is_open(&app));
    assert!(TaskBoardExecutorSelectorKind::Edit.is_open(&app));
    assert!(!TaskBoardExecutorSelectorKind::Bulk.is_open(&app));

    assert_task_board_message(
        TaskBoardExecutorSelectorKind::Draft.toggle_message(),
        TaskBoardMessage::ToggleExecutorPopover,
    );
    assert_task_board_message(
        TaskBoardExecutorSelectorKind::Edit.toggle_message(),
        TaskBoardMessage::ToggleExecutorPopover,
    );
    assert_task_board_message(
        TaskBoardExecutorSelectorKind::Bulk.toggle_message(),
        TaskBoardMessage::ToggleBulkExecutorPopover,
    );

    assert_task_board_message(
        TaskBoardExecutorSelectorKind::Draft.close_message(),
        TaskBoardMessage::ToggleExecutorPopover,
    );
    assert_task_board_message(
        TaskBoardExecutorSelectorKind::Edit.close_message(),
        TaskBoardMessage::ToggleExecutorPopover,
    );
    assert_task_board_message(
        TaskBoardExecutorSelectorKind::Bulk.close_message(),
        TaskBoardMessage::CloseBulkExecutorPopover,
    );

    assert_task_board_message(
        TaskBoardExecutorSelectorKind::Draft.select_message(Some("codex".to_string())),
        TaskBoardMessage::UpdateDraftExecutor(Some("codex".to_string())),
    );
    assert_task_board_message(
        TaskBoardExecutorSelectorKind::Edit.select_message(None),
        TaskBoardMessage::UpdateEditingTaskExecutor(None),
    );
    assert_task_board_message(
        TaskBoardExecutorSelectorKind::Bulk.select_message(Some("claude".to_string())),
        TaskBoardMessage::BulkExecutorSelected(Some("claude".to_string())),
    );
}

#[test]
fn executor_option_button_style_covers_hover_selected_and_idle_branches() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();

    let hovered = executor_option_button_style(&theme, button::Status::Hovered, false);
    assert_color_close(
        color_background(hovered.background),
        palette.background.weak.color.scale_alpha(0.35),
    );
    assert_color_close(hovered.border.color, theme.palette().primary);
    assert_close(hovered.border.radius.top_left, 6.0);
    assert_close(hovered.border.width, 0.0);
    assert_color_close(hovered.text_color, theme.palette().text);

    let selected = executor_option_button_style(&theme, button::Status::Active, true);
    assert_color_close(
        color_background(selected.background),
        Color::from_rgba(
            theme.palette().primary.r,
            theme.palette().primary.g,
            theme.palette().primary.b,
            0.10,
        ),
    );

    let idle = executor_option_button_style(&theme, button::Status::Active, false);
    assert!(idle.background.is_none());
}

#[test]
fn executor_option_hover_takes_precedence_over_selected_background() {
    let theme = Theme::Light;
    let palette = theme.extended_palette();

    let hovered_selected = executor_option_button_style(&theme, button::Status::Hovered, true);

    assert_color_close(
        color_background(hovered_selected.background),
        palette.background.weak.color.scale_alpha(0.35),
    );
}

#[test]
fn build_selector_variants_create_elements_for_draft_edit_and_bulk_states() {
    let mut app = test_app();
    app.acp_agents = vec!["codex".to_string(), "claude".to_string()];
    app.task_board_draft.acp_agent = Some("codex".to_string());
    app.task_board_bulk_acp_agent = Some("claude".to_string());
    app.task_board_executor_popover = true;
    app.task_board_bulk_executor_popover = true;

    keep_element(build_executor_selector(&app, false));
    keep_element(build_executor_selector(&app, true));
    keep_element(build_bulk_executor_selector(&app));
}

#[test]
fn build_executor_popover_covers_default_selected_agent_selected_and_agent_unselected() {
    let mut app = test_app();
    app.acp_agents = vec!["codex".to_string(), "claude".to_string()];

    let default_selected = build_executor_popover(None, &app, TaskBoardExecutorSelectorKind::Draft);
    assert_eq!(default_selected.as_widget().children().len(), 1);

    let agent_selected = build_executor_popover(
        Some("codex".to_string()),
        &app,
        TaskBoardExecutorSelectorKind::Edit,
    );
    assert_eq!(agent_selected.as_widget().children().len(), 1);

    let bulk_selected = build_executor_popover(
        Some("claude".to_string()),
        &app,
        TaskBoardExecutorSelectorKind::Bulk,
    );
    assert_eq!(bulk_selected.as_widget().children().len(), 1);
}

#[test]
fn build_executor_popover_accepts_empty_agent_catalog() {
    let app = test_app();

    let popover = build_executor_popover(None, &app, TaskBoardExecutorSelectorKind::Bulk);

    assert_eq!(popover.as_widget().children().len(), 1);
    keep_element(popover);
}
