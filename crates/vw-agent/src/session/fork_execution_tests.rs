use super::*;

#[tokio::test]
async fn fork_execution_boundary_and_start_are_durable_and_scoped() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let project = format!("fork-state-{}", uuid::Uuid::new_v4());
    for (session, ids) in
        [("empty", vec![]), ("history", vec!["u1".into(), "a1".into(), "u2".into(), "a2".into()])]
    {
        assert!(load_scoped(&project, session).await.unwrap().is_none());
        create(&project, session, ids.clone()).await.unwrap();
        let state = load_scoped(&project, session).await.unwrap().unwrap();
        assert_eq!(state.inherited_message_ids, ids);
        assert!(!state.started);
        let mut info: super::super::session::Info = serde_json::from_value(serde_json::json!({
            "id":session, "slug":"pending-first-response:turn-fork-test", "projectID":project,
            "directory":"/", "title":"fork", "version":"test", "time":{"created":1,"updated":1},
            "fork_execution_started":true
        }))
        .unwrap();
        super::project(&mut info).await.unwrap();
        assert_eq!(info.fork_execution_started, Some(false));
        assert!(is_selectable(&project, session).await.unwrap());
        assert!(load_scoped("other-project", session).await.unwrap().is_none());
        super::super::ui_store::with_tenant_scope(
            vw_api_types::id::TenantScope::new("fork", "other").unwrap(),
            async {
                assert!(load_scoped(&project, session).await.unwrap().is_none());
            },
        )
        .await;
        start(&project, session).await.unwrap();
        start(&project, session).await.unwrap();
        assert!(!is_selectable(&project, session).await.unwrap());
        super::project(&mut info).await.unwrap();
        assert_eq!(info.fork_execution_started, Some(true));
        assert_eq!(
            load_scoped(&project, session).await.unwrap().unwrap().inherited_message_ids,
            ids
        );
        remove(&project, session).await.unwrap();
        assert!(load_scoped(&project, session).await.unwrap().is_none());
    }
    start(&project, "ordinary").await.unwrap();
    assert!(load_scoped(&project, "ordinary").await.unwrap().is_none());
}
