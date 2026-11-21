//! Final desktop composition for third-party resources and application details.
#[path = "third_party_skills_workspace.rs"]
pub(crate) mod skills_workspace;
#[path = "third_party_mcp_workspace.rs"]
pub(crate) mod mcp_workspace;
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
    settings_navigation::{ThirdPartyPage, ThirdPartySettingsTab},
    system_settings_common::settings_content_panel_style,
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
    let selected = state.expanded;
    let body: Element<'_, Message> = match state.third_party_page {
        ThirdPartyPage::Settings(tab) => return workspace(app, settings_page::view(app, tab)),
        ThirdPartyPage::AppsCatalog => {
            return workspace(app, super::system_settings_apps::view(state, i18n));
        }
        ThirdPartyPage::Agents => return workspace(app, agent_workspace(app)),
        ThirdPartyPage::Mcp => {
            let mut content = column![].spacing(12).width(Length::Fill).height(Length::Fill);
            if let Some(error) = &state.error {
                content = content.push(
                    container(super::system_settings_common::settings_error_banner(error.clone()))
                        .padding([0, 7])
                        .width(Length::Fill),
                );
            }
            return workspace(
                app,
                container(content.push(mcp_workspace::view(app)))
                    .padding([12, 16])
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into(),
            );
        }
        ThirdPartyPage::Skills => {
            let applications = state
                .rows
                .iter()
                .map(|r| &r.entry)
                .collect::<Vec<_>>();
            let skills = desktop_settings_skills::third_party::view(
                &state.third_party_skills,
                &applications,
                desktop_settings_apps::system_settings_apps::list::brand_icon,
            )
            .map(|event| message(AppsCatalogMessage::ThirdPartySkills(event)));
            return workspace(app, container(skills).padding([14, 16]).into());
        }
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
        #[cfg(not(target_arch = "wasm32"))]
        ThirdPartyPage::Usage => match selected {
            Some(id) => super::third_party_resources::usage(app, id),
            None => text("请先选择应用，再查看用量与额度").into(),
        },
        #[cfg(target_arch = "wasm32")]
        ThirdPartyPage::Authorization | ThirdPartyPage::Usage => {
            text("此平台不支持授权与用量管理").into()
        }
        ThirdPartyPage::ManagementTools => column![
            super::third_party_tools::view(app, selected),
            super::system_settings_data::migration_preview_ui::entry(),
            crate::app::message::settings::backup_inventory::view(app),
            super::system_settings_data::export_ui::view(app),
        ]
        .spacing(12)
        .into(),
    };
    let title = ThirdPartyPage::ALL
        .iter()
        .find(|(page, _)| *page == state.third_party_page)
        .map(|(_, label)| i18n.__(label))
        .unwrap_or("");
    let mut workspace = column![settings_page_heading(title, "")].spacing(16);
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
    use super::system_settings_apps;
    use desktop_settings_apps::system_settings_apps::{details, list, preferences};

    let state = &app.apps_catalog;
    let projection = system_settings_apps::view_state(state);
    let selected = state.rows.iter().find(|row| Some(row.entry.id) == state.expanded);
    let (header, body): (Element<'_, Message>, Element<'_, Message>) = if let Some(agent) = selected {
        use super::app_routing::lifecycle as routing;
        use super::system_settings_common::{settings_panel, settings_value_badge};
        use crate::app::message::settings::aggregation_list;
        use desktop_apps_ui::catalog_state::AgentWorkspaceTab as Tab;
        use desktop_settings_apps::system_settings_apps::Action;

        let id = agent.entry.id;
        let tab = state.agent_workspace_tab;
        let configuration = tab == Tab::Configuration;
        let aggregation = app.app_providers.aggregation.as_ref();
        use vw_api_app_aggregation::AggregationMode;
        let aggregation_mode =
            aggregation.and_then(|form| form.snapshot.as_ref()).map(|snapshot| snapshot.state.mode);
        let routing_attached =
            routing::attached_apps(state).is_some_and(|(apps, _)| apps.contains(&id));
        let intercepted = routing_attached
            || aggregation_mode.is_some_and(|mode| mode != AggregationMode::Direct);
        let title = || {
            let mut title = row![list::brand_icon(id, 22.0), text(&agent.entry.name).size(21)]
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
                    let mode = match aggregation_mode {
                        Some(AggregationMode::Aggregation) => "聚合",
                        Some(AggregationMode::Routing) => "路由",
                        _ if routing_attached => "路由",
                        Some(AggregationMode::Direct) => "直连",
                        None => "当前",
                    };
                    title = title.push(settings_value_badge(format!("{mode} · {label}")));
                }
            }
            title.wrap()
        };
        let select = |target| message(AppsCatalogMessage::AgentWorkspaceTab(target));
        let tabs = || {
            use desktop_ui_controls::system_settings_common::help::settings_hover_help_label;
            let modes: [(Tab, &str, bool); 3] = [
                (Tab::Direct, "直连", true),
                (Tab::Routing, "路由", routing::ROUTING_APPS.contains(&id)),
                (Tab::Aggregation, "聚合", aggregation_list::supported(id)),
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
                    text("连接模式").size(14),
                    text("直连 · 智能体直接连接供应商。").size(13),
                    text("路由 · 请求由路由服务转发，可配置备用队列。").size(13),
                    text("聚合 · 将已添加供应商的模型发布到客户端目录。").size(13),
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
                        routing::view(state),
                        settings_panel(
                            row![
                        super::app_routing::config::timeout_button(id),
                        super::app_routing::outbound::entry(id),
                        super::app_routing::requests::entry(id),
                        button(text("备用队列").size(13)).padding([8, 12])
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
                Tab::Aggregation => Some(match aggregation {
                    Some(form) => desktop_apps_ui::aggregation::list_view::page(
                        form,
                        &app.app_providers.query,
                    )
                    .map(aggregation_list::message),
                    None if app.app_providers.loading => text("正在读取供应商…").into(),
                    None => settings_panel(
                        button(text("读取聚合目录").size(13))
                            .padding([8, 12])
                            .style(rounded_action_btn_style)
                            .on_press(aggregation_list::message(aggregation_list::Event::Open)),
                    )
                    .into(),
                }),
                Tab::Direct if intercepted => {
                    let can_preview_exit =
                        aggregation_mode.is_some_and(|mode| mode != AggregationMode::Direct);
                    let next = if can_preview_exit {
                        aggregation_list::message(aggregation_list::Event::Publish(
                            desktop_apps_ui::aggregation::publish::Event::OpenExit,
                        ))
                    } else {
                        select(Tab::Routing)
                    };
                    Some(
                        settings_panel(
                            row![
                                text("当前连接仍由路由或聚合服务管理").size(13).width(Length::Fill),
                                button(
                                    text(if can_preview_exit {
                                        "预览恢复直连"
                                    } else {
                                        "查看路由"
                                    })
                                    .size(13)
                                )
                                .padding([8, 12])
                                .style(rounded_action_btn_style)
                                .on_press(next),
                            ]
                            .spacing(8)
                            .align_y(Alignment::Center),
                        )
                        .into(),
                    )
                }
                _ => None,
            };
            let mut tools = vec![
                ("查看会话", message(AppsCatalogMessage::Navigate(ThirdPartyPage::Sessions))),
                (
                    "配置目录",
                    message(AppsCatalogMessage::Navigate(ThirdPartyPage::Settings(
                        ThirdPartySettingsTab::Applications,
                    ))),
                ),
                ("安装与升级", select(Tab::Configuration)),
                ("侧栏应用设置", system_settings_apps::domain_event(Action::PreferencesOpen)),
            ];
            #[cfg(not(target_arch = "wasm32"))]
            tools.insert(
                1,
                ("查看用量", message(AppsCatalogMessage::Navigate(ThirdPartyPage::Usage))),
            );
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
                    replace_rows: tab == Tab::Aggregation,
                    can_launch,
                    can_switch: tab == Tab::Direct && !intercepted,
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
                text("供应商列表尚未加载").size(15),
                button(text("加载供应商").size(13))
                    .padding([8, 12])
                    .style(rounded_action_btn_style)
                    .on_press(providers::message(ListMessage::Open(agent.entry.id))),
            ]
            .spacing(10);
            (column![title(), tabs()].spacing(12).into(), empty.into())
        } else {
            let actions = details::workspace_actions(
                &projection,
                agent,
                system_settings_apps::local::available(),
                system_settings_apps::domain_event,
            );
            (
                column![
                    title(),
                    row![
                        button(text("返回供应商").size(13))
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
                system_settings_apps::details::view(state, agent),
            )
        }
    } else {
        let mut choices = column![].width(Length::Fill).spacing(8);
        let visible = preferences::visible_rows(&projection);
        if visible.is_empty() {
            choices = choices.push(
                text(if state.loading {
                    "正在读取智能体…"
                } else {
                    "没有可选的智能体"
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
            text("选择第三方智能体").size(22).into(),
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
            ThirdPartyPage::Agents | ThirdPartyPage::AppsCatalog | ThirdPartyPage::Settings(_) => continue,
            ThirdPartyPage::Mcp => Icon::HddNetwork,
            ThirdPartyPage::Skills => Icon::Box,
            ThirdPartyPage::Prompts => Icon::Box,
            ThirdPartyPage::Sessions => Icon::Clock,
            ThirdPartyPage::Authorization => Icon::ShieldLock,
            ThirdPartyPage::Usage => Icon::Speedometer2,
            ThirdPartyPage::ManagementTools => Icon::Wrench,
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
        icon_btn(Icon::ArrowClockwise, "刷新目录", Some(message(AppsCatalogMessage::Refresh))),
    ]
    .padding([2, 8])
    .spacing(6)
    .align_y(iced::Alignment::Center);

    let footer = row![
        sidebar_action(navigation_icon(Icon::Grid1x2), i18n.__("应用"), None, !showing_usage && state.third_party_page == ThirdPartyPage::AppsCatalog, message(AppsCatalogMessage::Navigate(ThirdPartyPage::AppsCatalog))),
        sidebar_action(
            navigation_icon(Icon::Gear),
            i18n.__("设置"),
            None,
            !showing_usage && matches!(state.third_party_page, ThirdPartyPage::Settings(_)),
            message(AppsCatalogMessage::Navigate(ThirdPartyPage::Settings(
                ThirdPartySettingsTab::Applications,
            ))),
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

pub(crate) fn mcp_scope(app: &App) -> Option<desktop_settings_mcp::third_party::Scope> {
    Some(desktop_settings_mcp::third_party::Scope {
        context: crate::app::message::settings::mcp_management_list::context(app)?,
        app_id: None,
        generation: app.apps_catalog.navigation_generation,
    })
}

pub(crate) fn navigate(app: &mut App, page: ThirdPartyPage) -> Task<Message> {
    if super::system_settings_prompts::list::switch_blocked(&app.apps_catalog.prompts) {
        return Task::none();
    }
    if app.usage_center.open {
        app.usage_center.close();
    }
    let provider_scope = if page == ThirdPartyPage::Usage && has_inline_providers(app) {
        app.app_providers.scope.take()
    } else {
        None
    };
    invalidate(app);
    if provider_scope.is_some() {
        app.app_providers.scope = provider_scope;
        app.app_providers.aggregation = None;
    }
    app.apps_catalog.third_party_page = page;
    use crate::app::message::settings;
    match page {
        ThirdPartyPage::Agents => ensure_agent_providers(app),
        ThirdPartyPage::AppsCatalog => settings::apps_catalog::update(app, AppsCatalogMessage::EnsureLoaded),
        ThirdPartyPage::Prompts => settings::apps_catalog::update(app, AppsCatalogMessage::EnsureLoaded),
        ThirdPartyPage::Settings(tab) => {
            let load = settings::apps_catalog::update(app, AppsCatalogMessage::EnsureLoaded);
            if tab != ThirdPartySettingsTab::Routing {
                return load;
            }
            use super::app_routing::lifecycle::{self, Event, Operation};
            Task::batch([load, lifecycle::update(app, Event::Run(Operation::Refresh))])
        }
        ThirdPartyPage::Mcp => {
            Task::batch([
                settings::apps_catalog::update(app, AppsCatalogMessage::EnsureLoaded),
                mcp_workspace::refresh(app),
            ])
        }
        ThirdPartyPage::Skills => {
            use desktop_settings_skills::third_party::{Action, Message as SkillMessage, workspace};
            use desktop_settings_skills::update::bindings_view;
            let catalog_load = settings::apps_catalog::update(app, AppsCatalogMessage::EnsureLoaded);
            let input = settings::skills::update_input(app);
            let scope = input.context.map(|context| desktop_settings_skills::third_party::Scope {
                context, project: input.project, host: input.host, directory: input.directory,
            });
            app.apps_catalog.third_party_skills.sync(scope, app.apps_catalog.navigation_generation);
            let Some(scope) = app.apps_catalog.third_party_skills.scope().cloned() else {
                return catalog_load;
            };
            let catalog = app.apps_catalog.navigation_generation;
            let scoped = |action| SkillMessage { scope: scope.clone(), catalog, action };
            let inventory = skills_update(app, scoped(Action::Workspace(
                workspace::Event::Tab(workspace::Tab::Installed),
            )));
            let targets = skills_update(app, scoped(Action::Binding(bindings_view::Event::Open)));
            Task::batch([catalog_load, inventory, targets])
        }
        ThirdPartyPage::Sessions => {
            app.apps_catalog.external_sessions.open = true;
            super::external_sessions::update(app, super::external_sessions::Event::Refresh)
        }
        #[cfg(not(target_arch = "wasm32"))]
        ThirdPartyPage::Authorization => crate::app::message::agent_apps_auth::update(
            app,
            crate::app::message::agent_apps_auth::Event::Refresh,
        ),
        _ => Task::none(),
    }
}

/// Invalidate transient UI and request generations without changing domain targets.
pub(crate) fn invalidate(app: &mut App) {
    app.apps_catalog.commands.invalidate();
    desktop_settings_mcp::management_probe::actions::invalidate(&mut app.mcp_settings.actions, None, &[]);
    use crate::app::message::settings::skills;
    for event in [
        SettingsMessage::SkillRepositories(skills::repo_modal::Event::Close),
        SettingsMessage::SkillsClaudeConfig(skills::claude_view::Event::Close),
        SettingsMessage::SkillsBindings(skills::bindings_view::Event::Close),
        SettingsMessage::SkillsImport(skills::import_modal::Event::Close),
        SettingsMessage::SkillsInstall(skills::install_view::Event::Close),
        SettingsMessage::SkillsMarket(skills::market_view::Event::Close),
        SettingsMessage::SkillsUpdates(skills::updates_view::Event::Close),
        SettingsMessage::SkillsJobs(skills::jobs_view::Event::Close),
        SettingsMessage::SkillsStorage(skills::storage_modal::Event::Close),
        SettingsMessage::SkillsBackups(skills::backup_modal::Event::Close),
    ] {
        // Close reducers only dismiss presentation; writes keep their original scope.
        let _ = skills::update(app, event);
    }
    // Busy reducers retain their captured request scope, but no presentation
    // may survive a navigation or host change.
    let skills = &mut app.skills_settings;
    skills.repositories.open = false;
    skills.claude_config.open = false;
    skills.bindings.open = false;
    skills.import.open = false;
    skills.install.open = false;
    skills.market.open = false;
    skills.updates.open = false;
    skills.jobs.open = false;
    skills.storage.open = false;
    skills.backups.open = false;
    let s = &mut app.apps_catalog;
    s.navigation_generation = s.navigation_generation.wrapping_add(1);
    s.agent_workspace_tab = Default::default();
    s.agent_provider_search_open = false;
    s.third_party_mcp_scope = None;
    app.mcp_settings.bindings.matrix.column_menu = None;
    s.third_party_skills.sync(None, s.generation);
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

pub(crate) fn mcp_update(
    app: &mut App,
    event: desktop_settings_mcp::third_party::Message,
) -> Task<Message> {
    let completion = matches!(&event.action, desktop_settings_mcp::third_party::Action::Bindings(
        desktop_settings_mcp::management_bindings::BindingsMessage::Finished { .. }
        | desktop_settings_mcp::management_bindings::BindingsMessage::MatrixFinished { .. }));
    let current = if completion {
        event.scope.clone()
    } else {
        if !is_visible(app) || app.apps_catalog.third_party_page != ThirdPartyPage::Mcp {
            return Task::none();
        }
        let Some(current) = mcp_scope(app) else { return Task::none() };
        current
    };
    let mut input = crate::app::message::settings::mcp_management_list::input(app);
    let task = desktop_settings_mcp::third_party::update(&mut input, &current, event)
        .map(|event| message(AppsCatalogMessage::ThirdPartyMcp(event)));
    let effects = std::mem::take(&mut input.effects);
    crate::app::message::settings::mcp_management_tools::apply_effects(app, task, effects)
}

pub(crate) fn skills_update(
    app: &mut App,
    event: desktop_settings_skills::third_party::Message,
) -> Task<Message> {
    if !is_visible(app) || app.apps_catalog.third_party_page != ThirdPartyPage::Skills {
        return Task::none();
    }
    use desktop_settings_skills::{
        third_party::{Action, DirectoryAction, Effect},
        update::bindings_view,
    };
    let mut input = crate::app::message::settings::skills::update_input(app);
    input.catalog = app.apps_catalog.navigation_generation;
    use desktop_settings_skills::third_party::workspace::Event as WorkspaceEvent;
    if matches!(&event.action,
        Action::Workspace(WorkspaceEvent::Install(_) | WorkspaceEvent::ConfirmRepository)
        | Action::Directory(DirectoryAction::Import)
    ) {
        let page = &app.apps_catalog.third_party_skills;
        if page.awaiting_target_snapshot() || app.apps_catalog.loading {
            return app.show_error_toast(if page.bindings.busy || app.apps_catalog.loading {
                "正在读取当前项目的安装目标，请稍候"
            } else {
                "安装目标读取失败，请在「安装到」中重试或选择「仅主副本」"
            });
        }
    }
    let inventory_readback = matches!(&event.action, Action::Workspace(WorkspaceEvent::Loaded(_, Ok(_))));
    let snapshot_version = app.apps_catalog.third_party_skills.bindings.snapshot.as_ref()
        .map(|snapshot| snapshot.version);
    match desktop_settings_skills::third_party::update(
        &mut app.apps_catalog.third_party_skills,
        &input,
        event,
    ) {
        Effect::None => Task::none(),
        Effect::Workspace { scope, catalog, effect } => {
            let page = &app.apps_catalog.third_party_skills;
            let reload_bindings = inventory_readback && !page.bindings.busy
                && page.workspace.inventory.as_ref().zip(page.bindings.snapshot.as_ref())
                    .is_some_and(|(inventory, snapshot)| inventory.version != snapshot.version);
            let refresh = reload_bindings.then(|| desktop_settings_skills::third_party::Message {
                scope: scope.clone(), catalog, action: Action::Binding(bindings_view::Event::Refresh),
            });
            let workspace = skills_workspace::execute(app, scope, catalog, effect);
            if let Some(event) = refresh {
                return Task::batch([workspace, skills_update(app, event)]);
            }
            workspace
        }
        Effect::Binding { scope, catalog, effect } => match effect {
            bindings_view::Effect::None => Task::none(),
            bindings_view::Effect::Error(error) => {
                let toast = app.show_error_toast(error);
                let versions_changed = app.apps_catalog.third_party_skills.bindings.snapshot.as_ref()
                    .zip(app.apps_catalog.third_party_skills.workspace.inventory.as_ref())
                    .is_some_and(|(snapshot, inventory)| {
                        Some(snapshot.version) != snapshot_version && snapshot.version != inventory.version
                    });
                if versions_changed {
                    return Task::batch([toast, crate::app::message::settings::skills::refresh_workspace(app)]);
                }
                toast
            }
            bindings_view::Effect::Success(notice) => Task::batch([
                app.show_success_toast(notice),
                crate::app::message::settings::skills::refresh_workspace(app),
            ]),
            bindings_view::Effect::Request { generation, scope: request_scope, change } => {
                let token = app.apps_catalog.token.clone();
                Task::perform(
                    async move {
                        desktop_session::session_gateway::gateway_skill_bindings_request(
                            crate::app::config::gateway_client_endpoint(),
                            request_scope,
                            token,
                            change,
                        )
                        .await
                    },
                    move |result| {
                        message(AppsCatalogMessage::ThirdPartySkills(
                            desktop_settings_skills::third_party::Message {
                                scope: scope.clone(),
                                catalog,
                                action: Action::Binding(bindings_view::Event::Completed(
                                    generation, result,
                                )),
                            },
                        ))
                    },
                )
            }
        },
        Effect::Directory { action, .. } => {
            use crate::app::message::settings::skills;
            let event = match action {
                DirectoryAction::Claude => {
                    SettingsMessage::SkillsClaudeConfig(skills::claude_view::Event::Open)
                }
                DirectoryAction::Repositories => {
                    SettingsMessage::SkillRepositories(skills::repo_modal::Event::Open)
                }
                DirectoryAction::Import => {
                    SettingsMessage::SkillsImport(skills::import_modal::Event::Open)
                }
                DirectoryAction::Install => {
                    SettingsMessage::SkillsInstall(skills::install_view::Event::Open)
                }
                DirectoryAction::Market => {
                    SettingsMessage::SkillsMarket(skills::market_view::Event::Open)
                }
                DirectoryAction::Updates => {
                    SettingsMessage::SkillsUpdates(skills::updates_view::Event::Open)
                }
                DirectoryAction::Jobs => {
                    SettingsMessage::SkillsJobs(skills::jobs_view::Event::Open)
                }
                DirectoryAction::Storage => {
                    SettingsMessage::SkillsStorage(skills::storage_modal::Event::Open)
                }
                DirectoryAction::Backup => {
                    SettingsMessage::SkillsBackups(skills::backup_modal::Event::Open)
                }
            };
            let opening_import = matches!(action, DirectoryAction::Import) && !app.skills_settings.import.open;
            let task = skills::update(app, event);
            if opening_import && app.skills_settings.import.open {
                let page = &app.apps_catalog.third_party_skills;
                let import = &mut app.skills_settings.import;
                import.available_targets = page.available_targets();
                import.targets = page.bindings.targets.iter()
                    .filter(|target| import.available_targets.contains(target))
                    .cloned()
                    .collect();
                let scan_sources = import.targets.iter().map(|target| target.app_id)
                    .filter(|id| desktop_settings_skills::import_modal::APPS.contains(id));
                if let Some(source) = scan_sources.clone()
                    .find(|id| *id == vw_api_types::agent_apps::AppId::Codex)
                    .or_else(|| scan_sources.into_iter().next())
                {
                    import.apps.push(source);
                    let scan = skills::update(app, SettingsMessage::SkillsImport(
                        skills::import_modal::Event::Scan,
                    ));
                    return Task::batch([task, scan]);
                }
            }
            if matches!(action, DirectoryAction::Backup) {
                // The list readback validates these choices against the same scope.
                app.skills_settings.backups.targets = app.apps_catalog.third_party_skills.bindings.targets.clone();
            }
            task
        }
    }
}

pub(crate) fn overlays<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let base = match app.apps_catalog.third_party_page {
        ThirdPartyPage::Skills => super::system_settings_skills::third_party_overlays(app, base),
        ThirdPartyPage::Mcp if app.apps_catalog.third_party_mcp_scope == mcp_scope(app)
            && app.apps_catalog.third_party_mcp_scope.is_some() =>
                mcp_workspace::overlays(app, base),
        _ => base,
    };
    if matches!(
        app.apps_catalog.third_party_page,
        ThirdPartyPage::ManagementTools | ThirdPartyPage::Settings(ThirdPartySettingsTab::Data)
    ) {
        super::system_settings_data::migration_preview_ui::overlay(
            app,
            super::system_settings_data::export_ui::overlay(
                app,
                super::third_party_tools::overlay(app, base),
            ),
        )
    } else {
        base
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
    if usage::is_visible(app) {
        return view(app);
    }
    let base = overlays(app, view(app));
    let mut base = super::system_settings_apps::view_overlays(
        &app.apps_catalog,
        crate::app::i18n::I18n::new(app.general_settings.desktop_locale),
        &app.appearance.app_theme,
        base,
    );
    base = super::app_providers::list::overlay_with_list(
        &app.app_providers,
        provider_help(app),
        base,
        !has_inline_providers(app) && app.apps_catalog.third_party_page != ThirdPartyPage::Usage,
    );
    if crate::app::message::agent_apps_high_availability::can_show(app) {
        base =
            super::agent_apps_high_availability::queue::overlay(&app.app_providers.failover, base);
    }
    if crate::app::message::agent_apps_diagnostics::model::can_show(app) {
        base = super::agent_apps_diagnostics::model_panel::overlay(
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
        base = super::system_settings_auth::overlay(&app.agent_apps_auth, base);
    }
    base
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
