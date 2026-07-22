use super::*;
use crate::app::state::ModelPopoverHover;

fn test_app() -> App {
    App::new().0
}

fn set_all_popovers(app: &mut App, visible: bool) {
    app.show_model_popover = visible;
    app.show_mode_popover = visible;
    app.show_send_mode_popover = visible;
    app.show_file_popover = visible;
    app.show_acp_popover = visible;
    app.show_reasoning_effort_popover = visible;
    app.show_usage_popover = visible;
    app.show_session_tool_selector_popover = visible;
    app.show_session_actions_popover = visible;
    app.show_executor_popover = visible;
}

fn visibility(app: &App) -> [bool; 10] {
    [
        app.show_model_popover,
        app.show_mode_popover,
        app.show_send_mode_popover,
        app.show_file_popover,
        app.show_acp_popover,
        app.show_reasoning_effort_popover,
        app.show_usage_popover,
        app.show_session_tool_selector_popover,
        app.show_session_actions_popover,
        app.show_executor_popover,
    ]
}

fn set_target_visibility(app: &mut App, message: &ViewMessage, visible: bool) {
    match message {
        ViewMessage::ToggleModelPopover => app.show_model_popover = visible,
        ViewMessage::ToggleModePopover => app.show_mode_popover = visible,
        ViewMessage::ToggleSendModePopover => app.show_send_mode_popover = visible,
        ViewMessage::ToggleFilePopover => app.show_file_popover = visible,
        ViewMessage::ToggleAcpPopover => app.show_acp_popover = visible,
        ViewMessage::ToggleReasoningEffortPopover => {
            app.show_reasoning_effort_popover = visible;
        }
        ViewMessage::ToggleUsagePopover => app.show_usage_popover = visible,
        ViewMessage::ToggleSessionToolSelectorPopover => {
            app.show_session_tool_selector_popover = visible;
        }
        ViewMessage::ToggleSessionActionsPopover => {
            app.show_session_actions_popover = visible;
        }
        ViewMessage::ToggleExecutorPopover => app.show_executor_popover = visible,
        _ => panic!("expected a popover toggle message"),
    }
}

#[test]
fn opening_each_popover_applies_its_existing_exclusivity_rules() {
    let cases = [
        (
            ViewMessage::ToggleModelPopover,
            [true, false, false, false, false, false, false, false, true, false],
        ),
        (
            ViewMessage::ToggleModePopover,
            [false, true, false, false, false, false, false, false, true, false],
        ),
        (
            ViewMessage::ToggleSendModePopover,
            [false, false, true, false, false, false, false, false, true, false],
        ),
        (
            ViewMessage::ToggleFilePopover,
            [false, false, false, true, false, false, false, false, true, false],
        ),
        (
            ViewMessage::ToggleAcpPopover,
            [false, false, false, false, true, false, false, false, false, false],
        ),
        (
            ViewMessage::ToggleReasoningEffortPopover,
            [false, false, false, false, false, true, false, false, false, false],
        ),
        (
            ViewMessage::ToggleUsagePopover,
            [false, false, false, false, false, false, true, false, false, false],
        ),
        (
            ViewMessage::ToggleSessionToolSelectorPopover,
            [false, false, false, false, false, false, false, true, false, false],
        ),
        (
            ViewMessage::ToggleSessionActionsPopover,
            [false, false, false, false, false, false, false, false, true, false],
        ),
        (
            ViewMessage::ToggleExecutorPopover,
            [false, false, false, false, false, false, false, false, false, true],
        ),
    ];

    for (message, expected) in cases {
        let mut app = test_app();
        set_all_popovers(&mut app, true);
        set_target_visibility(&mut app, &message, false);
        app.current_session_runtime_mut().is_requesting = true;
        app.model_settings.loading = true;
        app.acp_agents.push("codex".to_string());

        let _ = update(&mut app, message);

        assert_eq!(visibility(&app), expected);
    }
}

