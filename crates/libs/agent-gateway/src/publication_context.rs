//! Frozen authorized publication inputs; live state remains with routing/storage owners.
use agent_database::GlobalDatabase;
use app_routing_services::RoutingListener;
use axum::http::StatusCode;
use std::{path::PathBuf, sync::Arc};
use vw_api_transport::id::ProjectScope;
use vw_api_app_identity::AppId;
use vw_agent_apps::providers::live::{commit::*, write_guard::WriteError};
#[derive(Clone)]
pub struct PublicationContext {
    pub database: GlobalDatabase,
    pub project: ProjectScope,
    pub host: String,
    pub app: AppId,
    pub home: PathBuf,
    pub root: PathBuf,
    pub routing: Option<Arc<RoutingListener>>,
    pub expected: AppState,
    pub principal: String,
    pub revision: String,
    pub failure: Arc<std::sync::Mutex<Option<StatusCode>>>,
}
pub trait PublicationHost {
    fn revision(&self, context: &PublicationContext, provider: Option<&str>) -> Result<String, StatusCode>;
    fn publish_aggregation(&self, context: &PublicationContext, default: &str,
        policy: vw_agent_apps::aggregation::codex_publish::CatalogPolicy, row: &serde_json::Value) -> Result<bool, StatusCode>;
}
impl PublicationContext {
    pub fn validate_inputs(&self, host: &impl PublicationHost) -> Result<(), WriteError> {
        let tx = ProviderTransactions::open_existing(&self.root, ExecutionHostId::new(&self.host)?)?
            .ok_or(WriteError::Conflict)?;
        if tx.state(self.app)? != self.expected { return Err(WriteError::Conflict); }
        if host.revision(self, None).map_err(|_| WriteError::Conflict)? != self.revision {
            return Err(WriteError::Conflict);
        }
        Ok(())
    }
}
