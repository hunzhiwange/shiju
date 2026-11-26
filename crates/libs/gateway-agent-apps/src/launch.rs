//! Terminal launches are resolved on the granted local execution host.
use app_settings_store::settings_store::GatewaySettingsStore;
use agent_database::GlobalDatabase;
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path as RoutePath, Query},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use vw_agent_apps::apps::launch::{
    posix::launch_posix_terminal,
    request::{LaunchMaterial, validate_launch_request},
};
use {
    vw_api_app_identity::LaunchRequest,
    vw_api_app_identity::LaunchResult,
    vw_api_transport::id::TenantScope,
};
#[derive(Clone)]
pub(super) struct Context {
    pub(super) policy: Option<PathBuf>,
    sessions: super::launch_sessions::Sessions,
    terminal: fn(Option<&str>, &Path, &LaunchMaterial) -> Result<(), String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Target {
    target_host_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    #[serde(flatten)]
    launch: LaunchRequest,
    provider_id: Option<String>,
    account_id: Option<String>,
}
pub fn router() -> Router {
    router_with_terminal(
        std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from),
        launch_posix_terminal,
    )
}
pub fn router_with_terminal(
    policy: Option<PathBuf>,
    terminal: fn(Option<&str>, &Path, &LaunchMaterial) -> Result<(), String>,
) -> Router {
    Router::new()
        .route("/agent-apps/launch", post(launch))
        .route("/agent-apps/launch-identities", get(super::launch_identities::list))
        .route("/agent-apps/launch-sessions/{id}", get(session_status))
        .layer(DefaultBodyLimit::max(32768))
        .layer(Extension(Context { policy, terminal, sessions: Default::default() }))
}
async fn launch(
    Extension(context): Extension<Context>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Query(target): Query<Target>,
    Json(request): Json<Request>,
) -> Result<Json<LaunchResult>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let authorization = super::access::authorize_identity_launch(context.policy.as_deref(), &headers, &tenant, &target.target_host_id)?;
        if (request.provider_id.is_some() && request.account_id.is_some()) || request.launch.skip_permissions { return Err(StatusCode::UNPROCESSABLE_ENTITY); }
        authorize_native_login(&authorization, request.launch.app_id)?;
        #[cfg(target_os="macos")]
        let agy_owner = context.policy.is_some() && authorization.native_login_owner(vw_api_app_auth::AuthService::AgyOauth) == Some(true);
        let host = authorization.host; let principal = authorization.principal;
        let cwd = PathBuf::from(&request.launch.cwd);
        let root = host.home.canonicalize().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        if !cwd.is_absolute()
            || !cwd.canonicalize().is_ok_and(|p| p.is_dir() && p.starts_with(&root))
            || request.launch.cwd.chars().any(char::is_control)
        {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        #[cfg(not(unix))]
        {
            let _ = (host, database, principal);
            return Err(StatusCode::NOT_IMPLEMENTED);
        }
        #[cfg(unix)]
        {
            let observed = discover_installations(
                request.launch.app_id, &host.id, &host.home, &host.search_paths, host.applications,
            )?;
            let db = match database {
                Some(Extension(db)) => db,
                None => GlobalDatabase::for_home(&agent_paths::paths().home)
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
            };
            let store =
                GatewaySettingsStore::new(db.clone()).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            let preferences: vw_api_app_identity::AppHostPreferences = store
                .get(&tenant, "agent_apps_host", &host.id)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
                .map(|s| serde_json::from_str(&s))
                .transpose()
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
                .unwrap_or_default();
            let session_id = if request.provider_id.is_some() || request.account_id.is_some() {
                let terminal = context.terminal;
                let preferred = preferences.preferred_terminal;
                let scope = super::launch_sessions::Scope { tenant: tenant.clone(), host: host.id.clone(), principal };
                if let Some(provider_id) = request.provider_id {
                    let store=app_provider_services::AppProviderStore::new(db).map_err(|_|StatusCode::SERVICE_UNAVAILABLE)?;
                    #[cfg(target_os="macos")]
                    let bound = match request.launch.app_id {
                        vw_api_app_identity::AppId::Claude | vw_api_app_identity::AppId::Codex => {
                            let service = match request.launch.app_id {
                                vw_api_app_identity::AppId::Claude => vw_api_app_auth::AuthService::ClaudeOfficial,
                                _ => vw_api_app_auth::AuthService::CodexOauth,
                            };
                            let binding_scope = agent_app_services::auth_accounts::AuthAccountScope { tenant: &tenant, principal: &scope.principal, host: &host.id, service };
                            let material = validate_launch_request(&request.launch,&host.id,&observed.installations,std::slice::from_ref(&root),false)
                                .map_err(|_|StatusCode::UNPROCESSABLE_ENTITY)?;
                            store.prepare_bound_official_launch(binding_scope, &provider_id, host.credential_root.as_deref(), material)
                                .map_err(|_|StatusCode::UNPROCESSABLE_ENTITY)?
                        }
                        _ => None,
                    };
                    #[cfg(not(target_os="macos"))]
                    let bound: Option<agent_app_services::auth_accounts::AuthAccountLaunch> = None;
                    if let Some(prepared) = bound {
                        Some(context.sessions.start(scope, move |ready| {
                            let transport = vw_agent_apps::apps::launch::terminal_session::TerminalSession::connect_with(Path::new("/tmp"), |_, material| terminal(preferred.as_deref(), Path::new("/bin/sh"), material))?;
                            prepared.run_with(|command| transport.run(command, move || { let _ = ready.send(()); }))
                        })?)
                    } else {
                    #[cfg(target_os="macos")]
                    let agy=if request.launch.app_id==vw_api_app_identity::AppId::Agy {
                        let material=validate_launch_request(&request.launch,&host.id,&observed.installations,std::slice::from_ref(&root),false).map_err(|_|StatusCode::UNPROCESSABLE_ENTITY)?;
                        let account_scope = agent_app_services::auth_accounts::AuthAccountScope { tenant: &tenant, principal: &scope.principal, host: &host.id, service: vw_api_app_auth::AuthService::AgyOauth };
                        let native = vw_agent_apps::auth::agy_account::AgyNativeTarget { home: &host.home, exclusive_owner: agy_owner.then_some(account_scope) };
                        Some(store.prepare_agy_launch(account_scope,native,&provider_id,host.credential_root.as_deref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?,material).map_err(|_|StatusCode::UNPROCESSABLE_ENTITY)?)
                    }else{None};
                    #[cfg(not(target_os="macos"))]
                    let agy: Option<vw_agent_apps::apps::launch::provider::ProviderLaunch>=None;
                    let prepared=if let Some(prepared)=agy{prepared}else{store.prepare_launch(&tenant, &vw_api_app_provider_base::ProviderLaunchRequest { provider_id, launch: request.launch.clone() }, &host.id, &observed.installations, &[root], false, Path::new("/tmp"))
                        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?};
                    Some(context.sessions.start(scope, move |ready| {
                        let transport = vw_agent_apps::apps::launch::terminal_session::TerminalSession::connect_with(Path::new("/tmp"), |_, material| terminal(preferred.as_deref(), Path::new("/bin/sh"), material))?;
                        prepared.run_with(|command| transport.run(command, move || { let _ = ready.send(()); }))
                    })?)
                    }
                } else {
                    let credential_root = host.credential_root.ok_or(StatusCode::NOT_IMPLEMENTED)?;
                    let prepared = agent_app_services::auth_accounts::AuthAccountStore::new(db, &credential_root)
                        .and_then(|store| store.prepare_launch(&tenant, &host.id, request.account_id.as_deref().ok_or("account missing")?, &request.launch, &observed.installations, &[root], Path::new("/tmp")))
                        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
                    Some(context.sessions.start(scope, move |ready| {
                        let transport = vw_agent_apps::apps::launch::terminal_session::TerminalSession::connect_with(Path::new("/tmp"), |_, material| terminal(preferred.as_deref(), Path::new("/bin/sh"), material))?;
                        prepared.run_with(|command| transport.run(command, move || { let _ = ready.send(()); }))
                    })?)
                }
            } else {
                let material = validate_launch_request(&request.launch, &host.id, &observed.installations, std::slice::from_ref(&root), false).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
                let mut command = vw_agent_apps::apps::launch::host::command(material, &root, &host.search_paths).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
                let terminal = context.terminal;
                let preferred = preferences.preferred_terminal;
                let scope = super::launch_sessions::Scope { tenant, host: host.id, principal };
                Some(context.sessions.start(scope, move |ready| {
                    let transport = vw_agent_apps::apps::launch::terminal_session::TerminalSession::connect_with(Path::new("/tmp"), |_, material| terminal(preferred.as_deref(), Path::new("/bin/sh"), material))?;
                    let status = transport.run(&mut command, move || { let _ = ready.send(()); }).map_err(|_| "execution-host process failed")?;
                    if !status.success() { return Err("execution-host process exited unsuccessfully".into()); }
                    Ok(())
                })?)
            };
            Ok(Json(LaunchResult { session_id, installation_id: request.launch.installation_id }))
        }
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
async fn session_status(
    Extension(context): Extension<Context>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Query(target): Query<Target>,
    RoutePath(id): RoutePath<String>,
) -> Result<Json<vw_api_app_identity::LaunchSessionStatus>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let (host, principal) = super::access::authorize_launch_session(
            context.policy.as_deref(),
            &headers,
            &tenant,
            &target.target_host_id,
        )?;
        context
            .sessions
            .get(&super::launch_sessions::Scope { tenant, host: host.id, principal }, &id)
            .map(Json)
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}


