//! 新 TUI 的输入状态模型。
//!
//! 本模块只定义 prompt 的内部状态，不直接处理按键或终端事件。当前先收口：
//! - 输入值与 cursor
//! - 历史记录游标
//! - 忙碌期间的 queued commands
//! - 一次提交请求的最小生命周期模型

use crate::cli::tui::editor::{
    EditorCommand, EditorState, apply_editor_command, grapheme_count,
};
use crate::cli::tui::paste::PromptAttachment;
use crate::cli::tui::vim::VimMode;

use super::session_mode::SessionExecutionSnapshot;

/// 输入框当前所处的模式。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum PromptMode {
    #[default]
    Compose,
    SlashCommand,
    Bash,
    Search,
    QuestionReply,
    TodoEdit,
    CommandPalette,
    Busy,
}

/// 输入框 cursor 状态。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct PromptCursor {
    pub(crate) char_index: usize,
    pub(crate) preferred_column: Option<usize>,
}

impl PromptCursor {
    /// 把 cursor 放到给定文本末尾。
    pub(crate) fn at_end(value: &str) -> Self {
        Self { char_index: grapheme_count(value), preferred_column: None }
    }
}

/// 输入历史状态。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct PromptHistoryState {
    pub(crate) entries: Vec<String>,
    pub(crate) selected_index: Option<usize>,
    pub(crate) draft: Option<String>,
}

impl PromptHistoryState {
    /// 追加一条新的历史输入。
    ///
    /// 空白输入不会写入；与最后一条完全相同的输入也会被去重，避免无意义重复。
    pub(crate) fn push(&mut self, entry: impl Into<String>) {
        let entry = entry.into();
        let normalized = entry.trim();
        if normalized.is_empty() {
            self.reset_navigation();
            return;
        }
        if is_sensitive_history_entry(normalized) {
            self.reset_navigation();
            return;
        }

        self.entries.retain(|entry| entry != normalized);
        self.entries.push(normalized.to_string());
        self.reset_navigation();
    }

    pub(crate) fn select_previous(&mut self, current_value: &str) -> Option<String> {
        if self.entries.is_empty() {
            return None;
        }

        let next_index = match self.selected_index {
            Some(index) => index.saturating_sub(1),
            None => {
                self.draft = Some(current_value.to_string());
                self.entries.len().saturating_sub(1)
            }
        };
        self.selected_index = Some(next_index);
        self.entries.get(next_index).cloned()
    }

    pub(crate) fn select_next(&mut self) -> Option<String> {
        let selected_index = self.selected_index?;
        if selected_index + 1 < self.entries.len() {
            let next_index = selected_index + 1;
            self.selected_index = Some(next_index);
            return self.entries.get(next_index).cloned();
        }

        self.selected_index = None;
        Some(self.draft.take().unwrap_or_default())
    }

    pub(crate) fn reset_navigation(&mut self) {
        self.selected_index = None;
        self.draft = None;
    }
}

fn is_sensitive_history_entry(entry: &str) -> bool {
    let normalized = entry.to_ascii_lowercase();
    [
        "api_key",
        "api-key",
        "password",
        "passwd",
        "secret=",
        "token=",
        "authorization:",
        "/deny ",
        "拒绝原因",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

/// prompt 光标移动方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PromptMotion {
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
}

/// 忙碌期间排队等待重放的命令类型。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum QueuedPromptCommandKind {
    #[default]
    Submit,
    SlashCommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub(crate) enum QueuedPromptPriority {
    Now,
    NextTool,
    #[default]
    EndTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum QueuedPromptSource {
    #[default]
    User,
    Recovered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QueueDrainBoundary {
    Immediate,
    Tool,
    Turn,
}

/// 忙碌期间缓存的一条待执行命令。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct QueuedPromptCommand {
    pub(crate) raw: String,
    pub(crate) kind: QueuedPromptCommandKind,
    pub(crate) enqueued_ms: Option<u64>,
    pub(crate) priority: QueuedPromptPriority,
    pub(crate) source: QueuedPromptSource,
    pub(crate) session_id: Option<String>,
    pub(crate) attachments: Vec<PromptAttachment>,
    pub(crate) execution: Box<SessionExecutionSnapshot>,
}

/// 一次 prompt 提交请求的内部模型。
///
/// 该结构刻意与当前 legacy processor request 的稳定交集保持一致，
/// 让后续 cutover/shadow compare 能在不引入 transport 细节的前提下复用同一份提交语义。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptSubmission {
    pub(crate) stream_id: Option<u64>,
    pub(crate) session_id: Option<String>,
    pub(crate) text: String,
    pub(crate) root: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) execution: Box<SessionExecutionSnapshot>,
    pub(crate) history_len: usize,
    pub(crate) status: PromptSubmissionStatus,
}

