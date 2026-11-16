//! Native home and authorized catalog-read ports.
use agent_acp::host_migration_53_catalog_contracts as contracts;
struct NativeCatalogPorts;
#[async_trait::async_trait]
impl contracts::CatalogPorts for NativeCatalogPorts {
    type Host = super::backend::NativeAcpHost;
    fn home() -> &'static std::path::Path { &crate::global::paths().home }
    async fn read_catalog_mounts(project: &vw_api_transport::ProjectScope, id: &str) -> Result<agent_acp::catalog_mounts::CatalogMountSnapshot, acp_server::server::BackendError> {
        super::read_catalog_mounts(project, id).await
    }
}
#[tokio::test]
async fn cc_mcp_t29_read_mounts_checks_exact_identity_and_exposes_only_committed_references() {
    contracts::cc_mcp_t29_read_mounts_checks_exact_identity_and_exposes_only_committed_references::<NativeCatalogPorts>().await;
}
