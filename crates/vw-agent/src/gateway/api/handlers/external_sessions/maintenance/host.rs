//! Host input resolution and retained ledger ownership; no client path is opened.
use super::*;
use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    ffi::OsStr,
    fs::{self, OpenOptions},
    io,
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
use vw_agent_apps::{
    providers::live::write_guard::lock_app,
    sessions::{
        maintenance::{HistorySnapshot, MaintenanceAction, MaintenanceContext, MaintenanceOutcome},
        migration::unify::{BackupGeneration, GenerationBinding},
        source::SourceRegistry,
    },
};
use vw_api_types::{agent_apps::AppId, task::TaskStatus};

/// Shared by the history-options owner. Defaults never authorize migration.
pub(super) type Options = HistoryOptions;
fn storage() -> Result<GatewaySettingsStore, StatusCode> {
    GlobalDatabase::for_home(&crate::global::paths().home)
        .and_then(GatewaySettingsStore::new)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}
fn key(target: &HistoryMaintenanceTarget) -> Result<String, StatusCode> {
    let bytes = serde_json::to_vec(target).map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
pub(super) fn options(target: &HistoryMaintenanceTarget) -> Result<Options, StatusCode> {
    storage()?
        .get(target.scope.tenant_scope(), "external_history_options", &key(target)?)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .map(|s| serde_json::from_str(&s).map_err(|_| StatusCode::SERVICE_UNAVAILABLE))
        .transpose()
        .map(|o| o.unwrap_or_default())
}
#[derive(Serialize, Deserialize)]
struct StoredProgress {
    executor_pid: u32,
    progress: HistoryMaintenanceProgress,
}
fn record_key(
    target: &HistoryMaintenanceTarget,
    owner: &str,
    root: &Path,
    id: &str,
) -> Result<String, StatusCode> {
    if uuid::Uuid::parse_str(id).is_err() {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(format!(
        "{}:{:x}:{:x}:{id}",
        key(target)?,
        Sha256::digest(owner.as_bytes()),
        Sha256::digest(root.as_os_str().as_encoded_bytes())
    ))
}
pub(super) fn save(
    target: &HistoryMaintenanceTarget,
    owner: &str,
    root: &Path,
    progress: &HistoryMaintenanceProgress,
) -> Result<(), StatusCode> {
    storage()?
        .upsert(
            target.scope.tenant_scope(),
            "external_history_jobs",
            &record_key(target, owner, root, &progress.operation_id)?,
            &serde_json::to_string(&StoredProgress {
                executor_pid: std::process::id(),
                progress: progress.clone(),
            })
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        )
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}
pub(super) fn load(
    target: &HistoryMaintenanceTarget,
    owner: &str,
    root: &Path,
    id: &str,
) -> Result<HistoryMaintenanceProgress, StatusCode> {
    let text = storage()?
        .get(
            target.scope.tenant_scope(),
            "external_history_jobs",
            &record_key(target, owner, root, id)?,
        )
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let stored: StoredProgress =
        serde_json::from_str(&text).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let mut p = stored.progress;
    if p.phase != "terminal" {
        // Another live executor may own this operation. A missing local handle
        // cannot establish interruption or grant cross-process cancellation.
        if stored.executor_pid > 0
            && (unsafe { libc::kill(stored.executor_pid as i32, 0) } == 0
                || io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH))
        {
            return Err(StatusCode::CONFLICT);
        }
        p.status = TaskStatus::Failed;
        p.phase = "terminal".into();
        p.interrupted = true;
        save(target, owner, root, &p)?;
    }
    Ok(p)
}
/// Returns the authenticated principal and the exact registered Codex root.
pub(in super::super) fn authorize(
    policy: &Policy,
    headers: &HeaderMap,
    target: &HistoryMaintenanceTarget,
) -> Result<(String, PathBuf, PathBuf, PathBuf), StatusCode> {
    let (principal, home, instances, private) =
        super::super::super::agent_apps::authorize_external_history_maintenance(
            policy.0.as_deref(),
            headers,
            &target.scope,
            &target.target_host,
        )?;
    let instance = instances
        .into_iter()
        .find(|i| i.provider == SessionProvider::Codex && i.instance_id == target.instance_id)
        .ok_or(StatusCode::FORBIDDEN)?;
    if fs::canonicalize(&instance.root).ok().as_ref() != Some(&instance.root) {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok((principal, home, instance.root, private))
}
fn unavailable(_: StatusCode) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "maintenance authorization unavailable")
}
fn private_child(parent: &Path, name: &str) -> io::Result<PathBuf> {
    let path = parent.join(name);
    match fs::DirBuilder::new().mode(0o700).create(&path) {
        Ok(()) => (),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (),
        Err(e) => return Err(e),
    }
    let meta = fs::symlink_metadata(&path)?;
    if !meta.is_dir()
        || meta.uid() != unsafe { libc::geteuid() }
        || meta.mode() & 0o077 != 0
        || fs::canonicalize(&path)? != path
    {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "invalid private ledger"));
    }
    Ok(path)
}
fn config(root: &Path) -> io::Result<String> {
    // Drop the non-reentrant configuration guard before Context::execute.
    let guard =
        lock_app(AppId::Codex, root).map_err(|_| io::Error::other("configuration unavailable"))?;
    let bytes = guard
        .read(OsStr::new("config.toml"))
        .map_err(|_| io::Error::other("configuration unavailable"))?
        .unwrap_or_default();
    String::from_utf8(bytes).map_err(|_| io::Error::other("invalid configuration"))
}
fn active(id: &ExternalSessionIdentity) -> io::Result<()> {
    if vw_agent_apps::sessions::activity::is_active(id)? {
        return Err(io::Error::new(io::ErrorKind::WouldBlock, "Codex is active"));
    }
    Ok(())
}
pub(super) fn execute(
    policy: &Policy,
    headers: &HeaderMap,
    owner: &str,
    expected_root: &Path,
    request: &HistoryMaintenanceRequest,
    generation_id: &str,
    operation_id: &str,
    snapshots: &super::super::catalog::Snapshots,
    context: &MaintenanceContext,
) -> io::Result<MaintenanceOutcome> {
    let target = &request.target;
    let (principal, home, root, private) =
        authorize(policy, headers, target).map_err(unavailable)?;
    if principal != owner || root != expected_root {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "principal changed"));
    }
    // A private administrator-owned transaction root also anchors the cross-process
    // history reservation. Do not hold AppWriteGuard here: migrations acquire it.
    let ledger = private_child(&private, "external-history")?;
    let root_hash = format!("{:x}", Sha256::digest(root.as_os_str().as_encoded_bytes()));
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(ledger.join(format!("{root_hash}.lock")))?;
    use std::os::fd::AsRawFd;
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err(io::Error::new(io::ErrorKind::WouldBlock, "maintenance busy"));
    }
    let ledger =
        private_child(&ledger, &format!("{}-{root_hash}", key(target).map_err(unavailable)?))?;
    let config_text = config(&root)?;
    let options = options(target).map_err(unavailable)?;
    let seed = ExternalSessionIdentity {
        scope: target.scope.clone(),
        target_host: target.target_host.clone(),
        instance_id: target.instance_id.clone(),
        provider: SessionProvider::Codex,
        session_id: ExternalSessionId("maintenance".into()),
        source: SessionSource::File { relative_path: "maintenance".into() },
    };
    active(&seed)?;
    let databases = vw_agent_apps::sessions::codex_state::codex_state_db_paths(
        &root,
        &config_text,
        &home,
        None,
    )
    .map_err(|_| io::Error::other("invalid database configuration"))?;
    // This permit covers only the registered application root, never a sqlite_home
    // escape into another application's data.
    if databases.iter().any(|p| !p.starts_with(&root)) {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "database outside instance"));
    }
    let sessions = vw_agent_apps::sessions::providers::codex::scan_sessions(
        &root,
        &config_text,
        &home,
        None,
        &seed,
    )?;
    let mut registry = SourceRegistry::new(
        target.scope.clone(),
        target.target_host.clone(),
        target.instance_id.clone(),
        SessionProvider::Codex,
        vec![root.clone()],
    )?;
    let mut identities = Vec::new();
    for session in sessions {
        let relative = match &session.identity.source {
            SessionSource::File { relative_path } => relative_path,
            SessionSource::Sqlite { database, .. } => database,
        };
        registry.register(&session, root.join(relative))?;
        identities.push(session.identity);
    }
    let binding = GenerationBinding {
        scope: target.scope.clone(),
        target_host: target.target_host.clone(),
        instance_id: target.instance_id.clone(),
        codex_root: root.clone(),
    };
    let source_path = ledger.join(generation_id);
    if request.generation_id.is_some() && !source_path.is_dir() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "generation unavailable"));
    }
    let path = private_child(&ledger, generation_id)?;
    let mut generation = BackupGeneration::open(&path, binding.clone())?;
    let history = HistorySnapshot {
        registry: &registry,
        binding: &binding,
        identities: &identities,
        database_paths: &databases,
        live_config: &config_text,
    };
    let mut current = || {
        let (principal, _, fresh_root, fresh_private) =
            authorize(policy, headers, target).map_err(unavailable)?;
        active(&seed)?;
        Ok(principal == owner
            && root == fresh_root
            && private == fresh_private
            && config(&root)? == config_text
            && self::options(target).map_err(unavailable)? == options)
    };
    snapshots.deletion_attempted(&seed).map_err(unavailable)?;
    let result = match request.action {
        HistoryMaintenanceAction::Unify => context.execute(
            &history,
            MaintenanceAction::Unify {
                generation: &mut generation,
                enabled: options.unify_enabled,
                migrate_existing_requested: options.migrate_existing,
            },
            &mut current,
        ),
        HistoryMaintenanceAction::LegacyOfficial => context.execute(
            &history,
            MaintenanceAction::LegacyOfficial { backup: &mut generation },
            &mut current,
        ),
        HistoryMaintenanceAction::Restore => {
            let undo_path = private_child(&ledger, operation_id)?;
            let mut undo = BackupGeneration::open(&undo_path, binding.clone())?;
            context.execute(
                &history,
                MaintenanceAction::Restore {
                    generations: &[generation],
                    undo: &mut undo,
                    unify_enabled: options.unify_enabled,
                },
                &mut current,
            )
        }
    };
    // Expire cursors even if a native mutation failed after a partial commit.
    snapshots.deletion_attempted(&seed).map_err(unavailable)?;
    result
}

