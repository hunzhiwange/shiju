//! Gateway lifetime usage scanning over administrator-bound native sources.
use std::{collections::{HashMap, HashSet}, path::{Path, PathBuf}, sync::{Mutex, OnceLock}, time::Instant};
use vw_agent_apps::{sessions::{source::SourceRegistry, providers as p}, usage::{self, scan_jobs::*}};
use {
    vw_api_transport::ProjectScope,
    vw_api_external_session::*,
};

static SCHEDULES: OnceLock<Mutex<HashMap<ScanOwner, usage::scan_schedule::ScanSchedule>>> = OnceLock::new();
pub fn schedules() -> &'static Mutex<HashMap<ScanOwner, usage::scan_schedule::ScanSchedule>> {
    SCHEDULES.get_or_init(Default::default)
}

pub async fn run() {
    let mut timer = tokio::time::interval(std::time::Duration::from_secs(1));
    timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        timer.tick().await;
        if tokio::task::spawn_blocking(cycle).await.is_err() {
            tracing::warn!("usage scan worker failed");
        }
    }
}

fn cycle() {
    if cycle_inner().is_err() { tracing::debug!("usage scan unavailable or unauthorized"); }
}

fn cycle_inner() -> anyhow::Result<()> {
    let mut scopes: HashSet<_> = schedules().lock()
        .map_err(|_| anyhow::anyhow!("usage schedule unavailable"))?
        .keys().map(|owner| owner.scope.clone()).collect();
    // Scan the default tenant even before the desktop opens usage.
    scopes.insert(agent_usage::context::local_scope(vw_api_transport::TenantScope::default()));
    let mut targets = Vec::new();
    for scope in scopes {
        targets.extend(app_access_service::access::usage_scan_targets(&scope)
            .map_err(|_| anyhow::anyhow!("local usage sources unavailable"))?);
    }
    let database = agent_usage::context::database(&agent_paths::paths().home)?;
    let mut connection = database.connect_sqlite()?;
    usage::store::initialize_usage_store(&mut connection)?;
    usage::cursors::initialize_usage_cursors(&connection)?;
    usage::source_settings::initialize_source_settings(&connection)?;
    let host = agent_usage::context::local_host()?;
    let mut groups: HashMap<ScanOwner, Vec<_>> = HashMap::new();
    for (scope, principal, home, instance) in targets {
        groups.entry(ScanOwner { scope, principal, target_host: host.clone() })
            .or_default().push((home, instance));
    }
    let mut schedules = schedules().lock().map_err(|_| anyhow::anyhow!("usage schedule unavailable"))?;
    // Retain registered owners and job snapshots when no source is installed.
    for owner in schedules.keys() {
        groups.entry(owner.clone()).or_default();
    }
    let mut executions = Vec::new();
    for (owner, instances) in groups {
        let schedule = schedules.entry(owner.clone()).or_insert_with(||
            usage::scan_schedule::ScanSchedule::new(ScanJobs::default(), owner.clone()));
        schedule.set_preferences(usage::source_settings::load_scan_preferences(
            &connection, &owner.scope, &host,
        )?);
        let mut has_enabled_source = false;
        for (adapter, app, provider) in ADAPTERS {
            let settings = usage::source_settings::load_source_settings(&connection, &owner.scope, &host, app)?.settings;
            has_enabled_source |= settings.scan_enabled
                && instances.iter().any(|(_, source)| source.provider == provider);
            schedule.set_source(adapter, settings);
        }
        if let Some(job) = schedule.tick(Instant::now(), has_enabled_source)
            .map_err(|_| anyhow::anyhow!("scan unavailable"))?
        {
            executions.push((owner, instances, schedule.clone(), job));
        }
    }
    drop(schedules);
    for (owner, instances, schedule, job) in executions {
            let mut all_files: HashMap<ScanAdapter, Vec<PathBuf>> = HashMap::new();
            let mut pending: HashMap<ScanAdapter, std::vec::IntoIter<Work>> = HashMap::new();
            schedule.execute(&job, |adapter| {
                let Some((_, app, provider)) = ADAPTERS.iter().find(|(a,_,_)| *a == adapter) else {
                    return ScanStep::Failed(ScanFailure::InvalidSource);
                };
                if let std::collections::hash_map::Entry::Vacant(e) = pending.entry(adapter) {
                    let discovered = discover(&owner.scope, &host, *provider, &instances);
                    match discovered {
                        Ok(files) => { all_files.insert(adapter, files.iter().map(|work| work.path.clone()).collect()); e.insert(files.into_iter()); }
                        Err(_) => return ScanStep::Failed(ScanFailure::SourceUnavailable),
                    }
                }
                let all = all_files.get(&adapter).cloned().unwrap_or_default();
                let Some(work) = pending.get_mut(&adapter).and_then(Iterator::next) else { return ScanStep::End };
                let key = usage::cursors::UsageCursorKey { target_host: host.clone(), app: (*app).into(),
                    instance_id: work.identity.instance_id.clone(), file_identity: match &work.identity.source {
                        SessionSource::Sqlite { database, .. } => database.clone(),
                        SessionSource::File { relative_path } if adapter == ScanAdapter::Codex =>
                            work.path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_else(|| relative_path.clone()),
                        SessionSource::File { relative_path } => relative_path.clone(),
                    } };
                match dispatch(&mut connection, &owner.scope, &key, &work, all) {
                    Ok(imported) => ScanStep::Committed(ScanCounts { imported: imported as u64, files_scanned: 1, ..Default::default() }),
                    Err(error) if adapter == ScanAdapter::Codex => {
                        use usage::cursors::UsageCursorError;
                        match error.downcast_ref::<UsageCursorError>() {
                            Some(UsageCursorError::Io(_) | UsageCursorError::SourceChangedDuringScan) =>
                                ScanStep::FileFailed(ScanFailure::SourceUnavailable),
                            Some(UsageCursorError::InvalidRecord) =>
                                ScanStep::FileFailed(ScanFailure::InvalidSource),
                            Some(UsageCursorError::Storage(_)) =>
                                ScanStep::Failed(ScanFailure::StorageUnavailable),
                            _ => ScanStep::Failed(ScanFailure::SourceUnavailable),
                        }
                    }
                    Err(_) => ScanStep::Failed(ScanFailure::SourceUnavailable),
                }
            }).map_err(|_| anyhow::anyhow!("scan execution unavailable"))?;
    }
    Ok(())
}

