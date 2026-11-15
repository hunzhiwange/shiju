//! 配置领域拆分后的门面兼容性。
use serde_json::json;
use crate::client::test_support;

#[tokio::test]
async fn config_facade_preserves_global_path_requests() {
    let server = test_support::server(vec![(200, json!({"ui": {"theme": "dark"}})), (200, json!({}))]);
    assert_eq!(server.client().global_config_get_path(&["ui", "theme"]).await.unwrap(), Some(json!("dark")));
    server.client().global_config_patch_path(&["ui", "theme"], json!("light")).await.unwrap();
    assert_eq!(server.take_request().path, "/v1/global/config");
    let request = server.take_request();
    assert_eq!(request.method, "PATCH");
    assert_eq!(request.body, json!({"ui": {"theme": "light"}}));
    server.join();
}
