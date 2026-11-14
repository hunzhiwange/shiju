//! Catalog requests use only SDK reads and reject stale gateway/host responses.
use super::SettingsMessage;
use crate::app::components::system_settings_apps::catalog::{self, CatalogRow};
use crate::app::{App, Message, state::AgentAppsContext};
use iced::Task;

#[derive(Clone)]
pub enum AppsCatalogMessage {
    Routing(crate::app::components::app_routing::lifecycle::Event),
    OpenClawTools(crate::app::components::openclaw_tools::Event),
    OpenClawEnv(crate::app::components::openclaw_env::Event),
    OpenClawCatalog(crate::app::components::openclaw_catalog::Event),
    OpenClawModel(crate::app::components::openclaw_model::Event),
    OpenClawDefaults(crate::app::components::openclaw_defaults::Event),
    WorkspaceFiles(crate::app::components::workspace_files::Event),
    DailyMemory(crate::app::components::daily_memory::Event),
    HermesMemory(crate::app::components::hermes_memory::Event),
    HermesDashboard(crate::app::components::hermes_dashboard::Event),
    Launch(crate::app::components::system_settings_apps::launch::LaunchMessage),
    Preferences(crate::app::components::system_settings_apps::preferences::PreferencesMessage),
    Lifecycle(super::apps_lifecycle::LifecycleMessage),
    QueryInput(String),
    ToggleDetails(vw_gateway_client::vw_api_types::agent_apps::AppId),
    Drag(crate::app::components::system_settings_apps::drag::DragMessage),
    LocalConnect,
    LocalReady {
        context: AgentAppsContext,
        generation: u64,
        result: Result<vw_shared::local_apps::Credentials, String>,
    },
    ConnectionOpen,
    ConnectionClose,
    HostInput(String),
    TokenInput(String),
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
pub(super) fn update(app: &mut App, message: AppsCatalogMessage) -> Task<Message> {
    match message {
        AppsCatalogMessage::Routing(event) => return crate::app::components::app_routing::lifecycle::update(app, event),
        AppsCatalogMessage::OpenClawTools(event) => return crate::app::components::openclaw_tools::update(app, event),
        AppsCatalogMessage::OpenClawEnv(event) => return crate::app::components::openclaw_env::update(app, event),
        AppsCatalogMessage::OpenClawCatalog(event) => return crate::app::components::openclaw_catalog::update(app, event),
        AppsCatalogMessage::OpenClawModel(event) => return crate::app::components::openclaw_model::update(app, event),
        AppsCatalogMessage::OpenClawDefaults(event) => return crate::app::components::openclaw_defaults::update(app, event),
        AppsCatalogMessage::WorkspaceFiles(event) => return crate::app::components::workspace_files::update(app, event),
        AppsCatalogMessage::DailyMemory(event) => return crate::app::components::daily_memory::update(app, event),
        AppsCatalogMessage::HermesMemory(event) => return crate::app::components::hermes_memory::update(app, event),
        AppsCatalogMessage::HermesDashboard(event) => return crate::app::components::hermes_dashboard::update(app, event),
        AppsCatalogMessage::QueryInput(query) => {
            app.apps_catalog.query = query;
            app.apps_catalog.drag = Default::default();
        }
        AppsCatalogMessage::ToggleDetails(id) => {
            app.apps_catalog.openclaw_tools.invalidate();
            app.apps_catalog.openclaw_env.invalidate();
            app.apps_catalog.workspace_files.invalidate();
            app.apps_catalog.daily_memory.invalidate();
            app.apps_catalog.hermes_memory.invalidate();
            app.apps_catalog.expanded = (app.apps_catalog.expanded != Some(id)).then_some(id);
        }
        AppsCatalogMessage::Drag(value) => {
            return crate::app::components::system_settings_apps::drag::update(app, value);
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
        AppsCatalogMessage::LocalConnect => {
            if app.apps_catalog.loading {
                return Task::none();
            }
            let client = match crate::app::config::gateway_client() {
                Ok(client) if client.endpoint().is_loopback() => client,
                _ => return app.show_error_toast("请先切换到本机网关"),
            };
            let context = AgentAppsContext {
                gateway: client.endpoint().base_url(),
                tenant: client.endpoint().tenant_scope().clone(),
                project: app.project_path.clone(),
            };
            let generation = app.apps_catalog.begin(context.clone());
            app.app_providers.model_catalog = None;
            app.apps_catalog.token.clear();
            app.apps_catalog.host.clear();
            return Task::perform(
                crate::app::components::system_settings_apps::local::connect(client),
                move |result| {
                    Message::Settings(SettingsMessage::AppsCatalog(
                        AppsCatalogMessage::LocalReady {
                            context: context.clone(),
                            generation,
                            result,
                        },
                    ))
                },
            );
        }
        AppsCatalogMessage::LocalReady { context, generation, result } => {
            let current =
                crate::app::config::gateway_client().ok().map(|client| AgentAppsContext {
                    gateway: client.endpoint().base_url(),
                    tenant: client.endpoint().tenant_scope().clone(),
                    project: app.project_path.clone(),
                });
            if current.as_ref() != Some(&context)
                || app.apps_catalog.generation != generation
                || !app.apps_catalog.loading
            {
                return Task::none();
            }
            app.apps_catalog.loading = false;
            match result {
                Ok(credentials) => {
                    app.apps_catalog.model_diagnostics.invalidate_access();
                    app.apps_catalog.token = credentials.token;
                    app.apps_catalog.host = vw_shared::local_apps::HOST_ID.into();
                    return update(app, AppsCatalogMessage::Refresh);
                }
                Err(error) => {
                    app.apps_catalog.error = Some(error.clone());
                    return app.show_error_toast(error);
                }
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
            app.app_providers.model_catalog = None;
            app.apps_catalog.lifecycle.invalidate();
            app.apps_catalog.routing.invalidate();
            app.apps_catalog.preferences.invalidate();
            app.apps_catalog.launch.invalidate();
            app.apps_catalog.openclaw_catalog.invalidate();
            app.apps_catalog.openclaw_model.invalidate();
            app.apps_catalog.openclaw_tools.invalidate();
            app.apps_catalog.openclaw_env.invalidate();
            app.apps_catalog.openclaw_defaults.invalidate();
            app.apps_catalog.hermes_dashboard.invalidate();
            app.apps_catalog.workspace_files.invalidate();
            app.apps_catalog.daily_memory.invalidate();
            app.apps_catalog.hermes_memory.invalidate();
            app.apps_catalog.model_diagnostics.invalidate_access();
            app.apps_catalog.host = host;
            app.apps_catalog.drag = Default::default();
            app.apps_catalog.expanded = None;
            app.apps_catalog.generation = app.apps_catalog.generation.wrapping_add(1);
            app.apps_catalog.loading = false;
            app.apps_catalog.rows.clear();
            app.apps_catalog.loaded_at = None;
            app.apps_catalog.error = None;
        }
        AppsCatalogMessage::TokenInput(token) => {
            app.app_providers.model_catalog = None;
            app.apps_catalog.context =
                crate::app::config::gateway_client().ok().map(|client| AgentAppsContext {
                    gateway: client.endpoint().base_url(),
                    tenant: client.endpoint().tenant_scope().clone(),
                    project: app.project_path.clone(),
                });
            app.apps_catalog.lifecycle.invalidate();
            app.apps_catalog.routing.invalidate();
            app.apps_catalog.preferences.invalidate();
            app.apps_catalog.launch.invalidate();
            app.apps_catalog.openclaw_catalog.invalidate();
            app.apps_catalog.openclaw_model.invalidate();
            app.apps_catalog.openclaw_tools.invalidate();
            app.apps_catalog.openclaw_env.invalidate();
            app.apps_catalog.openclaw_defaults.invalidate();
            app.apps_catalog.hermes_dashboard.invalidate();
            app.apps_catalog.workspace_files.invalidate();
            app.apps_catalog.daily_memory.invalidate();
            app.apps_catalog.hermes_memory.invalidate();
            app.apps_catalog.model_diagnostics.invalidate_access();
            app.apps_catalog.token = token;
            app.apps_catalog.drag = Default::default();
            app.apps_catalog.expanded = None;
            app.apps_catalog.generation = app.apps_catalog.generation.wrapping_add(1);
            app.apps_catalog.loading = false;
            app.apps_catalog.rows.clear();
            app.apps_catalog.loaded_at = None;
            app.apps_catalog.error = None;
        }
        AppsCatalogMessage::Refresh | AppsCatalogMessage::EnsureLoaded => {
            let use_cache = matches!(message, AppsCatalogMessage::EnsureLoaded);
            let client = match crate::app::config::gateway_client() {
                Ok(client) => client,
                Err(_) => {
                    app.apps_catalog.loading = false;
                    app.apps_catalog.generation = app.apps_catalog.generation.wrapping_add(1);
                    app.apps_catalog.rows.clear();
                    app.apps_catalog.error = Some("网关不可用，请配置连接后重试".into());
                    return app.show_error_toast("网关不可用，请配置连接后重试");
                }
            };
            let context = AgentAppsContext {
                gateway: client.endpoint().base_url(),
                tenant: client.endpoint().tenant_scope().clone(),
                project: app.project_path.clone(),
            };
            app.apps_catalog.reset_if_context_changed(&context);
            if use_cache && app.apps_catalog.cache_is_fresh() {
                return Task::none();
            }
            if app.apps_catalog.host.trim().is_empty() || app.apps_catalog.token.is_empty() {
                if client.endpoint().is_loopback() && cfg!(unix) {
                    return update(app, AppsCatalogMessage::LocalConnect);
                }
                app.apps_catalog.connection_open = true;
                return Task::none();
            }
            if app.apps_catalog.loading {
                return Task::none();
            }
            let generation = app.apps_catalog.begin(context.clone());
            let host = app.apps_catalog.host.clone();
            let token = app.apps_catalog.token.clone();
            app.apps_catalog.connection_open = false;
            let local = client.endpoint().is_loopback() && host == vw_shared::local_apps::HOST_ID;
            return Task::perform(catalog::load(client, host.clone(), token), move |result| {
                Message::Settings(SettingsMessage::AppsCatalog(AppsCatalogMessage::Loaded {
                    context: context.clone(),
                    host: host.clone(),
                    generation,
                    result: result.map_err(|e| {
                        if local
                            && matches!(
                                e,
                                vw_gateway_client::AgentAppsReadError::Http { status: 503, .. }
                            )
                        {
                            "本机应用服务尚未就绪，请重启新版网关后重试".into()
                        } else {
                            catalog::error_label(&e).into()
                        }
                    }),
                }))
            });
        }
        AppsCatalogMessage::Loaded { context, host, generation, result } => {
            let current =
                crate::app::config::gateway_client().ok().map(|client| AgentAppsContext {
                    gateway: client.endpoint().base_url(),
                    tenant: client.endpoint().tenant_scope().clone(),
                    project: app.project_path.clone(),
                });
            if current.as_ref() != Some(&context) || host != app.apps_catalog.host {
                return Task::none();
            }
            if app.apps_catalog.finish(&context, generation, result) {
                if let Some(error) = app.apps_catalog.error.clone() {
                    return app.show_error_toast(error);
                }
                return Task::batch([
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
