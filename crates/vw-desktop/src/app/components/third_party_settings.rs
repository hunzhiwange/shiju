//! Final desktop composition for third-party resources and application details.
#[path = "third_party_settings_page.rs"]
mod settings_page;
use super::system_settings_common::{
    icon_btn, icon_svg, rounded_action_btn_style, settings_divider, settings_muted_text_style,
    settings_page_heading, settings_tab_button, settings_tab_row,
};
use crate::app::{
    App, Message,
    message::{SettingsMessage, ViewMessage, settings::AppsCatalogMessage},
    views::usage::center as usage,
};
use desktop_assets::assets::Icon;
use desktop_ui_controls::{
    settings_navigation::ThirdPartyPage,
    system_settings_common::{SETTINGS_PAGE_TITLE_SIZE, settings_content_panel_style},
    tooltip::tooltip_dark_style,
};
use iced::{
    Alignment, Element, Length, Task, Theme,
    widget::{Space, button, column, container, keyed_column, row, rule, scrollable, text, tooltip},
};

fn message(event: AppsCatalogMessage) -> Message {
    Message::Settings(SettingsMessage::AppsCatalog(event))
}

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    if usage::is_visible(app) {
        return workspace(app, container(usage::view(app)).padding([12, 16]).into());
    }

    let state = &app.apps_catalog;
    let i18n = crate::app::i18n::I18n::new(app.general_settings.desktop_locale);
    let body: Element<'_, Message> = match state.third_party_page {
        ThirdPartyPage::Settings => return workspace(app, settings_page::view(app)),
        ThirdPartyPage::AppsCatalog => {
            return workspace(app, super::system_settings_apps::view(state, i18n));
        }
        ThirdPartyPage::Agents => return workspace(app, agent_workspace(app)),
        ThirdPartyPage::Mcp | ThirdPartyPage::Skills => return workspace(app, Space::new().into()),
        ThirdPartyPage::Prompts => {
            return workspace(app, super::system_settings_prompts::list::page(app));
        }
        ThirdPartyPage::Sessions => super::third_party_resources::sessions(app),
        #[cfg(not(target_arch = "wasm32"))]
        ThirdPartyPage::Authorization => {
            let mut content = column![].spacing(12).height(Length::Fill);
            if let Some(error) = &state.error {
                content = content
                    .push(super::system_settings_common::settings_error_banner(error.clone()));
            }
            content = content.push(super::third_party_resources::authorization(app));
            return workspace(app, container(content).padding(24).into());
        }
        #[cfg(target_arch = "wasm32")]
        ThirdPartyPage::Authorization => text(i18n.__("此平台不支持授权管理")).into(),
    };
    let mut workspace = column![].spacing(16);
    if state.third_party_page != ThirdPartyPage::Sessions {
        let title = ThirdPartyPage::ALL
            .iter()
            .find(|(page, _)| *page == state.third_party_page)
            .map(|(_, label)| i18n.__(label))
            .unwrap_or("");
        workspace = workspace.push(settings_page_heading(title, ""));
    }
    if let Some(error) = &state.error {
        workspace =
            workspace.push(super::system_settings_common::settings_error_banner(error.clone()));
    }
    if state.third_party_page == ThirdPartyPage::Sessions {
        workspace = workspace.push(container(body).padding([0, 7]).height(Length::Fill));
        return self::workspace(app, container(workspace.height(Length::Fill)).padding(24).into());
    }
    workspace = workspace.push(
        scrollable(container(body).padding([0, 7]))
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4),
            ))
            .height(Length::Fill),
    );
    self::workspace(app, container(workspace).padding(24).into())
}

