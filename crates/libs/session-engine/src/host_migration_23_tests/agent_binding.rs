use crate::agent_binding::AgentBinding;
use config_agent::AgentDefinitionConfig;
use config_agent_profile::{AgentDefault,AgentDefaults};
#[tokio::test]
async fn agent_session_identity_storage_round_trip_is_scoped_and_secret_free() {
    let home = tempfile::tempdir().unwrap();
    let tenant = vw_api_transport::TenantScope::new("binding-platform", "binding-company").unwrap();
    session_store::ui_store::with_tenant_scope(tenant, async {
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
        crate::binding_store::save_scoped(home.path().to_path_buf(), session_store::ui_store::current_tenant_scope(), &project, "session", &binding).await.unwrap();
        let restored = crate::binding_store::load_scoped(home.path().to_path_buf(), session_store::ui_store::current_tenant_scope(), &project, "session").await.unwrap().unwrap();
        let serialized = serde_json::to_string(&restored).unwrap();
        assert!(!serialized.contains("SECRET_SENTINEL"));
        assert!(!serialized.contains("api_key"));
        assert_eq!(restored.system_prompt.as_deref(), Some("original rule"));
        assert!(matches!(restored.defaults.skills, AgentDefault::Fixed(ids) if ids.is_empty()));
        assert!(crate::binding_store::load_scoped(home.path().to_path_buf(), session_store::ui_store::current_tenant_scope(), "other-project", "session").await.unwrap().is_none());
        session_store::ui_store::with_tenant_scope(
            vw_api_transport::TenantScope::new("other-platform", "binding-company").unwrap(),
            async {
                assert!(crate::binding_store::load_scoped(home.path().to_path_buf(), session_store::ui_store::current_tenant_scope(), &project, "session").await.unwrap().is_none());
            },
        )
        .await;
    })
    .await;
}

#[tokio::test]
async fn agent_config_cleanup_full_access_session_state_preserves_explicit_false() {
    let home = tempfile::tempdir().unwrap();
    let project = format!("full-access-{}", uuid::Uuid::new_v4());
    assert_eq!(crate::binding_store::SessionSettings::load(home.path().to_path_buf(), session_store::ui_store::current_tenant_scope(), &project, "new").await.unwrap().full_access, None);
    assert!(crate::binding_store::resolve_full_access_scoped(home.path().to_path_buf(), session_store::ui_store::current_tenant_scope(), &project, "new", None, true).await.unwrap());
    assert_eq!(crate::binding_store::SessionSettings::load(home.path().to_path_buf(), session_store::ui_store::current_tenant_scope(), &project, "new").await.unwrap().full_access, Some(true));
    assert!(!crate::binding_store::resolve_full_access_scoped(home.path().to_path_buf(), session_store::ui_store::current_tenant_scope(), &project, "new", Some(false), true).await.unwrap());
    assert_eq!(crate::binding_store::SessionSettings::load(home.path().to_path_buf(), session_store::ui_store::current_tenant_scope(), &project, "new").await.unwrap().full_access, Some(false));
    assert!(!crate::binding_store::resolve_full_access_scoped(home.path().to_path_buf(), session_store::ui_store::current_tenant_scope(), &project, "new", None, true).await.unwrap());
    assert!(crate::binding_store::resolve_full_access_scoped(home.path().to_path_buf(), session_store::ui_store::current_tenant_scope(), &project, "other", None, true).await.unwrap());
    assert!(!crate::binding_store::resolve_full_access_scoped(home.path().to_path_buf(), session_store::ui_store::current_tenant_scope(), &project, "off", None, false).await.unwrap());
    assert!(!crate::binding_store::resolve_full_access_scoped(home.path().to_path_buf(), session_store::ui_store::current_tenant_scope(), "other-project", "new", None, false).await.unwrap());
}
