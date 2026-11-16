use super::*;

#[tokio::test]
async fn agent_runtime_lock_patch_preserves_saved_engines_and_is_atomic() {
    let _env_lock = global_async_env_lock().lock().await;
    let home = TempDir::new().unwrap();
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let config_dir = home.path().join("config");
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let tenant = TenantScope::default();
    config::update_global(serde_json::json!({"agents": {
        "custom-native": {"runtime": {"kind": "native"}},
        "custom-codex": {"runtime": {"kind": "acp", "backend": "codex"}},
        "custom-claude": {"runtime": {"kind": "acp", "backend": "claude"}}
    }}))
    .await
    .unwrap();
    for (key, runtime) in [
        ("custom-native", serde_json::json!({"kind":"native"})),
        ("custom-codex", serde_json::json!({"kind":"acp","backend":"codex"})),
        ("custom-claude", serde_json::json!({"kind":"acp","backend":"claude"})),
    ] {
        config::update_global(serde_json::json!({"agents": {key: {
            "label":"Renamed", "runtime":runtime
        }}}))
        .await
        .unwrap();
    }
    let current = config::get_global_for_tenant(&tenant).await.unwrap();
    let state = state(current.clone(), false, &[]);
    let store = local_store(home.path());
    let before = store.get(&tenant).unwrap();
    let skeys_before = config::load_gateway_skeys_for_tenant(&tenant).await.unwrap();
    for (key, runtime) in [
        ("custom-native", serde_json::json!({"kind":"acp","backend":"codex"})),
        ("custom-codex", serde_json::json!({"kind":"native","backend":null})),
        ("custom-codex", serde_json::json!({"backend":"claude"})),
        ("custom-codex", serde_json::json!({"backend":"unknown-backend"})),
        ("custom-codex", Value::Null),
    ] {
        let result = crate::handlers::config_patch_stateful::<TestHost>(
            Extension(tenant.clone()),
            State(state.clone()),
            Query(InstanceQuery { directory: None }),
            HeaderMap::new(),
            Json(serde_json::json!({
                "default_model":"must-not-leak",
                "gateway":{"skeys":[{"name":"must-not-save","skey":"runtime-lock-secret"}]},
                "agents":{key:{"label":"must-not-leak","runtime":runtime}}
            })),
        )
        .await;
        assert!(result.is_err(), "engine change must be rejected: {key}");
        assert_eq!(store.get(&tenant).unwrap(), before);
        assert_eq!(
            serde_json::to_value(&*state.config.lock()).unwrap(),
            serde_json::to_value(&current).unwrap()
        );
        assert_eq!(
            serde_json::to_value(config::load_gateway_skeys_for_tenant(&tenant).await.unwrap())
                .unwrap(),
            serde_json::to_value(&skeys_before).unwrap()
        );
    }
    config::update_global(serde_json::json!({"agents":{"custom-codex":null}})).await.unwrap();
    assert!(
        !config::get_global_for_tenant(&tenant).await.unwrap().agents.contains_key("custom-codex")
    );
}

#[tokio::test]
async fn agent_runtime_lock_patch_is_scoped_to_each_tenant() {
    let _env_lock = global_async_env_lock().lock().await;
    let home = TempDir::new().unwrap();
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let config_dir = home.path().join("config");
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let first = TenantScope::new("runtime-lock", "first").unwrap();
    let second = TenantScope::new("runtime-lock", "second").unwrap();
    for (tenant, runtime) in [
        (&first, serde_json::json!({"kind":"native"})),
        (&second, serde_json::json!({"kind":"acp","backend":"codex"})),
    ] {
        config::update_global_for_tenant(
            tenant,
            serde_json::json!({"agents":{"custom":{"runtime":runtime}}}),
        )
        .await
        .unwrap();
    }
    let before = local_store(home.path()).get(&second).unwrap();
    assert!(
        config::update_global_for_tenant(
            &first,
            serde_json::json!({"agents":{"custom":{"runtime":{"kind":"acp","backend":"codex"}}}})
        )
        .await
        .is_err()
    );
    config::update_global_for_tenant(
        &second,
        serde_json::json!({"agents":{"custom":{"label":"Allowed"}}}),
    )
    .await
    .unwrap();
    assert_ne!(local_store(home.path()).get(&second).unwrap(), before);
    assert_eq!(
        config::get_global_for_tenant(&first).await.unwrap().agents["custom"].runtime,
        config_agent_profile::AgentRuntime::Native
    );
}