fn agent_workspace(app: &App) -> Element<'_, Message> {
    let i18n = desktop_i18n::I18n::new(app.general_settings.desktop_locale);
    use super::system_settings_apps;
    use desktop_settings_apps::system_settings_apps::{details, list, preferences};

    let state = &app.apps_catalog;
    let projection = system_settings_apps::view_state(state);
    let selected = state.rows.iter().find(|row| Some(row.entry.id) == state.expanded);
    let (header, body): (Element<'_, Message>, Element<'_, Message>) = if let Some(agent) = selected {
        use super::app_routing::lifecycle as routing;
        use super::system_settings_common::{settings_panel, settings_value_badge};
        use desktop_apps_ui::catalog_state::AgentWorkspaceTab as Tab;
        use desktop_settings_apps::system_settings_apps::Action;

        let id = agent.entry.id;
        let tab = state.agent_workspace_tab;
        let configuration = tab == Tab::Configuration;
        let routing_attached =
            routing::attached_apps(state).is_some_and(|(apps, _)| apps.contains(&id));
        let title = || {
            let mut title = row![list::brand_icon(id, 22.0), text(&agent.entry.name).size(SETTINGS_PAGE_TITLE_SIZE)]
                .spacing(10)
                .align_y(Alignment::Center);
            if let Some(snapshot) =
                app.app_providers.snapshot.as_ref().filter(|_| has_inline_providers(app))
            {
                use vw_api_app_provider_edit::catalog::ProviderSelection;
                if let ProviderSelection::Selected { id } = &snapshot.catalog.in_use {
                    let label = snapshot
                        .catalog
                        .rows
                        .iter()
                        .find(|row| row.provider.id == *id)
                        .map_or(id.as_str(), |row| row.provider.name.as_str());
                    let mode = if routing_attached { i18n.__("路由") } else { i18n.__("当前") };
                    title = title.push(settings_value_badge(format!("{mode} · {label}")));
                }
            }
            title.wrap()
        };
        let select = |target| message(AppsCatalogMessage::AgentWorkspaceTab(target));
        let tabs = || {
            use desktop_ui_controls::system_settings_common::help::settings_hover_help_label;
            let modes: [(Tab, &str, bool); 2] = [
                (Tab::Direct, i18n.__("直连"), true),
                (Tab::Routing, i18n.__("路由"), routing::ROUTING_APPS.contains(&id)),
            ];
            let buttons = settings_tab_row(
                modes.into_iter().filter(|(_, _, supported)| *supported).map(|(target, label, _)| {
                    settings_tab_button(
                        label,
                        tab == target,
                        (tab != target).then(|| select(target)),
                    )
                    .into()
                }),
            )
            .spacing(2);
            let segmented = container(buttons).padding(3).style(settings_content_panel_style);
            let help = settings_hover_help_label(
                text("").size(1),
                column![
                    text(i18n.__("连接模式")).size(14),
                    text(i18n.__("直连 · 智能体直接连接供应商。")).size(13),
                    text(i18n.__("路由 · 请求由路由服务转发，可配置备用队列。")).size(13),
                ]
                .spacing(6)
                .into(),
            );
            row![segmented, help].spacing(6).align_y(Alignment::Center).wrap()
        };
        if !configuration && has_inline_providers(app) {
            let lead = match tab {
                Tab::Routing => Some(
                    column![
                        routing::view(i18n, state),
                        settings_panel(
                            row![
                        super::app_routing::config::timeout_button(i18n, id),
                        super::app_routing::outbound::entry(i18n, id),
                        super::app_routing::requests::entry(i18n, id),
                        button(text(i18n.__("备用队列")).size(13)).padding([8, 12])
                            .style(rounded_action_btn_style)
                            .on_press(super::app_providers::list::map(
                                desktop_provider_apps::components::list::ListMessage::Failover)),
                    ]
                            .spacing(8)
                            .wrap()
                        ),
                    ]
                    .spacing(10)
                    .into(),
                ),
                Tab::Direct if routing_attached => Some(
                    settings_panel(
                        row![
                            text(i18n.__("当前连接由路由服务管理"))
                                .size(13)
                                .width(Length::Fill),
                            button(text(i18n.__("查看路由")).size(13))
                                .padding([8, 12])
                                .style(rounded_action_btn_style)
                                .on_press(select(Tab::Routing)),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    )
                    .into(),
                ),
                _ => None,
            };
            let tools = vec![
                (
                    i18n.__("查看会话"),
                    message(AppsCatalogMessage::Navigate(ThirdPartyPage::Sessions)),
                ),
                (i18n.__("安装与升级"), select(Tab::Configuration)),
                (
                    i18n.__("侧栏应用设置"),
                    system_settings_apps::domain_event(Action::PreferencesOpen),
                ),
            ];
            let can_launch =
                desktop_settings_apps::system_settings_apps::presentation::preferred(agent)
                    .is_some_and(|installation| {
                        matches!(
                            installation.runtime_status,
                            vw_api_app_identity::AppInstallationRuntimeStatus::Runnable
                        )
                    });
            let page = super::app_providers::list::page(
                &app.app_providers,
                provider_help(app),
                id,
                title().into(),
                desktop_provider_apps::components::list::Page {
                    tabs: tabs().into(),
                    lead,
                    can_launch,
                    can_switch: tab == Tab::Direct && !routing_attached,
                    tools,
                    search: Some((
                        state.agent_provider_search_open || !app.app_providers.query.is_empty(),
                        message(AppsCatalogMessage::ToggleAgentProviderSearch),
                    )),
                },
            );
            return container(keyed_column([(id, page)]).height(Length::Fill))
                .padding([16, 18])
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }
        if !configuration {
            use crate::app::message::app_providers::list::{self as providers, ListMessage};
            let empty = column![
                text(i18n.__("供应商列表尚未加载")).size(15),
                button(text(i18n.__("加载供应商")).size(13))
                    .padding([8, 12])
                    .style(rounded_action_btn_style)
                    .on_press(providers::message(ListMessage::Open(agent.entry.id))),
            ]
            .spacing(10);
            (column![title(), tabs()].spacing(12).into(), empty.into())
        } else {
            let actions = details::workspace_actions(i18n,
                &projection,
                agent,
                system_settings_apps::local::available(),
                system_settings_apps::domain_event,
            );
            (
                column![
                    title(),
                    row![
                        button(text(i18n.__("返回供应商")).size(13))
                            .padding([8, 12])
                            .style(rounded_action_btn_style)
                            .on_press(select(Tab::Direct)),
                        actions,
                    ]
                    .spacing(8)
                    .wrap()
                ]
                .spacing(12)
                .into(),
                system_settings_apps::details::view(i18n, state, agent),
            )
        }
    } else {
        let mut choices = column![].width(Length::Fill).spacing(8);
        let visible = preferences::visible_rows(&projection);
        if visible.is_empty() {
            choices = choices.push(
                text(if state.loading {
                    i18n.__("正在读取智能体…")
                } else {
                    i18n.__("没有可选的智能体")
                })
                .size(13)
                .style(settings_muted_text_style),
            );
        }
        for agent in visible {
            let entry = &agent.entry;
            choices = choices.push(
                button(
                    row![
                        container(list::brand_icon(entry.id, 22.0)).center_x(36).center_y(36),
                        text(&entry.name).size(15).width(Length::Fill),
                        navigation_icon(Icon::ChevronRight),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding([8, 12])
                .style(|theme, status| {
                    let mut style = rounded_action_btn_style(theme, status);
                    if status == button::Status::Active {
                        let panel = settings_content_panel_style(theme);
                        style.background = panel.background;
                        style.border = panel.border;
                    }
                    style
                })
                .on_press(message(AppsCatalogMessage::SelectApp(entry.id))),
            );
        }
        (
            text(i18n.__("选择第三方智能体")).size(22).into(),
            container(choices).width(Length::Fill).max_width(560).into(),
        )
    };

    let mut content =
        column![container(header).padding([0, 7]).width(Length::Fill), settings_divider()]
            .spacing(12)
            .width(Length::Fill);
    if let Some(error) = &state.error {
        content = content.push(super::system_settings_common::settings_error_banner(error.clone()));
    }
    let scroll = scrollable(container(body).width(Length::Fill).padding([0, 7]))
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        ));
    content
        .push(keyed_column([(state.expanded, scroll.into())]).height(Length::Fill))
        .padding([16, 18])
        .height(Length::Fill)
        .into()
}

