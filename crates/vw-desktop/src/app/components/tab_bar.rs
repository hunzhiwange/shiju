//! 标签栏的桌面消息适配。
pub use desktop_navigation::components::tab_bar::TAB_BAR_HEIGHT;
use crate::app::{App, Message};
pub fn view(app: &App) -> iced::Element<'_, Message> {
 desktop_navigation::components::tab_bar::view(&desktop_navigation::components::tab_bar::State {
 open_tabs: &app.navigation_tabs.open_tabs, active_tab_id: &app.navigation_tabs.active_tab_id, hovered_tab_id: &app.navigation_tabs.hovered_tab_id,
 screen: app.navigation_tabs.screen, desktop_locale: app.desktop_locale,
 }).map(super::top_bar::mapping::map_message)
}
