//! 移动外壳；全局通知、认证和审批浮层由 App::view 统一挂载。
use crate::app::{App, Message, Screen, state::LayoutPreference};
use crate::app::message::ViewMessage;
use iced::{Element, Length};
use iced::widget::{column, row, container, text};
pub mod navigation;
pub mod chat;
pub mod settings;
pub mod dialogs;

pub(super) fn action(label: &str, message: ViewMessage) -> Element<'_, Message> {
    iced::widget::button(text(label))
        .on_press(Message::View(message))
        .padding([10, 12]).height(44)
        .style(crate::app::components::system_settings_common::rounded_action_btn_style)
        .into()
}

pub fn view(app: &App) -> Element<'_, Message> {
    let title = if app.show_system_settings { "设置" } else { "会话" };
    let header = row![action("返回", ViewMessage::MobileBack),
        container(text(title)).width(Length::Fill).center_x(Length::Fill),
        action("项目", ViewMessage::MobileDrawerToggled)].spacing(4);
    let layouts = layout_selector(app);
    let content = if app.mobile_layout.drawer_open {
        navigation::view(app)
    } else if app.show_system_settings {
        settings::view(app)
    } else if matches!(app.screen, Screen::Home | Screen::Project) {
        chat::view(app)
    } else {
        column![text("此工具尚未适配手机，请切换桌面布局使用。"),
            action("返回会话", ViewMessage::MobileChat),
            action("桌面布局", ViewMessage::LayoutSelected(LayoutPreference::Desktop))].spacing(12).into()
    };
    let base = column![header, layouts,
        container(content).width(Length::Fill).height(Length::Fill),
        row![action("会话", ViewMessage::MobileChat), action("设置", ViewMessage::MobileSettings)].spacing(8)
    ].spacing(4).padding(7).width(Length::Fill).height(Length::Fill).into();
    dialogs::view(app, base)
}

pub(crate) fn layout_selector(app: &App) -> Element<'_ , Message> {
     row![
        action("Auto", ViewMessage::LayoutSelected(LayoutPreference::Auto)),
        action("Desktop", ViewMessage::LayoutSelected(LayoutPreference::Desktop)),
        action("Mobile", ViewMessage::LayoutSelected(LayoutPreference::Mobile)),
        text(match app.mobile_layout.preference() {
            LayoutPreference::Auto => "自动", LayoutPreference::Desktop => "桌面", LayoutPreference::Mobile => "手机",
        }),
    ].spacing(4).align_y(iced::Alignment::Center).into()
}