pub const ADAPTERS: [(ScanAdapter, &str, SessionProvider); 11] = [
    (ScanAdapter::Claude,"claude",SessionProvider::Claude), (ScanAdapter::Codex,"codex",SessionProvider::Codex),
    (ScanAdapter::Gemini,"gemini",SessionProvider::Gemini), (ScanAdapter::OpenCode,"opencode",SessionProvider::Opencode),
    (ScanAdapter::GrokBuild,"grokbuild",SessionProvider::Grokbuild), (ScanAdapter::Pi,"pi",SessionProvider::Pi),
    (ScanAdapter::Omp,"omp",SessionProvider::Omp), (ScanAdapter::MCode,"mcode",SessionProvider::Mcode),
    (ScanAdapter::Agy,"agy",SessionProvider::Agy), (ScanAdapter::Kimi,"kimi",SessionProvider::Kimi),
    (ScanAdapter::Dsh,"dsh",SessionProvider::Dsh),
];

pub struct Work {
    pub path: PathBuf,
    root: PathBuf,
    pub identity: ExternalSessionIdentity,
    registry: SourceRegistry,
}
pub fn discover(scope: &ProjectScope, host: &str, provider: SessionProvider,
    instances: &[(PathBuf, app_access_service::access::ExternalHistoryInstance)]) -> anyhow::Result<Vec<Work>> {
    let mut files = Vec::new();
    for (home, instance) in instances.iter().filter(|(_, i)| i.provider == provider) {
        let template = ExternalSessionIdentity { scope: scope.clone(), target_host: host.into(),
            instance_id: instance.instance_id.clone(), provider, session_id: ExternalSessionId("scan".into()),
            source: SessionSource::File { relative_path: "scan".into() } };
        let (sessions, truncated) = super::scan::scan(&instance.root, home, &template)
            .map_err(|_| anyhow::anyhow!("source discovery failed"))?;
        anyhow::ensure!(!truncated, "source discovery incomplete");
        let root = match provider {
            SessionProvider::Pi => match p::pi::session_roots(&p::pi::PiSessionContext {
                home, configured_agent_dir: Some(&instance.root), env_agent_dir: None,
                env_session_dir: None, project_dir: None,
            }) { p::pi::SessionRootResolution::Available { root, .. } => root, _ => anyhow::bail!("Pi source unavailable") },
            SessionProvider::Omp => match p::omp::session_roots(&p::omp::OmpSessionContext {
                agent_dir: &instance.root, native_agent_dir: &instance.root,
                profile: None, xdg_data_home: None, env_session_dir: None,
            }) { p::pi::SessionRootResolution::Available { root, .. } => root, _ => anyhow::bail!("OMP source unavailable") },
            _ => instance.root.clone(),
        };
        let mut seen = HashSet::new();
        for session in sessions {
            let relative = match &session.identity.source {
                SessionSource::File { relative_path } => relative_path,
                SessionSource::Sqlite { database, .. } => database,
            };
            let path = root.join(relative);
            if matches!(provider, SessionProvider::Pi | SessionProvider::Omp) {
                let shared = instances.iter().any(|(_, other)| other.root == instance.root
                    && other.provider != provider && matches!(other.provider, SessionProvider::Pi | SessionProvider::Omp));
                if shared && usage::omp::opens_with_title_slot(&path)? != (provider == SessionProvider::Omp) { continue; }
            }
            if !seen.insert(path.clone()) { continue; }
            let mut registry = SourceRegistry::new(scope.clone(), host.into(), instance.instance_id.clone(), provider, vec![root.clone()])?;
            registry.register(&session, path.clone())?;
            registry.resolve(&session.identity)?;
            files.push(Work { path, root: instance.root.clone(), identity: session.identity, registry });
        }
    }
    Ok(files)
}

