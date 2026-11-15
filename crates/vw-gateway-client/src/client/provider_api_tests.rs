//! 门面兼容请求测试。
use serde_json::json;
use crate::client::test_support;

#[tokio::test]
async fn provider_list_omits_directory_query_when_directory_is_none() {
    let server = test_support::server(vec![(
        200,
        json!({
            "all": [],
            "default": {},
            "connected": []
        }),
    )]);

    let response = server.client().provider_list(None).await.expect("provider list");

    assert!(response.all.is_empty());
    assert!(response.default.is_empty());
    assert!(response.connected.is_empty());
    let request = server.take_request();
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/v1/provider");
    server.join();
}
