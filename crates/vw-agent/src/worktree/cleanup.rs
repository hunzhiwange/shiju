//! Cleanup proposals and guarded host maintenance; proposals never authorize deletion.
use super::registry::Record;
use serde::{Deserialize, Serialize};
use vw_api_types::id::ProjectScope;
use vw_config_types::worktree::WorktreeSettings;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProtectionReason {
    Pinned,
    Busy,
    Initializing,
    Pool,
    Unknown,
    SharedRepository,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub id: String,
    pub fingerprint: String,
    pub title: String,
    pub last_used_at_ms: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Protected {
    pub id: String,
    pub reasons: Vec<ProtectionReason>,
}
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub candidates: Vec<Candidate>,
    pub protected: Vec<Protected>,
    pub remaining_excess: usize,
}

pub(super) struct Observation {
    pub record: Record,
    pub title: String,
    pub last_used_at_ms: u64,
    pub reasons: Vec<ProtectionReason>,
}

pub(super) fn plan(
    scope: &ProjectScope,
    settings: &WorktreeSettings,
    mut entries: Vec<Observation>,
) -> Proposal {
    if !settings.auto_cleanup_enabled {
        return Proposal::default();
    }
    entries.retain(|entry| {
        entry.record.scope == *scope && entry.record.state == super::registry::Lifecycle::Ready
    });
    entries
        .sort_by(|a, b| (a.last_used_at_ms, &a.record.id).cmp(&(b.last_used_at_ms, &b.record.id)));
    let ordinary =
        entries.iter().filter(|entry| !entry.reasons.contains(&ProtectionReason::Pool)).count();
    let mut excess = ordinary.saturating_sub(settings.keep_count as usize);
    let mut proposal = Proposal::default();
    for entry in entries {
        if !entry.reasons.is_empty() {
            proposal.protected.push(Protected { id: entry.record.id, reasons: entry.reasons });
        } else if excess > 0 {
            proposal.candidates.push(Candidate {
                fingerprint: fingerprint(&entry, settings),
                id: entry.record.id,
                title: entry.title,
                last_used_at_ms: entry.last_used_at_ms,
            });
            excess -= 1;
        }
    }
    proposal.remaining_excess = excess;
    proposal
}

fn fingerprint(entry: &Observation, settings: &WorktreeSettings) -> String {
    use sha2::{Digest, Sha256};
    let record = &entry.record;
    let mut hash = Sha256::new();
    // Length-delimited components preserve native path bytes without JSON path panics.
    for bytes in [
        b"shiju-cleanup-v1".as_slice(),
        record.id.as_bytes(),
        record.scope.platform_id().as_ref().as_bytes(),
        record.scope.company_id().as_ref().as_bytes(),
        record.scope.project_id().as_ref().as_bytes(),
        record.directory.as_os_str().as_encoded_bytes(),
        record.common_dir.as_os_str().as_encoded_bytes(),
        record.git_dir.as_os_str().as_encoded_bytes(),
        record.checkout_identity.as_bytes(),
        record.common_identity.as_bytes(),
        record.git_identity.as_bytes(),
        entry.title.as_bytes(),
        settings.root.as_bytes(),
    ] {
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    }
    hash.update(record.created_at_ms.to_le_bytes());
    hash.update(record.last_used_at_ms.to_le_bytes());
    hash.update(entry.last_used_at_ms.to_le_bytes());
    hash.update(settings.keep_count.to_le_bytes());
    hash.update([
        settings.auto_cleanup_enabled as u8,
        settings.ask_before_delete as u8,
        settings.fetch_before_create as u8,
    ]);
    hex::encode(hash.finalize())
}

use super::{Error, environment, registry};
use crate::session::{session::Info, status, ui_store};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

pub(super) struct ProtectionSnapshot {
    pub sessions: Vec<Info>,
    pub foreign_directories: Vec<String>,
    pub pinned: HashMap<String, HashSet<String>>,
    pub statuses: HashMap<String, status::Info>,
}

fn invalid(error: impl std::fmt::Display) -> Error {
    Error::Invalid(error.to_string())
}

