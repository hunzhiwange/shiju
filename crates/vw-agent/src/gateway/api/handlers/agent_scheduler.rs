//! Real scheduler gateway executor binding and host acceptance fixtures.
use std::sync::Arc;
use axum::Router;
use crate::agent_scheduler::runner::gateway_executor::SchedulerGatewayHost;
type StoreBackedAgentSchedulerGateway = gateway_agent_scheduler::store_gateway::StoreBackedAgentSchedulerGateway<SchedulerGatewayHost>;
type SharedAgentSchedulerGateway = Arc<dyn gateway_agent_scheduler::AgentSchedulerGateway>;
pub(crate) fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    router_with_gateway(Arc::new(StoreBackedAgentSchedulerGateway::default()))
}
fn router_with_gateway<S: Clone + Send + Sync + 'static>(gateway: SharedAgentSchedulerGateway) -> Router<S> {
    let router = gateway_agent_scheduler::router_with_gateway(gateway);
    #[cfg(test)]
    let router = router.layer(axum::middleware::from_fn(acceptance::test_default_tenant_scope));
    router
}
// Store-backed scheduling, locks and projections belong to gateway-agent-scheduler.

#[cfg(test)]
#[path = "agent_scheduler_acceptance.rs"]
mod acceptance;
