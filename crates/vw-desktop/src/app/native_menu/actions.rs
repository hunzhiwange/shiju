//! 原生菜单动作解析与状态快照。

use desktop_platform::native_menu::window_tab_title;

use crate::app::components::top_bar::menu_actions::NativeMenuAction;
use crate::app::components::top_bar::menus::{
    MenuEntry, edit_menu_entries, file_menu_entries, help_menu_entries, view_menu_entries,
};
use crate::app::state::WorkspaceMode;
use crate::app::{App, Message, Screen};

pub(crate) use desktop_platform::native_menu::{WindowTabEntry};

/// 驱动原生菜单重建的状态快照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeMenuSnapshot {
    pub(crate) locale: config_ui::DesktopLocale,
    pub(crate) is_project: bool,
    pub(crate) workspace_mode: WorkspaceMode,
    pub(crate) show_file_manager: bool,
    pub(crate) file_manager_show_changes: bool,
    pub(crate) open_window_tabs: Vec<WindowTabEntry>,
}



impl NativeMenuSnapshot {
    pub(crate) fn from_app(app: &App) -> Self {
        Self {
            locale: app.desktop_locale,
            is_project: matches!(app.screen, Screen::Project),
            workspace_mode: app.workspace_mode,
            show_file_manager: app.file_manager.show_file_manager,
            file_manager_show_changes: app.file_manager.file_manager_show_changes,
            open_window_tabs: window_tab_entries(app),
        }
    }
}

fn window_tab_entries(app: &App) -> Vec<WindowTabEntry> {
    app.open_tabs
        .iter()
        .filter(|tab| !tab.id.starts_with("find:"))
        .map(|tab| WindowTabEntry {
            id: tab.id.clone(),
            title: crate::app::i18n::native::tab_title(
                crate::app::i18n::I18n::new(app.desktop_locale),
                window_tab_title(tab.title.as_str(), tab.project_path.as_deref()),
                tab.title.trim().is_empty()
                    && tab.project_path.as_deref().is_none_or(|path| path.trim().is_empty()),
            ),
            active: app.active_tab_id.as_deref() == Some(tab.id.as_str()),
        })
        .collect()
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
