//! Host snapshot and publication adapters for gateway aggregation services.
use super::aggregation_resync::RecordPublicationContext;
use agent_gateway::publication_context::{PublicationContext, PublicationHost};
use axum::http::StatusCode;
use app_provider_services::AppProviderStore;
use vw_agent_apps::providers::live::{commit::*, record_publication::ProviderRecordChange};
use serde_json::Value;
pub(super) struct Host;
pub(super) fn frozen(context: &RecordPublicationContext) -> PublicationContext {
    PublicationContext {
        database: context.database.clone(),
        project: context.project.clone(),
        host: context.host.clone(),
        app: context.app.clone(),
        home: context.home.clone(),
        root: context.root.clone(),
        routing: context.routing.clone(),
        expected: context.expected.clone(),
        principal: context.principal.clone(),
        revision: context.revision.clone(),
        failure: context.failure.clone(),
    }
}
fn native(context: &PublicationContext) -> RecordPublicationContext {
    RecordPublicationContext {
        database: context.database.clone(),
        project: context.project.clone(),
        host: context.host.clone(),
        app: context.app.clone(),
        home: context.home.clone(),
        root: context.root.clone(),
        routing: context.routing.clone(),
        expected: context.expected.clone(),
        principal: context.principal.clone(),
        revision: context.revision.clone(),
        failure: context.failure.clone(),
    }
}
impl PublicationHost for Host {
    fn revision(&self, context: &PublicationContext, provider: Option<&str>) -> Result<String, StatusCode> {
        use super::api::handlers::aggregation_read_snapshot::{load, revision};
        let snapshot = load(&context.database, &context.project, &context.host, context.app,
            &context.home, &context.expected, provider)?;
        revision(&context.project, &context.host, context.app, &context.principal, &context.expected, &snapshot)
    }
    fn publish_aggregation(&self, context: &PublicationContext, default: &str,
        policy: vw_agent_apps::aggregation::codex_publish::CatalogPolicy, row: &Value) -> Result<bool, StatusCode> {
        native(context).publish_aggregation(default, policy, row)
    }
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn replace(context: &RecordPublicationContext, tx: &ProviderTransactions,
    store: &AppProviderStore, state: &AppState, candidate: &AppState, rows: &[(String, Value)],
    change: &ProviderRecordChange, target: PendingTarget, active: bool) -> Result<bool, StatusCode> {
    agent_gateway::aggregation_desktop_record::replace(&frozen(context), &Host, tx, store, state,
        candidate, rows, change, target, active)
}