pub(super) fn read_snapshot(scope: &ProjectScope) -> Result<ProtectionSnapshot, Error> {
    let (sessions, foreign_directories) =
        ui_store::load_worktree_protection_sessions(scope.tenant_scope()).map_err(invalid)?;
    let mut projects: HashSet<String> =
        sessions.iter().map(|session| session.project_id.clone()).collect();
    projects.insert(scope.project_id().as_ref().to_owned());
    let pinned = projects
        .into_iter()
        .map(|project| {
            let pins =
                ui_store::load_pinned_threads(scope.tenant_scope(), &project).map_err(invalid)?;
            Ok((project, pins))
        })
        .collect::<Result<_, Error>>()?;
    Ok(ProtectionSnapshot {
        sessions,
        foreign_directories,
        pinned,
        statuses: status::cleanup_snapshot().map_err(invalid)?,
    })
}

/// Only proposes candidates under the existing repository lock. Callers must re-plan
/// and compare fingerprints under the use/removal guard immediately before deletion.
pub async fn propose(
    scope: &ProjectScope,
    primary: &Path,
    settings: &WorktreeSettings,
) -> Result<Proposal, Error> {
    settings.validate().map_err(invalid)?;
    if !settings.auto_cleanup_enabled {
        return Ok(Proposal::default());
    }
    let _lifecycle = registry::lifecycle_lock(primary)?;
    propose_locked(scope, primary, settings).await
}

pub(super) async fn propose_locked(
    scope: &ProjectScope,
    primary: &Path,
    settings: &WorktreeSettings,
) -> Result<Proposal, Error> {
    let records = registry::records(scope).await?;
    let snapshot = crate::global_database::run_isolated({
        let scope = scope.clone();
        move || read_snapshot(&scope)
    })
    .await
    .map_err(invalid)?
    .ok();
    let entries =
        records.into_iter().map(|record| observe(primary, record, snapshot.as_ref())).collect();
    Ok(plan(scope, settings, entries))
}

pub(super) fn observe(
    primary: &Path,
    record: Record,
    snapshot: Option<&ProtectionSnapshot>,
) -> Observation {
    let title = record
        .directory
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| record.id.clone());
    let mut entry =
        Observation { last_used_at_ms: record.last_used_at_ms, title, record, reasons: vec![] };
    if entry
        .record
        .directory
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("slot-"))
    {
        entry.reasons.push(ProtectionReason::Pool);
    }
    if observe_protection(primary, &mut entry, snapshot).is_err() {
        entry.reasons.push(ProtectionReason::Unknown);
    }
    entry
}

fn observe_protection(
    primary: &Path,
    entry: &mut Observation,
    snapshot: Option<&ProtectionSnapshot>,
) -> Result<(), Error> {
    registry::validate(&entry.record, &entry.record.scope, primary)?;
    match registry::ensure_not_in_use(&entry.record) {
        Ok(()) => {}
        Err(Error::Invalid(_)) => entry.reasons.push(ProtectionReason::Busy),
        Err(error) => return Err(error),
    }
    let snapshot = snapshot.ok_or_else(|| invalid("Cleanup protection metadata is unavailable"))?;
    if repository_is_shared(primary, &entry.record, &snapshot.foreign_directories)? {
        entry.reasons.push(ProtectionReason::SharedRepository);
    }
    if matches!(
        environment::status(
            entry
                .record
                .directory
                .to_str()
                .ok_or_else(|| invalid("Cleanup requires a UTF-8 checkout path"))?
        )?,
        Some(environment::Status::Pending | environment::Status::Running)
    ) {
        entry.reasons.push(ProtectionReason::Initializing);
    }
    let mut latest: Option<&Info> = None;
    for session in &snapshot.sessions {
        let directory = std::fs::canonicalize(&session.directory)?;
        if !directory.starts_with(&entry.record.directory) {
            continue;
        }
        let pins = snapshot
            .pinned
            .get(&session.project_id)
            .ok_or_else(|| invalid("Project pin state is unavailable"))?;
        if pins.contains(&session.id) && !entry.reasons.contains(&ProtectionReason::Pinned) {
            entry.reasons.push(ProtectionReason::Pinned);
        }
        if (session.is_first_response_pending()
            || matches!(
                snapshot.statuses.get(&session.id),
                Some(status::Info::Busy | status::Info::Retry { .. })
            ))
            && !entry.reasons.contains(&ProtectionReason::Busy)
        {
            entry.reasons.push(ProtectionReason::Busy);
        }
        entry.last_used_at_ms = entry.last_used_at_ms.max(session.time.updated);
        if latest.is_none_or(|old| {
            (session.time.updated, &session.id, &session.project_id)
                > (old.time.updated, &old.id, &old.project_id)
        }) {
            latest = Some(session);
        }
    }
    if let Some(session) = latest {
        entry.title.clone_from(&session.title);
    }
    Ok(())
}

