use crate::menu_actions::NativeMenuAction;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowTabEntry {
    pub id: String,
    pub title: String,
    pub active: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuEntry {
    Item { action: Option<NativeMenuAction>, label: &'static str, shortcut: Option<&'static str> },
    Separator,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeMenuSnapshot {
    pub locale: config_ui::DesktopLocale,
    pub sections: [(&'static str, Vec<MenuEntry>); 3],
    pub help: (&'static str, Vec<MenuEntry>),
    pub open_window_tabs: Vec<WindowTabEntry>,
}
#[derive(Debug, Clone)]
pub enum Event { Quit, TabSelected(String), Action(NativeMenuAction) }
const WINDOW_TAB_MENU_ID_PREFIX: &str = "native.window.tab.";
pub fn window_tab_menu_id(tab_id: &str) -> String {
    format!("{WINDOW_TAB_MENU_ID_PREFIX}{tab_id}")
}
pub fn parse_window_tab_menu_id(id: &str) -> Option<&str> {
    id.strip_prefix(WINDOW_TAB_MENU_ID_PREFIX).filter(|tab_id| !tab_id.is_empty())
}
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::{install_or_sync, subscription};

use std::path::Path;
pub fn project_display_label(project_path: Option<&str>) -> String {
    let Some(path) = project_path.map(str::trim).filter(|path| !path.is_empty()) else {
        return "未打开项目".to_string();
    };
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map_or_else(|| path.to_string(), str::to_string)
}

pub fn window_tab_title(title: &str, project_path: Option<&str>) -> String {
    let title = title.trim();
    match project_path.map(str::trim).filter(|path| !path.is_empty()) {
        Some(path) => {
            let project = project_display_label(Some(path));
            if title.is_empty() || title == project {
                project
            } else {
                format!("{project} — {title}")
            }
        }
        None if title.is_empty() => "未命名".to_string(),
        None => title.to_string(),
    }
}


#[cfg(test)]
mod actions_tests;
