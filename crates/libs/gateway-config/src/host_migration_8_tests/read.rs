use super::*;

#[tokio::test]
async fn config_get_returns_serialized_instance_config() {
    let _env_lock = global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", temp.path().as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");

    let Json(body) =
        crate::handlers::config_get::<TestHost>(Extension(TenantScope::default()), instance_query(&temp), HeaderMap::new())
            .await
            .expect("config should load");

    assert!(body.is_object());
    assert!(body.get("default_provider").is_some());
}

#[tokio::test]
async fn config_get_stateful_returns_serialized_instance_config() {
    let _env_lock = global_async_env_lock().lock().await;
    let home = TempDir::new().expect("temp home");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");

    let Json(body) = crate::handlers::config_get_stateful::<TestHost>(
        Extension(TenantScope::default()),
        Query(InstanceQuery { directory: Some(home.path().to_string_lossy().to_string()) }),
        HeaderMap::new(),
    )
    .await
    .expect("stateful config should load");

    assert!(body.is_object());
    assert!(body.get("default_provider").is_some());
}

#[tokio::test]
async fn config_get_without_directory_restores_the_requested_tenant_after_restart() {
    let _env_lock = global_async_env_lock().lock().await;
    let home = TempDir::new().expect("temp home");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let tenant = TenantScope::new("desktop-platform", "desktop-company").expect("tenant");

    config::update_global_for_tenant(
        &tenant,
        serde_json::json!({"agents": {"main": {"max_iterations": 123}}}),
    )
    .await
    .expect("tenant config should persist");

    let Json(body) =
        crate::handlers::config_get::<TestHost>(Extension(tenant), Query(InstanceQuery { directory: None }), HeaderMap::new())
            .await
            .expect("tenant config should reload");

    assert_eq!(body["agents"]["main"]["max_iterations"], 123);
    assert_ne!(
        config::get().await.agents.get("main").map(|agent| agent.max_iterations),
        Some(123),
        "local gateway startup config must not replace the requested tenant"
    );
}

// 没有显式目录时补丁属于租户全局配置，不能被当前实例路径隐式改成项目覆盖。
#[tokio::test]
async fn handle_api_config_get_returns_pretty_toml_with_masked_secrets() {
    let _env_lock = global_async_env_lock().lock().await;
    let mut cfg = config::Config::default();
    cfg.api_key = Some("sk-secret".to_string());
    cfg.gateway.paired_tokens = vec!["paired-secret".to_string()];

    let response = crate::handlers::handle_api_config_get::<TestHost>(
        State(state(cfg, false, &[])),
        Extension(TenantScope::default()),
        HeaderMap::new(),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["format"], "toml");
    let content = body["content"].as_str().unwrap();
    assert!(content.contains("api_key = \"***MASKED***\""));
    assert!(!content.contains("sk-secret"));
    assert!(!content.contains("paired-secret"));
}