fn workspace<'a>(app: &'a App, content: Element<'a, Message>) -> Element<'a, Message> {
    row![
        sidebar(app),
        rule::vertical(1),
        container(content).width(Length::Fill).height(Length::Fill),
    ]
    .height(Length::Fill)
    .into()
}

fn sidebar(app: &App) -> Element<'_, Message> {
    let _i18n = desktop_i18n::I18n::new(app.general_settings.desktop_locale);
    let state = &app.apps_catalog;
    let i18n = crate::app::i18n::I18n::new(app.general_settings.desktop_locale);
    let showing_usage = usage::is_visible(app);
    let mut agents = column![].spacing(2);
    for row in desktop_settings_apps::system_settings_apps::preferences::visible_rows(
        &super::system_settings_apps::view_state(state),
    ) {
        let entry = &row.entry;
        let active = !showing_usage
            && state.third_party_page == ThirdPartyPage::Agents
            && state.expanded == Some(entry.id);
        let icon = desktop_settings_apps::system_settings_apps::list::brand_icon(entry.id, 18.0);
        agents = agents.push(sidebar_action(
            icon,
            &entry.name,
            None,
            active,
            message(AppsCatalogMessage::SelectApp(entry.id)),
        ));
    }

    let mut resources = column![].spacing(2);
    for &(page, label) in ThirdPartyPage::ALL {
        let icon = match page {
            ThirdPartyPage::Agents | ThirdPartyPage::AppsCatalog | ThirdPartyPage::Settings => continue,
            ThirdPartyPage::Mcp => Icon::HddNetwork,
            ThirdPartyPage::Skills => Icon::Box,
            ThirdPartyPage::Prompts => Icon::Box,
            ThirdPartyPage::Sessions => Icon::Clock,
            ThirdPartyPage::Authorization => Icon::ShieldLock,
        };
        resources = resources.push(sidebar_action(
            navigation_icon(icon),
            i18n.__(label),
            None,
            !showing_usage && state.third_party_page == page,
            message(AppsCatalogMessage::Navigate(page)),
        ));
    }
    let today = usage::today_summary(app);
    let statistics = sidebar_action(
        navigation_icon(Icon::BarChartFill),
        i18n.__("用量统计"),
        today.as_ref().map(|(summary, _)| summary.clone()),
        showing_usage,
        Message::View(ViewMessage::UsageCenter(usage::Event::Load(
            desktop_usage::center::range_picker::Preset::Month,
        ))),
    );
    let statistics: Element<'_, Message> = match today {
        Some((_, details)) => tooltip(statistics, text(details).size(13), tooltip::Position::Right)
            .style(tooltip_dark_style)
            .gap(6)
            .snap_within_viewport(true)
            .into(),
        None => statistics,
    };

    let header = row![
        text(i18n.__("第三方智能体")).size(15).width(Length::Fill),
        icon_btn(
            Icon::ArrowClockwise,
            i18n.__("刷新目录"),
            Some(message(AppsCatalogMessage::Refresh))
        ),
    ]
    .padding([2, 8])
    .spacing(6)
    .align_y(iced::Alignment::Center);

    let footer = row![
        sidebar_action(navigation_icon(Icon::Grid1x2), i18n.__("应用"), None, !showing_usage && state.third_party_page == ThirdPartyPage::AppsCatalog, message(AppsCatalogMessage::Navigate(ThirdPartyPage::AppsCatalog))),
        sidebar_action(
            navigation_icon(Icon::Bezier),
            i18n.__("本地路由"),
            None,
            !showing_usage && state.third_party_page == ThirdPartyPage::Settings,
            message(AppsCatalogMessage::Navigate(ThirdPartyPage::Settings)),
        ),
    ].spacing(4);

    container(column![
        header,
        scrollable(agents)
            .direction(scrollable::Direction::Vertical(scrollable::Scrollbar::new().width(4).scroller_width(4)))
            .width(Length::Fill)
            .height(Length::Fill),
        rule::horizontal(1),
        resources.push(statistics),
        rule::horizontal(1),
        footer,
    ].spacing(7).width(Length::Fill))
    .padding([10, 8])
    .width(214)
    .height(Length::Fill)
    .style(|theme: &Theme| {
        container::Style::default().background(theme.palette().text.scale_alpha(0.035))
    })
    .into()
}

