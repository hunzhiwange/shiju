//! Failover HTTP contracts use the real routing lifecycle and storage.
mod agent_apps_high_availability_tests;
mod agent_apps_ha_acceptance_tests;
mod aggregation_resync_tests;

async fn configure_with_policy(app: axum::Router, routing: &std::sync::Arc<app_routing_services::RoutingListener>, policy: std::path::PathBuf, database: agent_database::GlobalDatabase) -> axum::Router {
    let home = policy.parent().unwrap().to_owned();
    crate::server::lifecycle_api::configure_with_policy(app, routing, policy, database,
        crate::server::lifecycle_api::RoutingPaths { config: home.join(".config/shiju"), state: home.join(".local/state/shiju"), home },
        |database, policy, tenant, host| crate::host_migration_7_tests::fixture::Lifecycle(agent_gateway::aggregation_lifecycle::AggregationLifecycle { database, policy, tenant, host }),
    ).await
}

async fn inject_publication(routing: Option<axum::Extension<crate::server::lifecycle_api::RoutingControl>>, mut request: axum::extract::Request, next: axum::middleware::Next) -> axum::response::Response {
    request.extensions_mut().insert(crate::server::provider_publication_host::control(routing.map(|axum::Extension(c)| c)));
    next.run(request).await
}
