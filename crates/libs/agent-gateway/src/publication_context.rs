//! Frozen authorized publication inputs; live state remains with routing/storage owners.
use axum::http::StatusCode;
use vw_agent_apps::providers::live::{commit::*, write_guard::WriteError};
pub use crate::aggregation_resync::RecordPublicationContext as PublicationContext;
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