impl PromptSubmission {
    /// 构造一条新的提交记录。
    pub(crate) fn new(text: impl Into<String>) -> Self {
        Self {
            stream_id: None,
            session_id: None,
            text: text.into(),
            root: None,
            model: None,
            execution: Box::new(SessionExecutionSnapshot::default()),
            history_len: 0,
            status: PromptSubmissionStatus::Pending,
        }
    }

    /// 绑定 stream ID。
    pub(crate) fn with_stream_id(mut self, stream_id: u64) -> Self {
        self.stream_id = Some(stream_id);
        self
    }

    /// 绑定 session ID。
    pub(crate) fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// 绑定 root 目录。
    pub(crate) fn with_root(mut self, root: impl Into<String>) -> Self {
        self.root = Some(root.into());
        self
    }

    /// 绑定模型名。
    pub(crate) fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// 记录提交时使用的历史长度。
    pub(crate) fn with_history_len(mut self, history_len: usize) -> Self {
        self.history_len = history_len;
        self
    }
}

/// 一次提交的终态。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum PromptSubmissionStatus {
    #[default]
    Pending,
    Streaming,
    Done {
        finish_reason: Option<String>,
    },
    Cancelled {
        reason: Option<String>,
    },
    TimedOut {
        message: String,
    },
    Error {
        message: String,
    },
}

/// Prompt 的顶层状态模型。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct PromptState {
    pub(crate) value: String,
    pub(crate) cursor: PromptCursor,
    pub(crate) history: PromptHistoryState,
    pub(crate) selected_suggestion_index: Option<usize>,
    pub(crate) queued_commands: Vec<QueuedPromptCommand>,
    pub(crate) mode: PromptMode,
    pub(crate) active_submission: Option<PromptSubmission>,
    pub(crate) last_submission: Option<PromptSubmission>,
    pub(crate) editor: EditorState,
    pub(crate) completion_hidden: bool,
    pub(crate) attachments: Vec<PromptAttachment>,
    pub(crate) focused_attachment: Option<usize>,
    pub(crate) next_attachment_id: usize,
    pub(crate) vim_mode: VimMode,
    pub(crate) stashed_draft: Option<String>,
}

