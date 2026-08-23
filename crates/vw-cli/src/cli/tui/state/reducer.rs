//! TUI 的 reducer/action 系统。
//!
//! 当前 reducer 只负责“状态怎么变”，不负责事件从哪里来：
//! - controller/runtime 后续只需要把输入归一为 `TuiAction`
//! - renderer 只读取 `TuiState` 与 selectors，不直接修改内部字段
//! - prompt 提交生命周期、assistant turn 终态、overlay 栈、question/todo 镜像都在这里收口

use std::path::PathBuf;

use vw_shared::session::ui_types::ChatSession;

use super::selectors::derive_search_matches_with_cache;
use super::{
    TuiModelCatalogEntry, TuiRuntimeState, TuiScrollState, TuiSessionPreview, TuiState,
    is_persistable_chat_message, persisted_slot_index_for_message_index,
};
use crate::cli::session::GitWorkspaceStatus;
use crate::cli::tui::model::{
    PromptMode, PromptMotion, PromptSubmission, PromptSubmissionStatus, QueuedPromptCommand,
    UiAssistantMessage, UiMessage, UiMessageBase, UiMessageId, UiOverlay, UiQuestionOverlay,
    UiSearchOverlay, UiStep, UiStepState, UiThinkingBlock, UiThinkingPhase, UiThinkingTiming,
    UiTodoOverlay, UiTokenUsage, UiToolCall, UiToolCallState, UiToolResult, UiTurnTerminal,
    UiUserMessage,
};

/// assistant turn 终态更新时附带的补充元数据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiTerminalUpdate {
    pub(crate) terminal: UiTurnTerminal,
    pub(crate) usage: Option<UiTokenUsage>,
    pub(crate) message_id: Option<String>,
    pub(crate) parent_message_id: Option<String>,
}

/// tool 调用消息的状态更新输入。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiToolCallUpdate {
    pub(crate) tool_name: String,
    pub(crate) summary: Option<String>,
    pub(crate) arguments: Option<String>,
    pub(crate) append_arguments: bool,
    pub(crate) state: UiToolCallState,
    pub(crate) result: Option<TuiToolResultUpdate>,
}

/// tool 结果消息的最小载荷。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiToolResultUpdate {
    pub(crate) content: String,
    pub(crate) is_error: bool,
}

/// TUI 状态层统一消费的动作枚举。
#[derive(Debug, Clone)]
pub(crate) enum TuiAction {
    ReplaceFromSnapshot {
        snapshot: ChatSession,
        scope: Option<String>,
        path: Option<PathBuf>,
    },
    ProjectWorkspaceRootSet(Option<PathBuf>),
    ProjectInfoSet(String),
    ProjectGitStatusSet(GitWorkspaceStatus),
    SessionPreviewSet(Option<TuiSessionPreview>),
    SessionTitleSet(String),
    SessionUpdatedMsSet(u64),
    SessionScopeSet(Option<String>),
    SessionPathSet(Option<PathBuf>),
    ModelCatalogReplaced(Vec<TuiModelCatalogEntry>),
    StatusProviderSet(Option<String>),
    StatusModelSet(Option<String>),
    StatusErrorSet(Option<String>),
    ScrollSet(TuiScrollState),
    PromptValueSet(String),
    PromptInsert(String),
    PromptBackspace,
    PromptDelete,
    PromptCursorMove(PromptMotion),
    PromptHistoryPrevious,
    PromptHistoryNext,
    PromptModeSet(PromptMode),
    PromptSuggestionSelectionSet(Option<usize>),
    PromptCommandQueued(QueuedPromptCommand),
    PromptSubmissionStarted(PromptSubmission),
    OverlayPushed(UiOverlay),
    OverlayPopped,
    OverlayCleared,
    SearchQuerySet(String),
    QuestionsReplaced(Vec<UiQuestionOverlay>),
    TodoOverlayReplaced(Option<UiTodoOverlay>),
    TaskSyncErrorSet(Option<String>),
    MessagePushed(UiMessage),
    ThinkingDeltaReceived(String),
    ThinkingClosed,
    ToolCallUpdated(TuiToolCallUpdate),
    AssistantDeltaReceived(String),
    StepStarted {
        step_index: u32,
        started_ms: u64,
        model: Option<String>,
    },
    StepFinished {
        step_index: u32,
        finished_ms: u64,
        usage: UiTokenUsage,
        finish_reason: Option<String>,
        model: Option<String>,
    },
    AssistantTerminalUpdated(TuiTerminalUpdate),
}

