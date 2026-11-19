//! Final desktop composition for third-party resources and application details.
#[path = "third_party_skills_workspace.rs"]
pub(crate) mod skills_workspace;
use super::system_settings_common::{icon_btn, icon_svg, rounded_action_btn_style, settings_page_heading};
use crate::app::{
    App, Message,
    message::{SettingsMessage, ViewMessage, settings::AppsCatalogMessage},
    views::usage::center as usage,
};
use desktop_assets::assets::Icon;
use desktop_ui_controls::{settings_navigation::ThirdPartyPage, tooltip::tooltip_dark_style};
use iced::{
    Element, Length, Task, Theme,
    widget::{Space, button, column, container, row, rule, scrollable, text, tooltip},
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
        ThirdPartyPage::AppsCatalog => {
            return workspace(app, super::system_settings_apps::view(state, i18n));
        }
        ThirdPartyPage::Agents => {
            match state.rows.iter().find(|entry| Some(entry.entry.id) == selected) {
                Some(entry) => column![
                    desktop_settings_apps::system_settings_apps::details::actions(
                        &super::system_settings_apps::view_state(state),
                        entry,
                        super::system_settings_apps::local::available(),
                        super::system_settings_apps::domain_event
                    ),
                    super::system_settings_apps::details::view(state, entry),
                ]
                .spacing(16)
                .into(),
                None => iced::widget::container(text(if state.loading {
                    "正在读取智能体…"
                } else {
                    "从左侧选择智能体，管理配置与运行环境"
                }))
                .center_x(iced::Length::Fill)
                .center_y(iced::Length::Fill)
                .into(),
            }
        }
        ThirdPartyPage::Mcp => match mcp_scope(app) {
            Some(scope) if state.third_party_mcp_scope.as_ref() == Some(&scope) => {
                let installations = state.rows.iter().filter_map(|row| row.observation.as_ref().ok())
                    .flat_map(|observation| observation.installations.iter().cloned()).collect::<Vec<_>>();
                // The component captures the scope by value in messages, not this projection.
                column![super::system_settings_mcp::third_party_panel(app), mcp_view(app, scope, &installations)].spacing(12).into()
            }
            _ => text("请连接网关并重新打开 MCP 页面以读取当前目录").into(),
        },
        ThirdPartyPage::Skills => {
            let applications = state
                .rows
                .iter()
                .map(|r| desktop_settings_skills::bindings_view::BindingApplication {
                    entry: &r.entry,
                    observation: r.observation.as_ref().ok(),
                })
                .collect::<Vec<_>>();
            desktop_settings_skills::third_party::view(&state.third_party_skills, &applications)
                .map(|event| message(AppsCatalogMessage::ThirdPartySkills(event)))
        }
        ThirdPartyPage::Prompts => super::system_settings_prompts::list::page(app),
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
            super::system_settings_data::migration_preview_ui::view(app),
            crate::app::message::settings::backup_inventory::view(app),
            super::system_settings_data::export_ui::view(app),
        ]
        .spacing(12)
        .into(),
    };
    let title = if state.third_party_page == ThirdPartyPage::Agents {
        state
            .rows
            .iter()
            .find(|entry| Some(entry.entry.id) == selected)
            .map(|entry| entry.entry.name.as_str())
            .unwrap_or(i18n.__("第三方智能体"))
    } else {
        ThirdPartyPage::ALL
            .iter()
            .find(|(page, _)| *page == state.third_party_page)
            .map(|(_, label)| i18n.__(label))
            .unwrap_or("")
    };
    let mut workspace = column![settings_page_heading(title, "")].spacing(16);
    if let Some(error) = &state.error {
        workspace =
            workspace.push(super::system_settings_common::settings_error_banner(error.clone()));
    }
    if state.third_party_page == ThirdPartyPage::Prompts && state.prompts.editor.is_some() {
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
    for row in &state.rows {
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
            ThirdPartyPage::Agents | ThirdPartyPage::AppsCatalog => continue,
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
        sidebar_action(navigation_icon(Icon::Gear), i18n.__("设置"), None, false, Message::View(ViewMessage::OpenSystemSettingsTab(
            desktop_ui_controls::settings_navigation::SystemTab::General,
        ))),
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

fn mcp_scope(app: &App) -> Option<desktop_settings_mcp::third_party::Scope> {
    Some(desktop_settings_mcp::third_party::Scope {
        context: crate::app::message::settings::mcp_management_list::context(app)?,
        app_id: None,
        generation: app.apps_catalog.navigation_generation,
    })
}

// The scope must live with the host state for the duration of the rendered Element.
fn mcp_view<'a>(
    app: &'a App,
    scope: desktop_settings_mcp::third_party::Scope,
    installations: &[vw_api_app_identity::AppInstallation],
) -> Element<'a, Message> {
    // Build the scoped projection using the cached scope owned by navigation.
    let Some(current) =
        app.apps_catalog.third_party_mcp_scope.as_ref().filter(|current| **current == scope)
    else {
        return text("请重新打开 MCP 页面以刷新作用域").into();
    };
    desktop_settings_mcp::third_party::view(desktop_settings_mcp::third_party::View {
        scope: current,
        state: &app.mcp_settings,
        installations,
        authorized: !app.apps_catalog.loading,
    })
    .map(|event| message(AppsCatalogMessage::ThirdPartyMcp(event)))
}

