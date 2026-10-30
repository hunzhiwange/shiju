use super::*;

fn skill_fixture() -> (tempfile::TempDir, crate::config::Config, crate::skills::Skill) {
    crate::session::ui_config::set_config_field_result(
        &crate::session::ui_store::current_tenant_scope(),
        "permission",
        serde_json::json!({"skill":"allow"}),
    )
    .unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let source = workspace.path().join("skills/fixture-review/SKILL.md");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    std::fs::write(
        &source,
        "---\nname: fixture-review\ndescription: Review fixture metadata\n---\nBODY_CORE_REVIEW $ARGUMENTS\n",
    )
    .unwrap();
    let mut config = crate::config::Config::default();
    config.workspace_dir = workspace.path().to_path_buf();
    config.config_path = workspace.path().join("config.toml");
    config.agents.insert(
        "main".into(),
        AgentDefinitionConfig {
            allowed_skills: vec!["fixture-review".into()],
            ..Default::default()
        },
    );
    let mut skill: crate::skills::Skill = serde_json::from_value(serde_json::json!({
        "name":"fixture-review", "description":"Review fixture metadata", "version":"1.0"
    }))
    .unwrap();
    skill.location = Some(source);
    (workspace, config, skill)
}

fn skill_agent(
    provider: Box<dyn Provider>,
    config: &crate::config::Config,
    skill: crate::skills::Skill,
) -> Agent {
    let security = Arc::new(crate::security::SecurityPolicy::from_configs(
        &config.autonomy,
        &config.security,
        &config.workspace_dir,
    ));
    Agent::builder()
        .provider(provider)
        .tools(vec![Box::new(crate::tools::skill::SkillTool::new_with_runtime_config(
            security.clone(),
            "core-skills-fixture".into(),
            false,
            config.workspace_dir.clone(),
            Arc::new(config.clone()),
        ))])
        .memory(Arc::new(NoneMemory))
        .observer(Arc::new(NoopObserver))
        .security(security)
        .config(vw_config_types::agent::effective_main_agent_config(&config.agents))
        .skills(vec![skill])
        .workspace_dir(config.workspace_dir.clone())
        .model_name("model-recovery/original".into())
        .build()
        .unwrap()
}

fn assert_skill_request(messages: &[ChatMessage]) {
    let catalogs = messages
        .iter()
        .filter(|message| message.role == "developer" && message.content.contains("fixture-review"))
        .collect::<Vec<_>>();
    assert_eq!(
        catalogs.len(),
        1,
        "the real provider request must contain one developer catalog: {messages:?}"
    );
    assert!(catalogs[0].content.contains("SKILL.md"));
    let bodies = messages
        .iter()
        .filter(|message| message.content.contains("BODY_CORE_REVIEW"))
        .collect::<Vec<_>>();
    assert_eq!(bodies.len(), 1, "the default body must be replayed once");
    assert_eq!(bodies[0].role, "user");
}

#[tokio::test]
async fn codex_skills_core_agent_default_catalog_is_ephemeral_across_two_turns() {
    let _guard = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    crate::config::update(serde_json::json!({"providers":{"model-recovery":{"models":{"original":{"limit":{"context":100000,"output":1024}}}}}})).await.unwrap();
    let (_workspace, config, skill) = skill_fixture();
    let provider = ProviderHandle::with_responses(vec![
        Ok(text_response("first")),
        Ok(text_response("second")),
    ]);
    let mut agent = skill_agent(provider.native_provider(), &config, skill);
    agent.turn("first task").await.unwrap();
    agent.turn("continue task").await.unwrap();
    assert_eq!(
        agent.skill_session.context_window_tokens, 100000,
        "fixture model limit must reach the catalog budget"
    );
    for request in provider.seen_messages() {
        assert_skill_request(&request);
    }
    assert_eq!(agent.history().len(), 4);
    assert!(agent.history().iter().all(
        |message| message.role != "developer" && !message.content.contains("BODY_CORE_REVIEW")
    ));
}

#[tokio::test]
async fn codex_skills_core_streaming_agent_preserves_skill_roles() {
    let _guard = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    crate::config::update(serde_json::json!({"providers":{"model-recovery":{"models":{"original":{"limit":{"context":100000,"output":1024}}}}}})).await.unwrap();
    let (_workspace, config, skill) = skill_fixture();
    let provider = ProviderHandle::with_responses(vec![Ok(text_response("streamed"))]);
    let mut agent = skill_agent(provider.native_provider(), &config, skill);
    let (tx, rx) = tokio::sync::mpsc::channel(16);
    assert_eq!(agent.turn_with_stream("stream task", tx).await.unwrap(), "streamed");
    assert!(!collect_chunks(rx).await.is_empty());
    assert_skill_request(&provider.seen_messages()[0]);
    assert!(agent.history().iter().all(|message| !message.content.contains("BODY_CORE_REVIEW")));
}

