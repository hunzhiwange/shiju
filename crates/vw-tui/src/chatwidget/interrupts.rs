//! Interrupt bookkeeping at the product-neutral v3 event boundary.

use std::collections::HashSet;

use super::*;

#[derive(Debug, Default)]
pub(super) struct InterruptState {
    cancelled_turn_ids: HashSet<String>,
}

impl InterruptState {
    fn record(&mut self, turn_id: String) {
        self.cancelled_turn_ids.insert(turn_id);
    }
}

impl ChatWidget {
    pub(super) fn on_turn_cancelled(&mut self, turn_id: String, reason: String) {
        if !self.begin_terminal(&turn_id, "turn cancelled") {
            return;
        }
        self.commit_stream_segment();
        self.exec_state.finish_running(130, &reason);
        self.commit_exec_group();
        self.interrupt_state.record(turn_id);
        self.transcript.push_history(V3DisplayEvent::System {
            text: format!("Cancelled: {reason}"),
        });
        self.set_status(V3ChatStatus::idle());
    }
}
