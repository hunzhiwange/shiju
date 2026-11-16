//! External execution ports for HTTP policy tests. Security uses real domain entrypoints.
use super::*;
use axum::{extract::{State, ConnectInfo}, http::{HeaderMap, StatusCode}, Json, response::Response};
use channel_support::runtime_state::ChannelRuntimeContext;
use channel_types::ChannelMessage;
fn runtime(_: &AppState) -> anyhow::Result<Arc<ChannelRuntimeContext>> {
    panic!("rejected or verification-only webhook must not initialize channel runtime")
}
fn dispatch(_: Arc<ChannelRuntimeContext>, _: ChannelMessage, _: tokio_util::sync::CancellationToken) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
    panic!("rejected or verification-only webhook must not dispatch")
}
pub(super) mod handlers {
    use super::*;
    pub async fn handle_qq_webhook(state: State<AppState>, headers: HeaderMap, body: axum::body::Bytes) -> Response {
        crate::handlers::handle_qq_webhook(state, headers, body, runtime, dispatch).await
    }
    pub async fn handle_nextcloud_talk_webhook(state: State<AppState>, headers: HeaderMap, body: axum::body::Bytes) -> Response {
        crate::handlers::handle_nextcloud_talk_webhook(state, headers, body, runtime, dispatch).await
    }
}
pub(super) mod webhook {
    use super::*;
    pub async fn handle_webhook(state: State<AppState>, peer: ConnectInfo<SocketAddr>, headers: HeaderMap, body: Result<Json<WebhookBody>, axum::extract::rejection::JsonRejection>) -> Response {
        crate::webhook::handle_webhook(state, peer, headers, body, |state, message| Box::pin(async move {
            state.provider.chat_with_system(None, message, &state.model, state.temperature).await
        })).await
    }
}
pub(super) mod agent {
    use super::*;
    struct RejectChat;
    impl crate::agent::AgentHost<(), GatewayChannels, ()> for RejectChat {
        fn client_key_from_request(peer: Option<SocketAddr>, headers: &HeaderMap, trust: bool) -> String { crate::util::client_key_from_request(peer, headers, trust) }
        fn require_auth(state: &AppState, headers: &HeaderMap) -> Result<(), (StatusCode, Json<serde_json::Value>)> { crate::api::auth::require_auth(state, headers) }
        fn sanitize_gateway_response(response: &str, tools: &[Box<dyn tool_types::Tool>]) -> String { crate::chat::sanitize_gateway_response(response, tools) }
        fn run_agent_chat<'a>(_: &'a AppState, _: &'a str, _: &'a str) -> futures_util::future::BoxFuture<'a, anyhow::Result<String>> { panic!("unauthorized agent request must not invoke chat") }
    }
    pub async fn handle_agent(state: State<AppState>, peer: ConnectInfo<SocketAddr>, headers: HeaderMap, body: Result<Json<AgentBody>, axum::extract::rejection::JsonRejection>) -> Response {
        crate::agent::handle_agent::<_, _, _, RejectChat>(state, peer, headers, body).await
    }
}