pub(crate) fn navigate(app: &mut App, page: ThirdPartyPage) -> Task<Message> {
    if super::system_settings_prompts::list::switch_blocked(&app.apps_catalog.prompts) {
        return Task::none();
    }
    if app.usage_center.open {
        app.usage_center.close();
    }
    invalidate(app);
    app.apps_catalog.third_party_page = page;
    use crate::app::message::settings;
    match page {
        ThirdPartyPage::AppsCatalog => settings::apps_catalog::update(app, AppsCatalogMessage::EnsureLoaded),
        ThirdPartyPage::Prompts => super::system_settings_prompts::list::open_selected(app),
        ThirdPartyPage::Mcp => {
            app.apps_catalog.third_party_mcp_scope = mcp_scope(app);
            settings::mcp_management_list::update(
                app,
                settings::mcp_management_list::CatalogMessage::Refresh,
            )
        }
        ThirdPartyPage::Skills => {
            let input = settings::skills::update_input(app);
            let scope = input.context.map(|context| desktop_settings_skills::third_party::Scope {
                context, project: input.project, host: input.host, directory: input.directory,
            });
            app.apps_catalog.third_party_skills.sync(scope, app.apps_catalog.navigation_generation);
            let Some(scope) = app.apps_catalog.third_party_skills.scope().cloned() else { return Task::none(); };
            let catalog = app.apps_catalog.navigation_generation;
            skills_update(app, desktop_settings_skills::third_party::Message {
                scope, catalog, action: desktop_settings_skills::third_party::Action::Workspace(
                    desktop_settings_skills::third_party::workspace::Event::Tab(desktop_settings_skills::third_party::workspace::Tab::Installed)),
            })
        }
        ThirdPartyPage::Sessions => {
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
    s.third_party_mcp_scope = None;
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
        desktop_settings_mcp::management_bindings::BindingsMessage::Finished { .. }));
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
    match desktop_settings_skills::third_party::update(
        &mut app.apps_catalog.third_party_skills,
        &input,
        event,
    ) {
        Effect::None => Task::none(),
        Effect::Workspace { scope, catalog, effect } => skills_workspace::execute(app, scope, catalog, effect),
        Effect::Binding { scope, catalog, effect } => match effect {
            bindings_view::Effect::None => Task::none(),
            bindings_view::Effect::Error(error) => app.show_error_toast(error),
            bindings_view::Effect::Success(notice) => app.show_success_toast(notice),
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
            skills::update(app, event)
        }
    }
}

pub(crate) fn overlays<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let base = match app.apps_catalog.third_party_page {
        ThirdPartyPage::Skills => super::system_settings_skills::third_party_overlays(app, base),
        ThirdPartyPage::Mcp if app.apps_catalog.third_party_mcp_scope == mcp_scope(app)
            && app.apps_catalog.third_party_mcp_scope.is_some() =>
                super::system_settings_mcp::third_party_overlays(app, base),
        _ => base,
    };
    if app.apps_catalog.third_party_page == ThirdPartyPage::ManagementTools {
        super::system_settings_data::export_ui::overlay(
            app,
            super::third_party_tools::overlay(app, base),
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
    let provider_help = super::app_providers::list::Help::new(
        crate::app::i18n::I18n::new(app.general_settings.desktop_locale),
        &app.appearance.app_theme,
    );
    #[cfg(not(target_arch = "wasm32"))]
    let provider_help = provider_help.with_usage(app);
    base = super::app_providers::list::overlay(&app.app_providers, provider_help, base);
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

/// Whether third-party resource operations currently belong to the visible page.
pub(crate) fn is_visible(app: &App) -> bool {
    app.navigation_tabs.screen == crate::app::Screen::ThirdPartyAgents
        && !app.navigation_panels.show_system_settings
}
