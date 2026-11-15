//! 顶部菜单栏组件。

pub(super) mod mapping;
mod controls;
mod drag;
mod external;
mod gateway;
pub(crate) mod menu_actions;
pub(crate) mod menus;
mod widgets;
mod workspace_mode;

use crate::app::{App, Message};
use iced::Element;
#[cfg(test)]
pub(super) use desktop_navigation::components::top_bar::top_bar_style;

/// 顶部菜单栏的高度（像素）
///
/// 此常量定义了顶部菜单栏的固定高度，用于布局计算和样式设置。
pub use desktop_navigation::components::top_bar::TOP_BAR_HEIGHT;

pub fn view(app: &App) -> Element<'_, Message> {
    let file_menu = menus::file_menu(app);
    let edit_menu = menus::edit_menu(app);
    let view_menu = menus::view_menu(app);
    let help_menu = menus::help_menu(app);
    let settings = controls::settings_button(crate::app::i18n::I18n::new(app.desktop_locale));
    let open_external_module = external::open_external_module(app);
    let gateway_services = gateway::gateway_services_module(app);
    let project_view_tools = controls::project_view_tools(app);
    let traffic_light_spacer = drag::traffic_light_spacer();
    let drag_spacer = drag::drag_spacer();
    let workspace_mode = workspace_mode::selector(app);

    desktop_navigation::components::top_bar::view(vec![traffic_light_spacer, workspace_mode, file_menu, edit_menu, view_menu, help_menu, drag_spacer, open_external_module, gateway_services, project_view_tools, settings])
}

#[cfg(test)]
mod controls_tests;
#[cfg(test)]
mod external_tests;
#[cfg(test)]
mod menus_tests;
#[cfg(test)]
#[path = "widgets_tests.rs"]
mod widgets_tests;
#[cfg(test)]
#[path = "top_bar/workspace_mode_tests.rs"]
mod workspace_mode_tests;
