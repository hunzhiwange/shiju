//! 桌面图标列表仅通过当前网关 SDK 加载，响应按上下文与请求代次隔离。
use super::SettingsMessage;
use crate::app::state::{AgentAppsContext, IconRenameState};
use crate::app::{App, Message};
use iced::Task;
use vw_gateway_client::{GatewayClient, vw_api_types::agent_apps::CustomIconEntry};

#[derive(Debug, Clone)]
pub enum AgentAppsMessage {
    Refresh,
    RenameOpen(String),
    RenameInput(String),
    RenameClose,
    RenameSubmit,
    Renamed {
        context: AgentAppsContext,
        generation: u64,
        result: Result<(), String>,
    },
    Loaded {
        context: AgentAppsContext,
        generation: u64,
        result: Result<Vec<CustomIconEntry>, String>,
    },
}

fn context(client: &GatewayClient, project: Option<String>) -> AgentAppsContext {
    AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project,
    }
}

async fn load(client: Result<GatewayClient, String>) -> Result<Vec<CustomIconEntry>, String> {
    client?.custom_icons_list().await
}

async fn rename(client: GatewayClient, form: IconRenameState) -> Result<(), String> {
    client
        .custom_icon_rename(
            &form.id,
            &vw_gateway_client::vw_api_types::agent_apps::RenameCustomIconRequest {
                name: form.name,
            },
        )
        .await?;
    Ok(())
}

pub(super) fn update(app: &mut App, message: AgentAppsMessage) -> Task<Message> {
    match message {
        AgentAppsMessage::RenameOpen(id) => {
            if app.agent_apps.loading || app.agent_apps.rename.is_some() {
                return Task::none();
            }
            let Ok(client) = crate::app::config::gateway_client() else {
                return app.show_error_toast("网关不可用");
            };
            let current = context(&client, app.project_path.clone());
            if !app.agent_apps.belongs_to(&current) {
                return update(app, AgentAppsMessage::Refresh);
            }
            let Some(icon) = app.agent_apps.icons.iter().find(|icon| icon.id == id) else {
                return Task::none();
            };
            app.agent_apps.rename_generation = app.agent_apps.rename_generation.wrapping_add(1);
            app.agent_apps.rename = Some(IconRenameState {
                id,
                name: icon.name.clone(),
                saving: false,
                error: None,
                context: current,
                generation: app.agent_apps.rename_generation,
            });
            iced::widget::operation::focus("custom-icon-rename")
        }
        AgentAppsMessage::RenameInput(name) => {
            if let Some(form) = &mut app.agent_apps.rename
                && !form.saving
            {
                form.name = name;
                form.error = None;
            }
            Task::none()
        }
        AgentAppsMessage::RenameClose => {
            if app.agent_apps.rename.as_ref().is_some_and(|form| !form.saving) {
                app.agent_apps.rename = None;
            }
            Task::none()
        }
        AgentAppsMessage::RenameSubmit => {
            let Some(form) = &mut app.agent_apps.rename else {
                return Task::none();
            };
            if form.saving {
                return Task::none();
            }
            let client = match crate::app::config::gateway_client() {
                Ok(client) => client,
                Err(error) => {
                    form.error = Some(error.clone());
                    return app.show_error_toast(error);
                }
            };
            if context(&client, app.project_path.clone()) != form.context {
                app.agent_apps.rename = None;
                return update(app, AgentAppsMessage::Refresh);
            }
            form.saving = true;
            form.error = None;
            let pending = form.clone();
            Task::perform(rename(client, pending.clone()), move |result| {
                Message::Settings(SettingsMessage::AgentApps(AgentAppsMessage::Renamed {
                    context: pending.context.clone(),
                    generation: pending.generation,
                    result,
                }))
            })
        }
        AgentAppsMessage::Renamed { context: requested, generation, result } => {
            let Some(form) = &mut app.agent_apps.rename else {
                return Task::none();
            };
            if !form.saving || form.context != requested || form.generation != generation {
                return Task::none();
            }
            let current = crate::app::config::gateway_client()
                .map(|client| context(&client, app.project_path.clone()));
            if current.as_ref() != Ok(&requested) {
                app.agent_apps.rename = None;
                return Task::none();
            }
            match result {
                Ok(()) => {
                    app.agent_apps.rename = None;
                    update(app, AgentAppsMessage::Refresh)
                }
                Err(error) => {
                    form.saving = false;
                    form.error = Some(error.clone());
                    app.show_error_toast(error)
                }
            }
        }
        AgentAppsMessage::Refresh => {
            let client = match crate::app::config::gateway_client() {
                Ok(client) => client,
                Err(error) => {
                    app.agent_apps.loading = false;
                    app.agent_apps.icons.clear();
                    app.agent_apps.error = Some(error.clone());
                    return app.show_error_toast(error);
                }
            };
            let context = context(&client, app.project_path.clone());
            let generation = app.agent_apps.begin(context.clone());
            Task::perform(load(Ok(client)), move |result| {
                Message::Settings(SettingsMessage::AgentApps(AgentAppsMessage::Loaded {
                    context: context.clone(),
                    generation,
                    result,
                }))
            })
        }
        AgentAppsMessage::Loaded { context: requested, generation, result } => {
            let Ok(client) = crate::app::config::gateway_client() else {
                return Task::none();
            };
            let current = context(&client, app.project_path.clone());
            if app.agent_apps.finish(&current, &requested, generation, result)
                && let Some(error) = app.agent_apps.error.clone()
            {
                return app.show_error_toast(error);
            }
            Task::none()
        }
    }
}

#[cfg(test)]
#[path = "agent_apps_tests.rs"]
mod tests;