fn authorize_native_login(
    authorization: &super::access::Authorization,
    app_id: vw_api_app_identity::AppId,
) -> Result<(), StatusCode> {
    use vw_api_app_auth::AuthService;
    use vw_api_app_identity::AppId;

    // An explicitly scoped native slot must not be reachable through ordinary launch.
    let service = match app_id {
        AppId::Claude => Some(AuthService::ClaudeOfficial),
        AppId::Agy => Some(AuthService::AgyOauth),
        _ => None,
    };
    if service.is_some_and(|service| authorization.native_login_owner(service) == Some(false)) {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(())
}

#[cfg(unix)]
fn discover_installations(
    app_id: vw_api_app_identity::AppId,
    host_id: &str,
    home: &Path,
    search_paths: &[PathBuf],
    applications: PathBuf,
) -> Result<vw_api_app_identity::AppReadResponse, StatusCode> {
    use vw_agent_apps::apps::discovery::read::{LocalReadEnvironment, read};

    read(app_id, &LocalReadEnvironment {
        host_id: host_id.to_owned(),
        home: home.to_owned(),
        search_paths: search_paths.to_vec(),
        applications,
    }).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}

#[cfg(target_os = "macos")]
fn workbench_authorization(tenant: &TenantScope) -> Result<super::access::Authorization, StatusCode> {
    let policy = std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from);
    let authorization = super::access::authorize_identity_launch(
        policy.as_deref(), &HeaderMap::new(), tenant, local_app_auth::HOST_ID,
    )?;
    if authorization.host.id != local_app_auth::HOST_ID {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(authorization)
}

#[cfg(target_os = "macos")]
fn workbench_installation(
    observed: &vw_api_app_identity::AppReadResponse,
) -> Option<&vw_api_app_identity::AppInstallation> {
    use vw_api_app_identity::AppInstallationRuntimeStatus;

    let mut runnable = observed.installations.iter().filter(|installation| {
        installation.runtime_status == AppInstallationRuntimeStatus::Runnable
    });
    let first = runnable.next()?;
    if first.is_path_default { Some(first) } else {
        Some(runnable.find(|candidate| candidate.is_path_default).unwrap_or(first))
    }
}

#[cfg(target_os = "macos")]
pub fn workbench_native_apps(tenant: &TenantScope) -> Vec<vw_api_types::cli_workbench::CliWorkbenchNativeApp> {
    use vw_api_app_identity::AppRuntimeKind;
    use vw_api_types::cli_workbench::CliWorkbenchNativeApp;

    let Ok(authorization) = workbench_authorization(tenant) else { return Vec::new() };
    let host = &authorization.host;
    vw_agent_apps::apps::catalog::catalog().into_iter().filter_map(|app| {
        let entry = app.metadata;
        if !entry.runtime_kinds.contains(&AppRuntimeKind::NativeCli)
            || authorize_native_login(&authorization, entry.id).is_err()
        {
            return None;
        }
        let observed = discover_installations(
            entry.id, &host.id, &host.home, &host.search_paths, host.applications.clone(),
        ).ok()?;
        workbench_installation(&observed)?;
        let serde_json::Value::String(id) = serde_json::to_value(entry.id).ok()? else { return None };
        Some(CliWorkbenchNativeApp { id, name: entry.name })
    }).collect()
}

/// Build the selected native CLI or interactive Shell on the authorized local host.
#[cfg(target_os = "macos")]
pub fn workbench_command(
    tenant: &TenantScope,
    target: &vw_api_types::cli_workbench::CliWorkbenchTarget,
    directory: &str,
) -> Result<std::process::Command, StatusCode> {
    workbench_command_inner(tenant, target, directory, None)
}

/// The executable is resolved on the gateway from the installed managed runtime.
#[cfg(target_os = "macos")]
pub fn workbench_managed_codex_command(
    tenant: &TenantScope,
    target: &vw_api_types::cli_workbench::CliWorkbenchTarget,
    directory: &str,
    executable: PathBuf,
) -> Result<std::process::Command, StatusCode> {
    workbench_command_inner(tenant, target, directory, Some(executable))
}

#[cfg(target_os = "macos")]
fn workbench_command_inner(
    tenant: &TenantScope,
    target: &vw_api_types::cli_workbench::CliWorkbenchTarget,
    directory: &str,
    managed_codex: Option<PathBuf>,
) -> Result<std::process::Command, StatusCode> {
    use vw_api_app_identity::{AppId, AppRuntimeKind};
    use vw_api_types::cli_workbench::CliWorkbenchTarget;

    let authorization = workbench_authorization(tenant)?;
    let host = &authorization.host;
    let root = host.home.canonicalize().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let cwd = Path::new(directory).canonicalize().map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    if cwd != Path::new(directory)
        || !cwd.is_dir()
        || !cwd.starts_with(&root)
        || directory.chars().any(char::is_control)
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let material = match target {
        CliWorkbenchTarget::ManagedCodex { .. } => {
            authorize_native_login(&authorization, AppId::Codex)?;
            LaunchMaterial {
                program: managed_codex.ok_or(StatusCode::UNPROCESSABLE_ENTITY)?,
                cwd,
                args: vec![
                    "--no-alt-screen".into(),
                    "--no-daemon".into(),
                    "-c".into(),
                    "shell_environment_policy.exclude=[\"SHIJU_CODEX_RELAY_TOKEN\"]".into(),
                ],
                skip_permissions: false,
            }
        }
        CliWorkbenchTarget::Shell => LaunchMaterial {
            program: PathBuf::from("/bin/zsh"),
            cwd,
            args: vec!["-i".into()],
            skip_permissions: false,
        },
        CliWorkbenchTarget::NativeCli { app_id } => {
            let app_id: AppId = serde_json::from_value(serde_json::Value::String(app_id.clone()))
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            if !vw_agent_apps::apps::catalog::catalog().iter().any(|app| {
                app.metadata.id == app_id
                    && app.metadata.runtime_kinds.contains(&AppRuntimeKind::NativeCli)
            }) {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            authorize_native_login(&authorization, app_id)?;
            let observed = discover_installations(
                app_id, &host.id, &host.home, &host.search_paths, host.applications.clone(),
            )?;
            let installation = workbench_installation(&observed).ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
            let request = LaunchRequest {
                app_id,
                installation_id: installation.id.clone(),
                cwd: directory.to_owned(),
                args: Vec::new(),
                skip_permissions: false,
            };
            let mut material = validate_launch_request(
                &request, &host.id, &observed.installations, std::slice::from_ref(&root), false,
            ).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            // These fixed arguments belong to the authorized local workbench launcher.
            material.args.extend(workbench_native_args(app_id).iter().map(|&flag| flag.to_owned()));
            if app_id == AppId::Codex && installation.version.as_deref().is_some_and(codex_supports_no_daemon) {
                material.args.push("--no-daemon".into());
            }
            material
        }
        CliWorkbenchTarget::Acp { .. } | CliWorkbenchTarget::Ssh { .. } => {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
    };
    let mut command = vw_agent_apps::apps::launch::host::command(material, &root, &host.search_paths)
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    if !["LANG", "LC_ALL", "LC_CTYPE"].into_iter()
        .any(|key| std::env::var_os(key).is_some_and(|value| !value.is_empty()))
    {
        command.env("LC_CTYPE", "en_US.UTF-8");
    }
    Ok(command)
}

#[cfg(target_os = "macos")]
fn codex_supports_no_daemon(version: &str) -> bool {
    let core = version.split(['-', '+']).next().unwrap_or(version);
    let mut numbers = core.split('.').map(str::parse::<u32>);
    match (numbers.next(), numbers.next(), numbers.next()) {
        (Some(Ok(major)), Some(Ok(minor)), Some(Ok(patch))) => {
            (major, minor, patch) > (0, 157, 0)
                || ((major, minor, patch) == (0, 157, 0) && !version.contains('-'))
        }
        _ => false,
    }
}

#[cfg(target_os = "macos")]
pub(super) fn workbench_native_args(app_id: vw_api_app_identity::AppId) -> &'static [&'static str] {
    use vw_api_app_identity::AppId;

    match app_id {
        // Preserve the embedded terminal transcript in scrollback.
        AppId::Codex => &["--no-alt-screen", "--dangerously-bypass-approvals-and-sandbox"],
        AppId::Claude => &["--dangerously-skip-permissions"],
        _ => &[],
    }
}
