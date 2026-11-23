use crate::host_migration_14_tests::agent_apps_ha_acceptance_tests::exercise_failover;

#[tokio::test]
async fn local_routing_failover_flow() -> anyhow::Result<()> {
    exercise_failover(
        true,
        concat!(module_path!(), "::local_routing_failover_flow")
            .strip_prefix("gateway_app_routing::")
            .unwrap(),
    )
    .await
}
