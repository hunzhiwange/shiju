use std::path::PathBuf;

use iced::Task;
use iced::widget::pane_grid::DragEvent;

use super::TerminalMessage;
use crate::app::state::ToastKind;
use crate::app::terminal::panes::SplitError;
use crate::app::{App, FocusArea, Message};

pub(super) fn update(app: &mut App, message: TerminalMessage) -> Task<Message> {
    match message {
        TerminalMessage::ResizePane(instance, event, size) => {
            let terminal = &mut app.terminal;
            if terminal.is_visible && terminal.panes.instance == instance {
                terminal.panes.resize(event, size);
            }
            Task::none()
        }
        TerminalMessage::DragPane(instance, event, size) => {
            let terminal = &mut app.terminal;
            if terminal.is_visible
                && terminal.panes.instance == instance
                && let DragEvent::Dropped { pane, target } = event
            {
                terminal.panes.dock(pane, target, size);
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
        TerminalMessage::Split(pane, direction, size) => {
            if !app.terminal.is_visible || app.terminal.panes.split_menu != Some(pane) {
                return Task::none();
            }
            app.terminal.panes.split_menu = None;
            let cwd = app.project_path.as_ref().map(PathBuf::from);
            let (kind, message) = match app.terminal.split_terminal(pane, direction, size, cwd) {
                Ok(()) => {
                    app.terminal.apply_app_theme(&app.app_theme);
                    return super::event::focus_active_terminal(app);
                }
                Err(SplitError::MissingPane) => return restore_focus(app),
                Err(SplitError::InsufficientSpace) => {
                    (ToastKind::Warning, "当前窗格空间不足，无法拆分终端")
                }
                Err(SplitError::Spawn) => (ToastKind::Error, "无法启动拆分终端"),
            };
            Task::batch([app.show_localized_toast(kind, message, vec![]), restore_focus(app)])
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
