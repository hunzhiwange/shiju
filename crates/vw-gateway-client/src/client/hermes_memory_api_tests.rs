use super::{GatewayClient, HermesMemoryError, HermesMemoryScope, test_support};
use serde_json::json;
use vw_api_types::{hermes_memory::HermesMemoryKind as Kind, id::{ProjectId, TenantScope}};

fn scope() -> HermesMemoryScope {
    HermesMemoryScope { project_id: ProjectId::new("project").unwrap(), target_host_id: "主机 /?".into() }
}

#[tokio::test]
async fn hermes_sdk_preserves_outcome_scope_and_write_results() {
    let limits = json!({"memory":2200,"user":1375,"memoryEnabled":false,"userEnabled":true});
    let server = test_support::server_with_tenant_scope(vec![
        (200, json!({"kind":"memory","content":"","exists":false})),
        (200, json!({"kind":"user","content":"","exists":true})),
        (200, json!({"backupPath":"backup/config.yaml"})),
        (200, json!({})),
        (200, limits),
        (200, json!(null)),
        (200, json!({"default":"model","provider":"provider","contextLength":4096})),
    ], TenantScope::new("platform", "company").unwrap());
    let client = server.client();
    let s = scope();
    let absent = client.hermes_memory(&s, Kind::Memory, "grant").await.unwrap();
    assert!(!absent.exists);
    assert!(absent.content.is_empty());
    assert!(client.hermes_memory(&s, Kind::User, "grant").await.unwrap().exists);
    let content = "中文😀".repeat(3000);
    let saved = client.set_hermes_memory(&s, Kind::Memory, content.clone(), "grant").await.unwrap();
    assert_eq!(saved.backup_path.as_deref(), Some("backup/config.yaml"));
    assert_eq!(client.set_hermes_memory_enabled(&s, Kind::User, true, "grant").await.unwrap().backup_path, None);
    let refreshed = client.hermes_memory_limits(&s, "grant").await.unwrap();
    assert_eq!(refreshed.memory, 2200);
    assert!(!refreshed.memory_enabled);
    assert!(refreshed.user_enabled);
    assert_eq!(client.hermes_model(&s, "grant").await.unwrap(), None);
    assert_eq!(client.hermes_model(&s, "grant").await.unwrap().unwrap().default.as_deref(), Some("model"));
    for (method, path, body) in [
        ("GET", "memory/memory", json!(null)),
        ("GET", "memory/user", json!(null)),
        ("PUT", "memory/memory", json!({"content":content})),
        ("PUT", "memory/user/enabled", json!({"enabled":true})),
        ("GET", "memory-limits", json!(null)),
        ("GET", "model", json!(null)),
        ("GET", "model", json!(null)),
    ] {
        let request = server.take_request();
        assert_eq!(request.method, method);
        assert_eq!(request.path, format!("/v1/agent-apps/hermes/{path}?projectId=project&targetHostId=%E4%B8%BB%E6%9C%BA+%2F%3F"));
        assert_eq!(request.header("x-platform-id"), Some("platform"));
        assert_eq!(request.header("x-company-id"), Some("company"));
        assert_eq!(request.header("x-agent-apps-token"), Some("grant"));
        assert_eq!(request.body, body);
    }
    server.join();
}

#[tokio::test]
async fn hermes_sdk_preserves_outcome_rejections_and_invalid_response() {
    for status in [401, 403, 404, 409, 422, 500, 503] {
        let server = test_support::server_raw(vec![(status, String::new()), (status, String::new())]);
        assert_eq!(server.client().set_hermes_memory(&scope(), Kind::Memory, "draft".into(), "grant").await, Err(HermesMemoryError::Http(status)));
        assert_eq!(server.client().set_hermes_memory_enabled(&scope(), Kind::User, false, "grant").await, Err(HermesMemoryError::Http(status)));
        server.join();
    }
    let server = test_support::server(vec![(200, json!({"unexpected":"private text"}))]);
    assert_eq!(server.client().hermes_memory(&scope(), Kind::Memory, "grant").await, Err(HermesMemoryError::Decode));
    server.join();
}

#[tokio::test]
async fn hermes_sdk_preserves_outcome_invalid_credentials_and_transport() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let client = GatewayClient::new(crate::endpoint::GatewayEndpoint::new("127.0.0.1", port)).unwrap();
    for token in ["", "bad\ncredential"] {
        assert_eq!(client.hermes_memory_limits(&scope(), token).await, Err(HermesMemoryError::InvalidInput));
    }
    let mut s = scope();
    s.target_host_id.clear();
    assert_eq!(client.hermes_memory_limits(&s, "grant").await, Err(HermesMemoryError::InvalidInput));
    assert_eq!(client.hermes_memory_limits(&scope(), "grant").await, Err(HermesMemoryError::Transport));
}
