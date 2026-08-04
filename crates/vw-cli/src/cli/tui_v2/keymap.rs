//! Prompt 与 transcript scroll 键位层。
//!
//! 本模块只把已归一化的按键映射为状态动作或 controller 命令，不读取磁盘配置。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::commands::registry::CommandExecutionPolicy;
use super::completion::slash_palette_overlay;
use super::controller::{TuiControllerCommand, TuiOverlayCommand, build_prompt_submission, now_ms};
use super::editor::{EditorCommand, apply_editor_command};
use super::input::{
    TuiPromptSuggestionMotion, TuiSlashCommandInvocation, apply_selected_suggestion,
    is_file_picker_suggestions, move_prompt_suggestion_selection, parse_slash_command,
    prompt_suggestions, selected_prompt_suggestion,
};
use super::model::{
    PromptMode, PromptMotion, QueuedPromptCommand, QueuedPromptCommandKind, UiErrorOverlay,
    UiOverlay,
};
use super::paste::{delete_focused_attachment, move_attachment_focus};
use super::state::{
    TuiAction, TuiScrollState, TuiState, reduce_tui_state, select_transcript_message_anchors,
    select_visible_grouped_transcript_window,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum KeymapLayerResult {
    Handled(TuiControllerCommand),
    Pass,
}

fn can_edit_prompt(modifiers: KeyModifiers, state: &TuiState) -> bool {
    state.overlays.active().is_none()
        && matches!(modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT)
}

fn append_prompt_char(state: &mut TuiState, ch: char) {
    reduce_tui_state(state, TuiAction::PromptInsert(ch.to_string()));
}

fn backspace_prompt(state: &mut TuiState) {
    reduce_tui_state(state, TuiAction::PromptBackspace);
}

fn insert_prompt_newline(state: &mut TuiState) {
    reduce_tui_state(state, TuiAction::PromptInsert("\n".to_string()));
}

fn delete_prompt(state: &mut TuiState) {
    reduce_tui_state(state, TuiAction::PromptDelete);
}

fn move_prompt_cursor(state: &mut TuiState, motion: PromptMotion) {
    reduce_tui_state(state, TuiAction::PromptCursorMove(motion));
}

fn prompt_history_previous(state: &mut TuiState) {
    reduce_tui_state(state, TuiAction::PromptHistoryPrevious);
}

fn prompt_history_next(state: &mut TuiState) {
    reduce_tui_state(state, TuiAction::PromptHistoryNext);
}

fn accept_prompt_suggestion(state: &mut TuiState) {
    if let Some(replacement) = apply_selected_suggestion(state) {
        reduce_tui_state(state, TuiAction::PromptValueSet(replacement));
    }
}

fn try_accept_prompt_suggestion(state: &mut TuiState) -> bool {
    if state.prompt.is_busy() {
        return false;
    }

    let Some(replacement) = apply_selected_suggestion(state) else {
        return false;
    };
    reduce_tui_state(state, TuiAction::PromptValueSet(replacement));
    true
}

fn can_navigate_prompt_suggestions(state: &TuiState) -> bool {
    selected_prompt_suggestion(state).is_some()
}

fn can_page_prompt_suggestions(state: &TuiState) -> bool {
    is_file_picker_suggestions(&prompt_suggestions(state))
}

fn move_prompt_suggestion_cursor(state: &mut TuiState, motion: TuiPromptSuggestionMotion) {
    if let Some(next_index) = move_prompt_suggestion_selection(state, motion) {
        reduce_tui_state(state, TuiAction::PromptSuggestionSelectionSet(Some(next_index)));
    }
}

fn submit_prompt_command(state: &TuiState) -> TuiControllerCommand {
    build_prompt_submission(state, state.prompt.value.as_str())
        .map(TuiControllerCommand::SubmitPrompt)
        .unwrap_or(TuiControllerCommand::Continue)
}

fn queue_prompt_command(state: &mut TuiState) {
    let text = state.prompt.value.trim().to_string();
    if text.is_empty() {
        return;
    }

    let kind = if text.starts_with('/') {
        QueuedPromptCommandKind::SlashCommand
    } else {
        QueuedPromptCommandKind::Submit
    };

    reduce_tui_state(
        state,
        TuiAction::PromptCommandQueued(QueuedPromptCommand {
            raw: text,
            kind,
            enqueued_ms: Some(now_ms()),
            session_id: state.session.session_id.clone(),
            attachments: state.prompt.attachments.clone(),
            execution: Box::new(state.session_options.execution_snapshot()),
            ..QueuedPromptCommand::default()
        }),
    );
    reduce_tui_state(state, TuiAction::PromptValueSet(String::new()));
}

fn submit_or_execute_prompt_command(state: &mut TuiState) -> TuiControllerCommand {
    let text = state.prompt.value.trim().to_string();
    if text.is_empty() {
        return TuiControllerCommand::Continue;
    }

    if state.prompt.is_busy() {
        if text.starts_with('/') {
            let invocation =
                parse_slash_command(state, text.as_str()).unwrap_or(TuiSlashCommandInvocation {
                    raw: text.clone(),
                    token: text.trim_start_matches('/').to_string(),
                    argument: None,
                    kind: None,
                    action: None,
                    execution_policy: None,
                });
            if invocation.execution_policy == Some(CommandExecutionPolicy::Immediate) {
                reduce_tui_state(state, TuiAction::PromptValueSet(String::new()));
                return TuiControllerCommand::ExecuteSlashCommand(invocation);
            }
        }
        queue_prompt_command(state);
        return TuiControllerCommand::Continue;
    }

    if matches!(state.prompt.mode, PromptMode::SlashCommand) || text.starts_with('/') {
        let invocation =
            parse_slash_command(state, text.as_str()).unwrap_or(TuiSlashCommandInvocation {
                raw: text.clone(),
                token: text.trim_start_matches('/').to_string(),
                argument: None,
                kind: None,
                action: None,
                execution_policy: None,
            });
        reduce_tui_state(state, TuiAction::PromptValueSet(String::new()));
        return TuiControllerCommand::ExecuteSlashCommand(invocation);
    }

    submit_prompt_command(state)
}

fn toggle_help_overlay(state: &mut TuiState) {
    if state.overlays.active().is_some() {
        reduce_tui_state(state, TuiAction::OverlayPopped);
        return;
    }

    reduce_tui_state(
        state,
        TuiAction::OverlayPushed(UiOverlay::Error(UiErrorOverlay {
            title: "TUI 输入帮助".to_string(),
            message: "区域：顶部会话栏 / 对话流 / 状态栏 / 输入区 / 弹层。\n按键：直接输入即可编辑，Shift+Enter 换行，Enter 发送；斜杠建议面板出现时可用 Up/Down 切换、Tab 或 Enter 接受当前项，接受后再按 Enter 执行命令；输入 `$` 选择技能，`/skills` 管理平台与启停；Ctrl+F 打开搜索，F2 打开问题或授权面板，F3 打开待办面板，F4 打开任务面板，F5 打开 MCP 服务器面板，F6 打开内存文件面板，PageUp/PageDown 滚动会话，Esc 关闭弹层或清空输入，Ctrl+C 复制输入选区（无选区时退出），Ctrl+D 退出。\n弹层按键：确认弹层用 Enter/Esc，搜索弹层支持文字输入 + Backspace/Delete + Tab/Shift+Tab + Enter + End/u + Ctrl+S，提问弹层支持 1-9 + Tab + Enter + Ctrl+R，待办弹层支持 Up/Down + Space + s/r + Esc，任务弹层支持 Up/Down + Enter + Esc，MCP/内存弹层支持 Up/Down + Esc。".to_string(),
            recoverable: true,
        })),
    );
}

pub(super) fn handle_prompt_layer_key(state: &mut TuiState, key: KeyEvent) -> KeymapLayerResult {
    if state.overlays.active().is_some() {
        return KeymapLayerResult::Pass;
    }

    match (key.modifiers, key.code) {
        (_, KeyCode::Esc) if !prompt_suggestions(state).is_empty() => {
            state.prompt.completion_hidden = true;
            state.prompt.set_selected_suggestion_index(None);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Esc) => KeymapLayerResult::Handled(handle_prompt_escape(state)),
        (KeyModifiers::CONTROL, KeyCode::Char('f')) => KeymapLayerResult::Handled(
            TuiControllerCommand::Overlay(TuiOverlayCommand::OpenSearchOverlay),
        ),
        (_, KeyCode::F(2)) => KeymapLayerResult::Handled(TuiControllerCommand::Overlay(
            TuiOverlayCommand::OpenPendingQuestions,
        )),
        (_, KeyCode::F(3)) => KeymapLayerResult::Handled(TuiControllerCommand::Overlay(
            TuiOverlayCommand::OpenTodoPanel,
        )),
        (_, KeyCode::F(4)) => KeymapLayerResult::Handled(TuiControllerCommand::Overlay(
            TuiOverlayCommand::OpenTaskPanel,
        )),
        (_, KeyCode::F(5)) => KeymapLayerResult::Handled(TuiControllerCommand::Overlay(
            TuiOverlayCommand::OpenMcpPanel,
        )),
        (_, KeyCode::F(6)) => KeymapLayerResult::Handled(TuiControllerCommand::Overlay(
            TuiOverlayCommand::OpenMemoryPanel,
        )),
        (_, KeyCode::F(1) | KeyCode::Char('?')) => {
            toggle_help_overlay(state);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (
            KeyModifiers::SHIFT,
            code
            @ (KeyCode::Left
            | KeyCode::Right
            | KeyCode::Up
            | KeyCode::Down
            | KeyCode::Home
            | KeyCode::End),
        ) => {
            let motion = match code {
                KeyCode::Left => PromptMotion::Left,
                KeyCode::Right => PromptMotion::Right,
                KeyCode::Up => PromptMotion::Up,
                KeyCode::Down => PromptMotion::Down,
                KeyCode::Home => PromptMotion::Home,
                KeyCode::End => PromptMotion::End,
                _ => unreachable!("selection motion is constrained by the pattern"),
            };
            apply_editor_command(&mut state.prompt, EditorCommand::Move { motion, select: true });
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::BackTab | KeyCode::Up) | (KeyModifiers::CONTROL, KeyCode::Char('p'))
            if can_navigate_prompt_suggestions(state) =>
        {
            move_prompt_suggestion_cursor(state, TuiPromptSuggestionMotion::Previous);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::PageUp) if can_page_prompt_suggestions(state) => {
            move_prompt_suggestion_cursor(state, TuiPromptSuggestionMotion::PreviousPage);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::PageDown) if can_page_prompt_suggestions(state) => {
            move_prompt_suggestion_cursor(state, TuiPromptSuggestionMotion::NextPage);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (KeyModifiers::SHIFT, KeyCode::Enter) => {
            insert_prompt_newline(state);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (KeyModifiers::CONTROL, KeyCode::Char('z')) => {
            apply_editor_command(&mut state.prompt, EditorCommand::Undo);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (modifiers, KeyCode::Char('Z'))
            if modifiers.contains(KeyModifiers::CONTROL | KeyModifiers::SHIFT) =>
        {
            apply_editor_command(&mut state.prompt, EditorCommand::Redo);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (KeyModifiers::CONTROL, KeyCode::Char('a')) => {
            apply_editor_command(&mut state.prompt, EditorCommand::SelectAll);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (KeyModifiers::CONTROL, KeyCode::Char('w')) => {
            apply_editor_command(&mut state.prompt, EditorCommand::DeleteWordBackward);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (KeyModifiers::CONTROL, KeyCode::Char('k')) => {
            apply_editor_command(&mut state.prompt, EditorCommand::KillLine);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (KeyModifiers::CONTROL, KeyCode::Char('y')) => {
            apply_editor_command(&mut state.prompt, EditorCommand::Yank);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (KeyModifiers::ALT, KeyCode::Left) if !state.prompt.attachments.is_empty() => {
            move_attachment_focus(&mut state.prompt, -1);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (KeyModifiers::ALT, KeyCode::Right) if !state.prompt.attachments.is_empty() => {
            move_attachment_focus(&mut state.prompt, 1);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (KeyModifiers::ALT, KeyCode::Backspace) if !state.prompt.attachments.is_empty() => {
            delete_focused_attachment(&mut state.prompt);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (KeyModifiers::ALT, KeyCode::Left) => {
            apply_editor_command(
                &mut state.prompt,
                EditorCommand::MoveWord { direction: -1, select: false },
            );
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (KeyModifiers::ALT, KeyCode::Right) => {
            apply_editor_command(
                &mut state.prompt,
                EditorCommand::MoveWord { direction: 1, select: false },
            );
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Tab) => {
            accept_prompt_suggestion(state);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Enter) if try_accept_prompt_suggestion(state) => {
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Enter) => KeymapLayerResult::Handled(submit_or_execute_prompt_command(state)),
        (_, KeyCode::Backspace) => {
            backspace_prompt(state);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Delete) => {
            delete_prompt(state);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Left) if state.prompt.can_move_cursor(PromptMotion::Left) => {
            move_prompt_cursor(state, PromptMotion::Left);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Right) if state.prompt.can_move_cursor(PromptMotion::Right) => {
            move_prompt_cursor(state, PromptMotion::Right);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Home) if !state.prompt.value.is_empty() => {
            move_prompt_cursor(state, PromptMotion::Home);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::End) if !state.prompt.value.is_empty() => {
            move_prompt_cursor(state, PromptMotion::End);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Down) | (KeyModifiers::CONTROL, KeyCode::Char('n'))
            if can_navigate_prompt_suggestions(state) =>
        {
            move_prompt_suggestion_cursor(state, TuiPromptSuggestionMotion::Next);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Up) if state.prompt.can_move_cursor(PromptMotion::Up) => {
            move_prompt_cursor(state, PromptMotion::Up);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Down) if state.prompt.can_move_cursor(PromptMotion::Down) => {
            move_prompt_cursor(state, PromptMotion::Down);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Up) | (KeyModifiers::CONTROL, KeyCode::Char('p'))
            if !state.prompt.history.entries.is_empty() =>
        {
            prompt_history_previous(state);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Down) | (KeyModifiers::CONTROL, KeyCode::Char('n'))
            if state.prompt.history.selected_index.is_some() =>
        {
            prompt_history_next(state);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        (_, KeyCode::Char(ch)) if can_edit_prompt(key.modifiers, state) => {
            let opens_slash_palette = ch == '/' && state.prompt.value.is_empty();
            append_prompt_char(state, ch);
            if opens_slash_palette {
                let overlay = slash_palette_overlay(&state.command_registry, "");
                reduce_tui_state(
                    state,
                    TuiAction::OverlayPushed(UiOverlay::CommandPalette(overlay)),
                );
            }
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        _ => KeymapLayerResult::Pass,
    }
}

pub(super) fn handle_scroll_layer_key(state: &mut TuiState, key: KeyEvent) -> KeymapLayerResult {
    match key.code {
        KeyCode::Up => {
            scroll_relative(state, -1);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        KeyCode::Down => {
            scroll_relative(state, 1);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        KeyCode::PageUp => {
            scroll_relative(state, -page_scroll_amount(state));
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        KeyCode::PageDown => {
            scroll_relative(state, page_scroll_amount(state));
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        KeyCode::Home => {
            scroll_to_top(state);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        KeyCode::End => {
            scroll_to_tail(state);
            KeymapLayerResult::Handled(TuiControllerCommand::Continue)
        }
        _ => KeymapLayerResult::Pass,
    }
}

fn handle_prompt_escape(state: &mut TuiState) -> TuiControllerCommand {
    if !state.prompt.value.is_empty() {
        reduce_tui_state(state, TuiAction::PromptValueSet(String::new()));
        return TuiControllerCommand::Continue;
    }

    if state.prompt.is_busy() {
        return TuiControllerCommand::CancelActiveSubmission;
    }

    // 有会话内容时弹二次确认，否则直接退出
    if state.messages.is_empty() && state.session.session_id.is_none() {
        return TuiControllerCommand::Quit;
    }
    TuiControllerCommand::Overlay(TuiOverlayCommand::ConfirmExit)
}

fn page_scroll_amount(state: &TuiState) -> isize {
    select_visible_grouped_transcript_window(state).viewport_summary().message_capacity.max(1)
        as isize
}

pub(super) fn scroll_relative(state: &mut TuiState, delta: isize) {
    let anchors = select_transcript_message_anchors(state);
    if anchors.is_empty() {
        return;
    }

    let current_anchor_index =
        anchors.iter().rposition(|anchor| *anchor <= state.scroll.top_message).unwrap_or_default();
    let max_top = anchors.len().saturating_sub(1);
    let mut scroll = state.scroll.clone();
    let next_top = offset_index(current_anchor_index, delta).min(max_top);
    scroll.top_message = anchors[next_top];
    scroll.follow_tail = next_top >= max_top;
    reduce_tui_state(state, TuiAction::ScrollSet(scroll));
}

pub(super) fn seek_main_scrollbar(
    state: &mut TuiState,
    position: usize,
    max_position: usize,
) {
    if position >= max_position {
        scroll_to_tail(state);
        return;
    }

    let anchors = select_transcript_message_anchors(state);
    let Some(anchor) = anchors.get(position.min(max_position)).copied() else {
        return;
    };
    let mut scroll = state.scroll.clone();
    scroll.top_message = anchor;
    scroll.follow_tail = false;
    reduce_tui_state(state, TuiAction::ScrollSet(scroll));
}

fn scroll_to_top(state: &mut TuiState) {
    let mut scroll = TuiScrollState { top_message: 0, follow_tail: false, ..state.scroll.clone() };
    scroll.follow_tail = false;
    reduce_tui_state(state, TuiAction::ScrollSet(scroll));
}

pub(super) fn scroll_to_tail(state: &mut TuiState) {
    let anchors = select_transcript_message_anchors(state);
    let mut scroll = state.scroll.clone();
    scroll.follow_tail = true;
    scroll.top_message = anchors.last().copied().unwrap_or_default();
    reduce_tui_state(state, TuiAction::ScrollSet(scroll));
}

fn offset_index(current: usize, delta: isize) -> usize {
    if delta < 0 {
        current.saturating_sub(delta.unsigned_abs())
    } else {
        current.saturating_add(delta.cast_unsigned())
    }
}
