//! Settings inside the third-party workspace.
use crate::app::{
    App, Message,
    components::{
        agent_apps_high_availability_policy, app_routing, openclaw_defaults, openclaw_env,
        openclaw_model, openclaw_tools, system_settings_apps, system_settings_data as data,
    },
    i18n::I18n,
    message::{ViewMessage, settings::AppsCatalogMessage},
};
use desktop_assets::assets::Icon;
use desktop_ui_controls::{
    settings_navigation::{ThirdPartyPage, ThirdPartySettingsTab as Tab},
    system_settings_common::*,
};
use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, keyed_column, row, scrollable, text},
};
use vw_api_app_identity::AppId;

pub(super) fn view(app: &App, tab: Tab) -> Element<'_, Message> {
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let tabs = settings_tab_row(
        [
            (Tab::Applications, i18n.__("应用配置")),
            (Tab::Routing, i18n.__("本地路由")),
            (Tab::Data, i18n.__("数据")),
        ]
        .map(|(target, label)| {
            settings_tab_button(
                i18n.__(label),
                tab == target,
                (tab != target).then_some(super::message(AppsCatalogMessage::Navigate(
                    ThirdPartyPage::Settings(target),
                ))),
            )
            .into()
        }),
    );
    let (icon, title) = match tab {
        Tab::Routing => (Icon::Bezier, i18n.__("本地路由")),
        Tab::Data => (Icon::HddNetwork, i18n.__("数据")),
        Tab::Applications => (Icon::Sliders, i18n.__("设置")),
    };
    let heading = row![
        container(desktop_ui_controls::icons::icon_svg(icon, 18.0))
            .center_x(36)
            .center_y(36)
            .style(settings_content_panel_style),
        text(i18n.__(title)).size(25).width(Length::Fill),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let mut body = column![].spacing(16);
    if let Some(error) = &app.apps_catalog.error {
        body = body.push(settings_error_banner(error.clone()));
    }
    body = body.push(match tab {
        Tab::Applications => applications(app),
        Tab::Routing => routing(app),
        Tab::Data => data_settings(app),
    });
    let content = scrollable(
        container(container(body).width(Length::Fill).max_width(820))
            .center_x(Length::Fill)
            .padding([16, 7]),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .direction(scrollable::Direction::Vertical(
        scrollable::Scrollbar::new().width(4).scroller_width(4),
    ));
    let header = container(column![heading, tabs.wrap(), settings_divider()].spacing(14))
        .width(Length::Fill)
        .max_width(820);

    column![
        container(header).center_x(Length::Fill).padding([0, 7]),
        keyed_column([(tab, content.into())]).height(Length::Fill),
    ]
    .padding(24)
    .height(Length::Fill)
    .into()
}

fn applications(app: &App) -> Element<'_, Message> {
    let i18n = desktop_i18n::I18n::new(app.general_settings.desktop_locale);
    let state = &app.apps_catalog;
    let apps = column![settings_group_heading(
        i18n.__("应用配置"),
        i18n.__("管理供应商与应用设置。已探测的配置目录会显示在对应应用下。"),
    )]
    .spacing(12);
    let can_reveal = cfg!(not(target_arch = "wasm32"))
        && crate::app::config::gateway_client().ok().is_some_and(|client| {
            crate::app::message::settings::data_management::locations_ui::can_reveal(
                client.endpoint().is_loopback(),
                &state.host,
            )
        });

    if state.rows.is_empty() {
        return apps
            .push(settings_panel(
                text(if state.loading {
                    i18n.__("正在读取应用目录…")
                } else {
                    i18n.__("请在「应用」中刷新目录。")
                })
                .size(13)
                .style(settings_muted_text_style),
            ))
            .into();
    }

    let mut entries = column![];
    for (index, app) in state.rows.iter().enumerate() {
        if index > 0 {
            entries = entries.push(settings_divider());
        }
        let id = app.entry.id;
        let heading = app_heading(id, &app.entry.name);
        let providers = button(text(i18n.__("供应商")).size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe((!state.loading).then(|| {
                system_settings_apps::domain_event(
                    desktop_settings_apps::system_settings_apps::Action::Providers(id),
                )
            }));
        let mut actions = row![providers].spacing(8);
        if id == AppId::OpenClaw {
            actions = actions
                .push(openclaw_model::entry())
                .push(openclaw_defaults::entry())
                .push(openclaw_env::entry())
                .push(openclaw_tools::entry());
        }
        let mut entry = column![heading, actions.wrap()].spacing(8);
        if let Ok(observation) = &app.observation {
            if let Some(desktop) = &observation.desktop
                && !desktop.config_directories.is_empty()
            {
                entry = entry.push(text(i18n.__("配置目录")).size(13));
                for path in &desktop.config_directories {
                    let mut location = row![
                        text(path.as_str())
                            .size(13)
                            .width(Length::Fill)
                            .wrapping(text::Wrapping::Glyph)
                            .style(settings_muted_text_style)
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center);
                    if can_reveal {
                        location = location.push(icon_btn(
                            Icon::FolderOpen,
                            i18n.__("定位目录"),
                            Some(Message::View(ViewMessage::OpenPathInFinder(path.clone()))),
                        ));
                    }
                    entry = entry.push(location);
                }
            }
        }
        entries = entries.push(container(entry).padding([14, 0]).width(Length::Fill));
    }
    apps.push(settings_group_card(entries)).into()
}

fn app_heading(id: AppId, name: &str) -> Element<'_, Message> {
    row![
        desktop_settings_apps::system_settings_apps::list::brand_icon(id, 18.0),
        text(name).size(15),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn routing(app: &App) -> Element<'_, Message> {
    let i18n = desktop_i18n::I18n::new(app.general_settings.desktop_locale);
    let catalog = &app.apps_catalog;
    column![
        column![text(i18n.__("服务")).size(18), app_routing::lifecycle::inline_service(catalog)]
            .spacing(12),
        routing_apps(app),
        failover(app),
        request_rectification(i18n),
    ]
    .spacing(28)
    .width(Length::Fill)
    .into()
}

fn request_rectification(i18n: desktop_i18n::I18n) -> Element<'static, Message> {
    let info = column![
        text(i18n.__("匹配到以下上游错误时，路由可修复请求并向同一供应商重试一次。"))
            .size(13)
            .style(settings_muted_text_style),
        settings_divider(),
        settings_field_label(i18n.__("Thinking 签名整流")),
        text(i18n.__("移除不兼容的 Thinking 内容。")).size(13).style(settings_muted_text_style),
        settings_divider(),
        settings_field_label(i18n.__("Thinking 预算整流")),
        text(i18n.__("修复预算和输出长度约束。")).size(13).style(settings_muted_text_style),
        settings_divider(),
        settings_field_label(i18n.__("图片输入降级")),
        text(i18n.__("用占位文本替换供应商无法接收的图片。"))
            .size(13)
            .style(settings_muted_text_style),
    ]
    .spacing(9);
    column![text(i18n.__("请求整流")).size(18), settings_group_card(info).padding([16, 22])]
        .spacing(12)
        .into()
}

