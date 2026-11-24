//! Cursor and renderer for a terminal whose process is owned by the gateway.
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use iced::{Element, Theme};
use iced_term::{Command, Event, Terminal, TerminalView, actions::Action, settings};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

pub struct AttachedTerminal {
    terminal: Terminal,
    cursor: u64,
    fresh: bool,
    theme: Theme,
}

impl AttachedTerminal {
    pub fn new(directory: PathBuf, theme: &Theme, fresh: bool) -> Self {
        let settings = settings::Settings {
            theme: settings::ThemeSettings::new(Box::new(crate::build_palette(theme))),
            font: crate::state::build_font_settings("", 14.0),
            backend: settings::BackendSettings {
                working_directory: Some(directory),
                ..Default::default()
            },
        };
        Self {
            terminal: Terminal::attach(NEXT_ID.fetch_add(1, Ordering::Relaxed), settings),
            cursor: 0,
            fresh,
            theme: theme.clone(),
        }
    }

    pub fn cursor(&self) -> u64 {
        self.cursor
    }

    /// Apply a read only to the renderer that requested it. A gap resets both
    /// the screen and parser, since an escape sequence may have been lost.
    pub fn apply_output(
        &mut self,
        requested: u64,
        start: u64,
        next: u64,
        bytes: &[u8],
        truncated: bool,
        reply: bool,
    ) -> bool {
        if requested != self.cursor
            || start < requested
            || truncated != (start > requested)
            || start.checked_add(bytes.len() as u64) != Some(next)
        {
            return false;
        }
        let live = reply && !truncated && (self.fresh || self.cursor != 0);
        self.terminal.feed_attached_output(bytes, truncated, live);
        self.cursor = next;
        if next != 0 {
            self.fresh = false;
        }
        true
    }

    pub fn set_theme(&mut self, theme: &Theme) {
        if &self.theme != theme {
            self.theme.clone_from(theme);
            self.terminal.handle(Command::ChangeTheme(Box::new(crate::build_palette(theme))));
        }
    }

    pub fn view(&self) -> Element<'_, Event> {
        TerminalView::show(&self.terminal)
    }

    pub fn widget_id(&self) -> iced::widget::Id {
        self.terminal.widget_id().clone()
    }

    pub fn handle(&mut self, event: Event) -> Action {
        let Event::BackendCall(id, command) = event;
        if id == self.terminal.id {
            self.terminal.handle(Command::ProxyToBackend(command))
        } else {
            Action::default()
        }
    }

    pub fn take_input(&mut self) -> (Vec<u8>, Option<(u16, u16)>, bool) {
        self.terminal.take_attached_input()
    }
}

#[cfg(test)]
#[path = "attached_tests.rs"]
mod tests;
