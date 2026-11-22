//! Map session stream events into the desktop application message.
use super::{AgentRequest, Message};
pub(crate) use desktop_session::agent_stream::{desktop_stream_request_id, task_plan_run_active};

#[cfg(not(target_arch = "wasm32"))]
pub(super) struct EmployeeStream {
    pub request: std::sync::Arc<AgentRequest>,
    pub endpoint: gateway_endpoint::GatewayEndpoint,
    pub credential: gateway_shiju_account::ShijuCredential,
    pub target: Option<desktop_session::session::agent::EmployeeTarget>,
}

#[cfg(not(target_arch = "wasm32"))]
impl std::hash::Hash for EmployeeStream {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.request, state);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn employee_stream(params: &EmployeeStream) -> desktop_session::agent_stream::AgentBoxStream<Message> {
    use iced::futures::StreamExt;
    desktop_session::agent_stream::employee_agent_stream(
        &params.request,
        crate::app::config::config_access(),
        params.endpoint.clone(),
        params.credential.clone(),
        params.target.clone(),
    ).map(|event| Message::Chat(event.into())).boxed()
}

pub(super) fn agent_stream(
    req: &AgentRequest,
) -> desktop_session::agent_stream::AgentBoxStream<Message> {
    use iced::futures::StreamExt;
    let stream = desktop_session::agent_stream::agent_stream(
        req,
        crate::app::config::config_access(),
        crate::app::config::gateway_client_endpoint(),
    )
    .map(|event| Message::Chat(event.into()));
    #[cfg(not(target_arch = "wasm32"))]
    {
        stream.boxed()
    }
    #[cfg(target_arch = "wasm32")]
    {
        stream.boxed_local()
    }
}
