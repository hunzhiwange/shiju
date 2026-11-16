use super::*;

#[tokio::test]
async fn config_patch_updates_global_config_without_explicit_directory() {
    let _env_lock = global_async_env_lock().lock().await;
    let home = TempDir::new().expect("temp home");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");

    let Json(body) = crate::handlers::config_patch::<TestHost>(
        Extension(TenantScope::default()),
        Query(InstanceQuery { directory: None }),
        HeaderMap::new(),
        Json(serde_json::json!({"default_provider": "global-provider"})),
    )
    .await
    .expect("global patch should succeed");

    assert_eq!(body["default_provider"], "global-provider");
    assert_eq!(config::get().await.default_provider.as_deref(), Some("global-provider"));
}

#[tokio::test]
async fn config_patch_without_directory_updates_only_the_requested_tenant() {
    let _env_lock = global_async_env_lock().lock().await;
    let home = TempDir::new().expect("temp home");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let tenant = TenantScope::new("desktop-platform", "desktop-company").expect("tenant");

    let Json(body) = crate::handlers::config_patch::<TestHost>(
        Extension(tenant.clone()),
        Query(InstanceQuery { directory: None }),
        HeaderMap::new(),
        Json(serde_json::json!({"default_provider": "tenant-provider"})),
    )
    .await
    .expect("tenant patch should succeed");

    assert_eq!(body["default_provider"], "tenant-provider");
    assert_eq!(
        config::get_global_for_tenant(&tenant).await.expect("tenant config").default_provider,
        Some("tenant-provider".to_string())
    );
    assert_ne!(config::get().await.default_provider, Some("tenant-provider".to_string()));
}

#[tokio::test]
async fn config_patch_uses_explicit_header_for_project_override() {
    let _env_lock = global_async_env_lock().lock().await;
    let home = TempDir::new().expect("temp home");
    let project = home.path().join("project");
    std::fs::create_dir(&project).expect("project dir");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-directory",
        HeaderValue::from_str(&project.to_string_lossy()).expect("project header"),
    );

    let Json(body) = crate::handlers::config_patch::<TestHost>(
        Extension(TenantScope::default()),
        Query(InstanceQuery { directory: None }),
        headers,
        Json(serde_json::json!({"default_provider": "header-project-provider"})),
    )
    .await
    .expect("header project patch should succeed");

    assert_eq!(body["default_provider"], "header-project-provider");
    let store = local_store(home.path());
    let override_value: Value = serde_json::from_str(
        &store
            .get_project(&TenantScope::default(), project.to_str().expect("project path"))
            .expect("read project config")
            .expect("project config"),
    )
    .expect("project config json");
    assert_eq!(override_value, serde_json::json!({"default_provider": "header-project-provider"}));
    assert!(!project.join(".vibewindow.json").exists());
}

#[tokio::test]
async fn config_patch_stateful_updates_active_runtime_config_without_explicit_directory() {
    let _env_lock = global_async_env_lock().lock().await;
    let home = TempDir::new().expect("temp home");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let state = state(config::Config::default(), false, &[]);

    let Json(body) = crate::handlers::config_patch_stateful::<TestHost>(
        Extension(TenantScope::default()),
        State(state.clone()),
        Query(InstanceQuery { directory: None }),
        HeaderMap::new(),
        Json(serde_json::json!({"default_provider": "openai"})),
    )
    .await
    .expect("active config patch should succeed");

    assert_eq!(body["default_provider"], "openai");
    assert_eq!(state.config.lock().default_provider.as_deref(), Some("openai"));
}

#[tokio::test]
async fn config_patch_stateful_leaves_runtime_config_when_directory_is_explicit() {
    let _env_lock = global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let home = TempDir::new().expect("temp home");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let state = state(config::Config::default(), false, &[]);
    let original_provider = state.config.lock().default_provider.clone();

    let Json(body) = crate::handlers::config_patch_stateful::<TestHost>(
        Extension(TenantScope::default()),
        State(state.clone()),
        instance_query(&temp),
        HeaderMap::new(),
        Json(serde_json::json!({"default_provider": "openai"})),
    )
    .await
    .expect("explicit directory config patch should succeed");

    assert_eq!(body["default_provider"], "openai");
    assert_eq!(state.config.lock().default_provider, original_provider);
}

#[tokio::test]
async fn config_patch_stateful_rejects_invalid_global_patch_without_mutating_runtime_config() {
    let _env_lock = global_async_env_lock().lock().await;
    let home = TempDir::new().expect("temp home");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let current = config::Config::default();
    let expected_provider = current.default_provider.clone();
    let state = state(current, false, &[]);

    let err = crate::handlers::config_patch_stateful::<TestHost>(
        Extension(TenantScope::default()),
        State(state.clone()),
        Query(InstanceQuery { directory: None }),
        HeaderMap::new(),
        Json(serde_json::json!("invalid-patch")),
    )
    .await
    .expect_err("non-object global patch should be rejected");

    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert!(err.message.contains("config patch must be a JSON object"));
    assert_eq!(state.config.lock().default_provider, expected_provider);
}

