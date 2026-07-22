//! 终端事件处理模块
//!
//! 本模块负责处理终端组件产生的各类事件，包括：
//! - 终端后端调用事件（如进程退出、信号处理等）
//! - 终端面板拖拽调整高度事件
//!
//! 该模块是终端消息处理子系统的一部分，与 [`super::TerminalMessage`] 紧密配合，
//! 实现终端 UI 的交互逻辑。

use super::TerminalMessage;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::FocusArea;
use crate::app::{App, Message, TerminalState};
use iced::Task;

fn apply_drag_started(terminal: &mut TerminalState, cursor_y: f32) {
    terminal.is_dragging = true;
    terminal.drag_anchor_y = Some(cursor_y);
    terminal.drag_start_height = terminal.height;
}

/// 处理终端相关事件并更新应用状态
///
/// 该函数是终端事件处理的核心入口，根据不同的终端消息类型执行相应的状态更新。
///
/// # 参数
///
/// * `app` - 应用程序状态的可变引用，包含终端相关的所有状态信息
/// * `message` - 终端消息枚举，表示需要处理的终端事件
///
/// # 返回值
///
/// 返回 `Task<Message>`，表示处理后可能产生的后续任务。
/// 大多数情况下返回 `Task::none()`，因为终端事件通常只需要更新内部状态。
///
/// # 处理的事件类型
///
/// ## 非 WASM 平台的 BackendCall 事件
///
/// 处理终端后端调用事件，例如：
/// - 将命令代理到后端 PTY 进程
/// - 检测终端关闭信号并清理资源
///
/// ## WASM 平台的 Event 事件
///
/// 在 WebAssembly 目标上，终端后端事件被忽略（返回空任务），
/// 因为浏览器环境的终端行为由不同的后端处理。
///
/// ## DragStarted 事件
///
/// 处理终端面板拖拽开始事件，记录：
/// - 拖拽状态标记
/// - 拖拽起始锚点的 Y 坐标
/// - 拖拽开始时终端面板的高度
///
/// # 示例
///
/// ```ignore
/// // 处理终端拖拽开始事件
/// let message = TerminalMessage::DragStarted;
/// let task = update(&mut app, message);
/// assert!(app.terminal.is_dragging);
/// ```
pub fn update(app: &mut App, message: TerminalMessage) -> Task<Message> {
    match message {
        // 处理非 WASM 平台的终端后端调用事件
        // 该事件由 iced_term 库产生，用于与底层 PTY 进程通信
        #[cfg(not(target_arch = "wasm32"))]
        TerminalMessage::Event(iced_term::Event::BackendCall(id, cmd)) => {
            // 将焦点区域切换到终端，表示用户正在与终端交互
            app.focus_area = FocusArea::Terminal;

            // 查找匹配指定 ID 的终端标签页
            if let Some(tab) = app.terminal.tabs.iter_mut().find(|t| t.id == id) {
                // 将命令代理到终端后端处理
                // 如果返回 Shutdown 动作，表示终端进程已退出
                if tab.term.handle(iced_term::Command::ProxyToBackend(cmd))
                    == iced_term::actions::Action::Shutdown
                {
                    // 关闭该终端标签页
                    app.terminal.close_terminal(id);

                    // 如果所有终端标签页都已关闭，隐藏终端面板
                    if app.terminal.tabs.is_empty() {
                        app.terminal.is_visible = false;
                    }
                    return Task::none();
                }
            }
            Task::none()
        }

        // WASM 平台下忽略所有终端后端事件
        // 浏览器环境的终端由 WebAssembly 特定的后端处理
        #[cfg(target_arch = "wasm32")]
        TerminalMessage::Event(_) => Task::none(),

        // 处理终端面板拖拽开始事件
        // 用户开始拖拽终端面板边缘以调整其高度
        TerminalMessage::DragStarted => {
            let cursor_y = app.cursor_position.y;
            apply_drag_started(&mut app.terminal, cursor_y);

            Task::none()
        }

        // 其他未处理的终端消息类型
        // 保留此分支以支持未来扩展，避免编译警告
        _ => Task::none(),
    }
}
#[cfg(test)]
#[path = "event_tests.rs"]
mod event_tests;