#[tokio::test]
async fn handle_api_config_get_reports_gateway_skey_store_failure() {
    let _env_lock = global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let invalid_home = temp.path().join("home-file");
    std::fs::write(&invalid_home, "not a directory").expect("invalid home file");
    let _home = EnvGuard::set("HOME", invalid_home.as_os_str());

    let response = crate::handlers::handle_api_config_get::<TestHost>(
        State(state(config::Config::default(), false, &[])),
        Extension(TenantScope::default()),
        HeaderMap::new(),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(
        response_json(response).await["error"]
            .as_str()
            .unwrap()
            .contains("Failed to load gateway skeys")
    );
}

// 写入端点的顺序契约是先认证、再解析、再校验、最后持久化和替换运行时状态。
#[tokio::test]
async fn handle_api_config_put_reports_gateway_skey_store_failure_without_mutating_runtime_state() {
    let _env_lock = global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let invalid_home = temp.path().join("home-file");
    std::fs::write(&invalid_home, "not a directory").expect("invalid home file");
    let _home = EnvGuard::set("HOME", invalid_home.as_os_str());
    let current = config::Config::default();
    let expected_provider = current.default_provider.clone();
    let body = toml::to_string_pretty(&current).expect("default config should serialize");
    let state = state(current, false, &[]);

    let response = crate::handlers::handle_api_config_put::<TestHost>(
        State(state.clone()),
        Extension(TenantScope::default()),
        HeaderMap::new(),
        body,
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(
        response_json(response).await["error"]
            .as_str()
            .unwrap()
            .contains("Failed to save gateway skeys")
    );
    assert_eq!(state.config.lock().default_provider, expected_provider);
}

#[tokio::test]
async fn handle_api_config_put_hydrates_validates_saves_and_updates_runtime_config() {
    let _env_lock = global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let _home = EnvGuard::set("HOME", temp.path().as_os_str());
    let mut current = config::Config::default();
    current.config_path = temp.path().join(".vibewindow.json");
    current.api_key = Some("sk-current".to_string());
    current.gateway.paired_tokens = vec!["paired-current".to_string()];

    let mut incoming = current.clone();
    incoming.default_provider = Some("openai".to_string());
    incoming.api_key = Some(agent_gateway::api::secrets::MASKED_SECRET.to_string());
    incoming.gateway.paired_tokens =
        vec![agent_gateway::api::secrets::MASKED_SECRET.to_string()];
    let body = toml::to_string_pretty(&incoming).expect("config should serialize as toml");
    let state = state(current, false, &[]);

    let response = crate::handlers::handle_api_config_put::<TestHost>(
        State(state.clone()),
        Extension(TenantScope::default()),
        HeaderMap::new(),
        body,
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await["status"], "ok");
    assert_eq!(state.config.lock().default_provider.as_deref(), Some("openai"));
    assert_eq!(state.config.lock().api_key.as_deref(), Some("sk-current"));
    assert_eq!(state.config.lock().gateway.paired_tokens, vec!["paired-current"]);
    let stored = local_store(temp.path())
        .get(&TenantScope::default())
        .expect("read stored config")
        .expect("stored config row");
    let stored: config::Config = serde_json::from_str(&stored).expect("parse stored config");
    assert_eq!(stored.default_provider.as_deref(), Some("openai"));
}

// 仪表盘密钥通过共享存储持久化，但同一键名仍必须按租户隔离。
#[tokio::test]
async fn dashboard_config_skeys_are_isolated_by_tenant_scope() {
    let _env_lock = global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let _home = EnvGuard::set("HOME", temp.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", temp.path().as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let first = TenantScope::new("platform-a", "company-a").expect("first tenant");
    let second = TenantScope::new("platform-b", "company-b").expect("second tenant");
    let mut current = config::Config::default();
    current.config_path = temp.path().join("vibewindow.json");
    let state = state(current, false, &[]);

    for (tenant, name, hash_byte) in [(&first, "first-skey", 'a'), (&second, "second-skey", 'b')] {
        let mut incoming = state.config.lock().clone();
        incoming.gateway.skeys = vec![config_gateway::GatewaySkey {
            enabled: true,
            skey: None,
            skey_hash: hash_byte.to_string().repeat(64),
            masked_skey: format!("{name}-masked"),
            name: name.to_string(),
            expires_at: None,
        }];
        let body = toml::to_string_pretty(&incoming).expect("serialize config");
        let response = crate::handlers::handle_api_config_put::<TestHost>(
            State(state.clone()),
            Extension(tenant.clone()),
            HeaderMap::new(),
            body,
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    for (tenant, expected_name) in [(&first, "first-skey"), (&second, "second-skey")] {
        let response = crate::handlers::handle_api_config_get::<TestHost>(
            State(state.clone()),
            Extension(tenant.clone()),
            HeaderMap::new(),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_json(response).await;
        let config: config::Config =
            toml::from_str(body["content"].as_str().expect("toml content"))
                .expect("parse config response");
        assert_eq!(config.gateway.skeys[0].name, expected_name);
    }
}
