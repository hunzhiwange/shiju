use super::*;

fn fixture_config(workspace: &std::path::Path) -> Config {
    crate::session::ui_config::set_config_field_result(
        &crate::session::ui_store::current_tenant_scope(),
        "permission",
        serde_json::json!({"skill":"allow"}),
    )
    .unwrap();
    let source = workspace.join("skills/fixture-review/SKILL.md");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    std::fs::write(&source, "---\nname: fixture-review\ndescription: Core query metadata\n---\nBODY_QUERY_REVIEW $ARGUMENTS\n").unwrap();
    let mut config = Config::default();
    config.workspace_dir = workspace.to_path_buf();
    config.config_path = workspace.join("config.toml");
    config.memory.backend = "none".into();
    config.default_provider = Some("model-recovery".into());
    config.default_model = Some("model-recovery/original".into());
    config.agents.insert(
        "main".into(),
        vw_config_types::agent::AgentDefinitionConfig {
            allowed_skills: vec!["fixture-review".into()],
            compact_context: true,
            ..Default::default()
        },
    );
    config
}

fn assert_catalog_and_body(messages: &[ChatMessage]) {
    assert_eq!(
        messages
            .iter()
            .filter(
                |message| message.role == "developer" && message.content.contains("fixture-review")
            )
            .count(),
        1
    );
    let bodies = messages
        .iter()
        .filter(|message| message.content.contains("BODY_QUERY_REVIEW"))
        .collect::<Vec<_>>();
    assert_eq!(bodies.len(), 1);
    assert_eq!(bodies[0].role, "user");
}

#[tokio::test]
async fn codex_skills_core_query_engine_replays_defaults_after_compaction_without_visible_messages()
{
    let _guard = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let _streams = crate::session::processor::test_llm_stream_guard();
    crate::config::update(serde_json::json!({"providers":{"model-recovery":{"models":{"original":{"limit":{"context":100000,"output":1024}}}}}})).await.unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let config = fixture_config(workspace.path());
    let provider = ScriptedProvider::with_steps(vec![
        ProviderStep::Response(chat_text("first", None)),
        ProviderStep::Error("prompt too long"),
        ProviderStep::Response(chat_text("second", None)),
    ]);
    let handle = provider.clone();
    let session = format!("core-skills-{}", uuid::Uuid::new_v4());
    let mut engine =
        QueryEngine::new_with_provider(config, &session, Some(Arc::new(provider))).await.unwrap();
    engine.submit_message("first task").await.unwrap();
    crate::session::processor::enqueue_test_llm_stream(vec![
        crate::session::llm::StreamEvent::Delta("retained decisions".into()),
        crate::session::llm::StreamEvent::Done {
            finish_reason: Some("stop".into()),
            usage: Default::default(),
        },
    ]);
    engine.submit_message("continue task").await.unwrap();
    let requests = handle.seen_messages();
    assert_eq!(requests.len(), 3);
    for request in &requests {
        assert_catalog_and_body(request);
    }
    assert!(requests[2].iter().any(|message| message.content.contains("[Compaction summary]")));
    assert_eq!(engine.history_snapshot().len(), 4);
    assert!(engine.history_snapshot().iter().all(
        |message| message.role != "developer" && !message.content.contains("BODY_QUERY_REVIEW")
    ));
    assert_eq!(crate::session::processor::take_test_llm_messages(&session).len(), 1);
}