// 项目补丁只保存相对基础配置的差异，避免复制密钥或未来继承字段。
#[tokio::test]
async fn project_config_patch_writes_only_database_override() {
    let _env_lock = global_async_env_lock().lock().await;
    let home = TempDir::new().expect("temp home");
    let project_one = home.path().join("project-one");
    let project_two = home.path().join("project-two");
    std::fs::create_dir_all(&project_one).expect("first project");
    std::fs::create_dir_all(&project_two).expect("second project");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");

    config::update_global(serde_json::json!({
        "default_provider": "base-provider",
        "default_model": "base-model"
    }))
    .await
    .expect("seed base config");
    let store = local_store(home.path());
    let base_before = store.get(&TenantScope::default()).unwrap().expect("local base row");

    let Json(response) = crate::handlers::config_patch::<TestHost>(
        Extension(TenantScope::default()),
        directory_query(&project_one),
        HeaderMap::new(),
        Json(serde_json::json!({"default_provider": "project-provider"})),
    )
    .await
    .expect("project patch");

    assert_eq!(response["default_provider"], "project-provider");
    assert_eq!(response["default_model"], "base-model");
    let override_value: Value = serde_json::from_str(
        &store
            .get_project(&TenantScope::default(), project_one.to_str().expect("first project path"))
            .expect("read first project config")
            .expect("first project config"),
    )
    .expect("project config json");
    assert_eq!(override_value, serde_json::json!({"default_provider": "project-provider"}));
    assert!(!project_one.join(".vibewindow.json").exists());
    assert!(!project_two.join(".vibewindow.json").exists());
    assert_eq!(
        store
            .get_project(
                &TenantScope::default(),
                project_two.to_str().expect("second project path"),
            )
            .expect("read second project config"),
        None
    );
    assert_eq!(
        store.get(&TenantScope::default()).unwrap().expect("local base row after patch"),
        base_before
    );
}

#[tokio::test]
async fn project_config_patch_isolates_requested_tenant() {
    let _env_lock = global_async_env_lock().lock().await;
    let home = TempDir::new().expect("temp home");
    let project = home.path().join("project");
    std::fs::create_dir_all(&project).expect("project");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let tenant_a = TenantScope::new("platform", "company-a").expect("first tenant");
    let tenant_b = TenantScope::new("platform", "company-b").expect("second tenant");

    for (tenant, provider) in
        [(tenant_a.clone(), "project-provider-a"), (tenant_b.clone(), "project-provider-b")]
    {
        let Json(_) = crate::handlers::config_patch::<TestHost>(
            Extension(tenant),
            directory_query(&project),
            HeaderMap::new(),
            Json(serde_json::json!({"default_provider": provider})),
        )
        .await
        .expect("tenant project patch");
    }

    let Json(first) = crate::handlers::config_get::<TestHost>(Extension(tenant_a), directory_query(&project), HeaderMap::new())
        .await
        .expect("first tenant project config");
    let Json(second) = crate::handlers::config_get::<TestHost>(Extension(tenant_b), directory_query(&project), HeaderMap::new())
        .await
        .expect("second tenant project config");

    assert_eq!(first["default_provider"], "project-provider-a");
    assert_eq!(second["default_provider"], "project-provider-b");
    assert!(!project.join(".vibewindow.json").exists());
}

#[tokio::test]
async fn project_config_patch_never_persists_effective_base_fields() {
    let _env_lock = global_async_env_lock().lock().await;
    let home = TempDir::new().expect("temp home");
    let project = home.path().join("project");
    std::fs::create_dir_all(&project).expect("project");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");

    config::update_global(serde_json::json!({
        "default_provider": "base-provider",
        "default_model": "base-model"
    }))
    .await
    .expect("seed base config");
    let store = local_store(home.path());
    store
        .upsert_project(
            &TenantScope::default(),
            project.to_str().expect("project path"),
            &serde_json::json!({"default_temperature": 0.4}).to_string(),
        )
        .expect("seed project config");
    let base_before = store.get(&TenantScope::default()).unwrap().expect("local base row");

    let Json(_) = crate::handlers::config_patch::<TestHost>(
        Extension(TenantScope::default()),
        directory_query(&project),
        HeaderMap::new(),
        Json(serde_json::json!({"default_provider": "project-provider"})),
    )
    .await
    .expect("project patch");

    let override_value: Value = serde_json::from_str(
        &store
            .get_project(&TenantScope::default(), project.to_str().expect("project path"))
            .expect("read project config")
            .expect("project config"),
    )
    .expect("project config json");
    assert_eq!(
        override_value,
        serde_json::json!({
            "default_provider": "project-provider",
            "default_temperature": 0.4
        })
    );
    assert!(override_value.get("default_model").is_none());
    assert!(!project.join(".vibewindow.json").exists());
    assert_eq!(
        store.get(&TenantScope::default()).unwrap().expect("local base row after patch"),
        base_before
    );
}

#[tokio::test]
async fn config_patch_stateful_rejects_invalid_project_patch_without_mutating_runtime_config() {
    let _env_lock = global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", temp.path().as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let current = config::Config::default();
    let expected_provider = current.default_provider.clone();
    let state = state(current, false, &[]);

    let error = crate::handlers::config_patch_stateful::<TestHost>(
        Extension(TenantScope::default()),
        State(state.clone()),
        instance_query(&temp),
        HeaderMap::new(),
        Json(serde_json::json!("invalid-patch")),
    )
    .await
    .expect_err("non-object project patch should be rejected");

    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    assert!(error.message.contains("config patch must be a JSON object"));
    assert_eq!(state.config.lock().default_provider, expected_provider);
}