/// 原地更新一份 `TuiState`。
pub(crate) fn reduce_tui_state(state: &mut TuiState, action: TuiAction) {
    match action {
        TuiAction::ReplaceFromSnapshot { snapshot, scope, path } => {
            let mut project = state.project.clone();
            project.memory_evidence = None;
            let mut next = TuiState::from_chat_session(&snapshot);
            next.project = project;
            next.session.scope = scope;
            next.session.path = path;
            next.refresh_session_preview();
            *state = next;
        }
        TuiAction::ProjectWorkspaceRootSet(workspace_root) => {
            if state.project.workspace_root != workspace_root {
                state.project.memory_evidence = None;
            }
            state.project.workspace_root = workspace_root;
        }
        TuiAction::ProjectInfoSet(info) => {
            state.project.info = info;
        }
        TuiAction::ProjectGitStatusSet(git_status) => {
            state.project.git_status = git_status;
        }
        TuiAction::SessionPreviewSet(preview) => {
            state.session.preview = preview;
        }
        TuiAction::SessionTitleSet(title) => {
            state.session.title = title;
            state.refresh_session_preview();
        }
        TuiAction::SessionUpdatedMsSet(updated_ms) => {
            state.session.updated_ms = updated_ms;
            state.refresh_session_preview();
        }
        TuiAction::SessionScopeSet(scope) => {
            state.session.scope = scope;
        }
        TuiAction::SessionPathSet(path) => {
            state.session.path = path;
        }
        TuiAction::ModelCatalogReplaced(model_catalog) => {
            state.model_catalog = model_catalog;
        }
        TuiAction::StatusProviderSet(provider_name) => {
            state.status.provider_name = provider_name;
        }
        TuiAction::StatusModelSet(model_name) => {
            state.status.model_name = model_name;
        }
        TuiAction::StatusErrorSet(error) => {
            state.status.last_error = error;
        }
        TuiAction::ScrollSet(scroll) => {
            if state.scroll.follow_tail && !scroll.follow_tail {
                state.unseen_baseline_id =
                    state.messages.last().map(|message| message.base().id.clone());
            }
            state.scroll = scroll;
            state.clamp_scroll();
        }
        TuiAction::PromptValueSet(value) => {
            state.prompt.set_value(value);
            refresh_search_overlays(state);
        }
        TuiAction::PromptInsert(text) => {
            state.prompt.insert_text(&text);
        }
        TuiAction::PromptBackspace => {
            state.prompt.backspace();
        }
        TuiAction::PromptDelete => {
            state.prompt.delete();
        }
        TuiAction::PromptCursorMove(motion) => {
            state.prompt.move_cursor(motion);
        }
        TuiAction::PromptHistoryPrevious => {
            state.prompt.select_previous_history();
        }
        TuiAction::PromptHistoryNext => {
            state.prompt.select_next_history();
        }
        TuiAction::PromptModeSet(mode) => {
            state.prompt.mode = mode;
        }
        TuiAction::PromptSuggestionSelectionSet(selected_index) => {
            state.prompt.set_selected_suggestion_index(selected_index);
        }
        TuiAction::PromptCommandQueued(command) => {
            state.prompt.queue_command(command);
        }
        TuiAction::PromptSubmissionStarted(submission) => {
            start_submission(state, submission);
            refresh_search_overlays(state);
        }
        TuiAction::OverlayPushed(mut overlay) => {
            if let UiOverlay::Search(search) = &mut overlay {
                refresh_search_overlay_matches(&state.messages, &state.search_index, search);
            }
            update_memory_evidence_from_overlay(state, Some(&overlay));
            state.overlays.push(overlay);
        }
        TuiAction::OverlayPopped => {
            let active_overlay = state.overlays.active().cloned();
            update_memory_evidence_from_overlay(state, active_overlay.as_ref());
            state.overlays.pop();
        }
        TuiAction::OverlayCleared => {
            let active_overlay = state.overlays.active().cloned();
            update_memory_evidence_from_overlay(state, active_overlay.as_ref());
            state.overlays.clear();
        }
        TuiAction::SearchQuerySet(query) => {
            set_search_query(state, query);
        }
        TuiAction::QuestionsReplaced(questions) => {
            state.tasks.pending_questions = questions;
        }
        TuiAction::TodoOverlayReplaced(todo_overlay) => {
            state.tasks.todo_overlay = todo_overlay;
        }
        TuiAction::TaskSyncErrorSet(error) => {
            state.tasks.sync_error = error.clone();
            state.status.last_error = error;
        }
        TuiAction::MessagePushed(message) => {
            state.append_message(message);
            state.clamp_scroll();
            refresh_search_overlays(state);
        }
        TuiAction::ThinkingDeltaReceived(delta) => {
            append_thinking_delta(state, delta);
            refresh_search_overlays(state);
        }
        TuiAction::ThinkingClosed => {
            close_thinking_block(state);
            refresh_search_overlays(state);
        }
        TuiAction::ToolCallUpdated(update) => {
            apply_tool_call_update(state, update);
            refresh_search_overlays(state);
        }
        TuiAction::AssistantDeltaReceived(delta) => {
            append_assistant_delta(state, delta);
            refresh_search_overlays(state);
        }
        TuiAction::StepStarted { step_index, started_ms, model } => {
            start_step(state, step_index, started_ms, model);
            refresh_search_overlays(state);
        }
        TuiAction::StepFinished { step_index, finished_ms, usage, finish_reason, model } => {
            finish_step(state, step_index, finished_ms, usage, finish_reason, model);
            refresh_search_overlays(state);
        }
        TuiAction::AssistantTerminalUpdated(update) => {
            apply_assistant_terminal_update(state, update);
            refresh_search_overlays(state);
        }
    }
}

