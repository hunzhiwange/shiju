//! Snapshot adapter for provider route recovery.
use agent_gateway::publication_context::{PublicationContext, PublicationHost};
use axum::http::StatusCode;
pub(super) struct Host;
impl PublicationHost for Host {
    fn revision(&self, context: &PublicationContext, provider: Option<&str>) -> Result<String, StatusCode> {
        use super::api::handlers::aggregation_read_snapshot::{load, revision};
        let snapshot = load(&context.database, &context.project, &context.host, context.app,
            &context.home, &context.expected, provider)?;
        revision(&context.project, &context.host, context.app, &context.principal, &context.expected, &snapshot)
    }
}
