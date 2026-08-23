//! 验证任务模式对子任务、执行器和运行时编辑器的布局回退。
//! 执行器缺失、编辑器不足和单子任务用例用于约束降级与移动按钮状态。

use super::task_mode::{
    TaskModeReasoningEffortOption, task_mode_form, task_mode_reasoning_effort_options,
};
use crate::app::task::TaskSplitMode;
use crate::app::{App, Message};
use iced::Length;
use vw_gateway_client::vw_api_types::ReasoningEffortDto;

fn test_app() -> App {
    App::new().0
}

fn keep(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn task_739_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("task_mode_tests.rs"));
}

#[test]
fn disabled_task_mode_returns_placeholder_element() {
    let app = test_app();

    let element = task_mode_form(
        &app,
        None,
        false,
        "5",
        "auto",
        TaskSplitMode::FollowSystem,
        None,
        None,
        &[],
    );

    assert_eq!(element.as_widget().size().width, Length::Shrink);
}

#[test]
fn enabled_task_mode_builds_with_empty_default_executor_and_no_runtime_editors() {
    let mut app = test_app();
    app.acp_agents = vec!["codex".to_string(), "claude".to_string()];
    app.show_executor_popover = true;

    keep(task_mode_form(
        &app,
        None,
        true,
        "1",
        "provider/model",
        TaskSplitMode::FollowSystem,
        None,
        None,
        &["first".to_string(), "second".to_string()],
    ));
}

#[test]
fn enabled_task_mode_builds_with_selected_executor_and_runtime_editors() {
    let mut app = test_app();
    app.acp_agents = vec!["codex".to_string(), "claude".to_string()];
    {
        let runtime = app.current_session_runtime_mut();
        runtime.task_mode_subtasks = vec![
            "line one\nline two\nline three\nline four".to_string(),
            "single line".to_string(),
        ];
        runtime.task_mode_subtask_editors = runtime
            .task_mode_subtasks
            .iter()
            .map(|subtask| iced::widget::text_editor::Content::with_text(subtask))
            .collect();
    }
    let runtime = app.current_session_runtime();

    keep(task_mode_form(
        &app,
        Some(&runtime),
        true,
        "99",
        "auto",
        TaskSplitMode::Enabled,
        Some("claude".to_string()),
        Some(ReasoningEffortDto::Max),
        &runtime.task_mode_subtasks,
    ));
}

#[test]
fn enabled_task_mode_builds_with_single_subtask_disabled_move_buttons() {
    let app = test_app();
    keep(task_mode_form(
        &app,
        None,
        true,
        "10",
        "",
        TaskSplitMode::Disabled,
        Some("custom-agent".to_string()),
        None,
        &["only child".to_string()],
    ));
}

#[test]
fn enabled_task_mode_with_no_agents_or_subtasks_fills_available_width() {
    let app = test_app();

    let element =
        task_mode_form(&app, None, true, "", "", TaskSplitMode::FollowSystem, None, None, &[]);

    assert_eq!(element.as_widget().size().width, Length::Fill);
}

#[test]
fn enabled_task_mode_builds_when_selected_executor_is_not_in_agent_list() {
    let mut app = test_app();
    app.acp_agents = vec!["codex".to_string()];
    let supported_task_modes = ["handoff".to_string(), "review".to_string(), "merge".to_string()];

    let element = task_mode_form(
        &app,
        None,
        true,
        "3",
        "auto",
        TaskSplitMode::FollowSystem,
        Some("external-agent".to_string()),
        Some(ReasoningEffortDto::Medium),
        &supported_task_modes,
    );

    assert_eq!(element.as_widget().size().width, Length::Fill);
}

#[test]
fn enabled_task_mode_falls_back_when_runtime_has_fewer_editors_than_subtasks() {
    let mut app = test_app();
    app.acp_agents = vec!["codex".to_string()];
    {
        let runtime = app.current_session_runtime_mut();
        runtime.task_mode_subtasks = vec![
            "rich editor\nsecond line".to_string(),
            "plain fallback".to_string(),
            "last fallback".to_string(),
        ];
        runtime.task_mode_subtask_editors =
            vec![iced::widget::text_editor::Content::with_text(&runtime.task_mode_subtasks[0])];
    }
    let runtime = app.current_session_runtime();

    let element = task_mode_form(
        &app,
        Some(&runtime),
        true,
        "2",
        "auto",
        TaskSplitMode::Enabled,
        Some("codex".to_string()),
        Some(ReasoningEffortDto::High),
        &runtime.task_mode_subtasks,
    );

    assert_eq!(element.as_widget().size().width, Length::Fill);
}

#[test]
fn task_mode_reasoning_effort_option_displays_every_shared_label() {
    let cases = [
        (None, "默认"),
        (Some(ReasoningEffortDto::None), "None"),
        (Some(ReasoningEffortDto::Low), "Low"),
        (Some(ReasoningEffortDto::Medium), "Medium"),
        (Some(ReasoningEffortDto::High), "High"),
        (Some(ReasoningEffortDto::Xhigh), "Xhigh"),
        (Some(ReasoningEffortDto::Max), "Max"),
    ];

    for (effort, expected) in cases {
        assert_eq!(TaskModeReasoningEffortOption(effort).to_string(), expected);
    }
}

#[test]
fn task_mode_reasoning_effort_handles_missing_and_claude_agents() {
    assert_eq!(task_mode_reasoning_effort_options(None), vec![None]);
    assert_eq!(
        task_mode_reasoning_effort_options(Some(" CLAUDE-CODE ")),
        vec![None, Some(ReasoningEffortDto::High), Some(ReasoningEffortDto::Max),]
    );
}

#[test]
fn task_mode_reasoning_effort_uses_shared_agent_fallback_options() {
    assert_eq!(
        task_mode_reasoning_effort_options(Some("codex")),
        vec![
            None,
            Some(ReasoningEffortDto::None),
            Some(ReasoningEffortDto::Low),
            Some(ReasoningEffortDto::Medium),
            Some(ReasoningEffortDto::High),
            Some(ReasoningEffortDto::Xhigh),
        ]
    );
    assert_eq!(task_mode_reasoning_effort_options(Some("unknown")), vec![None]);
}

#[test]
fn task_mode_reasoning_effort_accepts_codex_cli_alias() {
    assert_eq!(
        task_mode_reasoning_effort_options(Some(" CoDeX CLI ")),
        vec![
            None,
            Some(ReasoningEffortDto::None),
            Some(ReasoningEffortDto::Low),
            Some(ReasoningEffortDto::Medium),
            Some(ReasoningEffortDto::High),
            Some(ReasoningEffortDto::Xhigh),
        ]
    );
}
