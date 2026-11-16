use crate::agents_update_message::{Effect, Message, SettingsMessage};
use iced::Task;
pub(super) struct AgentsControllerFixture {
    pub agents_settings: crate::agents_state::AgentsSettingsState,
    pub project_path: Option<String>,
    pub endpoint: gateway_endpoint::GatewayEndpoint,
    pub effects: Vec<Effect>,
}
fn input(app: &mut AgentsControllerFixture) -> crate::agents_update::Update<'_> {
    crate::agents_update::Update {
        state: &mut app.agents_settings,
        access: desktop_config::ConfigAccess::new(app.endpoint.clone(), None),
        client: vw_gateway_client::GatewayClient::new(app.endpoint.clone()),
        acp_agents: &[],
        acp_loading: false,
        new_session_creating: false,
        project_path: app.project_path.clone(),
        validation_error: None,
        effects: Vec::new(),
    }
}
pub(super) fn update(app: &mut AgentsControllerFixture, message: SettingsMessage) -> Task<Message> {
    let mut context = input(app);
    let task = crate::agents_update::update(&mut context, message);
    let effects = std::mem::take(&mut context.effects);
    drop(context);
    app.effects.extend(effects);
    task
}
pub(super) fn handle_agents_config_saved(
    app: &mut AgentsControllerFixture,
    result: Result<(), String>,
) -> Task<Message> {
    let mut context = input(app);
    let task = crate::agents_update::handle_agents_config_saved(&mut context, result);
    let effects = std::mem::take(&mut context.effects);
    drop(context);
    app.effects.extend(effects);
    task
}
