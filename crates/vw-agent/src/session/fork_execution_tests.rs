//! Host tenant injection stays separate from durable domain state contracts.
#[tokio::test]
async fn fork_execution_uses_current_host_tenant_scope() {
    let tenant = vw_api_transport::TenantScope::new("fork", "host-injection").unwrap();
    super::super::ui_store::with_tenant_scope(tenant.clone(), async {
        let (_, actual) = super::scope();
        assert_eq!(actual, tenant);
    }).await;
}
