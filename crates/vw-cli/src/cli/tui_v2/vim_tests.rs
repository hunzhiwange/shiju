use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::controller::TuiControllerCommand;
use super::state::TuiState;
use super::vim::{VimMode, handle_vim_key};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn vim_normal_insert_and_external_editor_intent_preserve_draft() {
    let mut state = TuiState::default();
    state.prompt.set_value("hello");

    handle_vim_key(&mut state, KeyEvent::new(KeyCode::Char('v'), KeyModifiers::ALT));
    assert_eq!(state.prompt.vim_mode, VimMode::Normal);
    handle_vim_key(&mut state, key(KeyCode::Char('h')));
    assert_eq!(state.prompt.cursor.char_index, 4);
    handle_vim_key(&mut state, key(KeyCode::Char('i')));
    assert_eq!(state.prompt.vim_mode, VimMode::Insert);
    handle_vim_key(&mut state, key(KeyCode::Esc));
    assert_eq!(state.prompt.vim_mode, VimMode::Normal);

    let command = handle_vim_key(&mut state, key(KeyCode::Char('v'))).unwrap();
    let TuiControllerCommand::PromptExternalEditor(intent) = command else {
        panic!("expected prompt editor intent");
    };
    assert_eq!(intent.original_draft, "hello");
    assert_eq!(std::fs::read_to_string(&intent.path).unwrap(), "hello");
    std::fs::remove_file(intent.path).unwrap();
}
