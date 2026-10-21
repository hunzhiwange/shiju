//! 与 UI 解耦的流式文本帧调度器。
//!
//! 调度器只负责有界地释放事件；调用者负责提供单调递增的毫秒时间戳，并把输出应用到 UI。

use std::collections::{HashMap, VecDeque};

const ACTIVE_FRAME_INTERVAL_MS: u64 = 16;
const BACKGROUND_FRAME_INTERVAL_MS: u64 = 100;
const TARGET_CHARS_PER_FRAME: usize = 24;
const MAX_BACKLOG_MS: u64 = 500;
const MAX_CATCHUP_FRAMES: usize = 8;

/// 标识一条会话内的活动流，避免不同回复共享缓冲区。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct StreamItemKey {
    session_id: String,
    item_id: String,
}

impl StreamItemKey {
    pub(crate) fn new(session_id: impl Into<String>, item_id: impl Into<String>) -> Self {
        Self { session_id: session_id.into(), item_id: item_id.into() }
    }
}

/// 必须等候先前文本排空的终态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TerminalEvent {
    Completed,
    Cancelled,
    Error(String),
}

/// 进入调度器的事件。替换事件保留结构，不与文本拼接。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StreamDelta {
    Text(String),
    Replacement { target: String, content: String },
    Terminal(TerminalEvent),
}

/// 已满足释放条件、可由调用者按顺序应用的事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScheduledDelta {
    Text(String),
    Replacement { target: String, content: String },
    Terminal(TerminalEvent),
}

#[derive(Debug)]
enum PendingDelta {
    Text(TextBuffer),
    Replacement { target: String, content: String },
    Terminal(TerminalEvent),
}

#[derive(Debug)]
struct TextBuffer {
    content: String,
    byte_offset: usize,
}

impl TextBuffer {
    fn new(content: String) -> Self {
        Self { content, byte_offset: 0 }
    }

    fn append(&mut self, text: &str) {
        self.content.push_str(text);
    }

    fn take_chars(&mut self, limit: usize) -> String {
        let remaining = &self.content[self.byte_offset..];
        let end = remaining
            .char_indices()
            .nth(limit)
            .map_or(self.content.len(), |(index, _)| self.byte_offset + index);
        let output = self.content[self.byte_offset..end].to_owned();
        self.byte_offset = end;
        output
    }

    fn is_empty(&self) -> bool {
        self.byte_offset == self.content.len()
    }
}

#[derive(Debug, Default)]
struct ItemState {
    pending: VecDeque<PendingDelta>,
    last_text_release_ms: Option<u64>,
}

/// 按会话和 item 隔离缓冲区的确定性调度状态机。
#[derive(Debug, Default)]
pub(crate) struct DeltaScheduler {
    items: HashMap<StreamItemKey, ItemState>,
}

impl DeltaScheduler {
    pub(crate) fn push(&mut self, key: StreamItemKey, delta: StreamDelta, _now_ms: u64) {
        let state = self.items.entry(key).or_default();
        match delta {
            StreamDelta::Text(text) if text.is_empty() => {}
            StreamDelta::Text(text) => match state.pending.back_mut() {
                Some(PendingDelta::Text(buffer)) => buffer.append(&text),
                _ => state.pending.push_back(PendingDelta::Text(TextBuffer::new(text))),
            },
            StreamDelta::Replacement { target, content } => {
                state.pending.push_back(PendingDelta::Replacement { target, content })
            }
            StreamDelta::Terminal(terminal) => {
                state.pending.push_back(PendingDelta::Terminal(terminal));
            }
        }
    }

    pub(crate) fn has_pending(&self, key: &StreamItemKey) -> bool {
        self.items.get(key).is_some_and(|state| !state.pending.is_empty())
    }

    pub(crate) fn drain(
        &mut self,
        key: &StreamItemKey,
        now_ms: u64,
        is_active: bool,
    ) -> Vec<ScheduledDelta> {
        let Some(state) = self.items.get_mut(key) else {
            return Vec::new();
        };
        let interval_ms =
            if is_active { ACTIVE_FRAME_INTERVAL_MS } else { BACKGROUND_FRAME_INTERVAL_MS };
        let text_budget = match state.last_text_release_ms {
            None => TARGET_CHARS_PER_FRAME,
            Some(last_release_ms) => {
                let elapsed_ms = now_ms.saturating_sub(last_release_ms);
                if elapsed_ms < interval_ms {
                    0
                } else {
                    let catchup_frames = (elapsed_ms.min(MAX_BACKLOG_MS) / interval_ms)
                        .clamp(1, MAX_CATCHUP_FRAMES as u64)
                        as usize;
                    TARGET_CHARS_PER_FRAME * catchup_frames
                }
            }
        };

        let mut remaining_budget = text_budget;
        let mut output = Vec::new();
        loop {
            match state.pending.front_mut() {
                Some(PendingDelta::Text(_)) if remaining_budget == 0 => break,
                Some(PendingDelta::Text(buffer)) => {
                    let text = buffer.take_chars(remaining_budget);
                    let char_count = text.chars().count();
                    remaining_budget -= char_count;
                    if !text.is_empty() {
                        output.push(ScheduledDelta::Text(text));
                        state.last_text_release_ms = Some(now_ms);
                    }
                    if buffer.is_empty() {
                        state.pending.pop_front();
                    }
                }
                Some(PendingDelta::Replacement { .. }) => {
                    let Some(PendingDelta::Replacement { target, content }) =
                        state.pending.pop_front()
                    else {
                        unreachable!();
                    };
                    output.push(ScheduledDelta::Replacement { target, content });
                }
                Some(PendingDelta::Terminal(_)) => {
                    let Some(PendingDelta::Terminal(terminal)) = state.pending.pop_front() else {
                        unreachable!();
                    };
                    output.push(ScheduledDelta::Terminal(terminal));
                }
                None => break,
            }
        }

        if state.pending.is_empty() {
            self.items.remove(key);
        }
        output
    }
}
