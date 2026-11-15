use super::test_support;
use serde_json::json;

#[tokio::test]
async fn app_read_facade_forwards_catalog_and_icons() {
    let server = test_support::server(vec![(200, json!({"apps":[]})), (200, json!([]))]);
    assert!(server.client().agent_apps_catalog("host").await.unwrap().apps.is_empty());
    assert!(server.client().custom_icons_list().await.unwrap().is_empty());
    assert_eq!(server.take_request().path, "/v1/agent-apps/catalog?targetHostId=host");
    assert_eq!(server.take_request().path, "/v1/agent-apps/custom-icons");
    server.join();
}
