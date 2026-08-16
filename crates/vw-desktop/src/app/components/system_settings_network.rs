//! 网络工具配置页，统一承载网页搜索、HTTP 请求与浏览器配置。

use crate::app::components::system_settings::SystemTab;
use crate::app::components::system_settings_common::{
    settings_page_intro, settings_segment_button_style,
};
use crate::app::{App, Message, message};
use iced::widget::{button, column, row, text};
use iced::{Alignment, Element, Length, Theme};

/// 渲染网络配置及其内部标签。
pub fn view(app: &App) -> Element<'_, Message> {
    let active_tab = app.system_settings_tab;
    let tab_button = |tab, label| {
        let is_active = tab == active_tab;
        button(text(label).size(13))
            .on_press(Message::Settings(message::SettingsMessage::SystemTabSelected(tab)))
            .padding([8, 14])
            .style(move |theme: &Theme, status| {
                settings_segment_button_style(theme, status, is_active)
            })
    };

    let tabs = row![
        tab_button(SystemTab::WebSearch, "网页搜索"),
        tab_button(SystemTab::HttpRequest, "网络请求"),
        tab_button(SystemTab::Browser, "浏览器"),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let content = match active_tab {
        SystemTab::HttpRequest => {
            crate::app::components::system_settings_http_request::content(app)
        }
        SystemTab::Browser => crate::app::components::system_settings_browser::content(app),
        _ => crate::app::components::system_settings_web_search::content(app),
    };

    column![
        settings_page_intro(
            "网络配置",
            "通过标签切换网页搜索、网络请求与浏览器配置。",
        ),
        tabs,
        content,
    ]
    .spacing(16)
    .width(Length::Fill)
    .into()
}
