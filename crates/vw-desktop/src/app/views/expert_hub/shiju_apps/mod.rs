use iced::widget::{button, column, container, row, text};
use iced::{Element, Length};

use crate::app::message::ShijuAppsMessage;
use crate::app::shiju_apps::ShijuAppsTab;
use crate::app::{App, Message};

mod detail;
pub(super) mod mine;
mod store;
pub(super) mod styles;

pub(super) fn tab_specs() -> &'static [(&'static str, ShijuAppsTab)] {
    &[("我的应用", ShijuAppsTab::Mine), ("应用商店", ShijuAppsTab::Store)]
}

fn tab_button(label: &'static str, tab: ShijuAppsTab, selected: bool) -> Element<'static, Message> {
    button(text(label).size(14))
        .on_press(Message::ShijuApps(ShijuAppsMessage::SelectTab(tab)))
        .padding([8, 14])
        .style(move |theme, status| styles::tab_style(theme, status, selected))
        .into()
}

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let content = if let Some(target) = app.expert_hub.shiju_apps.detail {
        detail::view(app, target)
    } else {
        let tabs = tab_specs().iter().fold(row!().spacing(8), |tabs, (label, tab)| {
            tabs.push(tab_button(label, *tab, app.expert_hub.shiju_apps.tab == *tab))
        });
        let body = match app.expert_hub.shiju_apps.tab {
            ShijuAppsTab::Store => store::view(&app.expert_hub.shiju_apps.store),
            ShijuAppsTab::Mine => mine::view(&app.expert_hub.shiju_apps.mine),
        };

        column![
            text("识矩应用").size(24),
            text("浏览平台应用，或查看当前账户已经安装的应用。")
                .size(13)
                .style(styles::muted_text_style),
            tabs,
            body,
        ]
        .spacing(14)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    };

    container(content).width(Length::Fill).height(Length::Fill).padding([24, 28]).into()
}