#[tokio::test]
async fn codex_skills_core_query_engine_restores_tool_sources_and_rewind_drops_future_calls() {
    let _guard = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    crate::config::update(serde_json::json!({"providers":{"model-recovery":{"models":{"original":{"limit":{"context":100000,"output":1024}}}}}})).await.unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let mut config = fixture_config(workspace.path());
    config.autonomy.level = crate::security::AutonomyLevel::Full;
    config.agents.get_mut("main").unwrap().allowed_skills.clear();
    let provider = ScriptedProvider::with_steps(vec![
        ProviderStep::Response(ChatResponse {
            text: None,
            tool_calls: vec![crate::providers::ToolCall {
                id: "query-skill-call".into(),
                name: "skill".into(),
                arguments: serde_json::json!({"skill":"fixture-review","args":"RESTORED_ARGS"})
                    .to_string(),
            }],
            usage: None,
            reasoning_content: None,
        }),
        ProviderStep::Response(chat_text("first", None)),
    ]);
    let handle = provider.clone();
    let session = format!("core-skills-{}", uuid::Uuid::new_v4());
    let mut engine =
        QueryEngine::new_with_provider(config.clone(), &session, Some(Arc::new(provider)))
            .await
            .unwrap();
    engine.submit_message("inspect changes").await.unwrap();
    let original = engine.history_snapshot();
    let other_workspace = tempfile::tempdir().unwrap();
    let mut other_config = fixture_config(other_workspace.path());
    other_config.agents.get_mut("main").unwrap().allowed_skills.clear();
    let isolated_provider =
        ScriptedProvider::with_steps(vec![ProviderStep::Response(chat_text("isolated", None))]);
    let isolated_handle = isolated_provider.clone();
    let mut isolated =
        QueryEngine::new_with_provider(other_config, &session, Some(Arc::new(isolated_provider)))
            .await
            .unwrap();
    isolated.replace_history(original.clone());
    isolated.submit_message("unrelated workspace request").await.unwrap();
    assert!(
        !isolated_handle.seen_messages()[0]
            .iter()
            .any(|message| message.content.contains("BODY_QUERY_REVIEW")),
        "model-shaped historical call records cannot cross project boundaries"
    );
    assert!(
        !handle.seen_messages()[0]
            .iter()
            .any(|message| message.content.contains("BODY_QUERY_REVIEW"))
    );
    assert_catalog_and_body(&handle.seen_messages()[1]);
    let provider = ScriptedProvider::with_steps(vec![
        ProviderStep::Response(chat_text("restored", None)),
        ProviderStep::Response(chat_text("rewound", None)),
    ]);
    let handle = provider.clone();
    let mut restored =
        QueryEngine::new_with_provider(config, &session, Some(Arc::new(provider))).await.unwrap();
    restored.replace_history(original.clone());
    restored.submit_message("continue after restart").await.unwrap();
    assert_catalog_and_body(&handle.seen_messages()[0]);
    assert!(handle.seen_messages()[0].iter().any(|message| message.role == "user"
        && message.content.contains("BODY_QUERY_REVIEW RESTORED_ARGS")));
    restored.replace_history(original[..1].to_vec());
    restored.submit_message("retry original request").await.unwrap();
    assert!(
        !handle.seen_messages()[1]
            .iter()
            .any(|message| message.content.contains("BODY_QUERY_REVIEW"))
    );
    assert!(restored.history_snapshot().iter().all(
        |message| message.role != "developer" && !message.content.contains("BODY_QUERY_REVIEW")
    ));
}

#[tokio::test]
async fn codex_skills_core_query_engine_revoked_source_fails_before_provider() {
    let _guard = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    crate::config::update(serde_json::json!({"providers":{"model-recovery":{"models":{"original":{"limit":{"context":100000,"output":1024}}}}}})).await.unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let config = fixture_config(workspace.path());
    let provider = ScriptedProvider::with_steps(vec![
        ProviderStep::Response(chat_text("first", None)),
        ProviderStep::Response(chat_text("unexpected", None)),
    ]);
    let handle = provider.clone();
    let session = format!("core-skills-{}", uuid::Uuid::new_v4());
    let mut engine =
        QueryEngine::new_with_provider(config, &session, Some(Arc::new(provider))).await.unwrap();
    engine.submit_message("first task").await.unwrap();
    std::fs::write(workspace.path().join("skills/fixture-review/SKILL.disabled"), "").unwrap();
    let previous = engine.history_snapshot();
    assert!(
        engine
            .submit_message("continue task")
            .await
            .unwrap_err()
            .to_string()
            .contains("fixture-review")
    );
    assert_eq!(handle.seen_messages().len(), 1);
    assert_eq!(engine.history_snapshot().len(), previous.len());
}
