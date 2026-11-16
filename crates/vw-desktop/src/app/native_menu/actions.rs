//! 将 App 状态投影为原生菜单输入，装配跨域消息。
use crate::app::components::top_bar::menu_actions::NativeMenuAction;
use crate::app::{App, Message, Screen};
pub(crate) use desktop_navigation::native_menu::NativeMenuSnapshot;

pub(crate) fn snapshot(app: &App) -> NativeMenuSnapshot {
    NativeMenuSnapshot {
        locale: app.general_settings.desktop_locale,
        is_project: matches!(app.navigation_tabs.screen, Screen::Project),
        workspace_mode: app.navigation_mode.workspace_mode,
        show_file_manager: app.file_manager.show_file_manager,
        file_manager_show_changes: app.file_manager.file_manager_show_changes,
        open_window_tabs: desktop_navigation::native_menu::window_tab_entries(
            app.general_settings.desktop_locale,
            app.navigation_tabs.open_tabs.iter().map(|tab| {
                desktop_navigation::native_menu::TabInput {
                    id: &tab.id,
                    title: &tab.title,
                    project_path: tab.project_path.as_deref(),
                }
            }),
            app.navigation_tabs.active_tab_id.as_deref(),
        ),
    }
}

pub(crate) fn resolve_native_menu_action(app: &App, action: NativeMenuAction) -> Option<Message> {
    desktop_navigation::native_menu::resolve_native_menu_action(&snapshot(app), action)
        .map(crate::app::components::top_bar::menus::map)
}
