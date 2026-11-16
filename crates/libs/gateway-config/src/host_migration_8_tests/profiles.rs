use super::*;

#[tokio::test]
async fn agent_profile_api_rejects_builtin_delete_and_backend_change() {
    let _env_lock = global_async_env_lock().lock().await;
    let home = TempDir::new().unwrap();
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", home.path().as_os_str());
    let tenant = TenantScope::default();
    for directory in [None, Some(home.path().to_string_lossy().to_string())] {
        for mut patch in [
            serde_json::json!({"agents":{"acp-claude":null}}),
            serde_json::json!({"agents":null}),
            serde_json::json!({"agents":{"acp-claude":{"runtime":{"kind":"acp","backend":"codex"}}}}),
            serde_json::json!({"agents":{"acp-claude":{"runtime":{"kind":"native"}}}}),
        ] {
            let before = config::get_global_for_tenant(&tenant).await.unwrap();
            patch["default_provider"] = serde_json::json!("must-not-persist");
            let error = crate::handlers::apply_config_patch::<TestHost>(
                &tenant,
                &InstanceQuery { directory: directory.clone() },
                &HeaderMap::new(),
                patch,
            )
            .await;
            assert!(error.is_err(), "builtin identity mutation must be rejected");
            let after = config::get_global_for_tenant(&tenant).await.unwrap();
            assert_eq!(after.default_provider, before.default_provider);
        }
    }
}

#[tokio::test]
async fn agent_profile_api_persists_changes_after_reload() {
    let _env_lock = global_async_env_lock().lock().await;
    let home = TempDir::new().unwrap();
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", home.path().as_os_str());
    let tenant = TenantScope::new("profile", "company").unwrap();
    for directory in [None, Some(home.path().to_string_lossy().to_string())] {
        let query = InstanceQuery { directory };
        if query.directory.is_some() {
            let before =
                serde_json::to_value(config::get_global_for_tenant(&tenant).await.unwrap().agents)
                    .unwrap();
            assert!(
                crate::handlers::apply_config_patch::<TestHost>(
                    &tenant,
                    &query,
                    &HeaderMap::new(),
                    serde_json::json!({"agents":{"acp-snow":{"label":"must-not-persist"}}}),
                )
                .await
                .is_err(),
                "project patches cannot change tenant-global agent definitions"
            );
            let after =
                serde_json::to_value(config::get_global_for_tenant(&tenant).await.unwrap().agents)
                    .unwrap();
            assert_eq!(after, before);
            continue;
        }

        let (partial, _) = crate::handlers::apply_config_patch::<TestHost>(
            &tenant,
            &query,
            &HeaderMap::new(),
            serde_json::json!({"agents":{"acp-snow":{"label":"Partial edit"}}}),
        )
        .await
        .unwrap();
        assert_eq!(
            partial.agents["acp-snow"].runtime,
            config_agent_profile::AgentRuntime::Acp { backend: "snow".into() }
        );
        let profile = serde_json::json!({
            "label":"Custom name", "description":"", "recommended_prompts":[],
            "system_prompt":"", "enabled":false,
            "runtime":{"kind":"acp","backend":"claude"},
            "provider":"", "model":"",
            "defaults":{"skills":{"mode":"fixed","value":[]},"mcp":{"mode":"fixed","value":[]}}
        });
        crate::handlers::apply_config_patch::<TestHost>(
            &tenant,
            &query,
            &HeaderMap::new(),
            serde_json::json!({"agents":{"acp-claude":profile}}),
        )
        .await
        .unwrap();
        let Json(reloaded) = crate::handlers::config_get::<TestHost>(
            Extension(tenant.clone()),
            Query(InstanceQuery { directory: query.directory.clone() }),
            HeaderMap::new(),
        )
        .await
        .unwrap();
        let configs = serde_json::from_value(reloaded["agents"].clone()).unwrap();
        let catalog = agent_config::catalog::merged_agent_catalog(&configs);
        let saved = &catalog["acp-claude"];
        assert_eq!(saved.label.as_deref(), Some("Custom name"));
        assert_eq!(saved.description.as_deref(), Some(""));
        assert!(!saved.enabled);
        assert_eq!(saved.system_prompt.as_deref(), Some(""));
        assert!(saved.recommended_prompts.is_empty());
        assert_eq!(
            serde_json::to_value(&saved.defaults).unwrap()["skills"],
            profile["defaults"]["skills"]
        );
        let mut copy = serde_json::to_value(saved).unwrap();
        copy["builtin"] = serde_json::json!(false);
        copy["label"] = serde_json::json!("Copy");
        crate::handlers::apply_config_patch::<TestHost>(
            &tenant,
            &query,
            &HeaderMap::new(),
            serde_json::json!({"agents":{"custom-copy":copy}}),
        )
        .await
        .unwrap();
        let (reloaded, _) = crate::handlers::apply_config_patch::<TestHost>(
            &tenant,
            &query,
            &HeaderMap::new(),
            serde_json::json!({"agents":{"custom-copy":null}}),
        )
        .await
        .unwrap();
        assert!(!reloaded.agents.contains_key("custom-copy"));
        assert!(
            agent_config::catalog::merged_agent_catalog(&reloaded.agents).contains_key("acp-snow")
        );
        let (cleared, _) = crate::handlers::apply_config_patch::<TestHost>(
            &tenant,
            &query,
            &HeaderMap::new(),
            serde_json::json!({"agents":{"acp-claude":{"defaults":{"skills":{"mode":"inherit"}}}}}),
        )
        .await
        .unwrap();
        assert_eq!(
            serde_json::to_value(&cleared.agents["acp-claude"].defaults).unwrap()["skills"],
            serde_json::json!({"mode":"inherit"})
        );
    }
}

