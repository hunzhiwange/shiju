use crate::{
    management_context::Context, management_effect::Effect, management_state::McpSettingsState,
};
use vw_api_mcp_management::*;
use vw_gateway_client::GatewayClient;
pub struct Update<'a> {
    pub state: &'a mut McpSettingsState,
    pub context: Option<Context>,
    pub authorized: bool,
    pub token: String,
    pub client: Result<GatewayClient, String>,
    pub targets: Vec<McpAppTarget>,
    pub effects: Vec<Effect>,
}
impl Update<'_> {
    pub fn error<M: Send + 'static>(&mut self, value: impl Into<String>) -> iced::Task<M> {
        self.effects.push(Effect::Error(value.into()));
        iced::Task::none()
    }
    pub fn info<M: Send + 'static>(&mut self, value: impl Into<String>) -> iced::Task<M> {
        self.effects.push(Effect::Info(value.into()));
        iced::Task::none()
    }
    pub fn success<M: Send + 'static>(&mut self, value: impl Into<String>) -> iced::Task<M> {
        self.effects.push(Effect::Success(value.into()));
        iced::Task::none()
    }
    pub fn refresh<M: Send + 'static>(&mut self) -> iced::Task<M> {
        self.effects.push(Effect::Refresh);
        iced::Task::none()
    }
}
pub fn context(input: &Update<'_>) -> Option<Context> {
    input.context.clone()
}
pub fn visible<'a>(input: &'a Update<'_>) -> Option<&'a Result<McpCatalogResponse, String>> {
    (input.context.as_ref() == input.state.catalog.context.as_ref())
        .then_some(input.state.catalog.result.as_ref())
        .flatten()
}
pub fn loading(input: &Update<'_>) -> bool {
    input.state.catalog.loading && input.context.as_ref() == input.state.catalog.context.as_ref()
}
