//! Terminal state, local messages, updates and views; the host applies cross-domain effects.
pub mod message;
#[cfg(not(target_arch = "wasm32"))]
pub mod panes;
#[path = "components/terminal_panel.rs"]
pub mod terminal_panel;

/// 单个终端标签及其原生终端实例和重命名草稿。
pub struct TerminalTab {
    pub id: u64,
    pub title: String,
    pub default_title: bool,
    #[cfg(not(target_arch = "wasm32"))]
    pub term: iced_term::Terminal,
    #[cfg(target_arch = "wasm32")]
    pub term: (),
    pub edit_title: Option<String>,
}

/// Borrowed inputs used by terminal rendering; no application state is copied.
#[derive(Clone, Copy)]
pub struct ViewInput<'a> {
    pub tabs: &'a [TerminalTab],
    pub active_id: Option<u64>,
    pub is_visible: bool,
    pub height: f32,
    pub tab_context_menu_id: Option<u64>,
    pub tab_context_menu_pos: Option<(f32, f32)>,
    pub desktop_locale: config_ui::DesktopLocale,
    pub focused: bool,
    #[cfg(not(target_arch = "wasm32"))]
    pub panes: &'a panes::TerminalPanes,
    #[cfg(not(target_arch = "wasm32"))]
    pub content_menu: Option<(u64, iced::Point)>,
    #[cfg(not(target_arch = "wasm32"))]
    pub is_fullscreen: bool,
}

pub mod state;
pub use state::{
    TerminalState, TerminalTheme, build_palette, truncate_string_to_limit, truncate_terminal_content,
};
#[cfg(not(target_arch = "wasm32"))]
pub mod local;

pub mod shell;
pub use shell::Shell;
pub mod updates;

#[cfg(test)]
mod shell_compat_tests;

#[cfg(test)]
mod desktop_split_18_tests;

#[cfg(test)]
mod host_module_34_tests;
