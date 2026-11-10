use super::*;
use vw_config_types::agent::AgentDefault;

#[tokio::test]
async fn agent_session_identity_storage_round_trip_is_scoped_and_secret_free() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let tenant = vw_api_types::id::TenantScope::new("binding-platform", "binding-company").unwrap();
    crate::session::ui_store::with_tenant_scope(tenant, async {
        let project = format!("project-{}", uuid::Uuid::new_v4());
        let definition = AgentDefinitionConfig {
            api_key: Some("SECRET_SENTINEL".into()),
            options: std::collections::HashMap::from([(
                "token".into(),
                serde_json::json!("SECRET_SENTINEL"),
            )]),
            system_prompt: Some("original rule".into()),
            defaults: AgentDefaults { skills: AgentDefault::Fixed(vec![]), ..Default::default() },
            ..Default::default()
        };
        let binding = AgentBinding::from_definition("main".into(), &definition);
        save_scoped(&project, "session", &binding).await.unwrap();
        let restored = load_scoped(&project, "session").await.unwrap().unwrap();
        let serialized = serde_json::to_string(&restored).unwrap();
        assert!(!serialized.contains("SECRET_SENTINEL"));
        assert!(!serialized.contains("api_key"));
        assert_eq!(restored.system_prompt.as_deref(), Some("original rule"));
        assert!(matches!(restored.defaults.skills, AgentDefault::Fixed(ids) if ids.is_empty()));
        assert!(load_scoped("other-project", "session").await.unwrap().is_none());
        crate::session::ui_store::with_tenant_scope(
            vw_api_types::id::TenantScope::new("other-platform", "binding-company").unwrap(),
            async {
                assert!(load_scoped(&project, "session").await.unwrap().is_none());
            },
        )
        .await;
    })
    .await;
}

#[test]
fn agent_session_identity_rejects_engine_and_agent_changes() {
    let binding = AgentBinding::from_definition("main".into(), &AgentDefinitionConfig::default());
    assert!(binding.validate_selection(Some("main"), None).is_ok());
    assert!(binding.validate_selection(None, None).is_ok());
    assert!(binding.validate_selection(Some("other"), None).is_err());
    assert!(binding.validate_selection(Some("main"), Some("codex")).is_err());
    let acp = AgentBinding::from_definition(
        "acp-codex".into(),
        &AgentDefinitionConfig {
            runtime: AgentRuntime::Acp { backend: "codex".into() },
            ..Default::default()
        },
    );
    assert!(acp.validate_selection(None, Some("codex")).is_ok());
    assert!(acp.validate_selection(None, Some("claude")).is_err());
}

#[test]
fn agent_session_identity_keeps_live_credentials_and_security_rules() {
    let original =
        AgentDefinitionConfig { system_prompt: Some("old rule".into()), ..Default::default() };
    let binding = AgentBinding::from_definition("main".into(), &original);
    let mut current = AgentDefinitionConfig {
        enabled: false,
        permission: serde_json::json!({"*":"deny"}),
        api_key: Some("rotated-credential".into()),
        system_prompt: Some("edited rule".into()),
        ..Default::default()
    };
    binding.apply(&mut current);
    assert!(current.enabled);
    assert_eq!(current.system_prompt.as_deref(), Some("old rule"));
    assert_eq!(current.permission, serde_json::json!({"*":"deny"}));
    assert_eq!(current.api_key.as_deref(), Some("rotated-credential"));
}

#[tokio::test]
async fn agent_config_cleanup_full_access_session_state_preserves_explicit_false() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let project = format!("full-access-{}", uuid::Uuid::new_v4());
    assert_eq!(SessionSettings::load(&project, "new").await.unwrap().full_access, None);
    assert!(resolve_full_access_scoped(&project, "new", None, true).await.unwrap());
    assert_eq!(SessionSettings::load(&project, "new").await.unwrap().full_access, Some(true));
    assert!(!resolve_full_access_scoped(&project, "new", Some(false), true).await.unwrap());
    assert_eq!(SessionSettings::load(&project, "new").await.unwrap().full_access, Some(false));
    assert!(!resolve_full_access_scoped(&project, "new", None, true).await.unwrap());
    assert!(resolve_full_access_scoped(&project, "other", None, true).await.unwrap());
    assert!(!resolve_full_access_scoped(&project, "off", None, false).await.unwrap());
    assert!(!resolve_full_access_scoped("other-project", "new", None, false).await.unwrap());
}

#[test]
fn agent_config_cleanup_full_access_retains_permission_denials() {
    let mut definition = AgentDefinitionConfig::default();
    definition.defaults.full_access = true;
    let binding = AgentBinding::from_definition("main".into(), &definition);
    let mut current =
        AgentDefinitionConfig { permission: serde_json::json!({"*":"deny"}), ..Default::default() };
    binding.apply(&mut current);
    assert!(current.defaults.full_access);
    assert_eq!(current.permission, serde_json::json!({"*":"deny"}));
}