fn update_memory_evidence_from_overlay(state: &mut TuiState, overlay: Option<&UiOverlay>) {
    if let Some(UiOverlay::Memory(memory)) = overlay {
        state.project.memory_evidence = memory.entries.get(memory.selected_index).cloned();
    }
}

fn start_submission(state: &mut TuiState, submission: PromptSubmission) {
    let submission_text = submission.text.clone();
    let stream_id = submission.stream_id;
    state.runtime = TuiRuntimeState::default();

    if let Some(model) = submission.model.clone() {
        state.status.model_name = Some(model);
    }
    state.status.turn_terminal = UiTurnTerminal::Streaming;
    state.status.last_error = None;
    state.prompt.start_submission(submission);

    if let Some(stream_id) = stream_id {
        state.session.updated_ms = stream_id;
    }

    let mut base = next_local_base(state, "user");
    if let Some(created_ms) = stream_id {
        base = base.with_created_ms(created_ms);
    }

    state.append_message(UiMessage::User(UiUserMessage { base, text: submission_text }));
    state.clamp_scroll();
}

fn append_assistant_delta(state: &mut TuiState, delta: String) {
    let assistant_index = ensure_streaming_assistant_message(state);
    if let Some(UiMessage::Assistant(message)) = state.messages.get_mut(assistant_index) {
        message.text.push_str(&delta);
        message.terminal = UiTurnTerminal::Streaming;
        message.model = state.status.model_name.clone();
    }
    state.refresh_search_index_for_message(assistant_index);
    state.refresh_transcript_layout_for_message(assistant_index);
    state.status.turn_terminal = UiTurnTerminal::Streaming;
    state.clamp_scroll();
}

