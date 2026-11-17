//! Host task-scope adapter for provider transport.
use super::{RelayBackend, RelayIdentity, RelayRequest};
use axum::{http::StatusCode, response::Response};
use tokio_util::sync::CancellationToken;

pub struct ProviderRelayBackend(model_transport::provider_transport::ProviderTransport);
impl ProviderRelayBackend {
    pub fn new() -> Result<Self, reqwest::Error> {
        model_transport::provider_transport::ProviderTransport::new().map(Self)
    }
}


#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use model_protocol::endpoint_urls::chat_url;


#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use model_protocol::endpoint_urls::official_deepseek_messages_url;

#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use model_protocol::endpoint_urls::official_alibaba_messages_url;

#[cfg(any(test, target_arch = "wasm32"))]
pub(super) use model_protocol::endpoint_urls::messages_url;

#[cfg(any(test, target_arch = "wasm32"))]
pub(super) use model_protocol::endpoint_urls::responses_url;

#[cfg(any(test, target_arch = "wasm32"))]
pub(super) use model_protocol::endpoint_urls::gemini_url;

#[async_trait::async_trait]
impl RelayBackend for ProviderRelayBackend {
    async fn execute(&self, identity: &RelayIdentity, request: RelayRequest, cancel: CancellationToken) -> Result<Response, StatusCode> {
        if crate::session::ui_store::current_tenant_scope() != identity.tenant
            || crate::project::instance::project().is_none_or(|p| p.id != identity.project)
        {
            return Err(StatusCode::FORBIDDEN);
        }
        self.0.execute(identity, request.path, &request.body, cancel).await
    }
}
