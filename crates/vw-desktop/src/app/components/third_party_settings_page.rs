//! Settings inside the third-party workspace.
use crate::app::{
    App, Message,
    components::{
        app_routing, openclaw_defaults, openclaw_env, openclaw_model, openclaw_tools,
        system_settings_apps, system_settings_data as data, system_settings_device_sync,
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
        [(Tab::Applications, "应用配置"), (Tab::Routing, "本地路由"), (Tab::Data, "数据")].map(
            |(target, label)| {
                settings_tab_button(
                    i18n.__(label),
                    tab == target,
                    (tab != target).then_some(super::message(AppsCatalogMessage::Navigate(
                        ThirdPartyPage::Settings(target),
                    ))),
                )
                .into()
            },
        ),
    );
    let heading = row![
        container(icon_svg(Icon::Sliders, 18.0))
            .center_x(36)
            .center_y(36)
            .style(settings_content_panel_style),
        text(i18n.__("设置")).size(25).width(Length::Fill),
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
    let state = &app.apps_catalog;
    let apps = column![settings_group_heading(
        "应用配置",
        "管理供应商与应用设置。已探测的配置目录会显示在对应应用下。",
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
                    "正在读取应用目录…"
                } else {
                    "请在「应用」中刷新目录。"
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
        let providers = button(text("供应商").size(SETTINGS_CONTROL_TEXT_SIZE))
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
                entry = entry.push(text("配置目录").size(13));
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
                            "定位目录",
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
    let catalog = &app.apps_catalog;
    let mut entries = column![].spacing(12);
    let mut routing_apps = catalog
        .rows
        .iter()
        .filter(|row| {
            matches!(row.entry.id, AppId::Claude | AppId::Codex | AppId::Gemini | AppId::GrokBuild)
        })
        .peekable();
    if routing_apps.peek().is_none() {
        entries = entries.push(settings_panel(
            text("应用目录加载后，即可配置支持路由的应用。")
                .size(13)
                .style(settings_muted_text_style),
        ));
    }
    for app in routing_apps {
        let id = app.entry.id;
        let policy = button(text("高可用策略").size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press(crate::app::message::agent_apps_high_availability_policy::message(
                crate::app::message::agent_apps_high_availability_policy::Event::Open(id),
            ));
        entries = entries.push(settings_panel(
            column![
                app_heading(id, &app.entry.name),
                row![
                    app_routing::config::timeout_button(id),
                    app_routing::outbound::entry(id),
                    app_routing::requests::entry(id),
                    policy,
                ]
                .spacing(8)
                .wrap(),
            ]
            .spacing(10),
        ));
    }

    column![
        app_routing::lifecycle::view(catalog),
        settings_group_heading("应用路由", "按应用调整超时、出站连接和故障转移策略。"),
        entries,
    ]
    .spacing(16)
    .into()
}

fn data_settings(app: &App) -> Element<'_, Message> {
    column![
        settings_panel(
            column![
                settings_group_heading(
                    "导入与导出",
                    "迁入前预览授权来源，或导出可迁移的脱敏配置。"
                ),
                row![data::migration_preview_ui::entry(), data::export_ui::view(app)]
                    .spacing(8)
                    .wrap(),
            ]
            .spacing(12)
        ),
        settings_panel(
            column![
                settings_group_heading(
                    "备份与恢复",
                    "设置自动备份策略，管理备份与恢复。"
                ),
                row![data::backup_list_ui::view(app), data::recovery_ui::view(app)]
                    .spacing(8)
                    .wrap(),
                settings_divider(),
                crate::app::message::settings::backup_inventory::view(app),
            ]
            .spacing(12)
        ),
        settings_panel(
            column![
                settings_group_heading("云同步", "使用 WebDAV 或 S3 同步设备配置。"),
                system_settings_device_sync::entry_points(),
            ]
            .spacing(12)
        ),
    ]
    .spacing(16)
    .into()
}
