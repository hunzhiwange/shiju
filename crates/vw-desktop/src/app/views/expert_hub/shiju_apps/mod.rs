//! 组合识矩应用商店、我的应用、详情与密钥绑定视图。

use iced::widget::{button, column, container, row, text};
use iced::{Element, Length};

use crate::app::message::ShijuAppsMessage;
use crate::app::shiju_apps::ShijuAppsTab;
use crate::app::{App, Message};

pub(super) mod api_key_binding;
pub(super) mod mine;
pub(super) mod styles;

pub(super) use desktop_expert_hub::views::shiju_apps::app_title;

pub(super) fn tab_specs() -> &'static [(&'static str, ShijuAppsTab)] {
    &[("我的应用", ShijuAppsTab::Mine), ("应用商店", ShijuAppsTab::Store)]
}

fn tab_button(
    i18n: crate::app::i18n::I18n,
    label: &'static str,
    tab: ShijuAppsTab,
    selected: bool,
) -> Element<'static, Message> {
    button(text(i18n.__(label)).size(14))
        .on_press(Message::ShijuApps(ShijuAppsMessage::SelectTab(tab)))
        .padding([8, 14])
        .style(move |theme, status| styles::tab_style(theme, status, selected))
        .into()
}

/// 组合应用页签、详情与密钥绑定覆盖层。
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let content = if let Some(target) = app.shiju_apps.detail {
        desktop_expert_hub::views::shiju_apps::detail::view(&input(app), target).map(map)
    } else {
        let tabs = tab_specs().iter().fold(row!().spacing(8), |tabs, (label, tab)| {
            tabs.push(tab_button(
                crate::app::i18n::I18n::new(app.desktop_locale),
                label,
                *tab,
                app.shiju_apps.tab == *tab,
            ))
        });
        let body = match app.shiju_apps.tab {
            ShijuAppsTab::Store => desktop_expert_hub::views::shiju_apps::store::view(
                crate::app::i18n::I18n::new(app.desktop_locale),
                &app.shiju_apps.store,
            ).map(map),
            ShijuAppsTab::Mine => desktop_expert_hub::views::shiju_apps::mine::view(
                crate::app::i18n::I18n::new(app.desktop_locale),
                &app.shiju_apps.mine,
            ).map(map),
        };

        column![
            text(crate::app::i18n::I18n::new(app.desktop_locale).__("识矩应用")).size(24),
            text(
                crate::app::i18n::I18n::new(app.desktop_locale)
                    .__("浏览平台应用，或查看当前账户已经安装的应用。")
            )
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

    let base = container(column![super::section_tabs(app), content].spacing(24))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([24, 28])
        .into();
    with_overlay(app, base)
}

fn input(app: &App) -> desktop_expert_hub::views::shiju_apps::ViewInput<'_> {
 desktop_expert_hub::views::shiju_apps::ViewInput { shiju_apps: &app.shiju_apps, desktop_locale: app.desktop_locale }
}
fn map(message: desktop_expert_hub::views::shiju_apps::Message) -> Message {
 match message { desktop_expert_hub::views::shiju_apps::Message::ShijuApps(m) => Message::ShijuApps(m), desktop_expert_hub::views::shiju_apps::Message::StartBrowserLogin => Message::Account(crate::app::message::AccountMessage::StartBrowserLogin) }
}
fn with_overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
 desktop_expert_hub::views::shiju_apps::api_key_binding::with_overlay(&input(app), base, map)
}
