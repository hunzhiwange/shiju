use super::*;
use serde_json::json;

#[tokio::test]
async fn fork_acp_empty_session_preserves_ephemeral_requests() {
    let context = crate::project::instance::Context {
        directory: "/workspace".into(),
        worktree: "/workspace".into(),
        project: serde_json::from_value(json!({
            "id":"project", "worktree":"/workspace", "time":{"created":0,"updated":0}, "sandboxes":[]
        })).unwrap(),
    };
    crate::project::instance::provide_context(context, async {
        assert!(ForkReplay::load("", "/workspace").await.unwrap().is_none());
    })
    .await;
}
