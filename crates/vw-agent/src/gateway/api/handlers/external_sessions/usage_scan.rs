//! Gateway lifetime usage scanning over administrator-bound native sources.
use std::{collections::{HashMap, HashSet}, path::{Path, PathBuf}, sync::{Mutex, OnceLock}, time::Instant};
use vw_agent_apps::{sessions::{source::SourceRegistry, providers as p}, usage::{self, scan_jobs::*}};
use vw_api_types::{ProjectScope, external_session::*};

static SCHEDULES: OnceLock<Mutex<HashMap<ScanOwner, usage::scan_schedule::ScanSchedule>>> = OnceLock::new();
fn schedules() -> &'static Mutex<HashMap<ScanOwner, usage::scan_schedule::ScanSchedule>> {
    SCHEDULES.get_or_init(Default::default)
}

pub(crate) async fn run(policy: PathBuf) {
    let mut timer = tokio::time::interval(std::time::Duration::from_secs(1));
    timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        timer.tick().await;
        let policy = policy.clone();
        if tokio::task::spawn_blocking(move || cycle(&policy)).await.is_err() {
            tracing::warn!("usage scan worker failed");
        }
    }
}

fn cycle(policy: &Path) {
    if cycle_inner(policy).is_err() { tracing::debug!("usage scan unavailable or unauthorized"); }
}

