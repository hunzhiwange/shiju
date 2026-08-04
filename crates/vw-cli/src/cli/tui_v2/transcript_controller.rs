use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::controller::TuiControllerCommand;
use super::model::{
    TranscriptEffectIntent, TranscriptModeState, TranscriptScrollRestore, UiMessage,
};
use super::state::TuiState;

pub(crate) fn handle_transcript_mode_key(
    state: &mut TuiState,
    key: KeyEvent,
) -> Option<TuiControllerCommand> {
    if !state.transcript_mode.active {
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('o'))
        {
            enter_transcript_mode(state);
            return Some(TuiControllerCommand::Continue);
        }
        return None;
    }

    if state.transcript_mode.search_bar_open {
        return Some(handle_search_key(state, key));
    }

    if key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char('c'))
        || matches!(key.code, KeyCode::Esc | KeyCode::Char('q'))
    {
        exit_transcript_mode(state);
        return Some(TuiControllerCommand::Continue);
    }

    let page = state.transcript_mode.viewport_rows.max(1);
    match key.code {
        KeyCode::Char('/') => {
            let mode = &mut state.transcript_mode;
            mode.search_bar_open = true;
            mode.search_draft = mode.search_query.clone();
            mode.search_restore_top = mode.top_line;
            TuiControllerCommand::Continue
        }
        KeyCode::Char('n') => {
            move_search_match(&mut state.transcript_mode, 1);
            TuiControllerCommand::Continue
        }
        KeyCode::Char('N') => {
            move_search_match(&mut state.transcript_mode, -1);
            TuiControllerCommand::Continue
        }
        KeyCode::Char('g') | KeyCode::Home => {
            state.transcript_mode.top_line = 0;
            TuiControllerCommand::Continue
        }
        KeyCode::Char('G') | KeyCode::End => {
            state.transcript_mode.show_all = true;
            state.transcript_mode.top_line = state.transcript_mode.max_top_line();
            TuiControllerCommand::Continue
        }
        KeyCode::Char('j') | KeyCode::Down => {
            scroll_lines(&mut state.transcript_mode, 1);
            TuiControllerCommand::Continue
        }
        KeyCode::Char('k') | KeyCode::Up => {
            scroll_lines(&mut state.transcript_mode, -1);
            TuiControllerCommand::Continue
        }
        KeyCode::PageDown => {
            scroll_lines(&mut state.transcript_mode, page as isize);
            TuiControllerCommand::Continue
        }
        KeyCode::PageUp => {
            scroll_lines(&mut state.transcript_mode, -(page as isize));
            TuiControllerCommand::Continue
        }
        KeyCode::Char('[') => export_effect(&state.transcript_mode, false),
        KeyCode::Char('v') => export_effect(&state.transcript_mode, true),
        _ => TuiControllerCommand::Continue,
    }
    .into()
}

pub(crate) fn resize_transcript_mode(state: &mut TuiState, viewport_rows: u16) {
    if state.transcript_mode.active {
        state.transcript_mode.viewport_rows = usize::from(viewport_rows.max(1));
        state.transcript_mode.clamp_top_line();
    }
}

pub(super) fn scroll_transcript_mode(state: &mut TuiState, delta: isize) {
    scroll_lines(&mut state.transcript_mode, delta);
}

pub(super) fn seek_transcript_scrollbar(state: &mut TuiState, top_line: usize) {
    state.transcript_mode.top_line = top_line;
    state.transcript_mode.clamp_top_line();
}

fn enter_transcript_mode(state: &mut TuiState) {
    state.transcript_mode = TranscriptModeState {
        active: true,
        lines: transcript_plain_text_lines(&state.messages),
        viewport_rows: usize::from(state.scroll.viewport_height.max(1)),
        restore: Some(TranscriptScrollRestore {
            top_message: state.scroll.top_message,
            follow_tail: state.scroll.follow_tail,
        }),
        ..TranscriptModeState::default()
    };
}

fn exit_transcript_mode(state: &mut TuiState) {
    let restore = state.transcript_mode.restore;
    state.transcript_mode = TranscriptModeState::default();
    if let Some(restore) = restore {
        state.scroll.top_message = restore.top_message;
        state.scroll.follow_tail = restore.follow_tail;
        state.clamp_scroll();
    }
}

fn handle_search_key(state: &mut TuiState, key: KeyEvent) -> TuiControllerCommand {
    match key.code {
        KeyCode::Esc => {
            let mode = &mut state.transcript_mode;
            mode.search_bar_open = false;
            mode.search_draft = mode.search_query.clone();
            mode.top_line = mode.search_restore_top;
        }
        KeyCode::Enter => {
            let mode = &mut state.transcript_mode;
            mode.search_bar_open = false;
            mode.search_query = mode.search_draft.clone();
            refresh_search_matches(mode);
        }
        KeyCode::Backspace => {
            state.transcript_mode.search_draft.pop();
            preview_search(&mut state.transcript_mode);
        }
        KeyCode::Char(ch)
            if !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            state.transcript_mode.search_draft.push(ch);
            preview_search(&mut state.transcript_mode);
        }
        _ => {}
    }
    TuiControllerCommand::Continue
}

