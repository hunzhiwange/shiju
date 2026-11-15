use crate::Shell;
use super::{TerminalMessage, Effect};
type TerminalState = crate::TerminalState<Shell>;
// 终端事件处理模块
//
// 本模块负责处理终端组件产生的各类事件，包括：
// - 终端后端调用事件（如进程退出、信号处理等）
// - 终端面板拖拽调整高度事件
//
// 该模块是终端消息处理子系统的一部分，与 [`super::TerminalMessage`] 紧密配合，
// 实现终端 UI 的交互逻辑。

use iced::Task;

/// Transfer both the editor's independent canvas focus and Iced's widget focus.
#[cfg(not(target_arch = "wasm32"))]
pub fn focus_active_terminal(terminal: &mut TerminalState, focused: &mut bool, effects: &mut Vec<Effect>) -> Task<TerminalMessage> {
    let Some(tab) = terminal.tabs.iter().find(|tab| {
        Some(tab.id) == terminal.active_id
            && terminal.is_visible
            && tab.edit_title.is_none()
    }) else {
        return Task::none();
    };
    let widget_id = tab.term.widget_id().clone();
    *focused = true;
    effects.push(Effect::Focus);
    iced_term::TerminalView::focus(widget_id)
}

fn apply_drag_started(terminal: &mut TerminalState, cursor_y: f32) {
    terminal.is_dragging = true;
    terminal.drag_anchor_y = Some(cursor_y);
    terminal.drag_start_height = terminal.height;
}

/// Apply terminal-local state changes and return explicit host effects.
pub(super) fn update(
    terminal: &mut TerminalState,
    cursor_y: f32,
    focused: &mut bool,
    effects: &mut Vec<Effect>,
    message: TerminalMessage,
) -> Task<TerminalMessage> {
    #[cfg(target_arch = "wasm32")]
    let _ = (&focused, &effects);
    match message {
        // 处理非 WASM 平台的终端后端调用事件
        // 该事件由 iced_term 库产生，用于与底层 PTY 进程通信
        #[cfg(not(target_arch = "wasm32"))]
        TerminalMessage::Event(iced_term::Event::BackendCall(id, cmd)) => {
            // These commands are emitted by an actual pointer press, before any
            // keyboard input. Resize and PTY notifications must never take focus.
            let clicked = matches!(cmd, iced_term::BackendCommand::SelectStart(..))
                || matches!(
                    cmd,
                    iced_term::BackendCommand::MouseReport(
                        iced_term::MouseButton::LeftButton
                            | iced_term::MouseButton::MiddleButton
                            | iced_term::MouseButton::RightButton,
                        _,
                        _,
                        true
                    )
                );
            let terminal = &mut *terminal;
            if matches!(
                cmd,
                iced_term::BackendCommand::Write(_) | iced_term::BackendCommand::Paste(_)
            ) && (!terminal.is_visible
                || terminal.active_id != Some(id)
                || terminal.tabs.iter().any(|tab| tab.id == id && tab.edit_title.is_some()))
            {
                return Task::none();
            }
            let focus = if clicked && terminal.activate_visible_tab(id) {
                focus_active_terminal(terminal, focused, effects)
            } else {
                Task::none()
            };

            // 查找匹配指定 ID 的终端标签页
            if let Some(tab) = terminal.tabs.iter_mut().find(|t| t.id == id) {
                // 将命令代理到终端后端处理
                // 如果返回 Shutdown 动作，表示终端进程已退出
                if tab.term.handle(iced_term::Command::ProxyToBackend(cmd))
                    == iced_term::actions::Action::Shutdown
                {
                    return super::tabs::close_tab(terminal, focused, effects, id);
                }
            }
            focus
        }

        // WASM 平台下忽略所有终端后端事件
        // 浏览器环境的终端由 WebAssembly 特定的后端处理
        #[cfg(target_arch = "wasm32")]
        TerminalMessage::Event(_) => Task::none(),

        #[cfg(not(target_arch = "wasm32"))]
        TerminalMessage::ToggleFullscreen if terminal.is_visible => {
            terminal.set_fullscreen(!terminal.is_fullscreen);
            focus_active_terminal(terminal, focused, effects)
        }
        // 用户开始拖拽停靠终端的边缘以调整高度。
        TerminalMessage::DragStarted if !terminal.is_fullscreen => {
            apply_drag_started(&mut *terminal, cursor_y);

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
