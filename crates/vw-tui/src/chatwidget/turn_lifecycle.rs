//! Product-neutral agent-turn lifecycle state for `ChatWidget`.
//! 本模块负责产品无关的代理轮次状态机。
//! ChatWidget 仅维护展示状态、历史、输入队列与覆盖层；运行时业务由外层系统负责。

use std::collections::HashSet;
use std::time::Duration;
use std::time::Instant;

use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// 表示 `TurnStartOutcome` 对应状态转换的处理结果。
pub(super) enum TurnStartOutcome {
    Started,
    Duplicate,
    ActiveConflict,
    AlreadyTerminal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// 描述 `TurnEventOutcome` 在本模块状态转换中传递的数据。
pub(super) enum TurnEventOutcome {
    Ready,
    MissingBegin,
    ActiveConflict,
    AlreadyTerminal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// 表示 `TurnFinishOutcome` 对应状态转换的处理结果。
pub(super) enum TurnFinishOutcome {
    Finished,
    MissingBegin,
    ActiveConflict,
    DuplicateTerminal,
}

#[derive(Debug, Default)]
pub(super) struct TurnLifecycleState {
    active_turn_id: Option<String>,
    active_started_at: Option<Instant>,
    terminal_turn_ids: HashSet<String>,
}

impl TurnLifecycleState {
    pub(super) fn start(&mut self, turn_id: &str) -> TurnStartOutcome {
        if self.terminal_turn_ids.contains(turn_id) {
            return TurnStartOutcome::AlreadyTerminal;
        }
        match self.active_turn_id.as_deref() {
            Some(active) if active == turn_id => TurnStartOutcome::Duplicate,
            Some(_) => TurnStartOutcome::ActiveConflict,
            None => {
                self.active_turn_id = Some(turn_id.to_string());
                self.active_started_at = Some(Instant::now());
                TurnStartOutcome::Started
            }
        }
    }

    // 计算 `accept_event` 所需的界面结果。
    pub(super) fn accept_event(&mut self, turn_id: &str) -> TurnEventOutcome {
        if self.terminal_turn_ids.contains(turn_id) {
            return TurnEventOutcome::AlreadyTerminal;
        }
        match self.active_turn_id.as_deref() {
            Some(active) if active == turn_id => TurnEventOutcome::Ready,
            Some(_) => TurnEventOutcome::ActiveConflict,
            None => {
                self.active_turn_id = Some(turn_id.to_string());
                self.active_started_at = Some(Instant::now());
                TurnEventOutcome::MissingBegin
            }
        }
    }

    pub(super) fn finish(&mut self, turn_id: &str) -> TurnFinishOutcome {
        if self.terminal_turn_ids.contains(turn_id) {
            return TurnFinishOutcome::DuplicateTerminal;
        }
        match self.active_turn_id.as_deref() {
            Some(active) if active == turn_id => {
                self.active_turn_id = None;
                self.active_started_at = None;
                self.terminal_turn_ids.insert(turn_id.to_string());
                TurnFinishOutcome::Finished
            }
            Some(_) => TurnFinishOutcome::ActiveConflict,
            None => {
                self.terminal_turn_ids.insert(turn_id.to_string());
                TurnFinishOutcome::MissingBegin
            }
        }
    }

    pub(super) fn elapsed(&self) -> Duration {
        self.elapsed_at(Instant::now())
    }

    fn elapsed_at(&self, now: Instant) -> Duration {
        self.active_started_at
            .map(|started_at| now.saturating_duration_since(started_at))
            .unwrap_or_default()
    }
}

impl ChatWidget {
    pub(super) fn on_turn_started(&mut self, turn_id: String) {
        match self.turn_lifecycle.start(&turn_id) {
            TurnStartOutcome::Started => {
                self.set_status(V3ChatStatus::running("Working", None));
            }
            TurnStartOutcome::Duplicate => {
                self.push_diagnostic("duplicate turn start", format!("turn_id={turn_id}"));
            }
            TurnStartOutcome::ActiveConflict => self.push_diagnostic(
                "turn start while another turn is active",
                format!("turn_id={turn_id}"),
            ),
            TurnStartOutcome::AlreadyTerminal => self.push_diagnostic(
                "turn start after terminal",
                format!("turn_id={turn_id}"),
            ),
        }
    }

