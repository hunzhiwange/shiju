use serde_json::json;
use vw_api_types::id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER, TenantScope};

use crate::client::test_support;

#[tokio::test]
async fn channel_qr_client_uses_authenticated_v1_paths() {
    let tenant = TenantScope::new("platform-qr", "company-qr").expect("tenant");
    let server = test_support::server_with_tenant_scope(
        vec![
            (
                200,
                json!({
                    "channel": "dingtalk",
                    "qrcode_png_base64": "cG5n",
                    "handle": "one-time-handle",
                    "expires_at": "2026-10-19T12:00:00Z"
                }),
            ),
            (200, json!({"status": "pending"})),
        ],
        tenant.clone(),
    );

    server.client().channel_qr_fetch("dingtalk").await.expect("fetch QR code");
    server.client().channel_qr_status("dingtalk", "one-time-handle").await.expect("poll QR status");

    let fetch = server.take_request();
    assert_eq!(fetch.method, "POST");
    assert_eq!(fetch.path, "/v1/channels/dingtalk/qrcode");
    assert_eq!(fetch.header(PLATFORM_ID_HEADER), Some(tenant.platform_id().as_ref()));
    assert_eq!(fetch.header(COMPANY_ID_HEADER), Some(tenant.company_id().as_ref()));

    let poll = server.take_request();
    assert_eq!(poll.method, "GET");
    assert_eq!(poll.path, "/v1/channels/dingtalk/qrcode/status?handle=one-time-handle");
    assert_eq!(poll.header(PLATFORM_ID_HEADER), Some(tenant.platform_id().as_ref()));
    assert_eq!(poll.header(COMPANY_ID_HEADER), Some(tenant.company_id().as_ref()));
    server.join();
}
