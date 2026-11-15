//! 门面会话兼容测试。
use serde_json::json;

#[tokio::test]
async fn gateway_first_response_reads_preview_for_the_original_id_without_a_title() {
    let server = crate::client::test_support::server(vec![
        (404, json!({"message": "not published"})),
        (
            200,
            json!({
                "id": "s1",
                "title": "",
                "updated_ms": 2,
                "message_count": 2,
                "call_count": 0,
                "last_content": "successful retry"
            }),
        ),
    ]);
    let client = server.client();
    assert!(client.session_preview_meta_get("s1", Some("/repo")).await.unwrap().is_none());
    let preview = client.session_preview_meta_get("s1", Some("/repo")).await.unwrap().unwrap();
    assert_eq!(preview.id, "s1");
    assert!(preview.title.is_empty());
    assert_eq!(preview.last_content.as_deref(), Some("successful retry"));
    for _ in 0..2 {
        let request = server.take_request();
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/v1/session/s1/preview?directory=%2Frepo");
    }
    server.join();
}
