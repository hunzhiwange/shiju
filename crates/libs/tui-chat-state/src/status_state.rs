//! Read-only status data rendered above the copied composer.
//! 本模块负责输入框上方状态栏的只读数据。
//! ChatWidget 仅维护展示状态、历史、输入队列与覆盖层；运行时业务由外层系统负责。

use tui_contracts::presentation::V3ChatStatus;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StatusState {
    current: V3ChatStatus,
}

impl StatusState {
    pub fn from_task_running(is_task_running: bool) -> Self {
        let current = if is_task_running {
            V3ChatStatus::running("Working", None)
        } else {
            V3ChatStatus::idle()
        };
        Self { current }
    }

    pub fn set(&mut self, status: V3ChatStatus) {
        self.current = status;
    }

    pub fn set_task_running(&mut self, is_task_running: bool) {
        self.current.is_task_running = is_task_running;
        if is_task_running && self.current.label.is_none() {
            self.current.label = Some("Working".to_string());
        } else if !is_task_running {
            self.current.label = None;
            self.current.detail = None;
        }
    }

    pub fn current(&self) -> &V3ChatStatus {
        &self.current
    }
}