fn dispatch(connection: &mut rusqlite::Connection, scope: &ProjectScope,
    key: &usage::cursors::UsageCursorKey, work: &Work, all: Vec<PathBuf>) -> anyhow::Result<usize> {
    let source = work.registry.resolve(&work.identity)?;
    let path = source.path();
    let file = || std::fs::File::open(path);
    Ok(match work.identity.provider {
        SessionProvider::Claude => usage::claude_scan::scan_session(connection, &work.registry, &work.identity)?.records,
        SessionProvider::Codex => usage::snapshot_scan::scan_codex(connection, scope, key, path, all)?,
        SessionProvider::Pi | SessionProvider::Omp => usage::snapshot_scan::scan_pi(connection, scope, key, path)?,
        SessionProvider::Gemini => usage::gemini::scan_gemini_file(connection, scope, key, &mut file()?, |_| Ok(None))?.imported,
        SessionProvider::Grokbuild => usage::grokbuild::scan_grokbuild_file(connection, scope, key, &mut file()?, runtime_time::unix_seconds(), |_| Ok(None))?.imported,
        SessionProvider::Kimi => {
            let agent = path.parent().and_then(Path::file_name).and_then(|s| s.to_str())
                .ok_or_else(|| anyhow::anyhow!("Kimi agent unavailable"))?;
            usage::kimi::scan_kimi_file(connection, scope, key, &mut file()?, &work.identity.session_id.0, agent, |_| Ok(None))?.imported
        },
        SessionProvider::Dsh => usage::dsh::scan_dsh_file(connection, scope, key,
            &work.identity.session_id.0, &mut file()?, path.extension().is_some_and(|s| s == "zst"))?.records,
        SessionProvider::Mcode => usage::mcode::scan_mcode_database(connection, scope, key, &work.root, |_| Ok(None))?.imported,
        SessionProvider::Opencode => usage::opencode::scan_opencode_database(connection, scope, key, &work.root, |_| Ok(None))?.imported,
        SessionProvider::Agy => usage::agy::scan_agy_transcript(connection, scope, key, path, |_| Ok(None))?.imported,
        _ => anyhow::bail!("unsupported usage source"),
    })
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Activity {
    active: Option<bool>,
    paused: Option<bool>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ScanRequest {}

pub(super) fn router() -> axum::Router {
    axum::Router::new()
        .route("/usage/activity", axum::routing::post(page_activity))
        .route("/usage/scans", axum::routing::post(manual_scan))
}
pub fn authorized_owner(policy: Option<&Path>, tenant: vw_api_transport::TenantScope,
    headers: &axum::http::HeaderMap) -> Result<ScanOwner, axum::http::StatusCode> {
    let scope = agent_usage::context::local_scope(tenant);
    let target_host = agent_usage::context::local_host()
        .map_err(|_| axum::http::StatusCode::SERVICE_UNAVAILABLE)?;
    let (principal, _, _) = app_access_service::access::authorize_external_history(
        policy, headers, &scope, &target_host)?;
    let owner = ScanOwner { scope, principal, target_host };
    schedules().lock().map_err(|_| axum::http::StatusCode::SERVICE_UNAVAILABLE)?
        .entry(owner.clone()).or_insert_with(||
            usage::scan_schedule::ScanSchedule::new(ScanJobs::default(), owner.clone()));
    Ok(owner)
}
async fn page_activity(
    axum::Extension(policy): axum::Extension<super::catalog::Policy>,
    axum::Extension(tenant): axum::Extension<vw_api_transport::TenantScope>,
    headers: axum::http::HeaderMap, axum::Json(activity): axum::Json<Activity>,
) -> Result<axum::http::StatusCode, axum::http::StatusCode> {
    tokio::task::spawn_blocking(move || {
        let owner = authorized_owner(policy.0.as_deref(), tenant, &headers)?;
        let mut schedules = schedules().lock().map_err(|_| axum::http::StatusCode::SERVICE_UNAVAILABLE)?;
        let schedule = schedules.get_mut(&owner).ok_or(axum::http::StatusCode::SERVICE_UNAVAILABLE)?;
        if let Some(active) = activity.active { schedule.set_page_active(active); }
        if let Some(paused) = activity.paused { schedule.set_paused(paused); }
        Ok(axum::http::StatusCode::NO_CONTENT)
    }).await.map_err(|_| axum::http::StatusCode::SERVICE_UNAVAILABLE)?
}
async fn manual_scan(
    axum::Extension(policy): axum::Extension<super::catalog::Policy>,
    axum::Extension(tenant): axum::Extension<vw_api_transport::TenantScope>,
    headers: axum::http::HeaderMap,
    axum::Json(_): axum::Json<ScanRequest>,
) -> Result<(axum::http::StatusCode, axum::Json<ScanJob>), axum::http::StatusCode> {
    tokio::task::spawn_blocking(move || {
        let owner = authorized_owner(policy.0.as_deref(), tenant, &headers)?;
        let mut schedules = schedules().lock().map_err(|_| axum::http::StatusCode::SERVICE_UNAVAILABLE)?;
        let schedule = schedules.get_mut(&owner).ok_or(axum::http::StatusCode::SERVICE_UNAVAILABLE)?;
        let job = schedule.manual_sync(Instant::now()).map_err(|_| axum::http::StatusCode::SERVICE_UNAVAILABLE)?;
        Ok((axum::http::StatusCode::ACCEPTED, axum::Json(job)))
    }).await.map_err(|_| axum::http::StatusCode::SERVICE_UNAVAILABLE)?
}