#[test]
fn agent_profile_api_rejects_invalid_identity_and_model_conflicts() {
    use config_agent_profile::{AgentDefault, AgentModelReference, AgentRuntime};
    use config_agent::AgentDefinitionConfig;
    for key in ["", "../escape", "white space"] {
        let mut cfg = config::Config::default();
        cfg.agents.insert(key.into(), AgentDefinitionConfig::default());
        assert!(validate_config(&cfg).is_err(), "invalid key {key:?}");
    }
    let mut cfg = config::Config::default();
    cfg.agents.insert(
        "custom".into(),
        AgentDefinitionConfig {
            runtime: AgentRuntime::Acp { backend: "unknown-backend".into() },
            ..Default::default()
        },
    );
    assert!(validate_config(&cfg).is_err());
    cfg.agents.get_mut("custom").unwrap().runtime = AgentRuntime::Native;
    cfg.agents.get_mut("custom").unwrap().provider = "provider".into();
    cfg.agents.get_mut("custom").unwrap().model = "forced".into();
    cfg.agents.get_mut("custom").unwrap().force_model_selection = true;
    cfg.agents.get_mut("custom").unwrap().defaults.model =
        AgentDefault::Fixed(AgentModelReference {
            provider: "provider".into(),
            model: "other".into(),
        });
    assert!(validate_config(&cfg).is_err());
}

#[test]
fn agent_profile_api_native_metadata_and_empty_rules_are_editable() {
    use config_agent::{AgentDefinitionConfig, merged_agent_configs};
    let configured = std::collections::HashMap::from([(
        "explore".into(),
        AgentDefinitionConfig {
            label: Some("Edited".into()),
            description: Some(String::new()),
            system_prompt: Some(String::new()),
            ..Default::default()
        },
    )]);
    let merged = merged_agent_configs(&configured);
    assert_eq!(merged["explore"].label.as_deref(), Some("Edited"));
    assert_eq!(merged["explore"].description.as_deref(), Some(""));
    assert_eq!(merged["explore"].system_prompt.as_deref(), Some(""));
    assert!(merged["explore"].builtin);
    assert_eq!(merged["explore"].mode, "subagent");
    assert!(!merged["explore"].allowed_tools.iter().any(|tool| tool == "bash"));
}

#[tokio::test]
async fn agent_profile_api_full_replacement_cannot_remove_saved_builtin() {
    let _env_lock = global_async_env_lock().lock().await;
    let mut current = config::Config::default();
    current.agents = agent_config::catalog::merged_agent_catalog(&current.agents);
    let state = state(current.clone(), false, &[]);
    let incoming = config::Config::default();
    let response = crate::handlers::handle_api_config_put::<TestHost>(
        State(state.clone()),
        Extension(TenantScope::default()),
        HeaderMap::new(),
        toml::to_string_pretty(&incoming).unwrap(),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(response_json(response).await["error"].as_str().unwrap().contains("cannot be deleted"));
    assert!(state.config.lock().agents.contains_key("acp-claude"));
}
