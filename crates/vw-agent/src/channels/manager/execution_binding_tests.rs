use super::*;

fn fallback() -> ChannelRouteSelection {
    ChannelRouteSelection {
        provider: "default".into(),
        model: "default-model".into(),
        reasoning_effort: None,
        task_mode_enabled: false,
    }
}

#[test]
fn channel_execution_routes_independent_acp_backends_and_explicit_models() {
    for backend in ["codex", "claude"] {
        let agent = AgentDefinitionConfig {
            runtime: AgentRuntime::Acp { backend: backend.into() },
            provider: String::new(),
            model: String::new(),
            ..Default::default()
        };
        let (route, acp) =
            execution_route(&ChannelExecutionConfig::default(), &agent, fallback()).unwrap();
        assert!(acp);
        assert_eq!(route.provider, "acp");
        assert_eq!(route.model, backend);
        let execution = ChannelExecutionConfig {
            agent: Some(format!("acp-{backend}")),
            model: Some(vw_config_types::agent::AgentModelReference {
                provider: "custom".into(),
                model: "model/name".into(),
            }),
        };
        let (route, acp) = execution_route(&execution, &agent, fallback()).unwrap();
        assert!(acp);
        assert_eq!((route.provider.as_str(), route.model.as_str()), ("custom", "model/name"));
    }
}

#[test]
fn channel_execution_rejects_disabled_and_forced_model_conflicts() {
    let mut agent = AgentDefinitionConfig { enabled: false, ..Default::default() };
    assert!(execution_route(&ChannelExecutionConfig::default(), &agent, fallback()).is_err());
    agent.enabled = true;
    agent.force_model_selection = true;
    agent.provider = "provider".into();
    agent.model = "fixed".into();
    let execution = ChannelExecutionConfig {
        agent: None,
        model: Some(vw_config_types::agent::AgentModelReference {
            provider: "other".into(),
            model: "model".into(),
        }),
    };
    assert!(execution_route(&execution, &agent, fallback()).is_err());
}

#[test]
fn channel_execution_uses_native_agent_default_model() {
    let agent = AgentDefinitionConfig {
        defaults: vw_config_types::agent::AgentDefaults {
            model: AgentDefault::Fixed(vw_config_types::agent::AgentModelReference {
                provider: "agent-provider".into(),
                model: "agent-model".into(),
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    let (route, acp) =
        execution_route(&ChannelExecutionConfig::default(), &agent, fallback()).unwrap();
    assert!(!acp);
    assert_eq!(route.provider, "agent-provider");
    assert_eq!(route.model, "agent-model");
}

#[tokio::test]
async fn channel_execution_options_use_bound_acp_without_expanding_permissions() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let workspace = tempfile::tempdir().unwrap();
    let tenant = crate::session::ui_store::current_tenant_scope();
    let (project, worktree) =
        crate::project::from_directory(&tenant, workspace.path()).await.unwrap();
    let definition = AgentDefinitionConfig {
        runtime: AgentRuntime::Acp { backend: "codex".into() },
        provider: String::new(),
        model: String::new(),
        defaults: Default::default(),
        ..Default::default()
    };
    let session = format!("channel-test-{}", uuid::Uuid::new_v4());
    let binding = crate::session::agent_binding::AgentBinding::from_definition(
        "acp-codex".into(),
        &definition,
    );
    crate::session::agent_binding::save_scoped(&project.id, &session, &binding).await.unwrap();
    let snapshot = ChannelMessageSnapshot {
        scope: ChannelMessageScope {
            chat: ChannelChatScope {
                tenant,
                channel: "wechat".into(),
                instance: "default".into(),
                reply_target: "room".into(),
                thread: None,
                sender: "sender".into(),
            },
            project: project.id.clone(),
        },
        directory: workspace.path().to_path_buf(),
        project: project.clone(),
        source: ChannelProjectSource::Instance,
        execution: Some(ChannelExecutionConfig { agent: Some("acp-codex".into()), model: None }),
    };
    crate::project::instance::provide_context(
        crate::project::instance::Context {
            directory: workspace.path().to_string_lossy().into_owned(),
            worktree,
            project,
        },
        async {
            let route =
                execution_route(snapshot.execution.as_ref().unwrap(), &definition, fallback())
                    .unwrap()
                    .0;
            let options = execution_options(
                &snapshot,
                &session,
                &route,
                serde_json::json!({"full_access": false}),
            )
            .await
            .unwrap();
            assert_eq!(options["agent"], "acp-codex");
            assert_eq!(options["acp_agent"], "codex");
            assert_eq!(options["acp_model_source"], "cli_local");
            assert_eq!(options["full_access"], false);
            assert_eq!(options["acp_non_interactive_permissions"], "deny");
            assert_eq!(options["cwd"], workspace.path().to_string_lossy().as_ref());
            let mut cleared = snapshot.clone();
            cleared.execution = None;
            assert!(
                execution_options(&cleared, &session, &route, serde_json::json!({})).await.is_err()
            );
            let mut changed = snapshot.clone();
            changed.execution.as_mut().unwrap().agent = Some("acp-claude".into());
            assert!(
                execution_options(&changed, &session, &route, serde_json::json!({})).await.is_err()
            );
        },
    )
    .await;
}
