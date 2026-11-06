use super::*;
use vw_api_types::{agent_apps::*, app_providers::ProviderProtocol};
fn profile(key: &str) -> AppProviderWrite {
    AppProviderWrite {
        name: "fixture".into(),
        protocol: ProviderProtocol::Anthropic,
        api_key: key.into(),
        base_url: "https://provider.example.test".into(),
        model: "fixture-model".into(),
    }
}
#[test]
fn cc_apps_provider_launch_isolation_store_reopen_tenant_app_and_host_boundaries() {
    let home = tempfile::tempdir().unwrap();
    let db = GlobalDatabase::for_home(home.path()).unwrap();
    let a = TenantScope::new("p", "a").unwrap();
    let b = TenantScope::new("p", "b").unwrap();
    let c = TenantScope::new("other", "a").unwrap();
    let store = AppProviderStore::new(db).unwrap();
    store.save(&a, AppId::Claude, "selected", &profile("fixture-key-only")).unwrap();
    store.save(&b, AppId::Claude, "selected", &profile("other-key-only")).unwrap();
    drop(store);
    let store = AppProviderStore::new(GlobalDatabase::for_home(home.path()).unwrap()).unwrap();
    assert!(store.get_summary(&c, AppId::Claude, "selected").is_err());
    assert!(store.get_summary(&a, AppId::Codex, "selected").is_err());
    assert!(store.get_summary(&a, AppId::Claude, "missing").is_err());
    let summary =
        serde_json::to_string(&store.get_summary(&a, AppId::Claude, "selected").unwrap()).unwrap();
    assert!(!summary.contains("fixture-key-only"));
    let root = tempfile::tempdir().unwrap();
    let program = root.path().join("claude");
    std::fs::write(&program, "#!/bin/sh\ntest \"$ANTHROPIC_API_KEY\" = fixture-key-only || exit 31\nprintf '%s' \"$CLAUDE_CONFIG_DIR\" > private-path\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let request = ProviderLaunchRequest {
        provider_id: "selected".into(),
        launch: LaunchRequest {
            app_id: AppId::Claude,
            installation_id: AppInstallationId("observed".into()),
            cwd: root.path().to_str().unwrap().into(),
            args: vec![],
            skip_permissions: false,
        },
    };
    let installation = AppInstallation {
        id: request.launch.installation_id.clone(),
        app_id: AppId::Claude,
        target_host_id: "host".into(),
        environment: if cfg!(target_os = "macos") {
            AppInstallationEnvironment::Macos
        } else if cfg!(windows) {
            AppInstallationEnvironment::Windows
        } else {
            AppInstallationEnvironment::Linux
        },
        path: program.to_str().unwrap().into(),
        real_path: None,
        source: AppInstallationSource::Native,
        is_path_default: true,
        version: None,
        runtime_status: AppInstallationRuntimeStatus::Runnable,
    };
    let prepare = |tenant, r: &ProviderLaunchRequest, host| {
        store.prepare_launch(
            tenant,
            r,
            host,
            std::slice::from_ref(&installation),
            &[root.path().into()],
            false,
            root.path(),
        )
    };
    assert!(prepare(&c, &request, "host").is_err());
    assert!(prepare(&a, &request, "wrong-host").is_err());
    let mut unsafe_request = request.clone();
    unsafe_request.launch.skip_permissions = true;
    assert!(prepare(&a, &unsafe_request, "host").is_err());
    prepare(&a, &request, "host").unwrap().run().unwrap();
    let private = std::fs::read_to_string(root.path().join("private-path")).unwrap();
    assert!(!Path::new(&private).exists());
    // Same ID in another tenant selects only that tenant's key; fixture rejects it.
    assert!(prepare(&b, &request, "host").unwrap().run().is_err());
}