    // 计算 `accept_turn_event` 所需的界面结果。
    pub(super) fn accept_turn_event(&mut self, turn_id: &str, event_name: &str) -> bool {
        match self.turn_lifecycle.accept_event(turn_id) {
            TurnEventOutcome::Ready => true,
            TurnEventOutcome::MissingBegin => {
                self.push_diagnostic(
                    "missing turn start",
                    format!("event={event_name}; turn_id={turn_id}"),
                );
                self.set_status(V3ChatStatus::running("Working", None));
                true
            }
            TurnEventOutcome::ActiveConflict => {
                self.push_diagnostic(
                    "event for non-active turn",
                    format!("event={event_name}; turn_id={turn_id}"),
                );
                false
            }
            TurnEventOutcome::AlreadyTerminal => {
                self.push_diagnostic(
                    "event after turn terminal",
                    format!("event={event_name}; turn_id={turn_id}"),
                );
                false
            }
        }
    }

    pub(super) fn begin_terminal(&mut self, turn_id: &str, event_name: &str) -> bool {
        match self.turn_lifecycle.finish(turn_id) {
            TurnFinishOutcome::Finished => true,
            TurnFinishOutcome::MissingBegin => {
                self.push_diagnostic(
                    "turn terminal without start",
                    format!("event={event_name}; turn_id={turn_id}"),
                );
                true
            }
            TurnFinishOutcome::ActiveConflict => {
                self.push_diagnostic(
                    "terminal for non-active turn",
                    format!("event={event_name}; turn_id={turn_id}"),
                );
                false
            }
            TurnFinishOutcome::DuplicateTerminal => {
                self.push_diagnostic(
                    "duplicate turn terminal",
                    format!("event={event_name}; turn_id={turn_id}"),
                );
                false
            }
        }
    }

    pub(super) fn on_turn_completed(&mut self, turn_id: String) {
        if !self.begin_terminal(&turn_id, "turn completed") {
            return;
        }
        self.commit_stream_segment();
        if self.exec_state.has_running_calls() {
            let call_ids = self.exec_state.finish_running(
                130,
                "tool did not emit a terminal event before turn completion",
            );
            self.sync_exec_group();
            self.commit_exec_group();
            self.push_diagnostic(
                "turn completed with running tools",
                format!("call_ids={}", call_ids.join(",")),
            );
        } else {
            self.commit_exec_group();
        }
        self.set_status(V3ChatStatus::idle());
    }

    // 处理 `on_turn_failed` 对应的界面事件并同步相关状态。
    pub(super) fn on_turn_failed(&mut self, turn_id: String, message: String) {
        if !self.begin_terminal(&turn_id, "turn failed") {
            return;
        }
        self.commit_stream_segment();
        self.exec_state.finish_running(1, &message);
        self.commit_exec_group();
        self.transcript.push_history(V3DisplayEvent::Error { message });
        self.set_status(V3ChatStatus::idle());
    }

    pub(super) fn push_diagnostic(
        &mut self,
        type_name: impl Into<String>,
        detail: impl Into<String>,
    ) {
        self.transcript.push_history(V3DisplayEvent::Unknown {
            type_name: type_name.into(),
            detail: detail.into(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_terminal_is_explicit() {
        let mut state = TurnLifecycleState::default();
        assert_eq!(state.start("turn-1"), TurnStartOutcome::Started);
        assert_eq!(state.finish("turn-1"), TurnFinishOutcome::Finished);
        assert_eq!(state.finish("turn-1"), TurnFinishOutcome::DuplicateTerminal);
    }

    #[test]
    fn elapsed_seconds_tracks_only_the_active_turn() {
        let now = Instant::now();
        let mut state = TurnLifecycleState {
            active_started_at: Some(now - Duration::from_secs(7)),
            ..TurnLifecycleState::default()
        };

        assert_eq!(state.elapsed_at(now), Duration::from_secs(7));

        state.active_started_at = None;
        assert_eq!(state.elapsed_at(now), Duration::ZERO);
    }
}
