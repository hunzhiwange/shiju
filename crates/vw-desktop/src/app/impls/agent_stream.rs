//! Map session stream events into the desktop application message.
use super::{AgentRequest, Message};
pub(crate) use desktop_session::agent_stream::{desktop_stream_request_id, task_plan_run_active};

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
