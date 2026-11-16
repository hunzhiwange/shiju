mod aggregation;
mod authorization;

async fn configure_with_policy(app: axum::Router, routing: &std::sync::Arc<app_routing_services::RoutingListener>, policy: std::path::PathBuf, database: agent_database::GlobalDatabase) -> axum::Router {
    let home = policy.parent().unwrap().to_owned();
    crate::server::lifecycle_api::configure_with_policy(app, routing, policy, database,
        crate::server::lifecycle_api::RoutingPaths { config: home.join(".config/shiju"), state: home.join(".local/state/shiju"), home },
        |database, policy, tenant, host| crate::host_migration_7_tests::fixture::Lifecycle(agent_gateway::aggregation_lifecycle::AggregationLifecycle { database, policy, tenant, host }),
    ).await
}

mod startup_policy;