fn routing_apps(app: &App) -> Element<'_, Message> {
    let i18n = desktop_i18n::I18n::new(app.general_settings.desktop_locale);
    let catalog = &app.apps_catalog;
    let observed = app_routing::lifecycle::attached_apps(catalog);
    let heading = match observed {
        Some((apps, true)) => {
            desktop_i18n::__!(i18n, "正在使用路由的应用 · %s", (apps.len()).to_string())
                .expect("valid extension display template")
        }
        _ => i18n.__("正在使用路由的应用").into(),
    };
    let mut list = column![].spacing(10);
    if let Some((apps, _)) = observed {
        for &id in apps {
            let name = catalog
                .rows
                .iter()
                .find(|row| row.entry.id == id)
                .map(|row| row.entry.name.as_str())
                .unwrap_or_else(|| routing_app_name(id));
            list = list.push(
                button(app_heading(id, name))
                    .padding([8, 12])
                    .style(rounded_action_btn_style)
                    .on_press(super::message(AppsCatalogMessage::SelectApp(id))),
            );
        }
    }
    let note = match observed {
        Some(([], true)) => {
            Some(i18n.__("当前项目没有应用接入。可在应用页的「路由」或「聚合」中接入。"))
        }
        Some((_, false)) => Some(i18n.__("部分应用的接入状态暂不可读。刷新服务后可重试。")),
        None if catalog.loading => Some(i18n.__("正在读取应用目录…")),
        None if !catalog.host.is_empty() => {
            Some(i18n.__("等待当前项目的接入状态。可通过刷新服务重新读取。"))
        }
        None => Some(i18n.__("连接执行主机后显示当前项目的接入应用。")),
        _ => None,
    };
    if let Some(note) = note {
        list = list.push(text(note).size(13).style(settings_muted_text_style));
    }
    column![text(heading).size(18), settings_group_card(list).padding([14, 22])].spacing(12).into()
}

fn failover(app: &App) -> Element<'_, Message> {
    let i18n = desktop_i18n::I18n::new(app.general_settings.desktop_locale);
    use crate::app::message::agent_apps_high_availability_policy::{self as policy, Event};

    let catalog = &app.apps_catalog;
    let state = &catalog.high_availability;
    let available = app_routing::lifecycle::inline_available(catalog);
    let selected = state.selected_app();
    let tabs = settings_tab_row(app_routing::lifecycle::ROUTING_APPS.map(|id| {
        let enabled = available
            && !state.busy
            && selected != Some(id)
            && catalog.rows.iter().any(|row| row.entry.id == id);
        settings_tab_button(
            routing_app_name(id),
            selected == Some(id),
            enabled.then(|| policy::message(Event::Open(id))),
        )
        .into()
    }));
    let mut content = column![tabs.wrap()].spacing(14);
    if state.open && available {
        content = content.push(agent_apps_high_availability_policy::inline(state));
    } else if !available {
        content = content.push(
            text(i18n.__("路由管理可用后，可按应用调整故障转移策略。"))
                .size(13)
                .style(settings_muted_text_style),
        );
    }
    if let Some(id) = selected.filter(|_| available) {
        content = content.push(
            row![
                app_routing::config::timeout_button(id),
                app_routing::outbound::entry(id),
                app_routing::requests::entry(id),
            ]
            .spacing(8)
            .wrap(),
        );
    }
    column![text(i18n.__("自动故障转移")).size(18), settings_group_card(content).padding([18, 22]),]
        .spacing(12)
        .into()
}

fn routing_app_name(id: AppId) -> &'static str {
    match id {
        AppId::Claude => "Claude Code",
        AppId::Codex => "Codex",
        AppId::Gemini => "Gemini CLI",
        AppId::GrokBuild => "Grok Build",
        _ => "应用",
    }
}

fn data_settings(app: &App) -> Element<'_, Message> {
    let i18n = desktop_i18n::I18n::new(app.general_settings.desktop_locale);
    column![
        text(i18n.__("备份与恢复")).size(18),
        settings_group_card(
            column![
                data::backup_list_ui::view(app),
                settings_divider(),
                crate::app::message::settings::backup_inventory::view(app),
            ]
            .spacing(14)
        )
        .padding([16, 20]),
    ]
    .spacing(12)
    .into()
}
