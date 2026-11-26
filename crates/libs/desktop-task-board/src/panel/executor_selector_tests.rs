//! 任务详情执行器选择器的测试，覆盖候选项、当前选择与不可用状态。
//! 用例验证提供商图标和交互消息在空目录或混合配置下仍保持一致。
use iced::widget::button;
use iced::{Element, Theme};

use crate::message::TaskBoardMessage;
use crate::slice_view_fixture::Fixture;
use crate::message::TaskBoardMessage as Message;

use super::*;

fn test_app() -> Fixture {
    Fixture::new().0
}

fn assert_task_board_message(message: TaskBoardMessage, expected: TaskBoardMessage) {
    match (message, expected) {
        (TaskBoardMessage::ToggleExecutorPopover, TaskBoardMessage::ToggleExecutorPopover)
        | (TaskBoardMessage::CloseExecutorPopover, TaskBoardMessage::CloseExecutorPopover)
        | (
            TaskBoardMessage::ToggleBulkExecutorPopover,
            TaskBoardMessage::ToggleBulkExecutorPopover,
        )
        | (
            TaskBoardMessage::CloseBulkExecutorPopover,
            TaskBoardMessage::CloseBulkExecutorPopover,
        ) => {}
        (
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
        TaskBoardExecutorSelectorKind::Edit.current_executor(app.executor()),
        Some("draft-agent".to_string())
    );
    assert_eq!(
        TaskBoardExecutorSelectorKind::Bulk.current_executor(app.executor()),
        Some("bulk-agent".to_string())
    );
}

#[test]
fn selector_kind_maps_open_close_toggle_and_select_messages() {
    let mut app = test_app();
    app.task_board_executor_popover = true;
    app.task_board_bulk_executor_popover = false;

    assert!(TaskBoardExecutorSelectorKind::Edit.is_open(app.executor()));
    assert!(!TaskBoardExecutorSelectorKind::Bulk.is_open(app.executor()));

    assert_task_board_message(
        TaskBoardExecutorSelectorKind::Edit.toggle_message(),
        TaskBoardMessage::ToggleExecutorPopover,
    );
    assert_task_board_message(
        TaskBoardExecutorSelectorKind::Bulk.toggle_message(),
        TaskBoardMessage::ToggleBulkExecutorPopover,
    );

    assert_task_board_message(
        TaskBoardExecutorSelectorKind::Edit.close_message(),
        TaskBoardMessage::CloseExecutorPopover,
    );
    assert_task_board_message(
        TaskBoardExecutorSelectorKind::Bulk.close_message(),
        TaskBoardMessage::CloseBulkExecutorPopover,
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
fn executor_options_distinguish_selection_and_hover() {
    for theme in [Theme::Light, Theme::Dark] {
        let hovered = selector_option_style(&theme, button::Status::Hovered, false);
        let hovered_selected = selector_option_style(&theme, button::Status::Hovered, true);
        let selected = selector_option_style(&theme, button::Status::Active, true);
        let idle = selector_option_style(&theme, button::Status::Active, false);
        assert_eq!(hovered.background, hovered_selected.background);
        assert_ne!(hovered.background, selected.background);
        assert_ne!(selected.background, idle.background);
    }
}

#[test]
fn build_selector_variants_create_elements_for_draft_edit_and_bulk_states() {
    let mut app = test_app();
    app.acp_agents = vec!["codex".to_string(), "claude".to_string()];
    app.task_board_draft.acp_agent = Some("codex".to_string());
    app.task_board_bulk_acp_agent = Some("claude".to_string());
    app.task_board_executor_popover = true;
    app.task_board_bulk_executor_popover = true;

    keep_element(build_draft_agent_selector(app.executor(), &[]));
    keep_element(build_edit_executor_selector(app.executor()));
    keep_element(build_bulk_executor_selector(app.executor()));
}

#[test]
fn build_executor_popover_covers_default_selected_agent_selected_and_agent_unselected() {
    let mut app = test_app();
    app.acp_agents = vec!["codex".to_string(), "claude".to_string()];

    let default_selected = build_executor_popover(None, app.executor(), TaskBoardExecutorSelectorKind::Edit);
    assert_eq!(default_selected.as_widget().children().len(), 1);

    let agent_selected = build_executor_popover(
        Some("codex".to_string()),
        app.executor(),
        TaskBoardExecutorSelectorKind::Edit,
    );
    assert_eq!(agent_selected.as_widget().children().len(), 1);

    let bulk_selected = build_executor_popover(
        Some("claude".to_string()),
        app.executor(),
        TaskBoardExecutorSelectorKind::Bulk,
    );
    assert_eq!(bulk_selected.as_widget().children().len(), 1);
}

#[test]
fn build_executor_popover_accepts_empty_agent_catalog() {
    let app = test_app();

    let popover = build_executor_popover(None, app.executor(), TaskBoardExecutorSelectorKind::Bulk);

    assert_eq!(popover.as_widget().children().len(), 1);
    keep_element(popover);
}

#[test]
fn draft_agent_choices_merge_profiles_and_backends() {
    let mut app = test_app();
    app.acp_agents = vec!["codex".into(), "claude".into(), "opencode".into(), EMPLOYEE_ACP_BACKEND.into()];
    let profile = |key, backend: &str, enabled| {
        DelegateAgentSettingsEntry::from_config(
            key,
            Some(config_agent::DelegateAgentConfig {
                enabled,
                runtime: AgentRuntime::Acp { backend: backend.to_owned() },
                ..Default::default()
            }),
        )
    };
    let entries = [
        profile("acp-codex", "codex", true),
        profile("acp-claude", "claude", false),
        profile("custom", "opencode", true),
    ];
    let options = draft_agent_choices(app.executor(), &entries);
    assert_eq!(
        options.iter().map(|choice| (choice.key, choice.backend)).collect::<Vec<_>>(),
        [
            (MAIN_AGENT_KEY, None),
            ("acp-codex", Some("codex")),
            ("custom", Some("opencode")),
            (MAIN_AGENT_KEY, Some("opencode")),
        ],
    );
}