fn navigation_icon(icon: Icon) -> Element<'static, Message> {
    icon_svg(icon, 16.0)
        .style(|theme: &Theme, _| iced::widget::svg::Style {
            color: Some(theme.palette().text.scale_alpha(0.74)),
        })
        .into()
}

fn sidebar_action<'a>(
    icon: Element<'a, Message>,
    label: &'a str,
    trailing: Option<String>,
    active: bool,
    event: Message,
) -> Element<'a, Message> {
    let mut content = row![container(icon).center_x(20), text(label).size(13)]
        .spacing(8)
        .align_y(iced::Alignment::Center);
    if let Some(trailing) = trailing {
        content = content.push(Space::new().width(Length::Fill))
            .push(text(trailing).size(12).style(text::secondary));
    }
    button(container(content).center_y(Length::Fill))
        .width(Length::Fill)
        .height(30)
        .padding([0, 8])
        .style(move |theme, status| {
            let ink = theme.palette().text;
            let mut style = rounded_action_btn_style(theme, status);
            style.border = iced::border::rounded(7);
            style.background = match status {
                button::Status::Pressed => Some(ink.scale_alpha(0.11).into()),
                button::Status::Hovered if active => Some(ink.scale_alpha(0.09).into()),
                _ if active => Some(ink.scale_alpha(0.07).into()),
                button::Status::Hovered => Some(ink.scale_alpha(0.045).into()),
                _ => None,
            };
            style
        })
        .on_press(event)
        .into()
}