#[test]
fn toggling_an_open_popover_closes_only_that_popover() {
    let cases = [
        (ViewMessage::ToggleModelPopover, 0),
        (ViewMessage::ToggleModePopover, 1),
        (ViewMessage::ToggleSendModePopover, 2),
        (ViewMessage::ToggleFilePopover, 3),
        (ViewMessage::ToggleAcpPopover, 4),
        (ViewMessage::ToggleReasoningEffortPopover, 5),
        (ViewMessage::ToggleUsagePopover, 6),
        (ViewMessage::ToggleSessionToolSelectorPopover, 7),
        (ViewMessage::ToggleSessionActionsPopover, 8),
        (ViewMessage::ToggleExecutorPopover, 9),
    ];

    for (message, closed_index) in cases {
        let mut app = test_app();
        set_all_popovers(&mut app, true);
        app.current_session_runtime_mut().is_requesting = true;

        let _ = update(&mut app, message);

        let mut expected = [true; 10];
        expected[closed_index] = false;
        assert_eq!(visibility(&app), expected);
    }
}

#[test]
fn send_mode_popover_stays_closed_when_session_is_not_requesting() {
    let mut app = test_app();
    app.show_send_mode_popover = true;
    app.show_model_popover = true;

    let _ = update(&mut app, ViewMessage::ToggleSendModePopover);

    assert!(!app.show_send_mode_popover);
    assert!(app.show_model_popover);
}

#[test]
fn opening_acp_popover_with_empty_inventory_requests_bootstrap_and_closes_peers() {
    let mut app = test_app();
    set_all_popovers(&mut app, true);
    app.show_acp_popover = false;
    app.acp_agents.clear();

    let _bootstrap_task = update(&mut app, ViewMessage::ToggleAcpPopover);

    assert_eq!(
        visibility(&app),
        [false, false, false, false, true, false, false, false, false, false]
    );
    assert!(app.acp_agents.is_empty());
}

#[test]
fn close_popovers_clears_every_popover_and_model_hover() {
    let mut app = test_app();
    set_all_popovers(&mut app, true);
    app.model_popover_hover =
        Some(ModelPopoverHover::Text { text: "hovered".to_string(), anchor: None });

    let _ = update(&mut app, ViewMessage::ClosePopovers);

    assert_eq!(visibility(&app), [false; 10]);
    assert!(app.model_popover_hover.is_none());
}

#[test]
fn individual_close_messages_only_close_their_target() {
    let cases = [
        (ViewMessage::CloseModelPopover, 0),
        (ViewMessage::CloseModePopover, 1),
        (ViewMessage::CloseSendModePopover, 2),
        (ViewMessage::CloseFilePopover, 3),
        (ViewMessage::CloseAcpPopover, 4),
        (ViewMessage::CloseReasoningEffortPopover, 5),
        (ViewMessage::CloseUsagePopover, 6),
        (ViewMessage::CloseExecutorPopover, 9),
    ];

    for (message, closed_index) in cases {
        let mut app = test_app();
        set_all_popovers(&mut app, true);

        let _ = update(&mut app, message);

        let mut expected = [true; 10];
        expected[closed_index] = false;
        assert_eq!(visibility(&app), expected);
    }
}

#[test]
fn closing_model_popover_also_clears_hover_details() {
    let mut app = test_app();
    app.show_model_popover = true;
    app.model_popover_hover = Some(ModelPopoverHover::Model {
        provider_id: "provider".to_string(),
        model_id: "model".to_string(),
        anchor: None,
    });

    let _ = update(&mut app, ViewMessage::CloseModelPopover);

    assert!(!app.show_model_popover);
    assert!(app.model_popover_hover.is_none());
}

#[test]
fn model_hover_changed_replaces_and_clears_hover_state() {
    let mut app = test_app();

    let _ = update(
        &mut app,
        ViewMessage::ModelPopoverHoverChanged(Some(ModelPopoverHover::Text {
            text: "details".to_string(),
            anchor: None,
        })),
    );

    let Some(ModelPopoverHover::Text { text, anchor }) = app.model_popover_hover.as_ref() else {
        panic!("expected text hover state");
    };
    assert_eq!(text, "details");
    assert!(anchor.is_none());

    let _ = update(&mut app, ViewMessage::ModelPopoverHoverChanged(None));
    assert!(app.model_popover_hover.is_none());
}

#[test]
fn unrelated_view_message_leaves_popover_state_unchanged() {
    let mut app = test_app();
    set_all_popovers(&mut app, true);

    let _ = update(&mut app, ViewMessage::OpenKnowledge);

    assert_eq!(visibility(&app), [true; 10]);
}
