use super::*;
use crate::client::test_support;
use serde_json::json;
use vw_api_types::id::{PLATFORM_ID_HEADER, ProjectId, ProjectScope, TenantScope};
#[tokio::test]
async fn worktree_management_encodes_requests() {
    let server = test_support::server(vec![
        (200, json!({"items":[]})),
        (
            200,
            json!({"fingerprint":"token", "expires_at_ms":12,"confirmation_required":true,"has_changes":true,"has_ignored_files":false,"protection_reasons":[]}),
        ),
        (200, json!({"snapshot_id":"snapshot"})),
        (200, json!({"items":[]})),
        (200, json!({"ok":true,"message":null})),
        (200, json!({"candidates":[],"protected":[],"remaining_excess":0})),
        (409, json!({"error":"refresh required"})),
    ]);
    let scope = ProjectScope::for_default_tenant(ProjectId::from("项目 /?")).unwrap();
    let client = server.client();
    client.managed_worktrees(&scope).await.unwrap();
    client.managed_worktree_deletion_preflight(&scope, "tree /?").await.unwrap();
    client
        .managed_worktree_delete(
            &scope,
            "tree /?",
            &DeleteManagedWorktreeRequest {
                fingerprint: "token".into(),
                confirmed: true,
                confirm_ignored_loss: true,
            },
        )
        .await
        .unwrap();
    client.worktree_snapshots(&scope).await.unwrap();
    client.managed_worktree_retry(&scope, "tree /?").await.unwrap();
    client.worktree_cleanup_proposal(&scope).await.unwrap();
    assert!(
        client.worktree_snapshot_restore(&scope, "snapshot /?").await.unwrap_err().contains("409")
    );
    let base = "/v1/projects/%E9%A1%B9%E7%9B%AE%20%2F%3F";
    let request = server.take_request();
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, format!("{base}/managed-worktrees"));
    assert_eq!(request.header(PLATFORM_ID_HEADER), Some("10000"));
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, format!("{base}/managed-worktrees/tree%20%2F%3F/deletion-preflight"));
    let request = server.take_request();
    assert_eq!(request.method, "DELETE");
    assert_eq!(
        request.body,
        json!({"fingerprint":"token","confirmed":true,"confirm_ignored_loss":true})
    );
    assert_eq!(server.take_request().path, format!("{base}/worktree-snapshots"));
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert!(request.path.ends_with("/retry-initialization"));
    assert_eq!(server.take_request().path, format!("{base}/worktree-cleanup-proposal"));
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, format!("{base}/worktree-snapshots/snapshot%20%2F%3F/restore"));
    server.join();
}
#[tokio::test]
async fn worktree_management_rejects_tenant_mismatch_before_network() {
    let server = test_support::server(vec![]);
    let scope =
        ProjectScope::new(TenantScope::new("foreign", "tenant").unwrap(), ProjectId::from("p"))
            .unwrap();
    assert!(server.client().managed_worktrees(&scope).await.unwrap_err().contains("tenant scope"));
    assert!(
        server
            .client()
            .managed_worktree_delete(&scope, "tree", &Default::default())
            .await
            .unwrap_err()
            .contains("tenant scope")
    );
    server.join();
}

#[tokio::test]
async fn worktree_managed_ui_record_use_encodes_scoped_identity() {
    let server = test_support::server(vec![(200, json!({"ok":true}))]);
    let scope = ProjectScope::for_default_tenant(ProjectId::from("项目 /?")).unwrap();
    server.client().managed_worktree_record_use(&scope, "checkout /?").await.unwrap();
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(
        request.path,
        "/v1/projects/%E9%A1%B9%E7%9B%AE%20%2F%3F/managed-worktrees/checkout%20%2F%3F/record-use"
    );
    assert_eq!(request.body, json!({}));
    assert_eq!(request.header(PLATFORM_ID_HEADER), Some("10000"));
    server.join();
}
