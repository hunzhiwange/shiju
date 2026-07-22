use serde_json::json;

use vw_api_types::id::{
    COMPANY_ID_HEADER, PLATFORM_ID_HEADER, ProjectId, ProjectScope, TenantScope,
};
use vw_api_types::project::{
    ListProjectsRequest, ProjectStatus, ResolveProjectRequest, UpdateProjectRequest,
};
use vw_api_types::worktree::{
    CreateWorktreeRequest, DeleteWorktreeRequest, ResetMode, ResetWorktreeRequest,
};

use crate::client::test_support;

fn tenant_scope() -> TenantScope {
    TenantScope::default()
}

fn project_scope(project_id: &str) -> ProjectScope {
    ProjectScope::new(tenant_scope(), ProjectId::from(project_id)).expect("project scope")
}

#[test]
fn encode_path_segment_preserves_unreserved_and_encodes_utf8_bytes() {
    assert_eq!(super::encode_path_segment("AZaz09-._~"), "AZaz09-._~");
    assert_eq!(super::encode_path_segment("项目 /A?"), "%E9%A1%B9%E7%9B%AE%20%2FA%3F");
}

fn project_json() -> serde_json::Value {
    json!({
        "id": "proj_1",
        "name": "demo",
        "directory": "/work/demo",
        "display_path": "~/work/demo",
        "status": "ready",
        "created_at_ms": 1,
        "updated_at_ms": 2,
        "default_branch": null,
        "current_branch": null,
        "git": {
            "is_repo": true,
            "has_uncommitted_changes": false,
            "ahead": null,
            "behind": null
        },
        "active_worktree_id": null,
        "session_count": null
    })
}

fn worktree_json() -> serde_json::Value {
    json!({
        "id": "wt_1",
        "project_id": "proj_1",
        "name": "feature",
        "branch": "feature",
        "directory": "/work/demo-feature",
        "status": "ready",
        "created_at_ms": 1,
        "updated_at_ms": 2
    })
}

#[tokio::test]
async fn project_list_serializes_all_query_fields() {
    let server = test_support::server(vec![(
        200,
        json!({"items": [project_json()], "next_cursor": "next"}),
    )]);

    let request = ListProjectsRequest {
        cursor: Some("cur".to_string()),
        limit: Some(25),
        query: Some("demo".to_string()),
        status: Some(ProjectStatus::Indexing),
    };
    let response = server.client().project_list(&tenant_scope(), &request).await.expect("list");
    assert_eq!(response.items.len(), 1);
    assert_eq!(response.next_cursor.as_deref(), Some("next"));

    let recorded = server.take_request();
    assert_eq!(recorded.method, "GET");
    assert!(recorded.path.starts_with("/v1/projects?"));
    assert!(recorded.path.contains("cursor=cur"));
    assert!(recorded.path.contains("limit=25"));
    assert!(recorded.path.contains("query=demo"));
    // 状态被序列化为去掉引号的字符串形式。
    assert!(recorded.path.contains("status=indexing"));
    assert_eq!(recorded.header(PLATFORM_ID_HEADER), Some("10000"));
    assert_eq!(recorded.header(COMPANY_ID_HEADER), Some("100000000"));
    server.join();
}

#[tokio::test]
async fn project_list_omits_absent_query_fields() {
    let server = test_support::server(vec![(200, json!({"items": []}))]);

    let request = ListProjectsRequest { cursor: None, limit: None, query: None, status: None };
    let response = server.client().project_list(&tenant_scope(), &request).await.expect("list");
    assert!(response.items.is_empty());

    let recorded = server.take_request();
    assert_eq!(recorded.path, "/v1/projects");
    server.join();
}

#[tokio::test]
async fn project_resolve_posts_request_body() {
    let server = test_support::server(vec![(200, json!({"project": project_json()}))]);

    let request =
        ResolveProjectRequest { directory: "/work/demo".to_string(), create_if_missing: true };
    let response =
        server.client().project_resolve(&tenant_scope(), &request).await.expect("resolve");
    assert_eq!(response.project.id.0, "proj_1");

    let recorded = server.take_request();
    assert_eq!(recorded.method, "POST");
    assert_eq!(recorded.path, "/v1/projects/resolve");
    assert_eq!(recorded.body["directory"], "/work/demo");
    assert_eq!(recorded.body["create_if_missing"], true);
    server.join();
}

#[tokio::test]
async fn project_get_reads_single_project() {
    let server = test_support::server(vec![(200, json!({"project": project_json()}))]);

    let response = server.client().project_get(&project_scope("proj_1")).await.expect("get");
    assert_eq!(response.project.name, "demo");

    let recorded = server.take_request();
    assert_eq!(recorded.method, "GET");
    assert_eq!(recorded.path, "/v1/projects/proj_1");
    server.join();
}

#[tokio::test]
async fn project_change_records_passes_directory_query() {
    let server =
        test_support::server(vec![(200, json!({"items": [{"path": "a.txt", "patch": "@@"}]}))]);

    let response = server
        .client()
        .project_change_records(&project_scope("proj_1"), "/work/demo")
        .await
        .expect("records");
    assert_eq!(response.items.len(), 1);
    assert_eq!(response.items[0].path, "a.txt");

    let recorded = server.take_request();
    assert_eq!(recorded.method, "GET");
    assert!(recorded.path.starts_with("/v1/projects/proj_1/change-records?"));
    assert!(
        recorded.path.contains("directory=%2Fwork%2Fdemo")
            || recorded.path.contains("directory=/work/demo")
    );
    server.join();
}

