//! 活动 TUI 宿主使用的输入事件到语义命令的最小边界。
//!
//! 控制器只负责键位路由和提交快照构造；状态转换仍通过 reducer 执行，
//! Gateway 请求和对话框副作则交给宿主与 bridge。

use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};

use super::commands::CommandRoute;
use super::model::{PermissionDecision, PromptSubmission, UiConfirmIntent, UiQuestionOverlay};
use super::state::{TuiAction, TuiState, reduce_tui_state};

/// 控制器可接收的时钟或终端事件。
#[derive(Debug)]
pub(crate) enum TuiControllerEvent {
    Tick,
    Terminal(CrosstermEvent),
}

/// 需要由宿主提交给对应运行时的对话框结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TuiOverlayCommand {
    ConfirmDecision {
        request_id: String,
        intent: UiConfirmIntent,
        confirmed: bool,
    },
    PermissionDecision { request_id: String, decision: PermissionDecision },
    QuestionSubmitted(UiQuestionOverlay),
    QuestionRejected(UiQuestionOverlay),
}

/// 输入路由后产生的显式宿主命令。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TuiControllerCommand {
    Continue,
    FileSearchQueryChanged { query: Option<String> },
    Quit,
    CancelActiveSubmission,
    QueuePrompt { text: String, attachments: Vec<vw_tui::V3Attachment> },
    OpenTranscript,
    OpenExternalEditor,
    Command(CommandRoute),
    Overlay(TuiOverlayCommand),
    SubmitPrompt(PromptSubmission),
}

/// 无持久状态的 TUI 输入控制器。
#[derive(Debug, Clone, Default)]
pub(crate) struct TuiController;

impl TuiController {
    /// 处理一个输入事件；未映射事件显式返回 `Continue`。
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

    fn handle_key_event(&mut self, state: &mut TuiState, key: KeyEvent) -> TuiControllerCommand {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Enter) => {
                build_prompt_submission(state, state.prompt.value.as_str())
                    .map(TuiControllerCommand::SubmitPrompt)
                    .unwrap_or(TuiControllerCommand::Continue)
            }
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

/// 从当前会话状态捕获不可变的提交快照；空白文本不产生提交。
pub(crate) fn build_prompt_submission(state: &TuiState, text: &str) -> Option<PromptSubmission> {
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

/// 允许纯附件输入的提交快照构造器。
pub(crate) fn build_attachment_submission(
    state: &TuiState,
    text: &str,
) -> PromptSubmission {
    let mut submission = PromptSubmission::new(text.trim())
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
    submission
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default()
}