pub(crate) fn navigate(app: &mut App, page: ThirdPartyPage) -> Task<Message> {
    if super::system_settings_prompts::list::switch_blocked(&app.apps_catalog.prompts) {
        return Task::none();
    }
    if app.usage_center.open {
        app.usage_center.close();
    }
    invalidate(app);
    match page {
        ThirdPartyPage::Mcp => return super::system_settings_mcp::open(app),
        ThirdPartyPage::Skills => return super::system_settings_skills::open(app),
        _ => {}
    }
    app.apps_catalog.third_party_page = page;
    use crate::app::message::settings;
    match page {
        ThirdPartyPage::Agents => ensure_agent_providers(app),
        ThirdPartyPage::AppsCatalog => settings::apps_catalog::update(app, AppsCatalogMessage::EnsureLoaded),
        ThirdPartyPage::Prompts => settings::apps_catalog::update(app, AppsCatalogMessage::EnsureLoaded),
        ThirdPartyPage::Settings => {
            let load = settings::apps_catalog::update(app, AppsCatalogMessage::EnsureLoaded);
            use super::app_routing::lifecycle::{self, Event, Operation};
            Task::batch([load, lifecycle::update(app, Event::Run(Operation::Refresh))])
        }
        ThirdPartyPage::Mcp | ThirdPartyPage::Skills => Task::none(),
        ThirdPartyPage::Sessions => {
            app.apps_catalog.external_sessions.open = true;
            super::external_sessions::update(app, super::external_sessions::Event::Refresh)
        }
        #[cfg(not(target_arch = "wasm32"))]
        ThirdPartyPage::Authorization => crate::app::message::agent_apps_auth::update(
            app,
            crate::app::message::agent_apps_auth::Event::Refresh,
        ),
        #[cfg(target_arch = "wasm32")]
        ThirdPartyPage::Authorization => Task::none(),
    }
}