fn append_thinking_delta(state: &mut TuiState, delta: String) {
    let updated_ms = event_time_ms(state);
    if let Some((block_index, block)) =
        state.messages.iter_mut().enumerate().rev().find_map(|(index, message)| match message {
            UiMessage::Thinking(block)
                if block.timing.last().is_some_and(|timing| timing.end_ms.is_none()) =>
            {
                Some((index, block))
            }
            _ => None,
        })
    {
        block.content.push_str(&delta);
        refresh_thinking_block(block, updated_ms);
        state.refresh_search_index_for_message(block_index);
        state.refresh_transcript_layout_for_message(block_index);
        state.clamp_scroll();
        return;
    }

    let mut base = next_local_base(state, "thinking");
    if let Some(parent_id) = current_turn_parent_id(state) {
        base.parent_id = Some(parent_id);
    }

    state.append_message(UiMessage::Thinking(UiThinkingBlock {
        base,
        summary: None,
        content: delta,
        timing: vec![UiThinkingTiming {
            start_ms: updated_ms,
            end_ms: None,
            last_update_ms: updated_ms,
        }],
        collapsed: false,
        phase: UiThinkingPhase::Streaming,
    }));

    if let Some(UiMessage::Thinking(block)) = state.messages.last_mut() {
        refresh_thinking_block(block, updated_ms);
    }

    let thinking_index = state.messages.len().saturating_sub(1);
    state.refresh_search_index_for_message(thinking_index);
    state.refresh_transcript_layout_for_message(thinking_index);

    state.clamp_scroll();
}

fn close_thinking_block(state: &mut TuiState) {
    let finished_ms = event_time_ms(state);
    if let Some((block_index, block)) =
        state.messages.iter_mut().enumerate().rev().find_map(|(index, message)| match message {
            UiMessage::Thinking(block)
                if block.timing.last().is_some_and(|timing| timing.end_ms.is_none()) =>
            {
                Some((index, block))
            }
            _ => None,
        })
    {
        if let Some(timing) = block.timing.last_mut() {
            timing.end_ms = Some(finished_ms);
            timing.last_update_ms = finished_ms;
        }
        block.summary = thinking_summary(&block.content);
        block.collapsed = true;
        block.phase = UiThinkingPhase::Closed;
        state.refresh_search_index_for_message(block_index);
        state.refresh_transcript_layout_for_message(block_index);
    }
}

fn apply_tool_call_update(state: &mut TuiState, update: TuiToolCallUpdate) {
    let TuiToolCallUpdate {
        tool_name,
        summary,
        arguments,
        append_arguments,
        state: next_state,
        result,
    } = update;

    let parent_id = current_turn_parent_id(state);
    let call_index = find_open_tool_call_index(&state.messages, tool_name.as_str());
    let call_message_id = if let Some(index) = call_index {
        let Some(UiMessage::ToolCall(message)) = state.messages.get_mut(index) else {
            unreachable!("tool call index must point to tool message");
        };

        if let Some(summary) = summary.clone() {
            message.summary = Some(summary);
        }
        if let Some(arguments) = arguments.clone() {
            if append_arguments {
                message.arguments.get_or_insert_with(String::new).push_str(&arguments);
            } else {
                message.arguments = Some(arguments);
            }
        }
        if message.base.parent_id.is_none() {
            message.base.parent_id = parent_id.clone();
        }
        message.state = next_state.clone();
        message.base.id.clone()
    } else {
        let mut base = next_local_base(state, &format!("tool-{}", tool_name));
        if let Some(parent_id) = parent_id {
            base.parent_id = Some(parent_id);
        }

        state.append_message(UiMessage::ToolCall(UiToolCall {
            base,
            call_id: None,
            tool_name: tool_name.clone(),
            summary: summary.clone(),
            arguments: arguments.clone(),
            state: next_state.clone(),
        }));

        state
            .messages
            .last()
            .map(UiMessage::id)
            .cloned()
            .unwrap_or_else(|| UiMessageId::local(format!("tool-{}", state.messages.len())))
    };

    if let Some(index) = call_index {
        state.refresh_search_index_for_message(index);
        state.refresh_transcript_layout_for_message(index);
    }

    if let Some(result) = result {
        let mut base = next_local_base(state, &format!("tool-result-{}", tool_name));
        base.parent_id = Some(call_message_id);
        state.append_message(UiMessage::ToolResult(UiToolResult {
            base,
            call_id: None,
            tool_name,
            content: result.content,
            is_error: result.is_error,
        }));
    }

    state.clamp_scroll();
}