fn cycle_inner(policy: &Path) -> anyhow::Result<()> {
    let targets = super::super::agent_apps::usage_scan_targets(policy)
        .map_err(|_| anyhow::anyhow!("usage scan policy unavailable"))?;
    let database = crate::usage_bridge::context::database()?;
    let projects = crate::project::store::ProjectStore::new(database.clone())?;
    let mut connection = database.connect_sqlite()?;
    usage::store::initialize_usage_store(&mut connection)?;
    usage::cursors::initialize_usage_cursors(&connection)?;
    usage::source_settings::initialize_source_settings(&connection)?;
    let host = crate::usage_bridge::context::local_host()?;
    let mut groups: HashMap<ScanOwner, Vec<_>> = HashMap::new();
    for (scope, principal, home, instance) in targets {
        if projects.get(&scope)?.is_none() { continue; }
        groups.entry(ScanOwner { scope, principal, target_host: host.clone() })
            .or_default().push((home, instance));
    }
    let mut schedules = schedules().lock().map_err(|_| anyhow::anyhow!("usage schedule unavailable"))?;
    let owners: HashSet<_> = groups.keys().cloned().collect();
    schedules.retain(|owner, _| owners.contains(owner));
    let mut executions = Vec::new();
    for (owner, instances) in groups {
        let project = projects.get(&owner.scope)?.ok_or_else(|| anyhow::anyhow!("usage project unavailable"))?;
        let schedule = schedules.entry(owner.clone()).or_insert_with(||
            usage::scan_schedule::ScanSchedule::new(ScanJobs::default(), owner.clone()));
        for (adapter, app, _) in ADAPTERS {
            let settings = usage::source_settings::load_source_settings(&connection, &owner.scope, &host, app)?.settings;
            schedule.set_source(adapter, settings);
        }
        if let Some(job) = schedule.tick(Instant::now()).map_err(|_| anyhow::anyhow!("scan unavailable"))? {
            executions.push((owner, instances, project.worktree, schedule.clone(), job));
        }
    }
    drop(schedules);
    for (owner, instances, worktree, schedule, job) in executions {
            let mut all_files: HashMap<ScanAdapter, Vec<PathBuf>> = HashMap::new();
            let mut pending: HashMap<ScanAdapter, std::vec::IntoIter<Work>> = HashMap::new();
            schedule.execute(&job, |adapter| {
                let Some((_, app, provider)) = ADAPTERS.iter().find(|(a,_,_)| *a == adapter) else {
                    return ScanStep::Failed(ScanFailure::InvalidSource);
                };
                if !pending.contains_key(&adapter) {
                    let discovered = discover(&owner.scope, &host, *provider, &instances, &worktree);
                    match discovered {
                        Ok(files) => { all_files.insert(adapter, files.iter().map(|work| work.path.clone()).collect()); pending.insert(adapter, files.into_iter()); }
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
                    Err(_) => ScanStep::Failed(ScanFailure::SourceUnavailable),
                }
            }).map_err(|_| anyhow::anyhow!("scan execution unavailable"))?;
    }
    Ok(())
}

const ADAPTERS: [(ScanAdapter, &str, SessionProvider); 11] = [
    (ScanAdapter::Claude,"claude",SessionProvider::Claude), (ScanAdapter::Codex,"codex",SessionProvider::Codex),
    (ScanAdapter::Gemini,"gemini",SessionProvider::Gemini), (ScanAdapter::OpenCode,"opencode",SessionProvider::Opencode),
    (ScanAdapter::GrokBuild,"grokbuild",SessionProvider::Grokbuild), (ScanAdapter::Pi,"pi",SessionProvider::Pi),
    (ScanAdapter::Omp,"omp",SessionProvider::Omp), (ScanAdapter::MCode,"mcode",SessionProvider::Mcode),
    (ScanAdapter::Agy,"agy",SessionProvider::Agy), (ScanAdapter::Kimi,"kimi",SessionProvider::Kimi),
    (ScanAdapter::Dsh,"dsh",SessionProvider::Dsh),
];

struct Work {
    path: PathBuf,
    root: PathBuf,
    identity: ExternalSessionIdentity,
    registry: SourceRegistry,
}
fn discover(scope: &ProjectScope, host: &str, provider: SessionProvider,
    instances: &[(PathBuf, super::super::agent_apps::ExternalHistoryInstance)],
    worktree: &str) -> anyhow::Result<Vec<Work>> {
    let mut files = Vec::new();
    for (home, instance) in instances.iter().filter(|(_, i)| i.provider == provider) {
        let template = ExternalSessionIdentity { scope: scope.clone(), target_host: host.into(),
            instance_id: instance.instance_id.clone(), provider, session_id: ExternalSessionId("scan".into()),
            source: SessionSource::File { relative_path: "scan".into() } };
        let (sessions, truncated) = super::scan::scan(&instance.root, home, &template)
            .map_err(|_| anyhow::anyhow!("source discovery failed"))?;
        anyhow::ensure!(!truncated, "source discovery incomplete");
        // Database scanners consume a whole native database. Refuse mixed or
        // unknown project ownership instead of assigning another project's rows.
        if matches!(provider, SessionProvider::Mcode | SessionProvider::Opencode) {
            anyhow::ensure!(sessions.iter().all(|s| s.project_dir.as_deref() == Some(worktree)),
                "database has unbound project sessions");
        }
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
        for session in sessions.into_iter().filter(|s| s.project_dir.as_deref() == Some(worktree)) {
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
        SessionProvider::Grokbuild => usage::grokbuild::scan_grokbuild_file(connection, scope, key, &mut file()?, chrono::Utc::now().timestamp(), |_| Ok(None))?.imported,
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
struct Intent {
    project_id: vw_api_types::id::ProjectId,
    target_host: String,
    active: Option<bool>,
    paused: Option<bool>,
}
pub(super) fn router() -> axum::Router {
    axum::Router::new()
        .route("/usage/activity", axum::routing::post(page_activity))
        .route("/usage/scans", axum::routing::post(manual_scan))
}
fn authorized_owner(policy: &super::catalog::Policy, tenant: vw_api_types::TenantScope,
    headers: &axum::http::HeaderMap, intent: &Intent) -> Result<ScanOwner, axum::http::StatusCode> {
    let scope = ProjectScope::new(tenant, intent.project_id.clone()).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    let (principal, _, _) = super::super::agent_apps::authorize_external_history(
        policy.0.as_deref(), headers, &scope, &intent.target_host)?;
    Ok(ScanOwner { scope, principal, target_host: intent.target_host.clone() })
}
async fn page_activity(
    axum::Extension(policy): axum::Extension<super::catalog::Policy>,
    axum::Extension(tenant): axum::Extension<vw_api_types::TenantScope>,
    headers: axum::http::HeaderMap, axum::Json(intent): axum::Json<Intent>,
) -> Result<axum::http::StatusCode, axum::http::StatusCode> {
    tokio::task::spawn_blocking(move || {
        let owner = authorized_owner(&policy, tenant, &headers, &intent)?;
        let mut schedules = schedules().lock().map_err(|_| axum::http::StatusCode::SERVICE_UNAVAILABLE)?;
        let schedule = schedules.get_mut(&owner).ok_or(axum::http::StatusCode::SERVICE_UNAVAILABLE)?;
        if let Some(active) = intent.active { schedule.set_page_active(active); }
        if let Some(paused) = intent.paused { schedule.set_paused(paused); }
        Ok(axum::http::StatusCode::NO_CONTENT)
    }).await.map_err(|_| axum::http::StatusCode::SERVICE_UNAVAILABLE)?
}
async fn manual_scan(
    axum::Extension(policy): axum::Extension<super::catalog::Policy>,
    axum::Extension(tenant): axum::Extension<vw_api_types::TenantScope>,
    headers: axum::http::HeaderMap, axum::Json(intent): axum::Json<Intent>,
) -> Result<(axum::http::StatusCode, axum::Json<ScanJob>), axum::http::StatusCode> {
    tokio::task::spawn_blocking(move || {
        let owner = authorized_owner(&policy, tenant, &headers, &intent)?;
        let mut schedules = schedules().lock().map_err(|_| axum::http::StatusCode::SERVICE_UNAVAILABLE)?;
        let schedule = schedules.get_mut(&owner).ok_or(axum::http::StatusCode::SERVICE_UNAVAILABLE)?;
        let job = schedule.manual_sync(Instant::now()).map_err(|_| axum::http::StatusCode::SERVICE_UNAVAILABLE)?;
        Ok((axum::http::StatusCode::ACCEPTED, axum::Json(job)))
    }).await.map_err(|_| axum::http::StatusCode::SERVICE_UNAVAILABLE)?
}