impl PromptState {
    /// 基于初始值创建 prompt 状态。
    pub(crate) fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        Self { cursor: PromptCursor::at_end(&value), value, ..Self::default() }
    }

    /// 覆盖当前输入值，并把 cursor 移到末尾。
    pub(crate) fn set_value(&mut self, value: impl Into<String>) {
        self.value = value.into();
        self.cursor = PromptCursor::at_end(&self.value);
        self.editor = EditorState::default();
        self.completion_hidden = false;
        self.selected_suggestion_index = None;
        self.history.reset_navigation();
        self.refresh_mode_from_value();
    }

    pub(crate) fn insert_text(&mut self, text: &str) {
        self.selected_suggestion_index = None;
        self.completion_hidden = false;
        self.history.reset_navigation();
        apply_editor_command(self, EditorCommand::Insert(text.to_string()));
        self.refresh_mode_from_value();
    }

    pub(crate) fn backspace(&mut self) {
        self.selected_suggestion_index = None;
        self.completion_hidden = false;
        self.history.reset_navigation();
        apply_editor_command(self, EditorCommand::Backspace);
        self.refresh_mode_from_value();
    }

    pub(crate) fn delete(&mut self) {
        self.selected_suggestion_index = None;
        self.completion_hidden = false;
        self.history.reset_navigation();
        apply_editor_command(self, EditorCommand::Delete);
        self.refresh_mode_from_value();
    }

    /// 更新当前补全面板选中项。
    pub(crate) fn set_selected_suggestion_index(&mut self, selected_index: Option<usize>) {
        self.selected_suggestion_index = selected_index;
    }

    pub(crate) fn move_cursor(&mut self, motion: PromptMotion) -> bool {
        apply_editor_command(self, EditorCommand::Move { motion, select: false })
    }

    pub(crate) fn can_move_cursor(&self, motion: PromptMotion) -> bool {
        let mut clone = self.clone();
        apply_editor_command(&mut clone, EditorCommand::Move { motion, select: false })
    }

    pub(crate) fn select_previous_history(&mut self) -> bool {
        let Some(value) = self.history.select_previous(&self.value) else {
            return false;
        };

        self.replace_value_from_history(value);
        true
    }

    pub(crate) fn select_next_history(&mut self) -> bool {
        let Some(value) = self.history.select_next() else {
            return false;
        };

        self.replace_value_from_history(value);
        true
    }

    /// 开始一次新的提交。
    ///
    /// 当前规则与 legacy CLI 行为保持一致：
    /// - 将提交文本写入历史
    /// - 清空当前输入框
    /// - 切换到 busy 模式
    pub(crate) fn start_submission(&mut self, mut submission: PromptSubmission) {
        self.history.push(submission.text.clone());
        submission.status = PromptSubmissionStatus::Streaming;
        self.value.clear();
        self.cursor = PromptCursor::default();
        self.selected_suggestion_index = None;
        self.mode = PromptMode::Busy;
        self.active_submission = Some(submission);
    }

    /// 结束当前提交，并把终态沉淀为最近一次提交记录。
    pub(crate) fn finish_submission(&mut self, status: PromptSubmissionStatus) {
        if let Some(mut submission) = self.active_submission.take() {
            submission.status = status;
            self.last_submission = Some(submission);
        }
        self.selected_suggestion_index = None;
        self.mode = PromptMode::Compose;
    }

    /// 将一条命令放入 busy 队列。
    pub(crate) fn queue_command(&mut self, command: QueuedPromptCommand) {
        let insert_at = self
            .queued_commands
            .iter()
            .position(|queued| queued.priority > command.priority)
            .unwrap_or(self.queued_commands.len());
        self.queued_commands.insert(insert_at, command);
    }

    /// 取出下一条待执行命令。
    pub(crate) fn pop_queued_command(&mut self) -> Option<QueuedPromptCommand> {
        if self.queued_commands.is_empty() { None } else { Some(self.queued_commands.remove(0)) }
    }

    pub(crate) fn drain_queued_for(
        &mut self,
        boundary: QueueDrainBoundary,
        session_id: Option<&str>,
    ) -> Option<QueuedPromptCommand> {
        let max_priority = match boundary {
            QueueDrainBoundary::Immediate => QueuedPromptPriority::Now,
            QueueDrainBoundary::Tool => QueuedPromptPriority::NextTool,
            QueueDrainBoundary::Turn => QueuedPromptPriority::EndTurn,
        };
        let index = self.queued_commands.iter().position(|command| {
            command.priority <= max_priority && command.session_id.as_deref() == session_id
        })?;
        Some(self.queued_commands.remove(index))
    }

    pub(crate) fn reclaim_latest_queued(
        &mut self,
        session_id: Option<&str>,
    ) -> Option<QueuedPromptCommand> {
        let index = self
            .queued_commands
            .iter()
            .rposition(|command| command.session_id.as_deref() == session_id)?;
        self.queued_commands.remove(index).into()
    }

    pub(crate) fn stash_draft(&mut self) -> bool {
        if self.stashed_draft.is_some() || self.value.is_empty() {
            return false;
        }
        self.stashed_draft = Some(std::mem::take(&mut self.value));
        self.cursor = PromptCursor::default();
        true
    }

    pub(crate) fn restore_stashed_draft(&mut self) -> bool {
        if !self.value.is_empty() {
            return false;
        }
        let Some(draft) = self.stashed_draft.take() else {
            return false;
        };
        self.set_value(draft);
        true
    }

    /// 当前 prompt 是否处于 busy 状态。
    pub(crate) fn is_busy(&self) -> bool {
        matches!(self.mode, PromptMode::Busy)
    }

    fn replace_value_from_history(&mut self, value: String) {
        self.value = value;
        self.cursor = PromptCursor::at_end(&self.value);
        self.editor = EditorState::default();
        self.selected_suggestion_index = None;
        self.refresh_mode_from_value();
    }

    fn refresh_mode_from_value(&mut self) {
        match self.mode {
            PromptMode::Compose | PromptMode::SlashCommand | PromptMode::Bash => {
                self.mode = if self.value.starts_with('!') {
                    PromptMode::Bash
                } else if self.value.trim_start().starts_with('/') {
                    PromptMode::SlashCommand
                } else {
                    PromptMode::Compose
                };
            }
            PromptMode::Busy
            | PromptMode::Search
            | PromptMode::QuestionReply
            | PromptMode::TodoEdit
            | PromptMode::CommandPalette => {}
        }
    }
}
