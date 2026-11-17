//! Final desktop composition for third-party resources and application details.
use super::system_settings_common::{rounded_action_btn_style, settings_page_heading};
use crate::app::{
    App, Message,
    message::{SettingsMessage, settings::AppsCatalogMessage},
};
use desktop_ui_controls::settings_navigation::ThirdPartyPage;
use iced::{
    Element, Task,
    widget::{button, column, row, text},
};

fn message(event: AppsCatalogMessage) -> Message {
    Message::Settings(SettingsMessage::AppsCatalog(event))
}

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let state = &app.apps_catalog;
    let i18n = crate::app::i18n::I18n::new(app.general_settings.desktop_locale);
    let mut agents = column![].spacing(6);
    for entry in &state.rows {
        let active = state.third_party_page == ThirdPartyPage::Agents
            && state.expanded == Some(entry.entry.id);
        agents = agents.push(
            button(
                row![
                    desktop_settings_apps::system_settings_apps::list::brand(entry.entry.id),
                    text(&entry.entry.name).size(14)
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            )
            .width(iced::Length::Fill)
            .padding([10, 12])
            .style(move |theme, status| {
                if active {
                    iced::widget::button::primary(theme, status)
                } else {
                    rounded_action_btn_style(theme, status)
                }
            })
            .on_press(message(AppsCatalogMessage::SelectApp(entry.entry.id))),
        );
    }
    let mut resources = column![].spacing(6);
    for &(page, label) in ThirdPartyPage::ALL {
        if page == ThirdPartyPage::Agents {
            continue;
        }
        let active = state.third_party_page == page;
        resources = resources.push(
            button(text(i18n.__(label)).size(14))
                .width(iced::Length::Fill)
                .padding([10, 12])
                .style(move |theme, status| {
                    if active {
                        iced::widget::button::primary(theme, status)
                    } else {
                        rounded_action_btn_style(theme, status)
                    }
                })
                .on_press(message(AppsCatalogMessage::Navigate(page))),
        );
    }
    resources = resources.push(crate::app::views::usage::center::entry(app));
    let selected = state.expanded;
    let body: Element<'_, Message> = match state.third_party_page {
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
            Some(scope) => {
                let installations = state
                    .rows
                    .iter()
                    .find(|r| Some(r.entry.id) == selected)
                    .and_then(|r| r.observation.as_ref().ok())
                    .map(|r| r.installations.as_slice())
                    .unwrap_or(&[]);
                // The component captures the scope by value in messages, not this projection.
                mcp_view(app, scope, installations)
            }
            None => text("请先在智能体管理中选择当前主机上的应用").into(),
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
        ThirdPartyPage::Sessions => super::third_party_resources::sessions(app),
        #[cfg(not(target_arch = "wasm32"))]
        ThirdPartyPage::Authorization => super::third_party_resources::authorization(app),
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
    use iced::{
        Length,
        widget::{container, scrollable},
    };

    let sidebar = column![
        settings_page_heading(i18n.__("第三方智能体"), ""),
        scrollable(container(agents).padding([0, 7]))
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4)
            ))
            .height(Length::Fill),
        iced::widget::rule::horizontal(1),
        resources,
        button(text("刷新"))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press(message(AppsCatalogMessage::Refresh)),
    ]
    .spacing(12)
    .padding(16)
    .width(240)
    .height(Length::Fill);
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
    workspace = workspace.push(
        scrollable(container(body).padding([0, 7]))
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4),
            ))
            .height(Length::Fill),
    );
    row![
        sidebar,
        iced::widget::rule::vertical(1),
        container(workspace).padding(24).width(Length::Fill).height(Length::Fill)
    ]
    .height(Length::Fill)
    .into()
}

fn mcp_scope(app: &App) -> Option<desktop_settings_mcp::third_party::Scope> {
    let app_id = app.apps_catalog.expanded?;
    if app.apps_catalog.loading || !app.apps_catalog.rows.iter().any(|row| row.entry.id == app_id) {
        return None;
    }
    Some(desktop_settings_mcp::third_party::Scope {
        context: crate::app::message::settings::mcp_management_list::context(app)?,
        app_id,
        generation: app.apps_catalog.navigation_generation,
    })
}

// The scope must live with the host state for the duration of the rendered Element.
fn mcp_view<'a>(
    app: &'a App,
    scope: desktop_settings_mcp::third_party::Scope,
    installations: &'a [vw_api_app_identity::AppInstallation],
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
        authorized: true,
    })
    .map(|event| message(AppsCatalogMessage::ThirdPartyMcp(event)))
}

pub(crate) fn navigate(app: &mut App, page: ThirdPartyPage) -> Task<Message> {
    invalidate(app);
    app.apps_catalog.third_party_page = page;
    use crate::app::message::settings;
    match page {
        ThirdPartyPage::Mcp => {
            app.apps_catalog.third_party_mcp_scope = mcp_scope(app);
            settings::mcp_management_list::update(
                app,
                settings::mcp_management_list::CatalogMessage::Refresh,
            )
        }
        ThirdPartyPage::Skills => {
            let scope = app.apps_catalog.expanded.zip(app.apps_catalog.context.clone()).map(
                |(id, context)| desktop_provider_apps::Scope {
                    context,
                    host: app.apps_catalog.host.clone(),
                    app: id,
                },
            );
            app.apps_catalog.third_party_skills.sync(scope, app.apps_catalog.navigation_generation);
            Task::none()
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
    let Some(current) = mcp_scope(app) else {
        return Task::none();
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
        ThirdPartyPage::Skills => super::system_settings_skills::view_overlays(app, base),
        ThirdPartyPage::Mcp => super::system_settings_mcp::view_overlays(app, base),
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
