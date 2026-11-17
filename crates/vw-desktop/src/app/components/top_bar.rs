//! 顶部菜单栏组件。

mod controls;
mod drag;
mod external;
mod gateway;
pub(super) mod mapping;
pub(crate) mod menu_actions;
pub(crate) mod menus;
mod workspace_mode;

use crate::app::{App, Message};
use iced::Element;

/// 顶部菜单栏的高度（像素）
///
/// 此常量定义了顶部菜单栏的固定高度，用于布局计算和样式设置。
pub use desktop_navigation::components::top_bar::TOP_BAR_HEIGHT;

pub fn view(app: &App) -> Element<'_, Message> {
    let file_menu = menus::file_menu(app);
    let edit_menu = menus::edit_menu(app);
    let view_menu = menus::view_menu(app);
    let help_menu = menus::help_menu(app);
    let settings = controls::settings_button(crate::app::i18n::I18n::new(app.general_settings.desktop_locale));
    let open_external_module = iced::widget::row![
        crate::app::views::mobile::layout_switch(app),
        external::open_external_module(app),
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center)
    .into();
    let gateway_services = gateway::gateway_services_module(app);
    let project_view_tools = controls::project_view_tools(app);
    let traffic_light_spacer = drag::traffic_light_spacer();
    let drag_spacer = drag::drag_spacer();
    let workspace_mode = workspace_mode::selector(app);

    desktop_navigation::components::top_bar::toolbar(
        desktop_navigation::components::top_bar::Sections {
            traffic_light_spacer,
            workspace_mode,
            file_menu,
            edit_menu,
            view_menu,
            help_menu,
            drag_spacer,
            open_external_module,
            gateway_services,
            project_view_tools,
            settings,
        },
    )
}
