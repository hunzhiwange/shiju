//! Test-only composition: real scoped catalog, project storage and usage ledger.
//! Actual application task-scope wiring is covered by the retained host server tests.
use super::{RelayBinding, RelayIdentity, generation::GenerationProtocol};
use crate::{RelayBackend, RelayRequest};
use axum::{http::StatusCode, response::{IntoResponse, Response}};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;


pub struct RelayServer(crate::RelayServer);
impl RelayServer {
    pub async fn start(binding: Arc<RelayBinding>, backend: Arc<dyn RelayBackend>, context: agent_project::instance::Context) -> anyhow::Result<Self> {
        anyhow::ensure!(context.project.id == binding.identity().project && !context.directory.is_empty(), "relay project mismatch");
        let host = Arc::new(RelayContext { context, backend });
        crate::RelayServer::start(binding, host.clone(), host).await.map(Self)
    }
    pub fn address(&self) -> std::net::SocketAddr { self.0.address() }
    pub async fn shutdown(self) -> anyhow::Result<()> { self.0.shutdown().await }
}
struct RelayContext {
    context: agent_project::instance::Context,
    backend: Arc<dyn RelayBackend>,
}
#[async_trait::async_trait]
impl crate::RelayHost for RelayContext {
    fn usage(&self, identity: &RelayIdentity, protocol: GenerationProtocol) -> anyhow::Result<agent_usage::relay::RequestUsage> {
        let scope = vw_api_transport::ProjectScope::new(identity.tenant.clone(), identity.project.clone().into())?;
        agent_usage::relay::RequestUsage::new(agent_usage::context::database(&agent_paths::paths().home)?, scope,
            agent_usage::context::local_host()?, "shiju-relay".into(), Some(identity.provider.clone()),
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
        with_tenant_scope(
            identity.tenant.clone(),
            agent_project::instance::provide_context(self.context.clone(), async {
                let model = agent_provider::provider::get_model(
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
        with_tenant_scope(
            identity.tenant.clone(),
            agent_project::instance::provide_context(self.context.clone(), async {
                agent_provider::provider::get_model(
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

pub struct ProviderRelayBackend(model_transport::provider_transport::ProviderTransport);
impl ProviderRelayBackend {
    pub fn new() -> Result<Self, reqwest::Error> {
        model_transport::provider_transport::ProviderTransport::new().map(Self)
    }
}







#[async_trait::async_trait]
impl RelayBackend for ProviderRelayBackend {
    async fn execute(&self, identity: &RelayIdentity, request: RelayRequest, cancel: CancellationToken) -> Result<Response, StatusCode> {
        self.0.execute(identity, request.path, &request.body, cancel).await
    }
}
tokio::task_local! { static TENANT: vw_api_transport::TenantScope; }
fn tenant() -> vw_api_transport::TenantScope {
    TENANT.try_with(Clone::clone).unwrap_or_default()
}
async fn with_tenant_scope<T>(tenant: vw_api_transport::TenantScope, future: impl std::future::Future<Output = T>) -> T {
    TENANT.scope(tenant, future).await
}
pub(crate) fn global_async_env_lock() -> &'static tokio::sync::Mutex<()> {
    ensure_host();
    static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    &LOCK
}
fn ensure_host() {
    agent_provider::provider::install_host(agent_provider::provider::Host {
        current_tenant_scope: tenant,
        read_catalogs: |config| Box::pin(async move {
            agent_provider::model_catalog::read_with(&config, |deployment| {
                let database = agent_database::GlobalDatabase::for_home(&agent_paths::paths().home)
                    .map_err(|_| agent_config::runtime::Error::Invalid("application provider storage unavailable".into()))?;
                let tenant = tenant();
                let record = app_provider_services::AppProviderStore::preview_record(
                    &database, &tenant, deployment.app_id, &deployment.provider_id,
                ).map_err(agent_config::runtime::Error::Invalid)?;
                app_models::edits::read(&tenant, deployment, &record)
                    .map_err(agent_config::runtime::Error::Invalid)
            })
        }),
    });
}