#[tokio::test]
async fn codex_skills_core_agent_successful_skill_replays_after_trim_and_clear_drops_it() {
    let _guard = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    crate::config::update(serde_json::json!({"providers":{"model-recovery":{"models":{"original":{"limit":{"context":100000,"output":1024}}}}}})).await.unwrap();
    let (_workspace, mut config, skill) = skill_fixture();
    config.autonomy.level = crate::security::AutonomyLevel::Full;
    let main = config.agents.get_mut("main").unwrap();
    main.allowed_skills.clear();
    main.max_history_messages = 2;
    let provider = ProviderHandle::with_responses(vec![
        Ok(ChatResponse {
            text: None,
            tool_calls: vec![ToolCall {
                id: "core-skill-call".into(),
                name: "skill".into(),
                arguments: serde_json::json!({"skill":"fixture-review","args":"LATEST_ARGS"})
                    .to_string(),
            }],
            usage: None,
            reasoning_content: None,
        }),
        Ok(text_response("completed")),
        Ok(text_response("continued")),
        Ok(text_response("fresh")),
    ]);
    let mut agent = skill_agent(provider.native_provider(), &config, skill);
    agent.turn("inspect changes").await.unwrap();
    agent.turn("continue task").await.unwrap();
    assert_eq!(
        agent.skill_session.context_window_tokens, 100000,
        "fixture model limit must reach the catalog budget"
    );
    assert_eq!(agent.history().len(), 2);
    agent.clear_history();
    agent.turn("start over").await.unwrap();
    let requests = provider.seen_messages();
    assert!(
        !requests[0].iter().any(|message| message.content.contains("BODY_CORE_REVIEW")),
        "metadata does not activate a body"
    );
    for request in &requests[1..3] {
        assert_skill_request(request);
        assert!(request.iter().any(|message| message.role == "user"
            && message.content.contains("BODY_CORE_REVIEW LATEST_ARGS")));
    }
    assert!(!requests[3].iter().any(|message| message.content.contains("BODY_CORE_REVIEW")));
    assert!(
        requests[1]
            .iter()
            .any(|message| message.role == "tool" && message.content.contains("core-skill-call"))
    );
    assert!(agent.history().iter().all(|message| !message.content.contains("BODY_CORE_REVIEW")));
}

#[tokio::test]
async fn codex_skills_core_agent_unavailable_default_fails_before_provider() {
    let _guard = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let (_workspace, mut config, skill) = skill_fixture();
    config.agents.get_mut("main").unwrap().allowed_skills = vec!["absent-fixture".into()];
    let provider = ProviderHandle::default();
    let mut agent = skill_agent(provider.native_provider(), &config, skill);
    let error = agent.turn("perform task").await.unwrap_err().to_string();
    assert!(error.contains("absent-fixture"));
    assert!(provider.seen_messages().is_empty());
}

#[tokio::test]
async fn codex_skills_core_agent_instruction_reference_preloads_from_real_source() {
    let _guard = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    crate::config::update(serde_json::json!({"providers":{"model-recovery":{"models":{"original":{"limit":{"context":100000,"output":1024}}}}}})).await.unwrap();
    let (workspace, mut config, skill) = skill_fixture();
    config.agents.get_mut("main").unwrap().allowed_skills.clear();
    std::fs::write(
        workspace.path().join("AGENTS.md"),
        "Use fixture-review when processing this workspace.",
    )
    .unwrap();
    let provider = ProviderHandle::default();
    let mut agent = skill_agent(provider.native_provider(), &config, skill);
    agent.turn("inspect changes").await.unwrap();
    assert_skill_request(&provider.seen_messages()[0]);
    assert!(agent.history().iter().all(|message| !message.content.contains("BODY_CORE_REVIEW")));
}

#[tokio::test]
async fn codex_skills_core_agent_tenant_deny_is_preserved_across_permission_bridge() {
    let _guard = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    crate::config::update(serde_json::json!({"providers":{"model-recovery":{"models":{"original":{"limit":{"context":100000,"output":1024}}}}}})).await.unwrap();
    let (_workspace, config, skill) = skill_fixture();
    let tenant = vw_api_types::id::TenantScope::new("platform", "core-skills-denied").unwrap();
    crate::session::ui_config::set_config_field_result(
        &tenant,
        "permission",
        serde_json::json!({"skill":{"fixture-review":"deny"}}),
    )
    .unwrap();
    let provider = ProviderHandle::default();
    let mut agent = skill_agent(provider.native_provider(), &config, skill);
    let error = crate::session::ui_store::with_tenant_scope(tenant, agent.turn("inspect changes"))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("fixture-review"));
    assert!(provider.seen_messages().is_empty());
}
