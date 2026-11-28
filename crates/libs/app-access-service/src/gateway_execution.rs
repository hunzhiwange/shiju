//! Execution metadata for application routes authenticated by the ShiJu Gateway.
use super::*;

pub(super) fn gateway_host(tenant: &TenantScope) -> Result<Host, StatusCode> {
    let paths = agent_paths::paths();
    let home = paths.home.clone();
    let scope = serde_json::to_vec(&(tenant.platform_id().as_ref(), tenant.company_id().as_ref()))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let legacy = local_app_auth::auth_policy_path(
        &home,
        tenant.platform_id().as_ref(),
        tenant.company_id().as_ref(),
    )
    .with_file_name("auth-credentials");
    let root = if legacy.is_dir() {
        legacy
    } else {
        paths.data.join("apps").join(hex::encode(Sha256::digest(scope)))
    };
    for directory in [
        &root,
        &paths.config.join("apps"),
    ] {
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            std::fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(directory)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        }
        #[cfg(not(unix))]
        std::fs::create_dir_all(directory).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    }
    let mut search_paths: Vec<_> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).filter(|p| p.is_absolute()).collect())
        .unwrap_or_default();
    for p in [
        paths.bin.clone(),
        home.join(".local/bin"),
        home.join(".cargo/bin"),
        home.join(".bun/bin"),
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
    ] {
        if !search_paths.contains(&p) {
            search_paths.push(p);
        }
    }
    let lifecycle = search_paths
        .iter()
        .map(|p| p.join(if cfg!(windows) { "npm.cmd" } else { "npm" }))
        .find(|p| p.is_file())
        .map(|npm| Lifecycle { prefix: paths.config.join("apps"), npm });
    let applications = if cfg!(target_os = "macos") {
        PathBuf::from("/Applications")
    } else {
        home.join(".local/share/applications")
    };
    Ok(Host {
        id: local_app_auth::HOST_ID.into(),
        online: true,
        binding: Binding::Local,
        applications,
        search_paths,
        lifecycle,
        home: home.clone(),
        credential_root: Some(root.clone()),
        _legacy_data_backup_root: (),
        _legacy_data_location_roots: (),
        _legacy_data_migration_root: (),
        _legacy_data_migration_sources: (),
        external_session_instances: native_instances(&home),
        _legacy_skills_root: (),
        _legacy_skill_install_files: (),
        pi_directory: None,
        github_enterprise_authorities: Vec::new(),
        native_login_owners: Vec::new(),
        #[cfg(target_os = "macos")]
        routing_startup: None,
    })
}

fn native_instances(home: &Path) -> Vec<ExternalHistoryInstance> {
    use vw_api_external_session::SessionProvider::*;
    [
        (Claude, ".claude"),
        (Codex, ".codex"),
        (Gemini, ".gemini"),
        (Opencode, ".local/share/opencode"),
        (Hermes, ".hermes"),
        (Pi, ".pi/agent"),
        (Openclaw, ".openclaw"),
        (Grokbuild, ".grok"),
        (Kimi, ".kimi"),
        (Omp, ".omp/agent"),
        (Mcode, ".minimax"),
        (Dsh, ".dsh"),
    ]
    .into_iter()
    .filter_map(|(provider, relative)| {
        let root = home.join(relative);
        root.is_dir().then_some(ExternalHistoryInstance {
            instance_id: "default".into(),
            provider,
            root,
            resume: None,
        })
    })
    .collect()
}
