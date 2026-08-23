//! Local queued-input state for the rendering shell.
//! 本模块负责纯本地的待提交输入队列。
//! ChatWidget 仅维护展示状态、历史、输入队列与覆盖层；运行时业务由外层系统负责。

use std::collections::VecDeque;

use super::UserMessage;

#[derive(Debug, Default)]
/// 描述 `InputQueueState` 在本模块状态转换中传递的数据。
pub(super) struct InputQueueState {
    queued: VecDeque<UserMessage>,
}

impl InputQueueState {
    pub(super) fn push(&mut self, message: UserMessage) {
        self.queued.push_back(message);
    }

    // 计算 `pop` 在当前界面流程中需要的局部结果。
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
