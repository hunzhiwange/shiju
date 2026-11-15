//! PTY 门面的类型与错误转发契约。
use super::{GatewayClient, PtyCreateRequest, PtyInfoDto, PtyUpdateRequest};

#[tokio::test]
async fn pty_facade_preserves_domain_types_and_empty_id_errors() {
    let client = GatewayClient::new(crate::GatewayEndpoint::new("127.0.0.1", 1))
        .expect("client");
    let _: gateway_pty::PtyCreateRequest = PtyCreateRequest::default();
    let update: gateway_pty::PtyUpdateRequest = PtyUpdateRequest::default();
    let result: Result<PtyInfoDto, String> = client.pty_get("  ", None).await;
    assert_eq!(result.unwrap_err(), "PTY id must not be empty");
    assert_eq!(client.pty_update("", &update, None).await.unwrap_err(), "PTY id must not be empty");
    assert_eq!(client.pty_delete("", None).await.unwrap_err(), "PTY id must not be empty");
    match client.pty_connect("", None, None).await {
        Ok(_) => panic!("empty PTY id must be rejected"),
        Err(error) => assert_eq!(error, "PTY id must not be empty"),
    }
}
