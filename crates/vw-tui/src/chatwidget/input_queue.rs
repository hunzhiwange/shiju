//! Local queued-input state for the rendering shell.

use std::collections::VecDeque;

use super::UserMessage;

#[derive(Debug, Default)]
pub(super) struct InputQueueState {
    queued: VecDeque<UserMessage>,
}

impl InputQueueState {
    pub(super) fn push(&mut self, message: UserMessage) {
        self.queued.push_back(message);
    }

    pub(super) fn pop(&mut self) -> Option<UserMessage> {
        self.queued.pop_front()
    }

    pub(super) fn len(&self) -> usize {
        self.queued.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queued_input_remains_fifo() {
        let mut queue = InputQueueState::default();
        queue.push(UserMessage::new("first", Vec::new()));
        queue.push(UserMessage::new("second", Vec::new()));

        assert_eq!(queue.pop().map(|message| message.text), Some("first".into()));
        assert_eq!(queue.pop().map(|message| message.text), Some("second".into()));
    }
}
