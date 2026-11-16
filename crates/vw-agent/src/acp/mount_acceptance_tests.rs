//! Production CLI and native tenant boundary entry points.
use agent_acp::host_migration_53_acceptance_contracts as contracts;

#[test]
fn acp_server_mount_acceptance_production_stdio_tools_update_and_restart() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    contracts::acp_server_mount_acceptance_production_stdio_tools_update_and_restart(&root);
}
#[test]
fn acp_server_mount_acceptance_tenant_and_same_directory_session_isolation() {
    contracts::acp_server_mount_acceptance_tenant_and_same_directory_session_isolation();
}
#[tokio::test]
async fn isolation_worker() {
    contracts::isolation_worker::<super::backend::NativeAcpHost>().await;
}
