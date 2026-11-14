use super::{test_support, WorkspaceMemoryScope, WorkspaceMemoryError};
use serde_json::json;
use vw_api_types::{id::{ProjectId, TenantScope}, workspace_memory::WorkspaceFilename};

fn scope() -> WorkspaceMemoryScope {
    WorkspaceMemoryScope { project_id: ProjectId::new("project").unwrap(), target_host_id: "主机 /?".into() }
}

#[tokio::test]
async fn workspace_sdk_preserves_scope_files_and_empty_content() {
    let server = test_support::server_with_tenant_scope(vec![
        (200, json!(null)), (200, json!("")), (204, json!(null)),
        (200, json!([])), (200, json!("中文😀")), (204, json!(null)), (204, json!(null)),
    ], TenantScope::new("platform", "company").unwrap());
    let c = server.client();
    let s = scope();
    let date = "2026-11-14.md".parse().unwrap();
    assert!(c.read_workspace_file(&s, WorkspaceFilename::Agents, "grant").await.unwrap().0.is_none());
    assert_eq!(c.read_workspace_file(&s, WorkspaceFilename::Soul, "grant").await.unwrap().0.as_deref(), Some(""));
    c.write_workspace_file(&s, WorkspaceFilename::Memory, "中文😀".into(), "grant").await.unwrap();
    assert!(c.list_daily_memory_files(&s, "grant").await.unwrap().is_empty());
    assert_eq!(c.read_daily_memory_file(&s, &date, "grant").await.unwrap().0.as_deref(), Some("中文😀"));
    c.write_daily_memory_file(&s, &date, "".into(), "grant").await.unwrap();
    c.delete_daily_memory_file(&s, &date, "grant").await.unwrap();
    for (method, path, body) in [
        ("GET", "files/AGENTS.md", json!(null)), ("GET", "files/SOUL.md", json!(null)),
        ("PUT", "files/MEMORY.md", json!({"content":"中文😀"})), ("GET", "daily", json!(null)),
        ("GET", "daily/2026-11-14.md", json!(null)), ("PUT", "daily/2026-11-14.md", json!({"content":""})),
        ("DELETE", "daily/2026-11-14.md", json!(null)),
    ] {
        let r = server.take_request();
        assert_eq!(r.method, method);
        assert_eq!(r.path, format!("/v1/agent-apps/openclaw/workspace/{path}?projectId=project&targetHostId=%E4%B8%BB%E6%9C%BA+%2F%3F"));
        assert_eq!(r.header("x-platform-id"), Some("platform"));
        assert_eq!(r.header("x-company-id"), Some("company"));
        assert_eq!(r.header("x-agent-apps-token"), Some("grant"));
        assert_eq!(r.body, body);
    }
    server.join();
}

#[tokio::test]
async fn workspace_sdk_preserves_scope_errors_and_cancel_stops_polling() {
    let progress = json!({"operationId":"search-1","status":"running","phase":"executing","results":null});
    let server = test_support::server(vec![(200, progress.clone()), (200, progress), (403, json!(null))]);
    let (mut search, initial) = server.client().search_daily_memory_files(&scope(), "中文 /?&+".into(), "grant").await.unwrap();
    assert!(initial.results.is_none());
    assert!(search.poll("grant").await.unwrap().results.is_none());
    assert_eq!(search.cancel("grant").await, Err(WorkspaceMemoryError::Http(403)));
    assert!(matches!(search.poll("grant").await, Err(WorkspaceMemoryError::Cancelled)));
    for (method, suffix) in [("POST", "search"), ("GET", "search/search-1"), ("DELETE", "search/search-1")] {
        let r = server.take_request();
        assert_eq!(r.method, method);
        assert!(r.path.starts_with(&format!("/v1/agent-apps/openclaw/workspace/{suffix}?")));
        assert_eq!(r.header("x-agent-apps-token"), Some("grant"));
        if method == "POST" { assert_eq!(r.body, json!({"query":"中文 /?&+"})); }
    }
    server.join();
    for status in [401, 403, 404, 409, 422, 500, 503] {
        let server = test_support::server(vec![(status, json!(null))]);
        assert!(matches!(server.client().list_daily_memory_files(&scope(), "grant").await, Err(WorkspaceMemoryError::Http(s)) if s == status));
        server.join();
    }
    let server = test_support::server(vec![(200, json!({"invalid":"private text"}))]);
    assert!(matches!(server.client().list_daily_memory_files(&scope(), "grant").await, Err(WorkspaceMemoryError::Decode)));
    server.join();
}
