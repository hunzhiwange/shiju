//! 经 NativeSessionIo 验证回滚入口的宿主契约；局部协议测试见 src/revert_tests.rs。

use super::super::session as session_api;

fn session_info(revert: Option<session_api::RevertInfo>) -> session_api::Info {
    session_api::Info {
        id: "session-1".to_string(),
        slug: "session-1".to_string(),
        project_id: "project-1".to_string(),
        directory: "/tmp".to_string(),
        parent_id: None,
        fork_execution_started: None,
        summary: None,
        share: None,
        title: "Session".to_string(),
        version: "test".to_string(),
        time: session_api::TimeInfo { created: 1, updated: 1, compacting: None, archived: None },
        permission: None,
        revert,
    }
}

#[tokio::test]
async fn cleanup_without_revert_is_a_noop() {
    let session = session_info(None);

    super::cleanup(&session).await.unwrap();

    assert!(session.revert.is_none());
}

#[tokio::test]
async fn unrevert_rejects_missing_project_context_before_snapshot_access() {
    let error = super::unrevert("session-1").await.unwrap_err();

    assert!(matches!(error, super::Error::Session(session_api::Error::NoProjectContext)));
}
