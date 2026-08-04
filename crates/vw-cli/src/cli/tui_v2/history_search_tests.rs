use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};
use unicode_segmentation::UnicodeSegmentation as _;

use super::controller::{TuiController, TuiControllerCommand, TuiControllerEvent};
use super::model::{PromptMotion, PromptSubmission};
use super::render::build_prompt_host;
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

#[test]
fn tui_prompt_footer_and_history_match_reference() {
    let mut state = TuiState::default();
    for entry in [
        "first command",
        "second command",
        "first command",
        "api_key=must-not-store",
        "/deny private refusal reason",
    ] {
        state.prompt.history.push(entry);
    }
    assert_eq!(
        state.prompt.history.entries,
        vec!["second command".to_string(), "first command".to_string()],
        "history deduplicates globally and rejects secret/rejection payloads"
    );

    state.prompt.set_value("line one\nline two");
    state.prompt.cursor.char_index = state.prompt.value.graphemes(true).count();
    let mut controller = TuiController::default();
    press(&mut controller, &mut state, KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(state.prompt.value, "line one\nline two");
    assert!(state.prompt.can_move_cursor(PromptMotion::Down));

    press(&mut controller, &mut state, KeyCode::Home, KeyModifiers::NONE);
    press(&mut controller, &mut state, KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(state.prompt.value, "first command");
    press(&mut controller, &mut state, KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(state.prompt.value, "line one\nline two", "history restores multiline draft");

    state.prompt.set_value("draft survives search");
    press(
        &mut controller,
        &mut state,
        KeyCode::Char('r'),
        KeyModifiers::CONTROL,
    );
    assert!(state.history_search.active);
    for ch in "second".chars() {
        press(&mut controller, &mut state, KeyCode::Char(ch), KeyModifiers::NONE);
    }
    assert_eq!(state.history_search.matches.len(), 1);
    let search_host = build_prompt_host(&state);
    assert!(
        search_host.footer_pills.iter().any(|pill| pill.label.contains("历史搜索")),
        "footer exposes search mode"
    );
    press(&mut controller, &mut state, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(state.prompt.value, "draft survives search");

    press(
        &mut controller,
        &mut state,
        KeyCode::Char('r'),
        KeyModifiers::CONTROL,
    );
    for ch in "first".chars() {
        press(&mut controller, &mut state, KeyCode::Char(ch), KeyModifiers::NONE);
    }
    press(&mut controller, &mut state, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(state.prompt.value, "first command");

    state.prompt.start_submission(PromptSubmission::new("busy command"));
    let busy_host = build_prompt_host(&state);
    assert!(
        busy_host.footer_pills.iter().any(|pill| pill.label.contains("忙碌")),
        "busy mode has a distinct footer"
    );
}
