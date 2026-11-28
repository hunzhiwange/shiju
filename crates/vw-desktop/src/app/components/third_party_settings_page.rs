//! Settings inside the third-party workspace.
use crate::app::{
    App, Message,
    components::{agent_apps_high_availability_policy, app_routing},
    i18n::I18n,
    message::settings::AppsCatalogMessage,
};
use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::{*, help};
use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, scrollable, text},
};
use vw_api_app_identity::AppId;

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let heading = row![
        container(desktop_ui_controls::icons::icon_svg(Icon::Bezier, 18.0))
            .center_x(36)
            .center_y(36)
            .style(settings_content_panel_style),
        text(i18n.__("本地路由")).size(SETTINGS_PAGE_TITLE_SIZE).width(Length::Fill),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let mut body = column![].spacing(16);
    if let Some(error) = &app.apps_catalog.error {
        body = body.push(settings_error_banner(error.clone()));
    }
    body = body.push(routing(app));
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
    let header = container(column![heading, settings_divider()].spacing(14))
        .width(Length::Fill)
        .max_width(820);

    column![
        container(header).center_x(Length::Fill).padding([0, 7]),
        content,
    ]
    .padding(24)
    .height(Length::Fill)
    .into()
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
        column![
            settings_group_header(text(i18n.__("服务")).size(18)),
            app_routing::lifecycle::inline_service(i18n, catalog),
        ]
        .spacing(12),
        routing_apps(app),
        failover(app),
        app_routing::lifecycle::inline_rectifier(i18n, catalog),
    ]
    .spacing(28)
    .width(Length::Fill)
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
            Some(i18n.__("当前项目没有应用接入。可在应用页的「路由」中接入。"))
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
    column![settings_group_header(text(heading).size(18)), settings_group_card(list).padding([14, 0])]
        .spacing(12)
        .into()
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
        content = content.push(agent_apps_high_availability_policy::inline(i18n, state));
    } else if !available {
        content = content.push(
            text(i18n.__("路由管理可用后，可按应用调整故障转移策略。"))
                .size(13)
                .style(settings_muted_text_style),
        );
    }
    let heading = help::settings_hover_help_label(
        text(i18n.__("自动故障转移")).size(18),
        text(agent_apps_high_availability_policy::overview(i18n)).size(13).into(),
    );
    column![settings_group_header(heading.wrap()), settings_group_card(content).padding([18, 0])]
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
