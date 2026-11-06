//! Catalog requests use only SDK reads and reject stale gateway/host responses.
use super::SettingsMessage;
use crate::app::components::system_settings_apps::catalog::{self, CatalogRow};
use crate::app::{App, Message, state::AgentAppsContext};
use iced::Task;

#[derive(Clone)]
pub enum AppsCatalogMessage {
    Lifecycle(super::apps_lifecycle::LifecycleMessage),
    ConnectionOpen,
    ConnectionClose,
    HostInput(String),
    TokenInput(String),
    Refresh,
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
        AppsCatalogMessage::Lifecycle(message) => {
            return super::apps_lifecycle::update(app, message);
        }
        AppsCatalogMessage::ConnectionOpen => app.apps_catalog.connection_open = true,
        AppsCatalogMessage::ConnectionClose => app.apps_catalog.connection_open = false,
        AppsCatalogMessage::HostInput(host) => {
            app.apps_catalog.lifecycle.invalidate();
            app.apps_catalog.host = host;
            app.apps_catalog.generation = app.apps_catalog.generation.wrapping_add(1);
            app.apps_catalog.loading = false;
            app.apps_catalog.rows.clear();
            app.apps_catalog.error = None;
        }
        AppsCatalogMessage::TokenInput(token) => {
            app.apps_catalog.context =
                crate::app::config::gateway_client().ok().map(|client| AgentAppsContext {
                    gateway: client.endpoint().base_url(),
                    tenant: client.endpoint().tenant_scope().clone(),
                    project: app.project_path.clone(),
                });
            app.apps_catalog.lifecycle.invalidate();
            app.apps_catalog.token = token;
            app.apps_catalog.generation = app.apps_catalog.generation.wrapping_add(1);
            app.apps_catalog.loading = false;
            app.apps_catalog.rows.clear();
            app.apps_catalog.error = None;
        }
        AppsCatalogMessage::Refresh => {
            if app.apps_catalog.host.trim().is_empty() || app.apps_catalog.token.is_empty() {
                app.apps_catalog.connection_open = true;
                return Task::none();
            }
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
            if app.apps_catalog.reset_if_context_changed(&context) {
                return Task::none();
            }
            if app.apps_catalog.loading {
                return Task::none();
            }
            let generation = app.apps_catalog.begin(context.clone());
            let host = app.apps_catalog.host.clone();
            let token = app.apps_catalog.token.clone();
            app.apps_catalog.connection_open = false;
            return Task::perform(catalog::load(client, host.clone(), token), move |result| {
                Message::Settings(SettingsMessage::AppsCatalog(AppsCatalogMessage::Loaded {
                    context: context.clone(),
                    host: host.clone(),
                    generation,
                    result: result.map_err(|e| catalog::error_label(&e).into()),
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
            if app.apps_catalog.finish(&context, generation, result)
                && let Some(error) = app.apps_catalog.error.clone()
            {
                return app.show_error_toast(error);
            }
        }
    }
    Task::none()
}
