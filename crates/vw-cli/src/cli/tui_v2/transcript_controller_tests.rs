use std::fs;

use crossterm::event::{
    Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers,
};

use super::controller::{
    TuiController, TuiControllerCommand, TuiControllerEvent,
};
use super::model::{
    TranscriptEffectIntent, UiAssistantMessage, UiMessage, UiMessageBase, UiMessageId,
    UiTokenUsage, UiToolCall, UiToolCallState, UiTurnTerminal, UiUserMessage,
};
use super::state::TuiState;

fn key(controller: &mut TuiController, state: &mut TuiState, code: KeyCode) -> TuiControllerCommand {
    controller.handle_event(
        state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            code,
            KeyModifiers::NONE,
        ))),
    )
}

#[test]
fn tui_transcript_mode_matches_reference_navigation_search_and_exit() {
    let user_id = UiMessageId::local("transcript-user");
    let mut state = TuiState::default();
    state.session.session_id = Some("transcript-session".to_string());
    state.messages = vec![
        UiMessage::User(UiUserMessage {
            base: UiMessageBase::new(user_id.clone()),
            text: "alpha prompt".to_string(),
        }),
        UiMessage::ToolCall(UiToolCall {
            base: UiMessageBase::new(UiMessageId::local("transcript-tool"))
                .with_parent_id(user_id.clone()),
            call_id: Some("tool".to_string()),
            tool_name: "read".to_string(),
            summary: Some("visible summary".to_string()),
            arguments: Some(r#"{"api_key":"must-not-enter-search"}"#.to_string()),
            state: UiToolCallState::Complete,
        }),
        UiMessage::Assistant(UiAssistantMessage {
            base: UiMessageBase::new(UiMessageId::local("transcript-answer"))
                .with_parent_id(user_id),
            text: "beta answer\nbeta continuation".to_string(),
            usage: UiTokenUsage::default(),
            step_count: 0,
            terminal: UiTurnTerminal::Done { finish_reason: None },
            model: None,
        }),
    ];
    state.refresh_transcript_projection();
    state.scroll.follow_tail = false;
    state.scroll.top_message = 1;
    state.scroll.sync_viewport(2, 60);
    let restore_top = state.scroll.top_message;
    let mut controller = TuiController::default();

    let command = controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
            KeyCode::Char('o'),
            KeyModifiers::CONTROL,
        ))),
    );
    assert!(matches!(command, TuiControllerCommand::Continue));
    assert!(state.transcript_mode.active);
    assert_eq!(state.transcript_mode.viewport_rows, 2);
    assert!(
        !state.transcript_mode.lines.join("\n").contains("must-not-enter-search"),
        "hidden tool arguments never enter transcript search"
    );
    let frozen = state.transcript_mode.lines.clone();
    state.append_message(UiMessage::User(UiUserMessage {
        base: UiMessageBase::new(UiMessageId::local("late-message")),
        text: "late stream".to_string(),
    }));
    assert_eq!(state.transcript_mode.lines, frozen, "entry freezes the visible stream");

    key(&mut controller, &mut state, KeyCode::Char('j'));
    assert_eq!(state.transcript_mode.top_line, 1);
    key(&mut controller, &mut state, KeyCode::PageDown);
    assert_eq!(state.transcript_mode.top_line, state.transcript_mode.max_top_line());
    key(&mut controller, &mut state, KeyCode::Char('g'));
    assert_eq!(state.transcript_mode.top_line, 0);

    key(&mut controller, &mut state, KeyCode::Char('/'));
    key(&mut controller, &mut state, KeyCode::Char('b'));
    key(&mut controller, &mut state, KeyCode::Char('e'));
    assert!(state.transcript_mode.search_query.is_empty());
    assert!(state.transcript_mode.top_line > 0, "draft search previews a match");
    key(&mut controller, &mut state, KeyCode::Esc);
    assert!(!state.transcript_mode.search_bar_open);
    assert_eq!(state.transcript_mode.top_line, 0, "cancel restores the pre-search view");

    key(&mut controller, &mut state, KeyCode::Char('/'));
    for ch in "beta".chars() {
        key(&mut controller, &mut state, KeyCode::Char(ch));
    }
    key(&mut controller, &mut state, KeyCode::Enter);
    assert_eq!(state.transcript_mode.search_query, "beta");
    assert_eq!(state.transcript_mode.search_matches.len(), 1);
    key(&mut controller, &mut state, KeyCode::Char('n'));
    assert_eq!(state.transcript_mode.selected_match, Some(0));
    key(&mut controller, &mut state, KeyCode::Char('N'));
    assert_eq!(state.transcript_mode.selected_match, Some(0));

    controller.handle_event(
        &mut state,
        TuiControllerEvent::Terminal(CrosstermEvent::Resize(80, 12)),
    );
    assert_eq!(state.transcript_mode.viewport_rows, 10);
    key(&mut controller, &mut state, KeyCode::Char('G'));
    assert!(state.transcript_mode.show_all);

    let TuiControllerCommand::TranscriptEffect(TranscriptEffectIntent::Dumped { path }) =
        key(&mut controller, &mut state, KeyCode::Char('['))
    else {
        panic!("dump should return an explicit effect");
    };
    assert_eq!(fs::read_to_string(&path).expect("read dump"), frozen.join("\n"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(fs::metadata(&path).expect("dump metadata").permissions().mode() & 0o777, 0o600);
    }
    fs::remove_file(&path).expect("remove owned dump");

    let TuiControllerCommand::TranscriptEffect(
        TranscriptEffectIntent::OpenExternalEditor { path },
    ) = key(&mut controller, &mut state, KeyCode::Char('v'))
    else {
        panic!("editor should return a safe path intent");
    };
    assert_eq!(fs::read_to_string(&path).expect("read editor file"), frozen.join("\n"));
    fs::remove_file(&path).expect("remove owned editor file");

    key(&mut controller, &mut state, KeyCode::Esc);
    assert!(!state.transcript_mode.active);
    assert_eq!(state.scroll.top_message, restore_top);
    assert!(!state.scroll.follow_tail);
}
