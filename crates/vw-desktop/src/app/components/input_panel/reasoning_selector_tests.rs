//! 验证推理等级选择器的选项映射、当前值与弹窗交互。
//! 未指定等级和不支持推理的模型用于确认选择器的隐藏与回退规则。

use super::reasoning_selector::{
    reasoning_effort_items, reasoning_effort_label, reasoning_effort_message,
    reasoning_effort_options, reasoning_effort_selector,
};
use crate::app::message::chat;
use crate::app::{Message, message};
use iced::Length;
use vw_gateway_client::vw_api_types::ReasoningEffortDto;

#[test]
fn chat_reasoning_effort_selector_uses_agent_options_and_current_session() {
    let codex = reasoning_effort_items("codex", Some(ReasoningEffortDto::High));
    assert_eq!(
        codex.iter().map(|item| item.label).collect::<Vec<_>>(),
        ["默认", "None", "Low", "Medium", "High", "Xhigh"]
    );
    assert_eq!(codex.iter().filter(|item| item.selected).count(), 1);
    assert!(codex[4].selected);

    let claude = reasoning_effort_items("claude", None);
    assert_eq!(claude.iter().map(|item| item.label).collect::<Vec<_>>(), ["默认", "High", "Max"]);
    assert!(claude[0].selected);

    let other = reasoning_effort_items("gemini", Some(ReasoningEffortDto::High));
    assert_eq!(other.iter().map(|item| item.label).collect::<Vec<_>>(), ["默认"]);
    assert_eq!(other.iter().filter(|item| item.selected).count(), 0);

    assert!(matches!(
        reasoning_effort_message(None),
        Message::Chat(message::ChatMessage::ReasoningEffortSelected(None))
    ));
    assert!(matches!(
        reasoning_effort_message(Some(ReasoningEffortDto::None)),
        Message::Chat(message::ChatMessage::ReasoningEffortSelected(Some(
            ReasoningEffortDto::None
        )))
    ));

    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("session-a".to_string());
    app.get_session_runtime_mut("session-a").acp_agent = Some("codex".to_string());
    let _ = crate::app::message::view::update(
        &mut app,
        message::ViewMessage::ToggleReasoningEffortPopover,
    );
    assert!(app.show_reasoning_effort_popover);
    let _ = chat::update(
        &mut app,
        message::ChatMessage::ReasoningEffortSelected(Some(ReasoningEffortDto::Xhigh)),
    );
    assert!(!app.show_reasoning_effort_popover);

    app.active_session_id = Some("session-b".to_string());
    app.get_session_runtime_mut("session-b").acp_agent = Some("claude".to_string());
    let _ = chat::update(
        &mut app,
        message::ChatMessage::ReasoningEffortSelected(Some(ReasoningEffortDto::Max)),
    );

    assert_eq!(
        app.get_session_runtime("session-a").reasoning_effort,
        Some(ReasoningEffortDto::Xhigh)
    );
    assert_eq!(
        app.get_session_runtime("session-b").reasoning_effort,
        Some(ReasoningEffortDto::Max)
    );

    let selector = reasoning_effort_selector(
        &app,
        Some("claude"),
        app.get_session_runtime("session-b").reasoning_effort,
    )
    .expect("ACP sessions show the reasoning selector");
    assert_eq!(selector.as_widget().size().width, Length::Shrink);
    assert!(reasoning_effort_selector(&app, None, None).is_none());
}

#[test]
fn reasoning_effort_options_accept_cli_aliases_case_insensitively() {
    for agent in [" CODEX ", "Codex CLI", "codex-cli"] {
        assert_eq!(
            reasoning_effort_options(agent),
            [
                None,
                Some(ReasoningEffortDto::None),
                Some(ReasoningEffortDto::Low),
                Some(ReasoningEffortDto::Medium),
                Some(ReasoningEffortDto::High),
                Some(ReasoningEffortDto::Xhigh),
            ]
        );
    }

    for agent in [" CLAUDE ", "Claude Code", "claude-code", "ClaudeCode"] {
        assert_eq!(
            reasoning_effort_options(agent),
            [None, Some(ReasoningEffortDto::High), Some(ReasoningEffortDto::Max),]
        );
    }
}

#[test]
fn reasoning_effort_label_displays_every_supported_value() {
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
        assert_eq!(reasoning_effort_label(effort), expected);
    }
}

#[test]
fn reasoning_effort_selector_builds_expanded_codex_and_default_agent_lists() {
    let (mut app, _task) = crate::app::App::new();
    app.show_reasoning_effort_popover = true;

    let codex_selector = reasoning_effort_selector(&app, Some("codex cli"), None)
        .expect("codex ACP sessions show the reasoning selector");
    assert_eq!(codex_selector.as_widget().size().width, Length::Shrink);

    let default_selector =
        reasoning_effort_selector(&app, Some("unknown"), Some(ReasoningEffortDto::Medium))
            .expect("unknown ACP sessions still show the default selector");
    assert_eq!(default_selector.as_widget().size().width, Length::Shrink);
}
