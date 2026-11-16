use super::*;

#[tokio::test]
async fn agent_binding_inherited_history_cannot_override_fork_selection() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let workspace = tempfile::tempdir().unwrap();
    crate::project::instance::provide(workspace.path(), None, move || {
        Box::pin(async move {
            let info = super::super::session::create_next(super::super::session::CreateInput {
                directory: crate::project::instance::directory(),
                ..Default::default()
            })
            .await
            .unwrap();
            let user: super::super::message::Info = serde_json::from_value(serde_json::json!({
                "role":"user", "id":"inherited-user", "sessionID":info.id,
                "time":{"created":1}, "agent":"acp-claude",
                "model":{"providerID":"model-recovery","modelID":"original"}
            }))
            .unwrap();
            super::super::message::write_message(&user).await.unwrap();
            let inherited = AgentBinding::from_definition(
                "acp-claude".into(),
                &crate::agent::get("acp-claude").await.unwrap(),
            );
            save_scoped(&info.project_id, &info.id, &inherited).await.unwrap();
            super::super::fork_execution::create(
                &info.project_id,
                &info.id,
                vec!["inherited-user".into()],
            )
            .await
            .unwrap();
            let path = std::path::PathBuf::from(&info.directory);
            for (agent, backend) in
                [("main", None), ("acp-codex", Some("codex")), ("acp-claude", Some("claude"))]
            {
                let (binding, _) =
                    prepare(&info.id, Some(agent), backend, Some("model-recovery/original"), &path)
                        .await
                        .unwrap();
                assert_eq!(binding.agent, agent);
            }
            super::super::fork_execution::start(&info.project_id, &info.id).await.unwrap();
            assert!(
                prepare(&info.id, Some("main"), None, Some("model-recovery/original"), &path)
                    .await
                    .err()
                    .expect("started fork rejects a new agent")
                    .to_string()
                    .contains("new session")
            );
            assert_eq!(load(&info.id).await.unwrap().unwrap().agent, "acp-claude");
        })
    })
    .await
    .unwrap();
}
