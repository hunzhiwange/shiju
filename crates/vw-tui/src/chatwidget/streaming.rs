//! Incremental thinking and answer cells for the copied `ChatWidget` surface.
//! 本模块负责增量推理与回答单元。
//! ChatWidget 仅维护展示状态、历史、输入队列与覆盖层；运行时业务由外层系统负责。

use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// 描述 `StreamKind` 在当前界面流程中传递的数据。
enum StreamKind {
    Thinking,
    Answer,
}

#[derive(Debug, Default)]
/// 描述 `StreamingState` 在本模块状态转换中传递的数据。
pub(super) struct StreamingState {
    kind: Option<StreamKind>,
    text: String,
}

impl ChatWidget {
    pub(super) fn on_thinking_delta(&mut self, turn_id: &str, delta: String) {
        if !self.accept_turn_event(turn_id, "thinking delta") {
            return;
        }
        self.commit_exec_group();
        self.append_stream_delta(StreamKind::Thinking, delta);
    }

    pub(super) fn on_text_delta(&mut self, turn_id: &str, delta: String) {
        if !self.accept_turn_event(turn_id, "text delta") {
            return;
        }
        self.commit_exec_group();
        self.append_stream_delta(StreamKind::Answer, delta);
    }

    fn append_stream_delta(&mut self, kind: StreamKind, delta: String) {
        if self.streaming.kind.is_some_and(|current| current != kind) {
            self.commit_stream_segment();
        }
        self.streaming.kind = Some(kind);
        self.streaming.text.push_str(&delta);
        let event = match kind {
            StreamKind::Thinking => V3DisplayEvent::Thinking {
                text: self.streaming.text.clone(),
            },
            StreamKind::Answer => V3DisplayEvent::Assistant {
                markdown: self.streaming.text.clone(),
            },
        };
        self.transcript.set_active_event(Some(event));
        self.transcript.set_live_tail_event(None);
    }

    pub(super) fn commit_stream_segment(&mut self) {
        if self.streaming.kind.take().is_some() {
            if !self.streaming.text.is_empty() {
                self.transcript.commit_active_cell();
            } else {
                self.transcript.set_active_event(None);
            }
            self.streaming.text.clear();
        }
        self.transcript.set_live_tail_event(None);
    }
}
