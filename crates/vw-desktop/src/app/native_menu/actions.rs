//! 原生菜单动作解析与状态快照。

use std::path::Path;

use crate::app::components::top_bar::menu_actions::NativeMenuAction;
use crate::app::components::top_bar::menus::{
    MenuEntry, edit_menu_entries, file_menu_entries, help_menu_entries, view_menu_entries,
};
use crate::app::state::WorkspaceMode;
use crate::app::{App, Message, Screen};

/// 窗口菜单中可选的标签项（用于切换已打开项目/页面）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WindowTabEntry {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) active: bool,
}

/// 驱动原生菜单重建的状态快照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeMenuSnapshot {
    pub(crate) is_project: bool,
    pub(crate) workspace_mode: WorkspaceMode,
    pub(crate) show_file_manager: bool,
    pub(crate) file_manager_show_changes: bool,
    pub(crate) open_window_tabs: Vec<WindowTabEntry>,
}

pub(crate) const WINDOW_TAB_MENU_ID_PREFIX: &str = "native.window.tab.";

impl NativeMenuSnapshot {
    pub(crate) fn from_app(app: &App) -> Self {
        Self {
            is_project: matches!(app.screen, Screen::Project),
            workspace_mode: app.workspace_mode,
            show_file_manager: app.show_file_manager,
            file_manager_show_changes: app.file_manager_show_changes,
            open_window_tabs: window_tab_entries(app),
        }
    }
}

fn project_display_label(project_path: Option<&str>) -> String {
    let Some(path) = project_path.map(str::trim).filter(|path| !path.is_empty()) else {
        return "未打开项目".to_string();
    };
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map_or_else(|| path.to_string(), str::to_string)
}

fn window_tab_entries(app: &App) -> Vec<WindowTabEntry> {
    app.open_tabs
        .iter()
        .filter(|tab| !tab.id.starts_with("find:"))
        .map(|tab| WindowTabEntry {
            id: tab.id.clone(),
            title: window_tab_title(tab.title.as_str(), tab.project_path.as_deref()),
            active: app.active_tab_id.as_deref() == Some(tab.id.as_str()),
        })
        .collect()
}

fn window_tab_title(title: &str, project_path: Option<&str>) -> String {
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

pub(crate) fn window_tab_menu_id(tab_id: &str) -> String {
    format!("{WINDOW_TAB_MENU_ID_PREFIX}{tab_id}")
}

pub(crate) fn parse_window_tab_menu_id(id: &str) -> Option<&str> {
    id.strip_prefix(WINDOW_TAB_MENU_ID_PREFIX).filter(|tab_id| !tab_id.is_empty())
}

/// 与 topbar 同源的菜单分区（不含窗口菜单；窗口菜单含动态标签列表）。
pub(crate) fn menu_sections(snapshot: &NativeMenuSnapshot) -> [(&'static str, Vec<MenuEntry>); 3] {
    [
        ("文件", file_menu_entries(snapshot.is_project)),
        ("编辑", edit_menu_entries(snapshot.is_project)),
        (
            "视图",
            view_menu_entries(
                snapshot.workspace_mode,
                snapshot.show_file_manager,
                snapshot.file_manager_show_changes,
            ),
        ),
    ]
}

pub(crate) fn help_section() -> (&'static str, Vec<MenuEntry>) {
    ("帮助", help_menu_entries())
}

pub(crate) fn menu_item_enabled(_action: NativeMenuAction, _snapshot: &NativeMenuSnapshot) -> bool {
    true
}

/// 按当前 App 状态解析原生菜单动作为与 UI 控件相同的 Message。
pub(crate) fn resolve_native_menu_action(app: &App, action: NativeMenuAction) -> Option<Message> {
    let snapshot = NativeMenuSnapshot::from_app(app);
    for (_title, entries) in menu_sections(&snapshot) {
        for entry in entries {
            if let MenuEntry::Item { action: Some(entry_action), message, .. } = entry
                && entry_action == action
            {
                return Some(*message);
            }
        }
    }
    let (_title, help_entries) = help_section();
    for entry in help_entries {
        if let MenuEntry::Item { action: Some(entry_action), message, .. } = entry
            && entry_action == action
        {
            return Some(*message);
        }
    }
    None
}

#[cfg(test)]
pub(crate) fn project_display_label_for_test(project_path: Option<&str>) -> String {
    project_display_label(project_path)
}

#[cfg(test)]
pub(crate) fn window_tab_title_for_test(title: &str, project_path: Option<&str>) -> String {
    window_tab_title(title, project_path)
}