fn start_step(state: &mut TuiState, step_index: u32, started_ms: u64, model: Option<String>) {
    let assistant_index = ensure_streaming_assistant_message(state);
    let assistant_id = state.messages[assistant_index].id().clone();

    if let Some(model) = model.clone() {
        state.status.model_name = Some(model.clone());
        if let Some(UiMessage::Assistant(message)) = state.messages.get_mut(assistant_index) {
            message.model = Some(model);
        }
    }

    if let Some(UiMessage::Assistant(message)) = state.messages.get_mut(assistant_index) {
        message.step_count = message.step_count.saturating_add(1);
        message.terminal = UiTurnTerminal::Streaming;
    }

    if let Some(step) = find_active_step_message_mut(&mut state.messages, step_index) {
        step.started_ms = started_ms;
        step.finished_ms = None;
        step.model = state.status.model_name.clone();
        step.state = UiStepState::Running;
        if let Some(step_index) = find_step_message_index(&state.messages, step_index) {
            state.refresh_search_index_for_message(step_index);
            state.refresh_transcript_layout_for_message(step_index);
        }
        return;
    }

    let mut base = next_local_base(state, &format!("step-{step_index}"));
    base.parent_id = Some(assistant_id);
    base.created_ms = Some(started_ms);
    state.append_message(UiMessage::Step(UiStep {
        base,
        step_index,
        started_ms,
        finished_ms: None,
        usage: UiTokenUsage::default(),
        finish_reason: None,
        model: state.status.model_name.clone(),
        state: UiStepState::Running,
    }));
    state.status.turn_terminal = UiTurnTerminal::Streaming;
    state.clamp_scroll();
}

fn finish_step(
    state: &mut TuiState,
    step_index: u32,
    finished_ms: u64,
    usage: UiTokenUsage,
    finish_reason: Option<String>,
    model: Option<String>,
) {
    let assistant_index = ensure_streaming_assistant_message(state);
    if let Some(step) = find_active_step_message_mut(&mut state.messages, step_index) {
        step.finished_ms = Some(finished_ms);
        step.usage = usage.clone();
        step.finish_reason = finish_reason.clone();
        step.model = model.clone().or_else(|| step.model.clone());
        step.state = step_state_from_finish_reason(step.finish_reason.as_deref());
        if let Some(step_index) = find_step_message_index(&state.messages, step_index) {
            state.refresh_search_index_for_message(step_index);
            state.refresh_transcript_layout_for_message(step_index);
        }
    } else {
        let assistant_id = state.messages[assistant_index].id().clone();
        let mut base = next_local_base(state, &format!("step-{step_index}"));
        base.parent_id = Some(assistant_id);
        base.created_ms = Some(finished_ms);
        state.append_message(UiMessage::Step(UiStep {
            base,
            step_index,
            started_ms: finished_ms,
            finished_ms: Some(finished_ms),
            usage: usage.clone(),
            finish_reason: finish_reason.clone(),
            model: model.clone(),
            state: step_state_from_finish_reason(finish_reason.as_deref()),
        }));
    }

    if let Some(UiMessage::Assistant(message)) = state.messages.get_mut(assistant_index) {
        add_usage(&mut message.usage, &usage);
        if let Some(model) = model {
            message.model = Some(model.clone());
            state.status.model_name = Some(model);
        }
    }
}

fn apply_assistant_terminal_update(state: &mut TuiState, update: TuiTerminalUpdate) {
    let assistant_index = ensure_streaming_assistant_message(state);
    let assistant_slot_index =
        persisted_slot_index_for_message_index(&state.messages, assistant_index);

    if let Some(UiMessage::Assistant(message)) = state.messages.get_mut(assistant_index) {
        message.terminal = update.terminal.clone();
        if let Some(usage) = update.usage.as_ref() {
            message.usage = usage.clone();
        }
        if let Some(parent_message_id) = update.parent_message_id.as_ref() {
            message.base.parent_id = Some(UiMessageId::gateway(parent_message_id.clone()));
        }
        if let Some(message_id) = update.message_id.as_ref() {
            message.base.id = UiMessageId::gateway(message_id.clone());
        }
    }

    if let (Some(slot_index), Some(message_id)) = (assistant_slot_index, update.message_id.as_ref())
        && let Some(metadata) = state.session.persisted_messages.get_mut(slot_index)
    {
        metadata.raw_message_id = Some(message_id.clone());
    }

    state.status.turn_terminal = update.terminal.clone();
    state.status.last_error = terminal_error_message(&update.terminal);

    if let Some(prompt_status) = prompt_status_from_terminal(&update.terminal) {
        state.prompt.finish_submission(prompt_status);
    }

    state.refresh_transcript_layout_for_message(assistant_index);

    state.clamp_scroll();
}