fn repository_is_shared(
    primary: &Path,
    record: &Record,
    foreign_directories: &[String],
) -> Result<bool, Error> {
    for directory in foreign_directories {
        if !Path::new(directory).is_absolute() {
            return Err(invalid("Foreign workspace identity requires an absolute path"));
        }
        let directory = std::fs::canonicalize(directory)?;
        let repo = match git2::Repository::discover(directory) {
            Ok(repo) => repo,
            Err(error) if error.code() == git2::ErrorCode::NotFound => continue,
            Err(error) => return Err(invalid(error)),
        };
        if std::fs::canonicalize(repo.commondir())? == record.common_dir {
            return Ok(true);
        }
    }
    // Foreign managed checkouts also protect the physical repo before their first chat.
    let repo = git2::Repository::open(primary).map_err(invalid)?;
    for name in repo.worktrees().map_err(invalid)?.iter() {
        let Some(name) = name.map_err(invalid)? else { continue };
        let worktree = repo.find_worktree(name).map_err(invalid)?;
        let linked = git2::Repository::open(worktree.path()).map_err(invalid)?;
        if registry::ownership(linked.path())?
            .is_some_and(|owner| owner.scope.tenant_scope() != record.scope.tenant_scope())
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Result of one host maintenance pass, persisted for management clients.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FlowStatus {
    pub proposal: Proposal,
    pub deleted: std::collections::BTreeMap<String, String>,
    pub running: Option<String>,
    pub errors: std::collections::BTreeMap<String, String>,
}
pub async fn refresh(
    scope: &ProjectScope,
    primary: &Path,
    settings: &WorktreeSettings,
    stop: &tokio_util::sync::CancellationToken,
) -> Result<FlowStatus, Error> {
    // Per-scope nonblocking host pass lock prevents overlapping startup/create/timer passes.
    let root = vw_config_types::paths::home_config_dir(&crate::global::paths().home)
        .join("storage")
        .join("worktree-cleanup");
    std::fs::create_dir_all(&root)?;
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(root.join(format!("{}.lock", registry::key(scope))))?;
    lock.try_lock().map_err(|_| invalid("Worktree cleanup pass already running"))?;
    let mut report =
        FlowStatus { proposal: propose(scope, primary, settings).await?, ..Default::default() };
    save_status(scope, &report).await?;
    if settings.ask_before_delete || !settings.auto_cleanup_enabled {
        return Ok(report);
    }
    for candidate in report.proposal.candidates.clone() {
        if stop.is_cancelled() {
            break;
        }
        report.running = Some(candidate.id.clone());
        save_status(scope, &report).await?;
        let result =
            super::management::delete_automatic(scope, primary, &candidate, settings).await;
        match result {
            Ok(result) => {
                report.deleted.insert(candidate.id, result.snapshot_id);
            }
            Err(error) => {
                report.errors.insert(candidate.id, error.to_string());
            }
        }
        report.running = None;
        save_status(scope, &report).await?;
    }
    report.proposal = propose(scope, primary, settings).await?;
    save_status(scope, &report).await?;
    Ok(report)
}
async fn save_status(scope: &ProjectScope, report: &FlowStatus) -> Result<(), Error> {
    crate::storage::write_atomic(&["worktree-cleanup", &registry::key(scope)], report)
        .await
        .map_err(invalid)
}
pub async fn flow_status(scope: &ProjectScope) -> Result<FlowStatus, Error> {
    match crate::storage::read::<FlowStatus>(&["worktree-cleanup", &registry::key(scope)]).await {
        Ok(mut report) => {
            if report.running.is_some() {
                let root = vw_config_types::paths::home_config_dir(&crate::global::paths().home)
                    .join("storage")
                    .join("worktree-cleanup");
                std::fs::create_dir_all(&root)?;
                let lock = std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(false)
                    .open(root.join(format!("{}.lock", registry::key(scope))))?;
                // No pass owns this scope: a persisted running item belongs to an interrupted host.
                if lock.try_lock().is_ok() {
                    report = crate::storage::read(&["worktree-cleanup", &registry::key(scope)])
                        .await
                        .map_err(invalid)?;
                    if let Some(id) = report.running.take() {
                        report.errors.insert(
                            id,
                            "Cleanup execution was interrupted; refresh required".into(),
                        );
                        save_status(scope, &report).await?;
                    }
                }
            }
            Ok(report)
        }
        Err(crate::storage::Error::NotFound(_)) => Ok(FlowStatus::default()),
        Err(error) => Err(invalid(error)),
    }
}