fn preview_search(mode: &mut TranscriptModeState) {
    let query = mode.search_draft.to_lowercase();
    if query.is_empty() {
        mode.top_line = mode.search_restore_top;
        return;
    }
    if let Some(line) = mode.lines.iter().position(|line| line.to_lowercase().contains(&query)) {
        mode.top_line = line;
        mode.clamp_top_line();
    }
}

fn refresh_search_matches(mode: &mut TranscriptModeState) {
    let query = mode.search_query.to_lowercase();
    mode.search_matches = if query.is_empty() {
        Vec::new()
    } else {
        mode.lines
            .iter()
            .enumerate()
            .filter_map(|(index, line)| line.to_lowercase().contains(&query).then_some(index))
            .collect()
    };
    mode.selected_match = (!mode.search_matches.is_empty()).then_some(0);
    if let Some(line) = mode.search_matches.first().copied() {
        mode.top_line = line.min(mode.max_top_line());
    }
}

fn move_search_match(mode: &mut TranscriptModeState, delta: isize) {
    if mode.search_matches.is_empty() {
        return;
    }
    let current = mode.selected_match.unwrap_or_default();
    let len = mode.search_matches.len();
    let next = if delta < 0 {
        current.checked_sub(delta.unsigned_abs()).unwrap_or(len.saturating_sub(1))
    } else {
        current.saturating_add(delta.cast_unsigned()) % len
    };
    mode.selected_match = Some(next);
    mode.top_line = mode.search_matches[next].min(mode.max_top_line());
}

fn scroll_lines(mode: &mut TranscriptModeState, delta: isize) {
    mode.top_line = if delta < 0 {
        mode.top_line.saturating_sub(delta.unsigned_abs())
    } else {
        mode.top_line.saturating_add(delta.cast_unsigned()).min(mode.max_top_line())
    };
}

fn export_effect(
    mode: &TranscriptModeState,
    open_editor: bool,
) -> TuiControllerCommand {
    match write_private_transcript(mode.lines.join("\n").as_str()) {
        Ok(path) if open_editor => {
            TuiControllerCommand::TranscriptEffect(TranscriptEffectIntent::OpenExternalEditor {
                path,
            })
        }
        Ok(path) => {
            TuiControllerCommand::TranscriptEffect(TranscriptEffectIntent::Dumped { path })
        }
        Err(error) => TuiControllerCommand::TranscriptEffect(
            TranscriptEffectIntent::Dumped { path: PathBuf::from(format!("ERROR:{error}")) },
        ),
    }
}

fn write_private_transcript(text: &str) -> std::io::Result<PathBuf> {
    let nonce =
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let path = std::env::temp_dir().join(format!(
        "vibewindow-transcript-{}-{nonce}.txt",
        std::process::id()
    ));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options.open(&path)?;
    file.write_all(text.as_bytes())?;
    Ok(path)
}

pub(crate) fn transcript_plain_text_lines(messages: &[UiMessage]) -> Vec<String> {
    messages
        .iter()
        .filter_map(|message| {
            let line = match message {
                UiMessage::User(message) => format!("❯ {}", message.text),
                UiMessage::Assistant(message) if !message.text.trim().is_empty() => {
                    format!("● {}", message.text)
                }
                UiMessage::Assistant(_) => return None,
                UiMessage::ToolCall(message) => format!(
                    "TOOL {} {}",
                    message.tool_name,
                    message.summary.as_deref().unwrap_or("")
                ),
                UiMessage::ToolResult(message) => {
                    format!("RESULT {} {}", message.tool_name, message.content)
                }
                UiMessage::Thinking(message) => format!(
                    "THINK {}",
                    message.summary.as_deref().unwrap_or("[hidden reasoning]")
                ),
                UiMessage::Step(message) => format!("STEP #{} {:?}", message.step_index, message.state),
                UiMessage::System(message) => format!("SYSTEM {}", message.text),
                UiMessage::Error(message) => format!("ERROR {}", message.message),
                UiMessage::Attachment(message) => format!("ATTACH {}", message.name),
                UiMessage::Progress(message) => format!("PROGRESS {}", message.label),
                UiMessage::CompactBoundary(message) => format!("COMPACT {}", message.summary),
                UiMessage::Unknown(message) => format!("UNKNOWN {}", message.summary),
            };
            Some(sanitize_visible_line(line.as_str()))
        })
        .collect()
}

fn sanitize_visible_line(line: &str) -> String {
    line.chars()
        .map(|ch| if ch.is_control() && ch != '\t' { '�' } else { ch })
        .collect()
}
