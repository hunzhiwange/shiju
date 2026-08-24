//! Construction and initial wiring for the pure rendering shell.
//! 本模块负责纯渲染外壳的构造与初始连线。
//! ChatWidget 仅维护展示状态、历史、输入队列与覆盖层；运行时业务由外层系统负责。

use super::*;
use crate::bottom_pane::BottomPaneParams;

/// Read-only display configuration for a `ChatWidget`.
/// “ChatWidget”的只读显示配置。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChatWidgetConfig {
    pub footer_data: V3FooterData,
    pub placeholder: String,
}

impl Default for ChatWidgetConfig {
    fn default() -> Self {
        Self {
            footer_data: V3FooterData::default(),
            placeholder: "Compose a VibeWindow task".to_string(),
        }
    }
}

impl ChatWidget {
    /// Constructs the copied chat layout without a client, sender, or runtime handle.
    /// 构建没有客户端、发件人或运行时句柄的复制聊天布局。
    #[must_use]
    pub fn new(config: ChatWidgetConfig) -> Self {
        let status_state = StatusState::from_task_running(config.footer_data.is_task_running);
        Self {
            bottom_pane: BottomPane::new(BottomPaneParams {
                footer_data: config.footer_data,
                placeholder: config.placeholder,
            }),
            transcript: TranscriptState::default(),
            input_queue: InputQueueState::default(),
            status_state,
            streaming: streaming::StreamingState::default(),
            turn_lifecycle: turn_lifecycle::TurnLifecycleState::default(),
            exec_state: exec_state::ExecState::default(),
            interrupt_state: interrupts::InterruptState::default(),
            overlay: None,
            clipboard_lease: None,
            last_rendered_width: Cell::new(None),
        }
    }
}
