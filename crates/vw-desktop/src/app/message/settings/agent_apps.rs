//! 桌面图标列表仅通过当前网关 SDK 加载，响应按上下文与请求代次隔离。
use super::SettingsMessage;
use crate::app::state::AgentAppsContext;
use crate::app::{App, Message};
use iced::Task;
use vw_gateway_client::{GatewayClient, vw_api_types::agent_apps::CustomIconEntry};

#[derive(Debug, Clone)]
pub enum AgentAppsMessage {
    Refresh,
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

pub(super) fn update(app: &mut App, message: AgentAppsMessage) -> Task<Message> {
    match message {
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
