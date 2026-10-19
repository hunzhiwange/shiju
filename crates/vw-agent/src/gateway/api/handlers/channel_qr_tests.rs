use axum::extract::{Extension, Path, Query};
use vw_api_types::id::TenantScope;

use super::{
    ChannelQrStatusQuery, channel_qr_fetch, channel_qr_status, seed_pending_handle_for_test,
};

#[tokio::test]
async fn channel_qr_rejects_unknown_and_cross_tenant_poll() {
    let tenant_a = TenantScope::new("platform-a", "company-a").expect("tenant A");
    let tenant_b = TenantScope::new("platform-b", "company-b").expect("tenant B");

    let unknown =
        channel_qr_fetch(Extension(tenant_a.clone()), Path("unknown-channel".to_string()))
            .await
            .expect_err("unknown channel must be rejected");
    assert_eq!(unknown.status.as_u16(), 400);

    let handle = seed_pending_handle_for_test(&tenant_a, "dingtalk");
    let cross_tenant = channel_qr_status(
        Extension(tenant_b),
        Path("dingtalk".to_string()),
        Query(ChannelQrStatusQuery { handle }),
    )
    .await
    .expect_err("a tenant must not poll another tenant's handle");
    assert_eq!(cross_tenant.status.as_u16(), 404);
}