fn set_search_query(state: &mut TuiState, query: String) {
    if let Some(UiOverlay::Search(search)) = state.overlays.stack.last_mut() {
        search.query = query;
        refresh_search_overlay_matches(&state.messages, &state.search_index, search);
        return;
    }

    let mut overlay = UiSearchOverlay { query, ..UiSearchOverlay::default() };
    refresh_search_overlay_matches(&state.messages, &state.search_index, &mut overlay);
    state.overlays.push(UiOverlay::Search(overlay));
}

fn refresh_search_overlays(state: &mut TuiState) {
    for overlay in &mut state.overlays.stack {
        if let UiOverlay::Search(search) = overlay {
            refresh_search_overlay_matches(&state.messages, &state.search_index, search);
        }
    }
}

fn refresh_search_overlay_matches(
    messages: &[UiMessage],
    search_index: &super::TuiSearchTextCache,
    search: &mut UiSearchOverlay,
) {
    search.matches = derive_search_matches_with_cache(
        messages,
        search_index,
        &search.query,
        search.case_sensitive,
    );
    search.selected_index = if search.matches.is_empty() {
        None
    } else {
        Some(search.selected_index.unwrap_or(0).min(search.matches.len().saturating_sub(1)))
    };
}

fn find_step_message_index(messages: &[UiMessage], target_step_index: u32) -> Option<usize> {
    messages.iter().enumerate().rev().find_map(|(index, message)| match message {
        UiMessage::Step(step) if step.step_index == target_step_index => Some(index),
        _ => None,
    })
}

fn ensure_streaming_assistant_message(state: &mut TuiState) -> usize {
    if let Some((assistant_index, _)) = last_assistant_indices(state)
        && let Some(UiMessage::Assistant(message)) = state.messages.get(assistant_index)
        && matches!(message.terminal, UiTurnTerminal::Pending | UiTurnTerminal::Streaming)
    {
        return assistant_index;
    }

    let mut base = next_local_base(state, "assistant");
    if let Some(user_id) = state.messages.iter().rev().find_map(last_user_message_id) {
        base.parent_id = Some(user_id);
    }

    state.append_message(UiMessage::Assistant(UiAssistantMessage {
        base,
        text: String::new(),
        usage: UiTokenUsage::default(),
        step_count: 0,
        terminal: UiTurnTerminal::Streaming,
        model: state.status.model_name.clone(),
    }));
    state.messages.len().saturating_sub(1)
}

fn find_open_tool_call_index(messages: &[UiMessage], tool_name: &str) -> Option<usize> {
    messages.iter().enumerate().rev().find_map(|(index, message)| {
        let UiMessage::ToolCall(tool_call) = message else {
            return None;
        };

        (tool_call.tool_name == tool_name
            && matches!(tool_call.state, UiToolCallState::Queued | UiToolCallState::Running))
        .then_some(index)
    })
}

fn current_turn_parent_id(state: &TuiState) -> Option<UiMessageId> {
    state.messages.iter().rev().find_map(|message| match message {
        UiMessage::Assistant(message)
            if matches!(message.terminal, UiTurnTerminal::Pending | UiTurnTerminal::Streaming) =>
        {
            Some(message.base.id.clone())
        }
        UiMessage::User(message) => Some(message.base.id.clone()),
        _ => None,
    })
}

fn refresh_thinking_block(block: &mut UiThinkingBlock, updated_ms: u64) {
    if let Some(timing) = block.timing.last_mut() {
        timing.last_update_ms = updated_ms;
    }
    block.summary = thinking_summary(&block.content);
}

pub(super) fn thinking_summary(content: &str) -> Option<String> {
    let first_line = content.lines().map(str::trim).find(|line| !line.is_empty())?;

    Some(truncate_summary(first_line, 96))
}