/// Invalidate transient UI and request generations without changing domain targets.
pub(crate) fn invalidate(app: &mut App) {
    app.apps_catalog.commands.invalidate();
    super::system_settings_skills::invalidate(app);
    let s = &mut app.apps_catalog;
    s.navigation_generation = s.navigation_generation.wrapping_add(1);
    s.agent_workspace_tab = Default::default();
    s.agent_provider_search_open = false;
    s.external_sessions.invalidate();
    s.launch.invalidate();
    s.preferences.invalidate();
    s.lifecycle.invalidate();
    s.workspace_files.invalidate();
    s.daily_memory.invalidate();
    s.hermes_memory.invalidate();
    s.openclaw_catalog.invalidate();
    s.openclaw_model.invalidate();
    s.openclaw_tools.invalidate();
    s.openclaw_env.invalidate();
    s.openclaw_defaults.invalidate();
    s.hermes_dashboard.invalidate();
    s.prompts.invalidate();
    s.routing.invalidate();
    s.high_availability.invalidate();
    s.connection_open = false;
    // Provider overlays retain their own request scope; dismiss their presentation.
    app.app_providers.scope = None;
    #[cfg(not(target_arch = "wasm32"))]
    {
        app.agent_apps_auth.form = None;
    }
}

#[cfg(test)]
#[path = "third_party_settings_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "third_party_settings_integration_tests.rs"]
mod integration_tests;

/// Main-page composition; all configuration dialogs stay attached to this page.
pub(crate) fn standalone(app: &App) -> Element<'_, Message> {
    let i18n = desktop_i18n::I18n::new(app.general_settings.desktop_locale);
    if usage::is_visible(app) {
        return view(app);
    }
    let base = view(app);
    let mut base = super::system_settings_apps::view_overlays(
        app,
        crate::app::i18n::I18n::new(app.general_settings.desktop_locale),
        &app.appearance.app_theme,
        base,
    );
    if has_inline_providers(app) {
        base = super::app_providers::list::overlay(
            i18n,
            &app.app_providers,
            provider_help(app),
            base,
        );
    }
    if crate::app::message::agent_apps_high_availability::can_show(app) {
        base =
            super::agent_apps_high_availability::queue::overlay(i18n, &app.app_providers.failover, base);
    }
    if crate::app::message::agent_apps_diagnostics::model::can_show(app) {
        base = super::agent_apps_diagnostics::model_panel::overlay(i18n,
            &app.apps_catalog.model_diagnostics,
            base,
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        base = super::apps_usage_settings::overlay(app, base);
    }
    #[cfg(not(target_arch = "wasm32"))]
    if app.apps_catalog.third_party_page == ThirdPartyPage::Authorization {
        base = super::system_settings_auth::overlay(desktop_i18n::I18n::new(app.general_settings.desktop_locale), &app.agent_apps_auth, base);
    }
    crate::app::message::app_providers::leave::overlay(app, base)
}

fn provider_help(app: &App) -> super::app_providers::list::Help<'_> {
    let help = super::app_providers::list::Help::new(
        crate::app::i18n::I18n::new(app.general_settings.desktop_locale),
        &app.appearance.app_theme,
    );
    #[cfg(not(target_arch = "wasm32"))]
    let help = help.with_usage(app);
    help
}

fn has_inline_providers(app: &App) -> bool {
    app.apps_catalog.third_party_page == ThirdPartyPage::Agents
        && app.app_providers.scope.as_ref().is_some_and(|scope| {
            Some(scope.app) == app.apps_catalog.expanded
                && scope.host == app.apps_catalog.host
                && app.apps_catalog.context.as_ref() == Some(&scope.context)
        })
}

pub(crate) fn ensure_agent_providers(app: &mut App) -> Task<Message> {
    if !is_visible(app)
        || app.apps_catalog.third_party_page != ThirdPartyPage::Agents
        || has_inline_providers(app)
    {
        return Task::none();
    }
    let Some(id) = app
        .apps_catalog
        .expanded
        .filter(|id| app.apps_catalog.rows.iter().any(|row| row.entry.id == *id))
    else {
        return Task::none();
    };
    use crate::app::message::app_providers::list::{self, ListMessage};
    list::update(app, ListMessage::Open(id))
}

/// Whether third-party resource operations currently belong to the visible page.
pub(crate) fn is_visible(app: &App) -> bool {
    app.navigation_tabs.screen == crate::app::Screen::ThirdPartyAgents
        && !app.navigation_panels.show_system_settings
}
