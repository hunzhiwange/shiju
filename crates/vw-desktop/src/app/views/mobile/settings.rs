//! 单列设置分类；详情复用真实设置页及网关消息链。
use crate::app::{App, Message, components::system_settings::{self, SystemTab}, message::{SettingsMessage, ViewMessage}};
use crate::app::components::system_settings_common::{settings_text_input, settings_muted_text_style};
use iced::{Element, Length};
use iced::widget::{column, container, scrollable, text};
use iced::widget::scrollable::{Direction, Scrollbar};

const CATEGORIES: &[(&str, SystemTab)] = &[
    ("网关连接与认证", SystemTab::GatewayClient),
    ("外观与主题", SystemTab::Themes),
    ("常规", SystemTab::General),
    ("归档会话", SystemTab::ArchivedChats),
];

fn matches_query(label: &str, tab: SystemTab, query: &str) -> bool {
    let needle = query.trim().to_lowercase();
    label.to_lowercase().contains(&needle)
        || system_settings::system_tab_keywords(tab).iter().any(|word| word.to_lowercase().contains(&needle))
}

pub fn view(app: &App) -> Element<'_, Message> {
    if app.mobile_layout.settings_detail {
        return system_settings::view(app);
    }
    let search = settings_text_input("搜索设置", &app.system_settings_query)
        .on_input(|value| Message::Settings(SettingsMessage::SystemTabQueryChanged(value)))
        .width(Length::Fill);
    let mut categories = column![].spacing(8).width(Length::Fill);
    let mut found = false;
    for &(label, tab) in CATEGORIES {
        if matches_query(label, tab, &app.system_settings_query) {
            found = true;
            categories = categories.push(container(super::action(label, ViewMessage::MobileSettingsTab(tab))).width(Length::Fill));
        }
    }
    if !found {
        categories = categories.push(text("没有匹配的移动设置").style(settings_muted_text_style));
    }
    categories = categories.push(text("浏览器使用远端网关；localhost 指当前手机。项目、模型及智能体等高级配置请切换桌面布局。")
        .size(13).style(settings_muted_text_style));
    column![search, scrollable(container(categories).padding([0, 7]))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Fill)].spacing(12).height(Length::Fill).into()
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