fn truncate_summary(value: &str, max_chars: usize) -> String {
    let mut summary = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        summary.push_str("...");
    }
    summary
}

fn find_active_step_message_mut(
    messages: &mut [UiMessage],
    step_index: u32,
) -> Option<&mut UiStep> {
    messages.iter_mut().rev().find_map(|message| match message {
        UiMessage::Step(step)
            if step.step_index == step_index
                && !matches!(
                    step.state,
                    UiStepState::Complete | UiStepState::Cancelled | UiStepState::Failed
                ) =>
        {
            Some(step)
        }
        _ => None,
    })
}

fn next_local_base(state: &TuiState, seed: &str) -> UiMessageBase {
    let mut base =
        UiMessageBase::new(UiMessageId::local(format!("{seed}-{}", state.messages.len())));
    if let Some(session_id) = state.session.session_id.as_deref() {
        base = base.with_session_id(session_id);
    }
    base
}

fn last_assistant_indices(state: &TuiState) -> Option<(usize, usize)> {
    let mut persisted_index = 0usize;
    let mut result = None;

    for (index, message) in state.messages.iter().enumerate() {
        if let UiMessage::Assistant(_) = message {
            result = Some((index, persisted_index));
        }
        if is_persistable_chat_message(message) {
            persisted_index = persisted_index.saturating_add(1);
        }
    }

    result
}

fn last_user_message_id(message: &UiMessage) -> Option<UiMessageId> {
    match message {
        UiMessage::User(_) => Some(message.id().clone()),
        _ => None,
    }
}

fn add_usage(target: &mut UiTokenUsage, usage: &UiTokenUsage) {
    target.input_tokens = target.input_tokens.saturating_add(usage.input_tokens);
    target.output_tokens = target.output_tokens.saturating_add(usage.output_tokens);
    target.cached_tokens = target.cached_tokens.saturating_add(usage.cached_tokens);
    target.reasoning_tokens = target.reasoning_tokens.saturating_add(usage.reasoning_tokens);
}

fn step_state_from_finish_reason(finish_reason: Option<&str>) -> UiStepState {
    if contains_marker(finish_reason, &["cancelled", "canceled", "aborted"]) {
        return UiStepState::Cancelled;
    }
    if contains_marker(finish_reason, &["error", "failed", "failure"]) {
        return UiStepState::Failed;
    }
    UiStepState::Complete
}

fn prompt_status_from_terminal(terminal: &UiTurnTerminal) -> Option<PromptSubmissionStatus> {
    match terminal {
        UiTurnTerminal::Pending | UiTurnTerminal::Streaming => None,
        UiTurnTerminal::Done { finish_reason } => {
            Some(PromptSubmissionStatus::Done { finish_reason: finish_reason.clone() })
        }
        UiTurnTerminal::Cancelled { reason } => {
            Some(PromptSubmissionStatus::Cancelled { reason: reason.clone() })
        }
        UiTurnTerminal::TimedOut { message } => {
            Some(PromptSubmissionStatus::TimedOut { message: message.clone() })
        }
        UiTurnTerminal::Error { message } => {
            Some(PromptSubmissionStatus::Error { message: message.clone() })
        }
    }
}

fn terminal_error_message(terminal: &UiTurnTerminal) -> Option<String> {
    match terminal {
        UiTurnTerminal::Pending | UiTurnTerminal::Streaming | UiTurnTerminal::Done { .. } => None,
        UiTurnTerminal::Cancelled { reason } => {
            Some(reason.clone().unwrap_or_else(|| "session cancelled".to_string()))
        }
        UiTurnTerminal::TimedOut { message } | UiTurnTerminal::Error { message } => {
            Some(message.clone())
        }
    }
}

fn contains_marker(value: Option<&str>, markers: &[&str]) -> bool {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return false;
    };

    let normalized = value.to_ascii_lowercase();
    markers.iter().any(|marker| normalized.contains(marker))
}

fn event_time_ms(state: &TuiState) -> u64 {
    state.runtime.event_time_ms.unwrap_or_else(now_ms)
}

fn now_ms() -> u64 {
    u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    )
    .unwrap_or(u64::MAX)
}