#[tokio::test]
async fn project_update_uses_patch_semantics() {
    let server = test_support::server(vec![(200, json!({"project": project_json()}))]);

    let request = UpdateProjectRequest {
        name: Some("renamed".to_string()),
        active_worktree_id: None,
        icon: None,
        commands: None,
    };
    let response =
        server.client().project_update(&project_scope("proj_1"), &request).await.expect("update");
    assert_eq!(response.project.id.0, "proj_1");

    let recorded = server.take_request();
    assert_eq!(recorded.method, "PATCH");
    assert_eq!(recorded.path, "/v1/projects/proj_1");
    assert_eq!(recorded.body["name"], "renamed");
    server.join();
}

#[tokio::test]
async fn project_worktrees_lists_under_project() {
    let server = test_support::server(vec![(200, json!({"items": [worktree_json()]}))]);

    let response =
        server.client().project_worktrees(&project_scope("proj_1")).await.expect("worktrees");
    assert_eq!(response.items.len(), 1);
    assert_eq!(response.items[0].id.0, "wt_1");

    let recorded = server.take_request();
    assert_eq!(recorded.method, "GET");
    assert_eq!(recorded.path, "/v1/projects/proj_1/worktrees");
    server.join();
}

#[tokio::test]
async fn project_worktree_create_posts_request_body() {
    let server = test_support::server(vec![(201, json!({"worktree": worktree_json()}))]);

    let request = CreateWorktreeRequest {
        name: "feature".to_string(),
        branch: "feature".to_string(),
        from_ref: Some("main".to_string()),
        checkout: true,
    };
    let response = server
        .client()
        .project_worktree_create(&project_scope("proj_1"), &request)
        .await
        .expect("create");
    assert_eq!(response.worktree.id.0, "wt_1");

    let recorded = server.take_request();
    assert_eq!(recorded.method, "POST");
    assert_eq!(recorded.path, "/v1/projects/proj_1/worktrees");
    assert_eq!(recorded.body["name"], "feature");
    assert_eq!(recorded.body["from_ref"], "main");
    server.join();
}

#[tokio::test]
async fn worktree_get_reads_single_worktree() {
    let server = test_support::server(vec![(200, json!({"worktree": worktree_json()}))]);

    let response =
        server.client().worktree_get(&project_scope("proj_1"), "wt_1").await.expect("get");
    assert_eq!(response.worktree.branch, "feature");

    let recorded = server.take_request();
    assert_eq!(recorded.method, "GET");
    assert_eq!(recorded.path, "/v1/projects/proj_1/worktrees/wt_1");
    server.join();
}

#[tokio::test]
async fn worktree_delete_sends_force_flag() {
    let server = test_support::server(vec![(200, json!({"ok": true}))]);

    let request = DeleteWorktreeRequest { force: true };
    let ack = server
        .client()
        .worktree_delete(&project_scope("proj_1"), "wt_1", &request)
        .await
        .expect("delete");
    assert!(ack.ok);

    let recorded = server.take_request();
    assert_eq!(recorded.method, "DELETE");
    assert_eq!(recorded.path, "/v1/projects/proj_1/worktrees/wt_1");
    assert_eq!(recorded.body["force"], true);
    server.join();
}

#[tokio::test]
async fn worktree_reset_posts_mode_and_target() {
    let server = test_support::server(vec![(200, json!({"ok": true, "message": "done"}))]);

    let request =
        ResetWorktreeRequest { mode: ResetMode::Hard, target_ref: Some("origin/main".to_string()) };
    let ack = server
        .client()
        .worktree_reset(&project_scope("proj_1"), "wt_1", &request)
        .await
        .expect("reset");
    assert!(ack.ok);
    assert_eq!(ack.message.as_deref(), Some("done"));

    let recorded = server.take_request();
    assert_eq!(recorded.method, "POST");
    assert_eq!(recorded.path, "/v1/projects/proj_1/worktrees/wt_1/reset");
    assert_eq!(recorded.body["mode"], "hard");
    assert_eq!(recorded.body["target_ref"], "origin/main");
    server.join();
}

#[tokio::test]
async fn project_get_propagates_error_status() {
    let server = test_support::server(vec![(404, json!({"error": "missing"}))]);

    let error = server.client().project_get(&project_scope("missing")).await.expect_err("error");
    assert!(error.contains("404"));

    let recorded = server.take_request();
    assert_eq!(recorded.path, "/v1/projects/missing");
    server.join();
}

#[tokio::test]
async fn project_api_rejects_scope_that_conflicts_with_gateway_endpoint() {
    let endpoint_tenant = TenantScope::new("platform-a", "company-a").expect("endpoint tenant");
    let request_tenant = TenantScope::new("platform-b", "company-b").expect("request tenant");
    let server = test_support::server_with_tenant_scope(Vec::new(), endpoint_tenant);

    let error = server
        .client()
        .project_list(
            &request_tenant,
            &ListProjectsRequest { cursor: None, limit: None, query: None, status: None },
        )
        .await
        .expect_err("conflicting tenant scope should be rejected before sending");

    assert!(error.contains("does not match gateway endpoint tenant scope"));
    server.join();
}
