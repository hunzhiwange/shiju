use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::controller::TuiControllerCommand;
use super::editor::{EditorCommand, apply_editor_command};
use super::model::PromptMotion;
use super::paste::create_prompt_editor_intent;
use super::state::TuiState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum VimMode {
    #[default]
    Disabled,
    Normal,
    Insert,
}

pub(crate) fn handle_vim_key(
    state: &mut TuiState,
    key: KeyEvent,
) -> Option<TuiControllerCommand> {
    if key.modifiers.contains(KeyModifiers::ALT) && key.code == KeyCode::Char('v') {
        state.prompt.vim_mode = match state.prompt.vim_mode {
            VimMode::Disabled => VimMode::Normal,
            VimMode::Normal | VimMode::Insert => VimMode::Disabled,
        };
        return Some(TuiControllerCommand::Continue);
    }
    match state.prompt.vim_mode {
        VimMode::Disabled => None,
        VimMode::Insert => {
            if key.code == KeyCode::Esc {
                state.prompt.vim_mode = VimMode::Normal;
                Some(TuiControllerCommand::Continue)
            } else {
                None
            }
        }
        VimMode::Normal => Some(handle_normal_key(state, key)),
    }
}

fn handle_normal_key(state: &mut TuiState, key: KeyEvent) -> TuiControllerCommand {
    let command = match key.code {
        KeyCode::Char('i') => {
            state.prompt.vim_mode = VimMode::Insert;
            return TuiControllerCommand::Continue;
        }
        KeyCode::Char('a') => {
            apply_editor_command(
                &mut state.prompt,
                EditorCommand::Move { motion: PromptMotion::Right, select: false },
            );
            state.prompt.vim_mode = VimMode::Insert;
            return TuiControllerCommand::Continue;
        }
        KeyCode::Char('h') | KeyCode::Left => {
            EditorCommand::Move { motion: PromptMotion::Left, select: false }
        }
        KeyCode::Char('l') | KeyCode::Right => {
            EditorCommand::Move { motion: PromptMotion::Right, select: false }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            EditorCommand::Move { motion: PromptMotion::Down, select: false }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            EditorCommand::Move { motion: PromptMotion::Up, select: false }
        }
        KeyCode::Char('0') | KeyCode::Home => {
            EditorCommand::Move { motion: PromptMotion::Home, select: false }
        }
        KeyCode::Char('$') | KeyCode::End => {
            EditorCommand::Move { motion: PromptMotion::End, select: false }
        }
        KeyCode::Char('w') => EditorCommand::MoveWord { direction: 1, select: false },
        KeyCode::Char('b') => EditorCommand::MoveWord { direction: -1, select: false },
        KeyCode::Char('x') => EditorCommand::Delete,
        KeyCode::Char('u') => EditorCommand::Undo,
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => EditorCommand::Redo,
        KeyCode::Char('v') => {
            return match create_prompt_editor_intent(&state.prompt) {
                Ok(intent) => TuiControllerCommand::PromptExternalEditor(intent),
                Err(_) => TuiControllerCommand::Continue,
            };
        }
        _ => return TuiControllerCommand::Continue,
    };
    apply_editor_command(&mut state.prompt, command);
    TuiControllerCommand::Continue
}
