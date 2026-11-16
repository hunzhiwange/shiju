use crate::fork_execution::*;

#[tokio::test]
async fn fork_execution_boundary_and_start_are_durable_and_scoped() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().to_path_buf();
    let tenant = vw_api_transport::TenantScope::default();
    let project = format!("fork-state-{}", uuid::Uuid::new_v4());
    for (session, ids) in
        [("empty", vec![]), ("history", vec!["u1".into(), "a1".into(), "u2".into(), "a2".into()])]
    {
        assert!(load_scoped(home.clone(), tenant.clone(), &project, session).await.unwrap().is_none());
        create(home.clone(), tenant.clone(), &project, session, ids.clone()).await.unwrap();
        let state = load_scoped(home.clone(), tenant.clone(), &project, session).await.unwrap().unwrap();
        assert_eq!(state.inherited_message_ids, ids);
        assert!(!state.started);
        let mut info: session_info::Info = serde_json::from_value(serde_json::json!({
            "id":session, "slug":"pending-first-response:turn-fork-test", "projectID":project,
            "directory":"/", "title":"fork", "version":"test", "time":{"created":1,"updated":1},
            "fork_execution_started":true
        }))
        .unwrap();
        crate::fork_execution::project(home.clone(), tenant.clone(), &mut info).await.unwrap();
        assert_eq!(info.fork_execution_started, Some(false));
        assert!(is_selectable(home.clone(), tenant.clone(), &project, session).await.unwrap());
        assert!(load_scoped(home.clone(), tenant.clone(), "other-project", session).await.unwrap().is_none());
        assert!(load_scoped(home.clone(), vw_api_transport::TenantScope::new("fork", "other").unwrap(), &project, session).await.unwrap().is_none());
        start(home.clone(), tenant.clone(), &project, session).await.unwrap();
        start(home.clone(), tenant.clone(), &project, session).await.unwrap();
        assert!(!is_selectable(home.clone(), tenant.clone(), &project, session).await.unwrap());
        crate::fork_execution::project(home.clone(), tenant.clone(), &mut info).await.unwrap();
        assert_eq!(info.fork_execution_started, Some(true));
        assert_eq!(
            load_scoped(home.clone(), tenant.clone(), &project, session).await.unwrap().unwrap().inherited_message_ids,
            ids
        );
        remove(home.clone(), tenant.clone(), &project, session).await.unwrap();
        assert!(load_scoped(home.clone(), tenant.clone(), &project, session).await.unwrap().is_none());
    }
    start(home.clone(), tenant.clone(), &project, "ordinary").await.unwrap();
    assert!(load_scoped(home.clone(), tenant.clone(), &project, "ordinary").await.unwrap().is_none());
}
