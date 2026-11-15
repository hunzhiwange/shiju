//! Local queued-input state for the rendering shell.
//! 本模块负责纯本地的待提交输入队列。
//! ChatWidget 仅维护展示状态、历史、输入队列与覆盖层；运行时业务由外层系统负责。

use std::collections::VecDeque;

use tui_contracts::presentation::UserMessage;

#[derive(Debug, Default)]
/// 描述 `InputQueueState` 在本模块状态转换中传递的数据。
pub struct InputQueueState {
    queued: VecDeque<UserMessage>,
}

impl InputQueueState {
    pub fn push(&mut self, message: UserMessage) {
        self.queued.push_back(message);
    }

    // 计算 `pop` 在当前界面流程中需要的局部结果。
    pub fn pop(&mut self) -> Option<UserMessage> {
        self.queued.pop_front()
    }

    pub fn pop_back(&mut self) -> Option<UserMessage> {
        self.queued.pop_back()
    }

    pub fn len(&self) -> usize {
        self.queued.len()
    }
}

#[cfg(test)]
#[path = "input_queue_tests.rs"]
mod tests;
