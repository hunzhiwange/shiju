//! Catalog requests use only SDK reads and reject stale gateway/host responses.
use super::SettingsMessage;
use crate::app::components::system_settings_apps::catalog::CatalogRow;
use crate::app::{App, Message, state::AgentAppsContext};
use iced::Task;

#[derive(Clone)]
pub enum AppsCatalogMessage {
    Commands(crate::app::components::system_settings_apps::commands::Event),
    SelectApp(vw_api_app_identity::AppId),
    AgentWorkspaceTab(desktop_apps_ui::catalog_state::AgentWorkspaceTab),
    ToggleAgentProviderSearch,
    SelectPromptApp(vw_api_app_identity::AppId),
    Navigate(desktop_ui_controls::settings_navigation::ThirdPartyPage),
    McpWorkspace(crate::app::components::third_party_settings::mcp_workspace::Event),
    ThirdPartyMcp(desktop_settings_mcp::third_party::Message),
    ThirdPartySkills(desktop_settings_skills::third_party::Message),
    ExternalSessions(crate::app::components::external_sessions::Event),
    ProjectProfiles(crate::app::message::project_profiles::Event),
    Routing(crate::app::components::app_routing::lifecycle::Event),
    Prompts(super::prompts::Event),
    WorkspaceFiles(crate::app::components::workspace_files::Event),
    DailyMemory(crate::app::components::daily_memory::Event),
    HermesMemory(crate::app::components::hermes_memory::Event),
    OpenClawCatalog(crate::app::components::openclaw_catalog::Event),
    OpenClawTools(crate::app::components::openclaw_tools::Event),
    OpenClawEnv(crate::app::components::openclaw_env::Event),
    OpenClawModel(crate::app::components::openclaw_model::Event),
    OpenClawDefaults(crate::app::components::openclaw_defaults::Event),
    HermesDashboard(crate::app::components::hermes_dashboard::Event),
    Launch(crate::app::components::system_settings_apps::launch::LaunchMessage),
    Preferences(crate::app::components::system_settings_apps::preferences::PreferencesMessage),
    HighAvailability(crate::app::message::agent_apps_high_availability_policy::Event),
    Lifecycle(super::apps_lifecycle::LifecycleMessage),
    QueryInput(String),
    ToggleDetails(vw_api_app_identity::AppId),
    Drag(crate::app::components::system_settings_apps::drag::DragMessage),
    LocalConnect,
    LocalReady {
        context: AgentAppsContext,
        generation: u64,
        result: Result<local_app_auth::Credentials, String>,
    },
    ConnectionOpen,
    ConnectionClose,
    HostInput(String),
    TokenInput(String),
    RefreshObservation(vw_api_app_identity::AppId),
    ObservationLoaded {
        context: AgentAppsContext,
        host: String,
        generation: u64,
        id: vw_api_app_identity::AppId,
        result: Result<vw_api_app_identity::AppReadResponse, gateway_http::AgentAppsReadError>,
    },
    BatchUpgrade,
    CheckUpdates,
    UpdatesLoaded { context: AgentAppsContext, host: String, generation: u64, results: Vec<desktop_apps_ui::catalog_state::UpdateCandidate> },
    Refresh,
    EnsureLoaded,
    Loaded {
        context: AgentAppsContext,
        host: String,
        generation: u64,
        result: Result<Vec<CatalogRow>, String>,
    },
}
impl std::fmt::Debug for AppsCatalogMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AppsCatalogMessage")
    }
}
pub(in crate::app) fn update(app: &mut App, message: AppsCatalogMessage) -> Task<Message> {
    match message {
        AppsCatalogMessage::BatchUpgrade => {
            if app.apps_catalog.loading || app.apps_catalog.updates.checking { return Task::none(); }
            let items = app.apps_catalog.updates.results.iter()
                .filter(|candidate| candidate.supported && app.apps_catalog.rows.iter().any(|row|
                    desktop_settings_apps::system_settings_apps::presentation::preferred(row) == Some(&candidate.installation)))
                .map(|candidate| candidate.installation.clone()).collect();
            return super::apps_lifecycle::update(app, super::apps_lifecycle::LifecycleMessage::Batch(items));
        }
        AppsCatalogMessage::CheckUpdates => {
            let Ok(client) = crate::app::config::gateway_client() else { return app.show_localized_toast(crate::app::state::ToastKind::Error, "网关不可用", vec![]); };
            let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.navigation_tabs.project_path.clone() };
            let state = &mut app.apps_catalog;
            if state.loading || state.updates.checking || state.context.as_ref() != Some(&context) { return Task::none(); }
            state.updates.checking = true;
            state.updates.error = None;
            let (host, token, rows, generation) = (state.host.clone(), state.token.clone(), state.rows.clone(), state.generation);
            return Task::perform(async move {
                let results = desktop_settings_apps::system_settings_apps::catalog_requests::check_updates(client, host.clone(), token, rows).await;
                Message::Settings(SettingsMessage::AppsCatalog(AppsCatalogMessage::UpdatesLoaded { context, host, generation, results }))
            }, |message| message);
        }
        AppsCatalogMessage::UpdatesLoaded { context, host, generation, results } => {
            let current = crate::app::config::gateway_client().ok().map(|client| AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.navigation_tabs.project_path.clone() });
            let state = &mut app.apps_catalog;
            if current.as_ref() != Some(&context) || state.context.as_ref() != Some(&context) || state.host != host || state.generation != generation { return Task::none(); }
            state.updates.checking = false;
            let failed = results.iter().any(|r| r.status == "检查失败，请重试" || r.status == "升级预检未通过");
            state.updates.error = failed.then(|| "部分应用检查失败，请重试".into());
            if !failed { state.updates.checked_at = Some(std::time::SystemTime::now()); }
            state.updates.results = results;
        }

        AppsCatalogMessage::Commands(event) => return crate::app::components::system_settings_apps::commands::update(app, event),
        AppsCatalogMessage::SelectPromptApp(id) => {
            return crate::app::components::system_settings_prompts::list::select(app, id);
        }
        AppsCatalogMessage::SelectApp(id) => {
            if crate::app::components::system_settings_prompts::list::switch_blocked(&app.apps_catalog.prompts) { return Task::none(); }
            if !app.apps_catalog.rows.iter().any(|row| row.entry.id == id) { return Task::none(); }
            let page = desktop_ui_controls::settings_navigation::ThirdPartyPage::Agents;
            app.apps_catalog.expanded = Some(id);
            return crate::app::components::third_party_settings::navigate(app, page);
        }
        AppsCatalogMessage::ToggleAgentProviderSearch => {
            if crate::app::components::third_party_settings::is_visible(app)
                && app.apps_catalog.third_party_page
                    == desktop_ui_controls::settings_navigation::ThirdPartyPage::Agents
            {
                app.apps_catalog.agent_provider_search_open =
                    !app.apps_catalog.agent_provider_search_open
                        && app.app_providers.query.is_empty();
                if !app.apps_catalog.agent_provider_search_open {
                    app.app_providers.query.clear();
                }
            }
        }
        AppsCatalogMessage::AgentWorkspaceTab(tab) => {
            use crate::app::components::third_party_settings as workspace;
            use desktop_apps_ui::catalog_state::AgentWorkspaceTab as Tab;
            if !workspace::is_visible(app)
                || app.apps_catalog.third_party_page
                    != desktop_ui_controls::settings_navigation::ThirdPartyPage::Agents
            {
                return Task::none();
            }
            app.apps_catalog.agent_workspace_tab = tab;
            let load = workspace::ensure_agent_providers(app);
            let open = match tab {
                Tab::Direct => Task::none(),
                Tab::Routing => crate::app::components::app_routing::lifecycle::update(
                    app,
                    desktop_apps_ui::app_routing::lifecycle::Event::Run(
                        desktop_apps_ui::app_routing::lifecycle::Operation::Refresh,
                    ),
                ),
                Tab::Aggregation => {
                    super::aggregation_list::update(app, super::aggregation_list::Event::Open)
                }
                Tab::Configuration => app
                    .apps_catalog
                    .expanded
                    .and_then(vw_api_project_profiles::ProjectProfileScope::for_app)
                    .map_or_else(Task::none, |scope| {
                        crate::app::message::project_profiles::update(
                            app,
                            crate::app::message::project_profiles::Event::Refresh(scope),
                        )
                    }),
            };
            return Task::batch([load, open]);
        }
        AppsCatalogMessage::Navigate(page) => return crate::app::components::third_party_settings::navigate(app, page),
        AppsCatalogMessage::McpWorkspace(event) => return crate::app::components::third_party_settings::mcp_workspace::update(app, event),
        AppsCatalogMessage::ThirdPartyMcp(event) => return crate::app::components::third_party_settings::mcp_update(app, event),
        AppsCatalogMessage::ThirdPartySkills(event) => return crate::app::components::third_party_settings::skills_update(app, event),
        AppsCatalogMessage::ExternalSessions(event) => return crate::app::components::external_sessions::update(app, event),
        AppsCatalogMessage::Routing(event) => return crate::app::components::app_routing::lifecycle::update(app, event),
        AppsCatalogMessage::OpenClawCatalog(event) => return crate::app::components::openclaw_catalog::update(app, event),
        AppsCatalogMessage::OpenClawTools(event) => return crate::app::components::openclaw_tools::update(app, event),
        AppsCatalogMessage::OpenClawEnv(event) => return crate::app::components::openclaw_env::update(app, event),
        AppsCatalogMessage::OpenClawModel(event) => return crate::app::components::openclaw_model::update(app, event),
        AppsCatalogMessage::OpenClawDefaults(event) => return crate::app::components::openclaw_defaults::update(app, event),
        AppsCatalogMessage::Prompts(super::prompts::Event::Open(id)) => {
            return crate::app::components::system_settings_prompts::list::select(app, id);
        }
        AppsCatalogMessage::Prompts(event) => return super::prompts::update(app, event),
        AppsCatalogMessage::WorkspaceFiles(event) => return crate::app::components::workspace_files::update(app, event),
        AppsCatalogMessage::DailyMemory(event) => return crate::app::components::daily_memory::update(app, event),
        AppsCatalogMessage::HermesMemory(event) => return crate::app::components::hermes_memory::update(app, event),
        AppsCatalogMessage::HermesDashboard(event) => return crate::app::components::hermes_dashboard::update(app, event),
        AppsCatalogMessage::ProjectProfiles(event) => return crate::app::message::project_profiles::update(app, event),
        AppsCatalogMessage::RefreshObservation(id) => {
            if app.apps_catalog.loading { return Task::none(); }
            let client = match crate::app::config::gateway_client() {
                Ok(client) => client,
                Err(_) => return app.show_localized_toast(crate::app::state::ToastKind::Error, "网关不可用，请重新连接", vec![]),
            };
            let context = AgentAppsContext {
                gateway: client.endpoint().base_url(),
                tenant: client.endpoint().tenant_scope().clone(),
                project: app.navigation_tabs.project_path.clone(),
            };
            let request = match catalog_input(&mut app.apps_catalog).begin_observation(&context, id) {
                Ok(Some(request)) => request,
                Ok(None) => return Task::none(),
                Err(error) => return app.show_error_toast(error),
            };
            app.apps_catalog.updates.checking = false;
            let desktop_settings_apps::system_settings_apps::catalog_update::ObservationRequest { generation, host, token } = request;
            return Task::perform(async move {
                let client = gateway_app_read::GatewayAppReadClient::new(client.transport().clone());
                let result = client.agent_apps_installations_authorized(&host, id, &token).await;
                Message::Settings(SettingsMessage::AppsCatalog(AppsCatalogMessage::ObservationLoaded {
                    context, host, generation, id, result,
                }))
            }, |message| message);
        }
        AppsCatalogMessage::ObservationLoaded { context, host, generation, id, result } => {
            let current = crate::app::config::gateway_client().ok().map(|client| AgentAppsContext {
                gateway: client.endpoint().base_url(),
                tenant: client.endpoint().tenant_scope().clone(),
                project: app.navigation_tabs.project_path.clone(),
            });
            let state = &mut app.apps_catalog;
            let (accepted, error) = desktop_settings_apps::system_settings_apps::catalog_update::finish_observation(
                desktop_settings_apps::system_settings_apps::catalog_update::ObservationInput {
                    context: &state.context, host: &state.host, generation: state.generation,
                    loading: &mut state.loading, rows: &mut state.rows,
                }, current.as_ref(), context, host, generation, id, result,
            );
            if accepted { state.loaded_at = None; }
            return error.map(|error| app.show_error_toast(error)).unwrap_or_else(Task::none);
        }
        AppsCatalogMessage::QueryInput(query) => {
            catalog_input(&mut app.apps_catalog).query(query);
        }
        AppsCatalogMessage::ToggleDetails(id) => {
            if crate::app::components::system_settings_prompts::list::switch_blocked(&app.apps_catalog.prompts) { return Task::none(); }
            crate::app::components::third_party_settings::invalidate(app);
            app.apps_catalog.openclaw_tools.invalidate();
            app.apps_catalog.openclaw_env.invalidate();
            app.apps_catalog.workspace_files.invalidate();
            app.apps_catalog.daily_memory.invalidate();
            app.apps_catalog.hermes_memory.invalidate();
            let application = app.apps_catalog.project_profiles.application.take().filter(|f| f.submitted);
            app.apps_catalog.project_profiles.invalidate();
            app.apps_catalog.project_profiles.application = application;
            catalog_input(&mut app.apps_catalog).toggle_details(id);
            app.apps_catalog.prompts.invalidate();
            if app.apps_catalog.expanded == Some(id) {
                if let Some(scope) = vw_api_project_profiles::ProjectProfileScope::for_app(id) {
                    return crate::app::message::project_profiles::update(app, crate::app::message::project_profiles::Event::Refresh(scope));
                }
            }
        }
        AppsCatalogMessage::Drag(value) => {
            return crate::app::components::system_settings_apps::drag::update(app, value);
        }
        AppsCatalogMessage::HighAvailability(event) => {
            return crate::app::message::agent_apps_high_availability_policy::update(app, event);
        }
        AppsCatalogMessage::Launch(message) => {
            return crate::app::components::system_settings_apps::launch::update(app, message);
        }
        AppsCatalogMessage::Preferences(message) => {
            return crate::app::components::system_settings_apps::preferences::update(app, message);
        }
        AppsCatalogMessage::Lifecycle(message) => {
            return super::apps_lifecycle::update(app, message);
        }
        AppsCatalogMessage::LocalConnect => return update(app, AppsCatalogMessage::Refresh),
        AppsCatalogMessage::LocalReady { context, generation, result } => {
            let current =
                crate::app::config::gateway_client().ok().map(|client| AgentAppsContext {
                    gateway: client.endpoint().base_url(),
                    tenant: client.endpoint().tenant_scope().clone(),
                    project: app.navigation_tabs.project_path.clone(),
                });
            match catalog_input(&mut app.apps_catalog).local_ready(current.as_ref(), &context, generation, result) {
                Ok(true) => {
                    app.apps_catalog.model_diagnostics.invalidate_access();
                    return update(app, AppsCatalogMessage::Refresh);
                }
                Ok(false) => return Task::none(),
                Err(error) => return app.show_error_toast(error),
            }
        }

        AppsCatalogMessage::ConnectionOpen => {
            app.apps_catalog.connection_open = true;
            if let Some(scope) = app.apps_catalog.project_profiles.application.as_ref().map(|f| f.scope) {
                return crate::app::message::project_profiles::update(app,
                    crate::app::message::project_profiles::Event::Apply(scope, crate::app::message::project_profiles::ApplyEvent::Recover));
            }
        }
        AppsCatalogMessage::ConnectionClose => app.apps_catalog.connection_open = false,
        AppsCatalogMessage::HostInput(host) => {
            let connection_open = app.apps_catalog.connection_open;
            crate::app::components::third_party_settings::invalidate(app);
            app.apps_catalog.connection_open = connection_open;
            app.apps_catalog.external_sessions.invalidate();
            app.apps_catalog.prompts.invalidate();
            app.app_providers.model_catalog = None;
            app.apps_catalog.routing.invalidate();
            app.apps_catalog.high_availability.invalidate();
            app.apps_catalog.hermes_dashboard.invalidate();
            app.apps_catalog.openclaw_catalog.invalidate();
            app.apps_catalog.openclaw_model.invalidate();
            app.apps_catalog.openclaw_tools.invalidate();
            app.apps_catalog.openclaw_env.invalidate();
            app.apps_catalog.openclaw_defaults.invalidate();
            app.apps_catalog.workspace_files.invalidate();
            app.apps_catalog.daily_memory.invalidate();
            app.apps_catalog.hermes_memory.invalidate();
            app.apps_catalog.project_profiles.invalidate();
            app.apps_catalog.model_diagnostics.invalidate_access();
            app.apps_catalog.updates = Default::default();
            catalog_input(&mut app.apps_catalog).credentials(Some(host), None);
        }
        AppsCatalogMessage::TokenInput(token) => {
            let connection_open = app.apps_catalog.connection_open;
            crate::app::components::third_party_settings::invalidate(app);
            app.apps_catalog.connection_open = connection_open;
            app.apps_catalog.external_sessions.invalidate();
            app.apps_catalog.prompts.invalidate();
            app.app_providers.model_catalog = None;
            app.apps_catalog.context =
                crate::app::config::gateway_client().ok().map(|client| AgentAppsContext {
                    gateway: client.endpoint().base_url(),
                    tenant: client.endpoint().tenant_scope().clone(),
                    project: app.navigation_tabs.project_path.clone(),
                });
            app.apps_catalog.routing.invalidate();
            app.apps_catalog.high_availability.invalidate();
            app.apps_catalog.hermes_dashboard.invalidate();
            app.apps_catalog.openclaw_catalog.invalidate();
            app.apps_catalog.openclaw_model.invalidate();
            app.apps_catalog.openclaw_tools.invalidate();
            app.apps_catalog.openclaw_env.invalidate();
            app.apps_catalog.openclaw_defaults.invalidate();
            app.apps_catalog.workspace_files.invalidate();
            app.apps_catalog.daily_memory.invalidate();
            app.apps_catalog.hermes_memory.invalidate();
            app.apps_catalog.project_profiles.invalidate();
            app.apps_catalog.model_diagnostics.invalidate_access();
            app.apps_catalog.updates = Default::default();
            catalog_input(&mut app.apps_catalog).credentials(None, Some(token));
        }
        AppsCatalogMessage::Refresh | AppsCatalogMessage::EnsureLoaded => {
            let use_cache = matches!(message, AppsCatalogMessage::EnsureLoaded);
            let client = match crate::app::config::gateway_client() {
                Ok(client) => client,
                Err(_) => {
                    let error = catalog_input(&mut app.apps_catalog).unavailable();
                    return app.show_error_toast(error);
                }
            };
            let context = AgentAppsContext {
                gateway: client.endpoint().base_url(),
                tenant: client.endpoint().tenant_scope().clone(),
                project: app.navigation_tabs.project_path.clone(),
            };
            if crate::app::state::agent_apps::reset_if_context_changed(
                &mut app.apps_catalog,
                &context,
            ) {
                app.app_providers.scope = None;
                app.apps_catalog.agent_workspace_tab = Default::default();
                app.apps_catalog.agent_provider_search_open = false;
            }
            use desktop_settings_apps::system_settings_apps::catalog_update::RefreshDecision;
            match catalog_input(&mut app.apps_catalog).refresh_decision(use_cache, client.endpoint().is_loopback()) {
                RefreshDecision::Skip
                    if app.apps_catalog.third_party_page == desktop_ui_controls::settings_navigation::ThirdPartyPage::Prompts =>
                {
                    return crate::app::components::system_settings_prompts::list::open_selected(app);
                }
                RefreshDecision::Skip | RefreshDecision::OpenConnection => return Task::none(),
                RefreshDecision::ConnectLocal => return update(app, AppsCatalogMessage::LocalConnect),
                RefreshDecision::Load => {}
            }
            use desktop_ui_controls::settings_navigation::ThirdPartyPage;
            let selected_app = app.apps_catalog.expanded.filter(|_| {
                matches!(
                    app.apps_catalog.third_party_page,
                    ThirdPartyPage::Agents | ThirdPartyPage::Prompts,
                )
            });
            let generation =
                crate::app::state::agent_apps::begin(&mut app.apps_catalog, context.clone());
            if let Some(id) = selected_app {
                app.apps_catalog.expanded = Some(id);
            }
            let host = app.apps_catalog.host.clone();
            let token = app.apps_catalog.token.clone();
            app.apps_catalog.connection_open = false;
            return Task::perform(desktop_settings_apps::system_settings_apps::catalog_requests::load(client, host.clone(), token), move |result| {
                Message::Settings(SettingsMessage::AppsCatalog(AppsCatalogMessage::Loaded {
                    context: context.clone(), host: host.clone(), generation, result,
                }))
            });
        }
        AppsCatalogMessage::Loaded { context, host, generation, result } => {
            let current =
                crate::app::config::gateway_client().ok().map(|client| AgentAppsContext {
                    gateway: client.endpoint().base_url(),
                    tenant: client.endpoint().tenant_scope().clone(),
                    project: app.navigation_tabs.project_path.clone(),
                });
            if current.as_ref() != Some(&context) || host != app.apps_catalog.host {
                return Task::none();
            }
            if crate::app::state::agent_apps::finish(&mut app.apps_catalog, &context, generation, result) {
                if let Some(error) = app.apps_catalog.error.clone() {
                    return app.show_error_toast(error);
                }
                if app.apps_catalog.third_party_page == desktop_ui_controls::settings_navigation::ThirdPartyPage::Skills
                    && app.apps_catalog.third_party_skills.scope().is_some_and(|scope| {
                        scope.context == context && scope.host == host
                    })
                {
                    let input = super::skills::update_input(app);
                    app.apps_catalog.third_party_skills.reconcile_targets(&input.targets);
                }
                let navigation = if let Some(id) = app.apps_catalog.pending_tray_app.take() {
                    if app.apps_catalog.rows.iter().any(|row| row.entry.id == id)
                        && app.apps_catalog.expanded != Some(id)
                    {
                        update(app, AppsCatalogMessage::ToggleDetails(id))
                    } else {
                        Task::none()
                    }
                } else {
                    Task::none()
                };
                let prompts = if app.apps_catalog.third_party_page == desktop_ui_controls::settings_navigation::ThirdPartyPage::Prompts {
                    crate::app::components::system_settings_prompts::list::open_selected(app)
                } else { Task::none() };
                return Task::batch([
                    prompts,
                    crate::app::components::third_party_settings::ensure_agent_providers(app),
                    crate::app::components::third_party_settings::mcp_workspace::refresh(app),
                    navigation,
                    crate::app::components::system_settings_apps::preferences::update(
                        app, crate::app::components::system_settings_apps::preferences::PreferencesMessage::Load,
                    ),
                    crate::app::components::app_routing::lifecycle::update(
                        app, crate::app::components::app_routing::lifecycle::Event::Run(
                            crate::app::components::app_routing::lifecycle::Operation::Refresh,
                        ),
                    ),
                ]);
            }
        }
    }
    Task::none()
}

pub(crate) fn catalog_input(state: &mut crate::app::state::AppsCatalogState) -> desktop_settings_apps::system_settings_apps::catalog_update::CatalogInput<'_> {
    desktop_settings_apps::system_settings_apps::catalog_update::CatalogInput {
        context: &mut state.context, connection_open: &mut state.connection_open,
        host: &mut state.host, token: &mut state.token, query: &mut state.query,
        generation: &mut state.generation, loading: &mut state.loading, rows: &mut state.rows,
        loaded_at: &mut state.loaded_at, error: &mut state.error, expanded: &mut state.expanded,
        hovered: &mut state.hovered, drag: &mut state.drag, lifecycle: &mut state.lifecycle,
        preferences: &mut state.preferences, launch: &mut state.launch,
    }
}

#[cfg(test)]
use crate::app::components::system_settings_apps::catalog;
