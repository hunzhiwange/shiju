//! Native resume is a separately authorized execution-host operation.
use super::catalog::Policy;
use axum::{Extension, Json, http::{HeaderMap, StatusCode}};
use vw_api_types::{TenantScope, external_session::*};

pub(super) async fn resume(
    Extension(policy): Extension<Policy>, Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap, Json(request): Json<SessionResumeRequest>,
) -> Result<Json<SessionResumeResponse>, StatusCode> {
    if request.identity.scope.tenant_scope() != &tenant { return Err(StatusCode::FORBIDDEN); }
    vw_agent_apps::sessions::identity::validate_identity(&request.identity).map_err(|_| StatusCode::BAD_REQUEST)?;
    vw_agent_apps::apps::lifecycle::coordinator::ToolLifecycleCoordinator.run(
        vec![super::delete::app(request.identity.provider)], move || Ok(execute(policy, headers, request, false)),
    ).await.map_err(|_| StatusCode::CONFLICT)?.and_then(|result| match result { Outcome::Resumed(value) => Ok(Json(value)), Outcome::Preview(_) => Err(StatusCode::INTERNAL_SERVER_ERROR) })
}
enum Outcome { Resumed(SessionResumeResponse), Preview(SessionResumeCommandResponse) }

pub(super) async fn preview(
    Extension(policy): Extension<Policy>, Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap, Json(request): Json<SessionResumeRequest>,
) -> Result<Json<SessionResumeCommandResponse>, StatusCode> {
    if request.identity.scope.tenant_scope() != &tenant { return Err(StatusCode::FORBIDDEN); }
    vw_agent_apps::sessions::identity::validate_identity(&request.identity).map_err(|_| StatusCode::BAD_REQUEST)?;
    vw_agent_apps::apps::lifecycle::coordinator::ToolLifecycleCoordinator.run(
        vec![super::delete::app(request.identity.provider)], move || Ok(execute(policy, headers, request, true)),
    ).await.map_err(|_| StatusCode::CONFLICT)?.and_then(|result| match result {
        Outcome::Preview(value) => Ok(Json(value)), Outcome::Resumed(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    })
}
#[cfg(not(target_os = "macos"))]
fn execute(_: Policy, _: HeaderMap, _: SessionResumeRequest, _: bool) -> Result<Outcome, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}
#[cfg(target_os = "macos")]
fn execute(policy: Policy, headers: HeaderMap, request: SessionResumeRequest, preview: bool) -> Result<Outcome, StatusCode> {
    use vw_agent_apps::{apps::{discovery::read::{self, LocalReadEnvironment}, launch::request::LaunchMaterial}, sessions::{resume::{build_resume_plan, ResumeContext}, source::SourceRegistry, providers::{omp, pi}, terminal as t}};
    let id = &request.identity;
    let (host, instance, principal, agy_owner) = super::super::agent_apps::authorize_external_history_resume(
        policy.0.as_deref(), &headers, id, request.terminal == SessionTerminal::Custom)?;
    if matches!(id.provider, SessionProvider::Hermes | SessionProvider::Openclaw | SessionProvider::Dsh) { return Err(StatusCode::NOT_IMPLEMENTED); }
    let settings = instance.resume.as_ref().ok_or(StatusCode::PRECONDITION_FAILED)?;
    let omp = if id.provider == SessionProvider::Omp { Some(omp::OmpSessionContext {
        agent_dir: &instance.root,
        native_agent_dir: settings.native_agent_dir.as_deref().ok_or(StatusCode::PRECONDITION_FAILED)?,
        profile: settings.profile.as_deref(), xdg_data_home: settings.xdg_data_home.as_deref(),
        env_session_dir: settings.session_dir.as_deref(),
    }) } else { None };
    let (sessions, source_root) = if let Some(context) = &omp {
        let root = match omp::session_roots(context) { pi::SessionRootResolution::Available { root, .. } => root, _ => return Err(StatusCode::UNPROCESSABLE_ENTITY) };
        (omp::scan_sessions(context, id).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?, root)
    } else {
        let (sessions, _) = super::scan::scan(&instance.root, &host.home, id).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let root = if id.provider == SessionProvider::Pi {
            match pi::session_roots(&pi::PiSessionContext { home: &host.home, configured_agent_dir: Some(&instance.root), env_agent_dir: None, env_session_dir: None, project_dir: None }) {
                pi::SessionRootResolution::Available { root, .. } => root, _ => return Err(StatusCode::UNPROCESSABLE_ENTITY),
            }
        } else { instance.root.clone() };
        (sessions, root)
    };
    let session = sessions.into_iter().find(|s| s.identity == *id).ok_or(StatusCode::NOT_FOUND)?;
    let relative = match &id.source { SessionSource::File { relative_path } => relative_path, SessionSource::Sqlite { database, .. } => database };
    let mut registry = SourceRegistry::new(id.scope.clone(), id.target_host.clone(), id.instance_id.clone(), id.provider, vec![source_root.clone()]).map_err(|_| StatusCode::FORBIDDEN)?;
    registry.register(&session, source_root.join(relative)).map_err(|_| StatusCode::FORBIDDEN)?;
    let observed = read::read(super::delete::app(id.provider), &LocalReadEnvironment { host_id: host.id.clone(), home: host.home.clone(), search_paths: host.search_paths.clone(), applications: host.applications.clone() }).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let installation = observed.installations.iter().find(|i| i.id == settings.installation_id).ok_or(StatusCode::FAILED_DEPENDENCY)?;
    let plan = build_resume_plan(&registry, id, installation, &ResumeContext {
        execution_home: &host.home, search_paths: &host.search_paths, data_dir: &instance.root,
        authorized_project_roots: &settings.project_roots, omp,
        pi_config_dir: settings.pi_config_dir.as_deref(), xdg_state_home: settings.xdg_state_home.as_deref(), xdg_cache_home: settings.xdg_cache_home.as_deref(),
    }).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    if preview {
        let command = plan.copy_command().map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        return Ok(Outcome::Preview(SessionResumeCommandResponse { command }));
    }
    let agy = if id.provider == SessionProvider::Agy {
        let db = crate::global_database::GlobalDatabase::for_home(&crate::global::paths().home).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let store = crate::app_providers::AppProviderStore::new(db).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let scope = crate::agent_apps::auth_accounts::AuthAccountScope { tenant: id.scope.tenant_scope(), principal: &principal, host: &host.id, service: vw_api_types::agent_apps_auth::AuthService::AgyOauth };
        Some(store.prepare_agy_launch(scope, vw_agent_apps::auth::agy_account::AgyNativeTarget { home: &host.home, exclusive_owner: agy_owner.then_some(scope) }, settings.agy_provider_id.as_deref().ok_or(StatusCode::PRECONDITION_FAILED)?, host.credential_root.as_deref().ok_or(StatusCode::PRECONDITION_FAILED)?, LaunchMaterial { program: plan.program().into(), cwd: plan.cwd().into(), args: vec![], skip_permissions: false }).map_err(|_| StatusCode::FORBIDDEN)?)
    } else { None };
    let credentials = |_: &ExternalSessionIdentity, command: &mut std::process::Command| {
        agy.as_ref().ok_or("AGY credentials unavailable".to_owned())?.apply_resume_credentials(command)
    };
    let result = match request.terminal {
        SessionTerminal::Terminal => t::apple::launch_resume(&plan, t::apple::AppleTerminal::Terminal, credentials),
        SessionTerminal::Iterm2 => t::apple::launch_resume(&plan, t::apple::AppleTerminal::Iterm2, credentials),
        SessionTerminal::Ghostty => t::ghostty_kitty::launch_resume(&plan, t::ghostty_kitty::Terminal::Ghostty, credentials),
        SessionTerminal::Kitty => t::ghostty_kitty::launch_resume(&plan, t::ghostty_kitty::Terminal::Kitty, credentials),
        SessionTerminal::Wezterm => t::wezterm_kaku::launch_resume(&plan, t::wezterm_kaku::Terminal::WezTerm, credentials),
        SessionTerminal::Kaku => t::wezterm_kaku::launch_resume(&plan, t::wezterm_kaku::Terminal::Kaku, credentials),
        SessionTerminal::Otty => t::otty::launch_resume(&plan, credentials),
        SessionTerminal::Alacritty => t::alacritty::launch_resume(&plan, credentials),
        SessionTerminal::Warp => t::warp::launch_resume(&plan, credentials),
        SessionTerminal::Custom => {
            let custom = settings.custom_terminal.as_ref().ok_or(StatusCode::PRECONDITION_FAILED)?;
            let terminal = t::custom::AuthorizedCustomTerminal::from_authorized_host_settings(custom.program.clone(), custom.arguments.clone()).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            t::custom::launch_resume(&plan, &terminal, credentials)
        }
    };
    result.map_err(|error| if error.contains("not installed") { StatusCode::FAILED_DEPENDENCY } else { StatusCode::BAD_GATEWAY })?;
    Ok(Outcome::Resumed(SessionResumeResponse { completed: true }))
}
