//! 原生菜单动作解析与状态快照。

use desktop_platform::native_menu::window_tab_title;

use crate::components::top_bar::menus::Message;
use crate::components::top_bar::menus::{
    MenuEntry, edit_menu_entries, file_menu_entries, help_menu_entries, view_menu_entries,
};
use crate::components::top_bar::types::WorkspaceMode;
use desktop_platform::menu_actions::NativeMenuAction;

pub use desktop_platform::native_menu::WindowTabEntry;

/// 驱动原生菜单重建的状态快照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeMenuSnapshot {
    pub locale: config_ui::DesktopLocale,
    pub is_project: bool,
    pub workspace_mode: WorkspaceMode,
    pub show_file_manager: bool,
    pub file_manager_show_changes: bool,
    pub open_window_tabs: Vec<WindowTabEntry>,
}

pub struct TabInput<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub project_path: Option<&'a str>,
}

pub fn window_tab_entries<'a>(
    locale: config_ui::DesktopLocale,
    tabs: impl IntoIterator<Item = TabInput<'a>>,
    active_tab_id: Option<&str>,
) -> Vec<WindowTabEntry> {
    tabs.into_iter()
        .filter(|tab| !tab.id.starts_with("find:"))
        .map(|tab| WindowTabEntry {
            id: tab.id.to_owned(),
            title: desktop_i18n::native::tab_title(
                desktop_i18n::I18n::new(locale),
                window_tab_title(tab.title, tab.project_path),
                tab.title.trim().is_empty()
                    && tab.project_path.is_none_or(|path| path.trim().is_empty()),
            ),
            active: active_tab_id == Some(tab.id),
        })
        .collect()
}

/// 与 topbar 同源的菜单分区（不含窗口菜单；窗口菜单含动态标签列表）。
pub fn menu_sections(snapshot: &NativeMenuSnapshot) -> [(&'static str, Vec<MenuEntry>); 3] {
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

pub fn help_section() -> (&'static str, Vec<MenuEntry>) {
    ("帮助", help_menu_entries())
}

/// 按局部快照解析原生菜单动作为与 UI 控件相同的 Message。
pub fn resolve_native_menu_action(
    snapshot: &NativeMenuSnapshot,
    action: NativeMenuAction,
) -> Option<Message> {
    for (_title, entries) in menu_sections(snapshot) {
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

/// Strip local messages before installing the native menu.
pub fn platform_snapshot(
    snapshot: NativeMenuSnapshot,
) -> desktop_platform::native_menu::NativeMenuSnapshot {
    use desktop_platform::native_menu as platform;
    fn entries(items: Vec<MenuEntry>) -> Vec<platform::MenuEntry> {
        items
            .into_iter()
            .map(|item| match item {
                MenuEntry::Separator => platform::MenuEntry::Separator,
                MenuEntry::Item { action, label, shortcut, .. } => {
                    platform::MenuEntry::Item { action, label, shortcut }
                }
            })
            .collect()
    }
    let sections = menu_sections(&snapshot).map(|(title, items)| (title, entries(items)));
    let (title, items) = help_section();
    platform::NativeMenuSnapshot {
        locale: snapshot.locale,
        sections,
        help: (title, entries(items)),
        open_window_tabs: snapshot.open_window_tabs,
    }
}
