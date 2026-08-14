use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};

use super::controller::{TuiController, TuiControllerCommand, TuiControllerEvent};
use super::model::{
    MessageActionIntent, PromptSubmission, UiAssistantMessage, UiMessage, UiMessageBase,
    UiMessageId, UiSystemMessage, UiSystemMessageLevel, UiTokenUsage, UiTurnTerminal,
    UiUserMessage,
};
use super::state::TuiState;

fn press(
    controller: &mut TuiController,
    state: &mut TuiState,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> TuiControllerCommand {
    controller.handle_event(
        state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(code, modifiers))),
    )
}

fn assistant(id: &str, parent: &UiMessageId, text: &str) -> UiMessage {
    UiMessage::Assistant(UiAssistantMessage {
        base: UiMessageBase::new(UiMessageId::local(id)).with_parent_id(parent.clone()),
        text: text.to_string(),
        usage: UiTokenUsage::default(),
        step_count: 0,
        terminal: UiTurnTerminal::Done { finish_reason: None },
        model: None,
    })
}

#[test]
fn tui_message_actions_select_stable_messages_and_emit_exact_intents() {
    let user1 = UiMessageId::local("action-u1");
    let user2 = UiMessageId::local("action-u2");
    let assistant2 = UiMessageId::local("action-a2");
    let mut state = TuiState::default();
    state.prompt.set_value("preserved draft");
    state.messages = vec![
        UiMessage::User(UiUserMessage {
            base: UiMessageBase::new(user1.clone()),
            text: "first prompt".to_string(),
        }),
        assistant("action-a1", &user1, "first answer"),
        UiMessage::User(UiUserMessage {
            base: UiMessageBase::new(user2.clone()),
            text: "second prompt".to_string(),
        }),
        assistant("action-a2", &user2, "second answer"),
    ];
    state.refresh_transcript_projection();
    let mut controller = TuiController::default();

    assert!(matches!(
        press(&mut controller, &mut state, KeyCode::Char('r'), KeyModifiers::ALT,),
        TuiControllerCommand::Continue
    ));
    assert!(state.message_actions.active);
    assert_eq!(state.message_actions.selected_id.as_ref(), Some(&assistant2));
    assert!(matches!(
        press(&mut controller, &mut state, KeyCode::Char('s'), KeyModifiers::NONE),
        TuiControllerCommand::MessageAction(MessageActionIntent::Summarize { message_id })
            if message_id == user2
    ));

    state.append_message(UiMessage::System(UiSystemMessage {
        base: UiMessageBase::new(UiMessageId::local("action-notice")),
        text: "new notice".to_string(),
        level: UiSystemMessageLevel::Success,
    }));
    press(&mut controller, &mut state, KeyCode::Char(' '), KeyModifiers::NONE);
    assert_eq!(
        state.message_actions.selected_id.as_ref(),
        Some(&assistant2),
        "append does not move a stable selection"
    );
    assert!(state.message_actions.expanded);

    state.messages.retain(|message| message.base().id != assistant2);
    state.refresh_transcript_projection();
    press(&mut controller, &mut state, KeyCode::Up, KeyModifiers::NONE);
    assert_ne!(state.message_actions.selected_id.as_ref(), Some(&assistant2));
    press(&mut controller, &mut state, KeyCode::Char('u'), KeyModifiers::NONE);
    assert_eq!(state.message_actions.selected_id.as_ref(), Some(&user1));
    press(&mut controller, &mut state, KeyCode::Char('U'), KeyModifiers::NONE);
    assert_eq!(state.message_actions.selected_id.as_ref(), Some(&user2));

    let TuiControllerCommand::MessageAction(MessageActionIntent::Copy { message_id, text }) =
        press(&mut controller, &mut state, KeyCode::Char('c'), KeyModifiers::NONE)
    else {
        panic!("copy intent");
    };
    assert_eq!(message_id, user2);
    assert_eq!(text, "second prompt");

    state.prompt.start_submission(PromptSubmission::new("active turn"));
    let TuiControllerCommand::MessageAction(MessageActionIntent::Edit {
        message_id,
        text,
        cancel_active_turn,
    }) = press(&mut controller, &mut state, KeyCode::Char('e'), KeyModifiers::NONE)
    else {
        panic!("edit intent");
    };
    assert_eq!(message_id, user2);
    assert_eq!(text, "second prompt");
    assert!(cancel_active_turn, "streaming edit requests cancellation first");

    for (key, expected) in [('f', "fork"), ('r', "restore"), ('s', "summarize"), ('w', "rewind")] {
        let command = press(&mut controller, &mut state, KeyCode::Char(key), KeyModifiers::NONE);
        let actual =
            match command {
                TuiControllerCommand::MessageAction(MessageActionIntent::Fork { message_id })
                    if message_id == user2 =>
                {
                    "fork"
                }
                TuiControllerCommand::MessageAction(MessageActionIntent::Restore {
                    message_id,
                }) if message_id == user2 => "restore",
                TuiControllerCommand::MessageAction(MessageActionIntent::Summarize {
                    message_id,
                }) if message_id == user2 => "summarize",
                TuiControllerCommand::MessageAction(MessageActionIntent::Rewind { message_id })
                    if message_id == user2 =>
                {
                    "rewind"
                }
                other => panic!("unexpected intent: {other:?}"),
            };
        assert_eq!(actual, expected);
    }

    press(&mut controller, &mut state, KeyCode::Esc, KeyModifiers::NONE);
    assert!(!state.message_actions.active);
    assert_eq!(state.prompt.value, "preserved draft");
}
