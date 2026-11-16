use crate::management_context::Context;
use gateway_http::AgentAppsReadError;
use vw_api_mcp_management::*;
use vw_gateway_client::GatewayClient;
#[derive(Debug, Clone, Default)]
pub struct CatalogState {
    pub context: Option<Context>,
    pub generation: u64,
    pub loading: bool,
    pub result: Option<Result<McpCatalogResponse, String>>,
}
#[derive(Debug, Clone)]
pub enum CatalogMessage {
    Refresh,
    Loaded { context: Context, generation: u64, result: Result<McpCatalogResponse, String> },
}
pub async fn load(
    client: GatewayClient,
    host: &str,
    token: &str,
) -> Result<McpCatalogResponse, AgentAppsReadError> {
    gateway_mcp_read::GatewayMcpReadClient::new(client.transport().clone())
        .mcp_management_catalog(host, token)
        .await
}
impl CatalogState {
    fn finish(
        &mut self,
        current: &Context,
        request: &Context,
        generation: u64,
        result: Result<McpCatalogResponse, String>,
    ) -> bool {
        if current != request {
            return false;
        }
        if self.context.as_ref() == Some(request) && self.generation == generation {
            self.loading = false;
            self.result = Some(result);
            return true;
        }
        false
    }
}

use crate::management_effect::Effect;
use iced::Task;
pub struct Update<'a> {
    pub state: &'a mut CatalogState,
    pub context: Option<Context>,
    pub authorized: bool,
    pub token: String,
    pub client: Result<GatewayClient, String>,
    pub effects: Vec<Effect>,
}
impl Update<'_> {
    fn show_error_toast(&mut self, value: impl Into<String>) -> Task<CatalogMessage> {
        self.effects.push(Effect::Error(value.into()));
        Task::none()
    }
}
pub fn update(app: &mut Update<'_>, message: CatalogMessage) -> Task<CatalogMessage> {
    match message {
        CatalogMessage::Refresh => {
            let Some(context) = app.context.clone() else {
                return app.show_error_toast("网关不可用");
            };
            let state = &mut *app.state;
            state.generation = state.generation.wrapping_add(1);
            state.context = Some(context.clone());
            state.result = None;
            state.loading = false;
            if context.host.is_empty() || app.token.is_empty() || !app.authorized {
                state.result = Some(Err("请先在应用管理中连接目标主机".into()));
                return Task::none();
            }
            let client = match app.client.clone() {
                Ok(client) => client,
                Err(error) => return app.show_error_toast(error),
            };
            let generation = state.generation;
            state.loading = true;
            let token = app.token.clone();
            Task::perform(
                async move {
                    let result = load(client, &context.host, &token).await.map_err(|error| {
                        desktop_settings_apps::system_settings_apps::catalog::error_label(&error)
                            .to_owned()
                    });
                    CatalogMessage::Loaded { context, generation, result }
                },
                std::convert::identity,
            )
        }
        CatalogMessage::Loaded { context: request, generation, result } => {
            if let Some(current) = app.context.clone() {
                let accepted = app.state.finish(&current, &request, generation, result);
                if accepted && let Some(Err(error)) = app.state.result.as_ref() {
                    return app.show_error_toast(error.clone());
                }
            }
            Task::none()
        }
    }
}

#[cfg(test)]
#[path = "management_list_tests.rs"]
mod tests;

pub fn loading(state: &CatalogState, current: Option<&crate::management_context::Context>) -> bool {
    state.loading && current == state.context.as_ref()
}
pub fn visible<'a>(
    state: &'a CatalogState,
    current: Option<&crate::management_context::Context>,
) -> Option<&'a Result<McpCatalogResponse, String>> {
    (current == state.context.as_ref()).then_some(state.result.as_ref()).flatten()
}
