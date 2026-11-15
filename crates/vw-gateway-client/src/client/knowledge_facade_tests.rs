//! 知识客户端兼容门面保留租户上下文。
use crate::client::test_support;
use vw_api_transport::id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER};

#[tokio::test]
async fn knowledge_facade_preserves_status_route_and_scope() {
    let server = test_support::server(vec![(200, serde_json::json!({
        "full_text": true, "vector": false, "hybrid": false, "rerank": false, "notes": {}
    }))]);
    assert!(server.client().knowledge_status().await.expect("status").full_text);
    let request = server.take_request();
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/v1/knowledge/status");
    assert_eq!(request.header(PLATFORM_ID_HEADER), Some("10000"));
    assert_eq!(request.header(COMPANY_ID_HEADER), Some("100000000"));
    server.join();
}
