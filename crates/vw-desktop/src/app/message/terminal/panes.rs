use std::path::PathBuf;

use iced::Task;

use super::TerminalMessage;
use crate::app::state::ToastKind;
use crate::app::terminal::panes::SplitError;
use crate::app::{App, FocusArea, Message};

pub(super) fn update(app: &mut App, message: TerminalMessage) -> Task<Message> {
    match message {
        TerminalMessage::ResizePane(instance, event) => {
            let terminal = &mut app.terminal;
            if terminal.is_visible && terminal.panes.instance == instance {
                terminal.panes.resize(event);
            }
            Task::none()
        }
        TerminalMessage::DockPane(instance, pane, target) => {
            let terminal = &mut app.terminal;
            if terminal.is_visible && terminal.panes.instance == instance {
                terminal.panes.dock(pane, target);
            }
            Task::none()
        }
        TerminalMessage::AddToPane(pane) => {
            if app.terminal.is_visible {
                super::tabs::add_to_pane(app, pane)
            } else {
                Task::none()
            }
        }
        TerminalMessage::ToggleSplitMenu(pane) => {
            if app.terminal.is_visible
                && app.terminal.panes.grid.get(pane).is_some_and(|pane| pane.active_id.is_some())
            {
                let terminal = &mut app.terminal;
                terminal.content_menu = None;
                terminal.tab_context_menu_id = None;
                terminal.tab_context_menu_pos = None;
                terminal.panes.split_menu =
                    (terminal.panes.split_menu != Some(pane)).then_some(pane);
                if terminal.panes.split_menu.is_none() {
                    return restore_focus(app);
                }
            }
            Task::none()
        }
        TerminalMessage::CloseSplitMenu => {
            app.terminal.panes.split_menu = None;
            restore_focus(app)
        }
        TerminalMessage::Split(instance, pane, direction) => {
            if !app.terminal.is_visible || app.terminal.panes.instance != instance {
                return Task::none();
            }
            app.terminal.panes.split_menu = None;
            let cwd = app.project_path.as_ref().map(PathBuf::from);
            match app.terminal.split_terminal(pane, direction, cwd) {
                Ok(()) => {
                    app.terminal.apply_app_theme(&app.app_theme);
                    super::event::focus_active_terminal(app)
                }
                Err(SplitError::MissingPane) => restore_focus(app),
                Err(SplitError::Spawn) => Task::batch([
                    app.show_localized_toast(ToastKind::Error, "无法启动拆分终端", vec![]),
                    restore_focus(app),
                ]),
            }
        }
        _ => unreachable!("unexpected terminal pane message"),
    }
}

fn restore_focus(app: &mut App) -> Task<Message> {
    if app.focus_area == FocusArea::Terminal {
        super::event::focus_active_terminal(app)
    } else {
        Task::none()
    }
}
