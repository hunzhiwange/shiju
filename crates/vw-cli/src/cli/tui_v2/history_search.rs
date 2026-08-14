use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::controller::TuiControllerCommand;
use super::state::TuiState;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct HistorySearchState {
    pub(crate) active: bool,
    pub(crate) query: String,
    pub(crate) matches: Vec<usize>,
    pub(crate) selected_index: Option<usize>,
    pub(crate) original_draft: String,
}

pub(crate) fn handle_history_search_key(
    state: &mut TuiState,
    key: KeyEvent,
) -> Option<TuiControllerCommand> {
    if !state.history_search.active {
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('r')) {
            state.history_search = HistorySearchState {
                active: true,
                original_draft: state.prompt.value.clone(),
                ..HistorySearchState::default()
            };
            refresh_matches(state);
            return Some(TuiControllerCommand::Continue);
        }
        return None;
    }

    match key.code {
        KeyCode::Esc => {
            let draft = state.history_search.original_draft.clone();
            state.history_search = HistorySearchState::default();
            state.prompt.set_value(draft);
        }
        KeyCode::Enter => {
            if let Some(value) = selected_history_value(state) {
                state.prompt.set_value(value);
            }
            state.history_search = HistorySearchState::default();
        }
        KeyCode::Up => move_selection(state, -1),
        KeyCode::Down => move_selection(state, 1),
        KeyCode::Backspace => {
            state.history_search.query.pop();
            refresh_matches(state);
        }
        KeyCode::Char(ch)
            if !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            state.history_search.query.push(ch);
            refresh_matches(state);
        }
        _ => {}
    }
    Some(TuiControllerCommand::Continue)
}

pub(crate) fn selected_history_value(state: &TuiState) -> Option<String> {
    let match_index = state.history_search.selected_index?;
    let entry_index = *state.history_search.matches.get(match_index)?;
    state.prompt.history.entries.get(entry_index).cloned()
}

fn refresh_matches(state: &mut TuiState) {
    let query = state.history_search.query.to_lowercase();
    state.history_search.matches = state
        .prompt
        .history
        .entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            (query.is_empty() || entry.to_lowercase().contains(&query)).then_some(index)
        })
        .collect();
    state.history_search.selected_index = (!state.history_search.matches.is_empty())
        .then_some(state.history_search.matches.len().saturating_sub(1));
}

fn move_selection(state: &mut TuiState, delta: isize) {
    let len = state.history_search.matches.len();
    if len == 0 {
        return;
    }
    let current = state.history_search.selected_index.unwrap_or(len.saturating_sub(1));
    let next = if delta < 0 {
        current.saturating_sub(delta.unsigned_abs())
    } else {
        current.saturating_add(delta.cast_unsigned()).min(len.saturating_sub(1))
    };
    state.history_search.selected_index = Some(next);
}
