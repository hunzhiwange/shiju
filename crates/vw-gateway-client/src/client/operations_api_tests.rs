//! 集成、记忆与诊断运维接口的线协议测试。

use serde_json::json;

use crate::client::test_support;

#[tokio::test]
async fn operations_api_uses_documented_routes() {
    let server = test_support::server(vec![
        (200, json!({"integrations": []})),
        (200, json!({"revision": "r1", "integrations": []})),
        (200, json!({"status": "ok", "revision": "r2"})),
        (200, json!({"entries": []})),
        (200, json!({"status": "ok"})),
        (200, json!({"status": "ok", "deleted": true})),
        (200, json!({"results": [], "summary": {"ok": 0, "warnings": 0, "errors": 0}})),
        (200, json!({"results": [], "summary": {"ok": 0, "warnings": 0, "errors": 0}})),
    ]);

    server.client().integrations_get().await.expect("integrations");
    server.client().integration_settings_get().await.expect("settings");
    server
        .client()
        .integration_credentials_put(
            "slack /团队",
            &json!({"revision": "r1", "fields": {"token": "secret"}}),
        )
        .await
        .expect("credentials");
    server
        .client()
        .memory_query_get(Some("find this"), Some("core"))
        .await
        .expect("memory query");
    server
        .client()
        .memory_store(&json!({"key": "one", "content": "remember", "category": "core"}))
        .await
        .expect("memory store");
    server.client().memory_delete("memory /一").await.expect("memory delete");
    server.client().doctor_get().await.expect("doctor get");
    server.client().doctor_post().await.expect("doctor post");

    let expected = [
        ("GET", "/v1/integrations", json!(null)),
        ("GET", "/v1/integrations/settings", json!(null)),
        (
            "PUT",
            "/v1/integrations/slack%20%2F%E5%9B%A2%E9%98%9F/credentials",
            json!({"revision": "r1", "fields": {"token": "secret"}}),
        ),
        ("GET", "/v1/memory?query=find+this&category=core", json!(null)),
        (
            "POST",
            "/v1/memory",
            json!({"key": "one", "content": "remember", "category": "core"}),
        ),
        ("DELETE", "/v1/memory/memory%20%2F%E4%B8%80", json!(null)),
        ("GET", "/v1/doctor", json!(null)),
        ("POST", "/v1/doctor", json!({})),
    ];
    for (method, path, body) in expected {
        let request = server.take_request();
        assert_eq!(request.method, method);
        assert_eq!(request.path, path);
        assert_eq!(request.body, body);
    }
    server.join();
}