#[tokio::test]
async fn agent_runtime_lock_put_rejects_replacement_and_implicit_switches() {
    let _env_lock = global_async_env_lock().lock().await;
    use config_agent_profile::AgentRuntime;
    use config_agent::AgentDefinitionConfig;
    let _env_lock = global_async_env_lock().lock().await;
    let home = TempDir::new().unwrap();
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let mut current = config::Config::default();
    current.agents.insert(
        "custom".into(),
        AgentDefinitionConfig {
            runtime: AgentRuntime::Acp { backend: "codex".into() },
            permission: serde_json::json!({}),
            ..Default::default()
        },
    );
    config::save_config(&current).await.unwrap();
    let state = state(current.clone(), false, &[]);
    let before = local_store(home.path()).get(&TenantScope::default()).unwrap();
    for runtime in [AgentRuntime::Native, AgentRuntime::Acp { backend: "claude".into() }] {
        let mut incoming = current.clone();
        incoming.agents.insert(
            "custom".into(),
            AgentDefinitionConfig {
                runtime,
                permission: serde_json::json!({}),
                ..Default::default()
            },
        );
        incoming.default_model = Some("must-not-leak".into());
        let response = crate::handlers::handle_api_config_put::<TestHost>(
            State(state.clone()),
            Extension(TenantScope::default()),
            HeaderMap::new(),
            toml::to_string_pretty(&incoming).unwrap(),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(
            response_json(response).await["error"]
                .as_str()
                .unwrap()
                .contains("runtime cannot be changed")
        );
        assert_eq!(local_store(home.path()).get(&TenantScope::default()).unwrap(), before);
        assert_eq!(
            serde_json::to_value(&*state.config.lock()).unwrap(),
            serde_json::to_value(&current).unwrap()
        );
    }
    let mut incoming: toml::Value =
        toml::from_str(&toml::to_string_pretty(&current).unwrap()).unwrap();
    incoming["agents"]["custom"].as_table_mut().unwrap().remove("runtime");
    let response = crate::handlers::handle_api_config_put::<TestHost>(
        State(state.clone()),
        Extension(TenantScope::default()),
        HeaderMap::new(),
        toml::to_string_pretty(&incoming).unwrap(),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(local_store(home.path()).get(&TenantScope::default()).unwrap(), before);
    let mut incoming = current.clone();
    incoming.agents.get_mut("custom").unwrap().label = Some("Renamed".into());
    let response = crate::handlers::handle_api_config_put::<TestHost>(
        State(state.clone()),
        Extension(TenantScope::default()),
        HeaderMap::new(),
        toml::to_string_pretty(&incoming).unwrap(),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(state.config.lock().agents["custom"].label.as_deref(), Some("Renamed"));
    incoming.agents.clear();
    let response = crate::handlers::handle_api_config_put::<TestHost>(
        State(state.clone()),
        Extension(TenantScope::default()),
        HeaderMap::new(),
        toml::to_string_pretty(&incoming).unwrap(),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(state.config.lock().agents.is_empty());
}

#[tokio::test]
async fn agent_runtime_lock_put_accepts_first_save_and_locks_native() {
    let _env_lock = global_async_env_lock().lock().await;
    use config_agent_profile::AgentRuntime;
    use config_agent::AgentDefinitionConfig;
    let _env_lock = global_async_env_lock().lock().await;
    let home = TempDir::new().unwrap();
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let state = state(config::Config::default(), false, &[]);
    let mut incoming = config::Config::default();
    for (key, runtime) in [
        ("custom-native", AgentRuntime::Native),
        ("custom-codex", AgentRuntime::Acp { backend: "codex".into() }),
        ("custom-claude", AgentRuntime::Acp { backend: "claude".into() }),
    ] {
        incoming.agents.insert(
            key.into(),
            AgentDefinitionConfig {
                runtime,
                permission: serde_json::json!({}),
                ..Default::default()
            },
        );
    }
    let response = crate::handlers::handle_api_config_put::<TestHost>(
        State(state.clone()),
        Extension(TenantScope::default()),
        HeaderMap::new(),
        toml::to_string_pretty(&incoming).unwrap(),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::OK);
    let saved = local_store(home.path()).get(&TenantScope::default()).unwrap();
    incoming.agents.get_mut("custom-native").unwrap().runtime =
        AgentRuntime::Acp { backend: "codex".into() };
    let response = crate::handlers::handle_api_config_put::<TestHost>(
        State(state.clone()),
        Extension(TenantScope::default()),
        HeaderMap::new(),
        toml::to_string_pretty(&incoming).unwrap(),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(local_store(home.path()).get(&TenantScope::default()).unwrap(), saved);
    assert_eq!(state.config.lock().agents["custom-native"].runtime, AgentRuntime::Native);
}

#[tokio::test]
async fn worktree_settings_patch_round_trips_and_rejects_project_override() {
    let _env_lock = global_async_env_lock().lock().await;
    let home = TempDir::new().unwrap();
    let project = home.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let tenant = TenantScope::default();
    let settings = serde_json::json!({
        "root": home.path().join("trees"), "fetch_before_create": true,
        "auto_cleanup_enabled": true, "keep_count": 7, "ask_before_delete": true
    });
    let Json(response) = crate::handlers::config_patch::<TestHost>(
        Extension(tenant.clone()),
        Query(InstanceQuery { directory: None }),
        HeaderMap::new(),
        Json(serde_json::json!({"worktree_settings": settings})),
    )
    .await
    .unwrap();
    assert_eq!(response["worktree_settings"], settings);
    let reloaded = config::get_global_for_tenant(&tenant).await.unwrap();
    assert_eq!(serde_json::to_value(&reloaded.worktree_settings).unwrap(), settings);
    let store = local_store(home.path());
    let before = store.get(&tenant).unwrap();
    for invalid in [
        serde_json::json!({"root":"relative"}),
        serde_json::json!({"keep_count":0}),
        serde_json::json!({"keep_count":-1}),
        serde_json::json!({"keep_count":4294967296_u64}),
    ] {
        let err = crate::handlers::config_patch::<TestHost>(
            Extension(tenant.clone()),
            Query(InstanceQuery { directory: None }),
            HeaderMap::new(),
            Json(serde_json::json!({"worktree_settings":invalid})),
        )
        .await
        .expect_err("invalid global settings");
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
        assert_eq!(store.get(&tenant).unwrap(), before);
    }
    for value in [settings.clone(), serde_json::Value::Null, serde_json::json!({})] {
        let err = crate::handlers::config_patch::<TestHost>(
            Extension(tenant.clone()),
            directory_query(&project),
            HeaderMap::new(),
            Json(serde_json::json!({"worktree_settings":value})),
        )
        .await
        .expect_err("settings are global");
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
        assert!(err.message.contains("worktree_settings"));
    }
    assert_eq!(store.get_project(&tenant, project.to_str().unwrap()).unwrap(), None);
    // Even a directly stored project payload cannot override host settings on load.
    store
        .upsert_project(
            &tenant,
            project.to_str().unwrap(),
            r#"{"worktree_settings":{"root":"relative","keep_count":0}}"#,
        )
        .unwrap();
    let effective = config::get_project_for_tenant(&tenant, &project).await.unwrap();
    assert_eq!(serde_json::to_value(effective.worktree_settings).unwrap(), settings);
    assert_eq!(store.get(&tenant).unwrap(), before);
}