pub(super) fn set_options(target: &HistoryMaintenanceTarget, options: &Options) -> Result<(), StatusCode> {
    storage()?.upsert(
        target.scope.tenant_scope(), "external_history_options", &key(target)?,
        &serde_json::to_string(options).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
    ).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}

/// Query only the server-selected private ledger; never create a generation.
pub(super) fn backups(target: &HistoryMaintenanceTarget, root: &Path, private: &Path) -> Result<HistoryBackups, StatusCode> {
    let root_hash = format!("{:x}", Sha256::digest(root.as_os_str().as_encoded_bytes()));
    let ledger = private.join("external-history");
    let directory = ledger.join(format!("{}-{root_hash}", key(target)?));
    for path in [&ledger, &directory] {
        let meta = match fs::symlink_metadata(path) {
            Ok(meta) => meta,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(HistoryBackups { generations: Vec::new() });
            }
            Err(_) => return Err(StatusCode::SERVICE_UNAVAILABLE),
        };
        if fs::canonicalize(path).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)? != *path
            || !meta.is_dir() || meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o077 != 0 {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    }
    let binding = GenerationBinding {
        scope: target.scope.clone(), target_host: target.target_host.clone(),
        instance_id: target.instance_id.clone(), codex_root: root.to_owned(),
    };
    let mut generations = Vec::new();
    for (index, entry) in fs::read_dir(&directory).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.enumerate() {
        if index >= 4096 { return Err(StatusCode::SERVICE_UNAVAILABLE); }
        let entry = entry.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let name = entry.file_name();
        let Some(id) = name.to_str() else { return Err(StatusCode::SERVICE_UNAVAILABLE); };
        let parsed = uuid::Uuid::parse_str(id).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        if parsed.to_string() != id { return Err(StatusCode::SERVICE_UNAVAILABLE); }
        if let Some(meta) = BackupGeneration::inspect(&entry.path(), &binding).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)? {
            if !meta.entries.is_empty() {
                generations.push(HistoryBackupSummary { generation_id: id.into(), complete: meta.complete, entries: meta.entries.len() });
            }
        }
    }
    generations.sort_by(|a, b| a.generation_id.cmp(&b.generation_id));
    Ok(HistoryBackups { generations })
}
