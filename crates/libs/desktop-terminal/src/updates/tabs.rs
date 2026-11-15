use crate::Shell;
use super::{TerminalMessage, Effect};
type TerminalState = crate::TerminalState<Shell>;
// 管理终端相关消息、配置和标签页状态更新。
//
// 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
// 不改变任何运行时行为。

use iced::Task;

pub(super) fn close_tab(terminal: &mut TerminalState, focused: &mut bool, effects: &mut Vec<Effect>, id: u64) -> Task<TerminalMessage> {
    #[cfg(not(target_arch = "wasm32"))]
    let was_active = terminal.active_id == Some(id);
    if !terminal.close_terminal(id) {
        return Task::none();
    }
    if terminal.tabs.is_empty() {
        terminal.set_visible(false);
        if *focused {
            *focused = false;
        }
    }
    let reap = reap_retired_task(terminal);
    #[cfg(not(target_arch = "wasm32"))]
    if was_active && *focused {
        return Task::batch([reap, super::event::focus_active_terminal(terminal, focused, effects)]);
    }
    reap
}

/// Schedule dropping retired terminals after iced cancels their subscriptions.
pub fn reap_retired_task(terminal: &TerminalState) -> Task<TerminalMessage> {
    if terminal.has_retired() {
        Task::done(TerminalMessage::ReapRetired)
    } else {
        Task::none()
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn add_to_pane(
    terminal: &mut TerminalState,
    theme: &iced::Theme,
    project_path: Option<&str>,
    focused: &mut bool,
    effects: &mut Vec<Effect>,
    pane: iced::widget::pane_grid::Pane,
) -> Task<TerminalMessage> {
    terminal.tab_context_menu_id = None;
    terminal.tab_context_menu_pos = None;
    terminal.content_menu = None;
    terminal.panes.split_menu = None;
    if terminal
        .add_terminal_to_pane(pane, project_path.map(std::path::PathBuf::from))
    {
        terminal.apply_app_theme(theme);
        return super::event::focus_active_terminal(terminal, focused, effects);
    }
    Task::none()
}

/// Apply terminal-local state changes and return explicit host effects.
pub(super) fn update(
    terminal: &mut TerminalState,
    theme: &iced::Theme,
    project_path: Option<&str>,
    focused: &mut bool,
    effects: &mut Vec<Effect>,
    message: TerminalMessage,
) -> Task<TerminalMessage> {
    #[cfg(target_arch = "wasm32")]
    let _ = (theme, project_path, &effects);
    match message {
        #[cfg(not(target_arch = "wasm32"))]
        TerminalMessage::Add => {
            let pane = terminal.panes.focused;
            add_to_pane(terminal, theme, project_path, focused, effects, pane)
        },
        #[cfg(target_arch = "wasm32")]
        TerminalMessage::Add => {
            terminal.tab_context_menu_id = None;
            terminal.tab_context_menu_pos = None;
            Task::none()
        }
        TerminalMessage::Select(id) => {
            terminal.tab_context_menu_id = None;
            terminal.tab_context_menu_pos = None;
            if !terminal.select_terminal(id) {
                return Task::none();
            }
            *focused = true;
            #[cfg(not(target_arch = "wasm32"))]
            terminal.apply_app_theme(theme);
            #[cfg(not(target_arch = "wasm32"))]
            return super::event::focus_active_terminal(terminal, focused, effects);
            #[cfg(target_arch = "wasm32")]
            Task::none()
        }
        TerminalMessage::Close(id) => {
            terminal.tab_context_menu_id = None;
            terminal.tab_context_menu_pos = None;
            if terminal.tabs.len() <= 1 {
                return Task::none();
            }
            close_tab(terminal, focused, effects, id)
        }
        TerminalMessage::ReapRetired => {
            terminal.reap_retired();
            Task::none()
        }
        TerminalMessage::TabContextOpen(id, x, y) => {
            if !terminal.select_terminal(id) {
                return Task::none();
            }
            *focused = true;
            #[cfg(not(target_arch = "wasm32"))]
            terminal.apply_app_theme(theme);
            terminal.tab_context_menu_id = Some(id);
            terminal.tab_context_menu_pos = Some((x, y));
            #[cfg(not(target_arch = "wasm32"))]
            return super::event::focus_active_terminal(terminal, focused, effects);
            #[cfg(target_arch = "wasm32")]
            Task::none()
        }
        TerminalMessage::TabContextClose => {
            terminal.tab_context_menu_id = None;
            terminal.tab_context_menu_pos = None;
            Task::none()
        }
        _ => Task::none(),
    }
}


#[cfg(test)]
#[path = "tabs_tests.rs"]
mod tabs_tests;
