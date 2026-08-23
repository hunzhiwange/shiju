//! Minimal input-to-command boundary used by the active TUI host.

use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};

use super::model::{PermissionDecision, PromptSubmission, UiQuestionOverlay};
use super::state::{TuiAction, TuiState, reduce_tui_state};

#[derive(Debug)]
pub(crate) enum TuiControllerEvent {
    Tick,
    Terminal(CrosstermEvent),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TuiOverlayCommand {
    PermissionDecision(PermissionDecision),
    QuestionSubmitted(UiQuestionOverlay),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TuiControllerCommand {
    Continue,
    Quit,
    CancelActiveSubmission,
    Overlay(TuiOverlayCommand),
    SubmitPrompt(PromptSubmission),
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TuiController;

impl TuiController {
    pub(crate) fn handle_event(
        &mut self,
        state: &mut TuiState,
        event: TuiControllerEvent,
    ) -> TuiControllerCommand {
        match event {
            TuiControllerEvent::Terminal(CrosstermEvent::Key(key)) => {
                self.handle_key_event(state, key)
            }
            TuiControllerEvent::Tick | TuiControllerEvent::Terminal(_) => {
                TuiControllerCommand::Continue
            }
        }
    }

    fn handle_key_event(
        &mut self,
        state: &mut TuiState,
        key: KeyEvent,
    ) -> TuiControllerCommand {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Enter) => build_prompt_submission(
                state,
                state.prompt.value.as_str(),
            )
            .map(TuiControllerCommand::SubmitPrompt)
            .unwrap_or(TuiControllerCommand::Continue),
            (KeyModifiers::NONE, KeyCode::Up) => {
                reduce_tui_state(state, TuiAction::PromptHistoryPrevious);
                TuiControllerCommand::Continue
            }
            (KeyModifiers::NONE, KeyCode::Down) => {
                reduce_tui_state(state, TuiAction::PromptHistoryNext);
                TuiControllerCommand::Continue
            }
            _ => TuiControllerCommand::Continue,
        }
    }
}

pub(crate) fn build_prompt_submission(
    state: &TuiState,
    text: &str,
) -> Option<PromptSubmission> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return None;
    }

    let mut submission = PromptSubmission::new(text)
        .with_stream_id(now_ms())
        .with_history_len(state.prompt.history.entries.len());
    submission.execution = Box::new(state.session_options.execution_snapshot());

    if let Some(session_id) = state.session.session_id.as_ref() {
        submission = submission.with_session_id(session_id.clone());
    }
    if let Some(root) = state.project.workspace_root.as_ref() {
        submission = submission.with_root(root.display().to_string());
    }
    if let Some(model) = state.status.model_name.as_ref() {
        submission = submission.with_model(model.clone());
    }
    Some(submission)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default()
}
