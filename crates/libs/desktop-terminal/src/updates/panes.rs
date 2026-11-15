use crate::Shell;
use super::{TerminalMessage, Effect};
type TerminalState = crate::TerminalState<Shell>;
use std::path::PathBuf;
use crate::panes::SplitError;

use iced::Task;


pub(super) fn update(
    terminal: &mut TerminalState,
    theme: &iced::Theme,
    project_path: Option<&str>,
    focused: &mut bool,
    effects: &mut Vec<Effect>,
    message: TerminalMessage,
) -> Task<TerminalMessage> {
    match message {
        TerminalMessage::TogglePaneMaximized(instance, pane) => {
            let terminal = &mut *terminal;
            if !terminal.is_visible || terminal.panes.instance != instance {
                return Task::none();
            }
            let Some(id) = terminal.panes.grid.get(pane).and_then(|pane| pane.active_id) else {
                return Task::none();
            };
            terminal.select_terminal(id);
            terminal.tab_context_menu_id = None;
            terminal.tab_context_menu_pos = None;
            if terminal.panes.grid.maximized() == Some(pane) {
                terminal.panes.grid.restore();
            } else {
                terminal.panes.grid.maximize(pane);
            }
            super::event::focus_active_terminal(terminal, focused, effects)
        }
        TerminalMessage::ResizePane(instance, event) => {
            let terminal = &mut *terminal;
            if terminal.is_visible && terminal.panes.instance == instance {
                terminal.panes.resize(event);
            }
            Task::none()
        }
        TerminalMessage::DockPane(instance, pane, target) => {
            let terminal = &mut *terminal;
            if terminal.is_visible && terminal.panes.instance == instance {
                terminal.panes.dock(pane, target);
            }
            Task::none()
        }
        TerminalMessage::AddToPane(pane) => {
            if terminal.is_visible {
                super::tabs::add_to_pane(terminal, theme, project_path, focused, effects, pane)
            } else {
                Task::none()
            }
        }
        TerminalMessage::ToggleSplitMenu(pane) => {
            if terminal.is_visible
                && terminal.panes.grid.get(pane).is_some_and(|pane| pane.active_id.is_some())
            {
                let terminal = &mut *terminal;
                terminal.content_menu = None;
                terminal.tab_context_menu_id = None;
                terminal.tab_context_menu_pos = None;
                terminal.panes.split_menu =
                    (terminal.panes.split_menu != Some(pane)).then_some(pane);
                if terminal.panes.split_menu.is_none() {
                    return restore_focus(terminal, focused, effects);
                }
            }
            Task::none()
        }
        TerminalMessage::CloseSplitMenu => {
            terminal.panes.split_menu = None;
            restore_focus(terminal, focused, effects)
        }
        TerminalMessage::Split(instance, pane, direction) => {
            if !terminal.is_visible || terminal.panes.instance != instance {
                return Task::none();
            }
            terminal.panes.split_menu = None;
            let cwd = project_path.map(PathBuf::from);
            match terminal.split_terminal(pane, direction, cwd) {
                Ok(()) => {
                    terminal.apply_app_theme(theme);
                    super::event::focus_active_terminal(terminal, focused, effects)
                }
                Err(SplitError::MissingPane) => restore_focus(terminal, focused, effects),
                Err(SplitError::Spawn) => Task::batch([
                    { effects.push(Effect::SplitFailed); Task::none() },
                    restore_focus(terminal, focused, effects),
                ]),
            }
        }
        _ => unreachable!("unexpected terminal pane message"),
    }
}

fn restore_focus(terminal: &mut TerminalState, focused: &mut bool, effects: &mut Vec<Effect>) -> Task<TerminalMessage> {
    if *focused {
        super::event::focus_active_terminal(terminal, focused, effects)
    } else {
        Task::none()
    }
}
