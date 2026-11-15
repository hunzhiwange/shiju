//! Application context adapter for the independent HTTP service.
use super::{RelayBinding, RelayIdentity, generation::GenerationProtocol};
use axum::{http::StatusCode, response::{IntoResponse, Response}};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
pub use model_relay_http::{RelayBackend, RelayRequest};

pub struct RelayServer(model_relay_http::RelayServer);
impl RelayServer {
    pub async fn start(binding: Arc<RelayBinding>, backend: Arc<dyn RelayBackend>, context: crate::project::instance::Context) -> anyhow::Result<Self> {
        anyhow::ensure!(context.project.id == binding.identity().project && !context.directory.is_empty(), "relay project mismatch");
        let host = Arc::new(RelayContext { context, backend });
        model_relay_http::RelayServer::start(binding, host.clone(), host).await.map(Self)
    }
    pub fn address(&self) -> std::net::SocketAddr { self.0.address() }
    pub async fn shutdown(self) -> anyhow::Result<()> { self.0.shutdown().await }
}
struct RelayContext {
    context: crate::project::instance::Context,
    backend: Arc<dyn RelayBackend>,
}
#[async_trait::async_trait]
impl model_relay_http::RelayHost for RelayContext {
    fn usage(&self, identity: &RelayIdentity, protocol: GenerationProtocol) -> anyhow::Result<agent_usage::relay::RequestUsage> {
        let scope = vw_api_transport::ProjectScope::new(identity.tenant.clone(), identity.project.clone().into())?;
        agent_usage::relay::RequestUsage::new(crate::usage_bridge::context::database()?, scope,
            crate::usage_bridge::context::local_host()?, "shiju-relay".into(), Some(identity.provider.clone()),
            Some(identity.model.clone()), Some(identity.session.clone()), protocol,
            Box::new(RelayUsageObserver(super::stream_observer::StreamObserver::new(protocol))))
    }
    fn error(&self, protocol: GenerationProtocol, status: StatusCode) -> Response {
        match protocol {
            GenerationProtocol::Gemini => super::gemini::error(status),
            GenerationProtocol::Responses => super::responses::error(status),
            GenerationProtocol::Messages => super::messages::error(status),
            _ => status.into_response(),
        }
    }
    async fn control(&self, identity: &RelayIdentity, kiro_target: &str, json: &serde_json::Value) -> Result<Response, StatusCode> {
        crate::session::ui_store::with_tenant_scope(
            identity.tenant.clone(),
            crate::project::instance::provide_context(self.context.clone(), async {
                let model = crate::provider::provider::get_model(
                    &identity.provider,
                    &identity.model,
                )
                .await
                .map_err(|_| StatusCode::FORBIDDEN)?;
                if model.status != "active" {
                    return Err(StatusCode::FORBIDDEN);
                }
                super::kiro::control(kiro_target, json, &identity.model)
                    .map(|value| axum::Json(value).into_response())
            }),
        )
        .await
    }
}
#[async_trait::async_trait]
impl RelayBackend for RelayContext {
    async fn execute(&self, identity: &RelayIdentity, request: RelayRequest, cancel: CancellationToken) -> Result<Response, StatusCode> {
        let RelayRequest { path, body } = request;
        crate::session::ui_store::with_tenant_scope(
            identity.tenant.clone(),
            crate::project::instance::provide_context(self.context.clone(), async {
                crate::provider::provider::get_model(
                    &identity.provider,
                    &identity.model,
                )
                .await
                .map_err(|_| StatusCode::FORBIDDEN)?;
                self.backend.execute(identity, RelayRequest { path, body }, cancel).await
            }),
        ).await
    }
}
struct RelayUsageObserver(super::stream_observer::StreamObserver);
impl agent_usage::relay::UsageStreamObserver for RelayUsageObserver {
    fn push(&mut self, bytes: &[u8]) -> anyhow::Result<u32> {
        self.0.push(bytes).map(|update| update.text_events).map_err(|_| anyhow::anyhow!("invalid usage stream"))
    }
    fn finish(&mut self) -> anyhow::Result<()> {
        self.0.finish().map(|_| ()).map_err(|_| anyhow::anyhow!("incomplete usage stream"))
    }
}
