//! Management composes the existing registry, protection and recovery services.
use super::{Error, cleanup, environment, recovery, registry};
use std::path::Path;
use vw_api_types::{common::TimestampMs, id::ProjectScope, worktree::*};
use vw_config_types::worktree::WorktreeSettings;

fn invalid(error: impl std::fmt::Display) -> Error {
    Error::Invalid(error.to_string())
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn reason(reason: cleanup::ProtectionReason) -> WorktreeProtectionReason {
    match reason {
        cleanup::ProtectionReason::Pinned => WorktreeProtectionReason::Pinned,
        cleanup::ProtectionReason::Busy => WorktreeProtectionReason::Busy,
        cleanup::ProtectionReason::Initializing => WorktreeProtectionReason::Initializing,
        cleanup::ProtectionReason::Pool => WorktreeProtectionReason::Pool,
        cleanup::ProtectionReason::Unknown => WorktreeProtectionReason::Unknown,
        cleanup::ProtectionReason::SharedRepository => WorktreeProtectionReason::SharedRepository,
    }
}
async fn snapshot(scope: &ProjectScope) -> Result<cleanup::ProtectionSnapshot, Error> {
    let scope = scope.clone();
    crate::global_database::run_isolated(move || cleanup::read_snapshot(&scope))
        .await
        .map_err(invalid)?
}
fn map(
    record: registry::Record,
    observation: cleanup::Observation,
    snapshot: Option<&cleanup::ProtectionSnapshot>,
) -> ManagedWorktreeDto {
    let directory = record.directory.to_string_lossy().into_owned();
    let status = match environment::status(&directory) {
        Ok(Some(environment::Status::Pending | environment::Status::Running)) => {
            WorktreeStatus::Creating
        }
        Ok(Some(environment::Status::Error)) | Err(_) => WorktreeStatus::Error,
        _ if observation.reasons.contains(&cleanup::ProtectionReason::Busy) => WorktreeStatus::Busy,
        _ => WorktreeStatus::Ready,
    };
    let mut sessions = snapshot
        .map(|s| {
            s.sessions
                .iter()
                .filter(|s| {
                    s.project_id == record.scope.project_id().as_ref()
                        && std::fs::canonicalize(&s.directory)
                            .is_ok_and(|p| p.starts_with(&record.directory))
                })
                .map(|s| WorktreeChatSummary {
                    id: s.id.clone(),
                    title: s.title.clone(),
                    updated_at_ms: s.time.updated,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    sessions.sort_by(|a, b| b.updated_at_ms.cmp(&a.updated_at_ms).then_with(|| a.id.cmp(&b.id)));
    ManagedWorktreeDto {
        worktree: WorktreeDto {
            id: record.id.into(),
            project_id: record.scope.project_id().clone(),
            name: record.directory.file_name().unwrap_or_default().to_string_lossy().into_owned(),
            branch: crate::git_repository::current_branch(&record.directory).unwrap_or_default(),
            directory,
            status,
            created_at_ms: TimestampMs(record.created_at_ms.min(i64::MAX as u64) as i64),
            updated_at_ms: TimestampMs(observation.last_used_at_ms.min(i64::MAX as u64) as i64),
        },
        title: observation.title,
        sessions,
        protection_reasons: observation.reasons.into_iter().map(reason).collect(),
        recovered_from: record.recovered_from,
    }
}
pub async fn list(
    scope: &ProjectScope,
    primary: &Path,
) -> Result<ListManagedWorktreesResponse, Error> {
    let _lock = registry::lifecycle_lock(primary)?;
    let snapshot = snapshot(scope).await.ok();
    let mut items = vec![];
    for record in registry::records(scope).await? {
        if record.state != registry::Lifecycle::Ready {
            continue;
        }
        let observation = cleanup::observe(primary, record.clone(), snapshot.as_ref());
        items.push(map(record, observation, snapshot.as_ref()));
    }
    Ok(ListManagedWorktreesResponse { items })
}
async fn preflight_locked(
    scope: &ProjectScope,
    primary: &Path,
    id: &str,
    settings: &WorktreeSettings,
    expires: u64,
) -> Result<WorktreeDeletionPreflight, Error> {
    use sha2::{Digest, Sha256};
    static SECRET: std::sync::LazyLock<String> =
        std::sync::LazyLock::new(|| uuid::Uuid::new_v4().to_string());
    let record = registry::authorize(scope, primary, id).await?;
    let snapshot = snapshot(scope).await?;
    let observation = cleanup::observe(primary, record.clone(), Some(&snapshot));
    let (contents, has_changes, has_ignored_files) = recovery::content_fingerprint(&record)?;
    let mut hash = Sha256::new();
    hash.update(SECRET.as_bytes());
    hash.update(serde_json::to_vec(&record).map_err(invalid)?);
    hash.update(serde_json::to_vec(settings).map_err(invalid)?);
    hash.update(serde_json::to_vec(&observation.reasons).map_err(invalid)?);
    hash.update(observation.last_used_at_ms.to_le_bytes());
    hash.update(contents.as_bytes());
    hash.update(expires.to_le_bytes());
    Ok(WorktreeDeletionPreflight {
        fingerprint: format!("{expires}:{}", hex::encode(hash.finalize())),
        expires_at_ms: expires,
        confirmation_required: settings.ask_before_delete || has_changes || has_ignored_files,
        has_changes,
        has_ignored_files,
        protection_reasons: observation.reasons.into_iter().map(reason).collect(),
    })
}
pub async fn preflight(
    scope: &ProjectScope,
    primary: &Path,
    id: &str,
    settings: &WorktreeSettings,
) -> Result<WorktreeDeletionPreflight, Error> {
    let _lock = registry::lifecycle_lock(primary)?;
    preflight_locked(scope, primary, id, settings, now() + 300_000).await
}
pub async fn delete(
    scope: &ProjectScope,
    primary: &Path,
    id: &str,
    settings: &WorktreeSettings,
    request: &DeleteManagedWorktreeRequest,
) -> Result<DeleteManagedWorktreeResponse, Error> {
    let lock = registry::lifecycle_lock(primary)?;
    delete_locked(scope, primary, id, settings, request, lock, false).await
}
async fn delete_locked(
    scope: &ProjectScope,
    primary: &Path,
    id: &str,
    settings: &WorktreeSettings,
    request: &DeleteManagedWorktreeRequest,
    lock: std::fs::File,
    automatic: bool,
) -> Result<DeleteManagedWorktreeResponse, Error> {
    ensure_current_settings(scope, settings).await?;
    let expires = request
        .fingerprint
        .split_once(':')
        .and_then(|(e, _)| e.parse::<u64>().ok())
        .ok_or_else(|| invalid("Refresh deletion preflight"))?;
    if expires < now() || expires > now() + 300_000 {
        return Err(invalid("Deletion preflight expired; refresh required"));
    }
    let current = preflight_locked(scope, primary, id, settings, expires).await?;
    if current.fingerprint != request.fingerprint || !current.protection_reasons.is_empty() {
        return Err(invalid("Worktree changed or is protected; refresh required"));
    }
    if current.confirmation_required && !request.confirmed {
        return Err(invalid("Explicit deletion confirmation required"));
    }
    if current.has_ignored_files && !request.confirm_ignored_loss {
        return Err(invalid("Explicit ignored-file loss confirmation required"));
    }
    let prepared = recovery::prepare_locked(scope, primary, id, automatic, lock).await?;
    prepared.revalidate(primary).await?;
    ensure_current_settings(scope, settings).await?;
    // Read the latest protection after the snapshot scan, retaining the lifecycle lock.
    let verified = preflight_locked(scope, primary, id, settings, expires).await?;
    if verified.fingerprint != request.fingerprint || !verified.protection_reasons.is_empty() {
        return Err(invalid("Worktree protection changed; refresh required"));
    }
    let directory = prepared
        .manifest
        .record
        .directory
        .to_str()
        .ok_or_else(|| invalid("Non-UTF-8 checkout path"))?;
    let output = super::native::run_git_guarded(
        &["worktree", "remove", "--force", "--", directory],
        primary.to_str().ok_or_else(|| invalid("Non-UTF-8 primary path"))?,
        Some(prepared._lifecycle.try_clone()?),
    )
    .await?;
    if !output.success {
        return Err(invalid(output.error_text("Failed to remove managed worktree")));
    }
    registry::mark_deleted(scope, id).await?;
    crate::project::remove_sandbox(scope, directory).await.map_err(invalid)?;
    Ok(DeleteManagedWorktreeResponse { snapshot_id: prepared.manifest.recovery_id.clone() })
}
pub async fn retry(scope: &ProjectScope, primary: &Path, id: &str) -> Result<(), Error> {
    let _lock = registry::lifecycle_lock(primary)?;
    let record = registry::authorize(scope, primary, id).await?;
    let directory = record.directory.to_str().ok_or_else(|| invalid("Non-UTF-8 checkout path"))?;
    let project = crate::project::get(scope)
        .await
        .map_err(invalid)?
        .ok_or_else(|| invalid("Project not found"))?;
    let context = crate::project::instance::Context {
        directory: directory.into(),
        worktree: primary.to_string_lossy().into_owned(),
        project,
    };
    crate::session::ui_store::with_tenant_scope(
        scope.tenant_scope().clone(),
        crate::project::instance::provide_context(
            context,
            environment::retry(directory, scope.project_id().as_ref()),
        ),
    )
    .await
}
pub async fn proposal(
    scope: &ProjectScope,
    primary: &Path,
    settings: &WorktreeSettings,
) -> Result<WorktreeCleanupProposal, Error> {
    let proposal = cleanup::propose(scope, primary, settings).await?;
    Ok(WorktreeCleanupProposal {
        candidates: proposal
            .candidates
            .into_iter()
            .map(|c| WorktreeCleanupCandidate {
                id: c.id,
                fingerprint: c.fingerprint,
                title: c.title,
                last_used_at_ms: c.last_used_at_ms,
            })
            .collect(),
        protected: proposal
            .protected
            .into_iter()
            .map(|p| WorktreeCleanupProtected {
                id: p.id,
                reasons: p.reasons.into_iter().map(reason).collect(),
            })
            .collect(),
        remaining_excess: proposal.remaining_excess,
    })
}

/// Automatic removal must still be an unchanged retention candidate under the same
/// lock used for preflight, snapshot publication, final verification and removal.
pub(super) async fn delete_automatic(
    scope: &ProjectScope,
    primary: &Path,
    candidate: &cleanup::Candidate,
    settings: &WorktreeSettings,
) -> Result<DeleteManagedWorktreeResponse, Error> {
    let lock = registry::lifecycle_lock(primary)?;
    let latest = crate::config::get_global_for_tenant(scope.tenant_scope())
        .await
        .map_err(invalid)?
        .worktree_settings;
    if &latest != settings || latest.ask_before_delete || !latest.auto_cleanup_enabled {
        return Err(invalid("Automatic cleanup settings changed; refresh required"));
    }
    let proposal = cleanup::propose_locked(scope, primary, &latest).await?;
    if !proposal
        .candidates
        .iter()
        .any(|c| c.id == candidate.id && c.fingerprint == candidate.fingerprint)
    {
        return Err(invalid("Cleanup candidate changed or is protected; refresh required"));
    }
    let preflight =
        preflight_locked(scope, primary, &candidate.id, &latest, now() + 300_000).await?;
    // Dirty tracked/untracked data is recoverable; ignored files always require a human.
    if preflight.has_ignored_files {
        return Err(invalid("Automatic cleanup cannot discard ignored files"));
    }
    let request = DeleteManagedWorktreeRequest {
        fingerprint: preflight.fingerprint,
        confirmed: true,
        confirm_ignored_loss: false,
    };
    delete_locked(scope, primary, &candidate.id, &latest, &request, lock, true).await
}

async fn ensure_current_settings(
    scope: &ProjectScope,
    expected: &WorktreeSettings,
) -> Result<(), Error> {
    let latest = crate::config::get_global_for_tenant(scope.tenant_scope())
        .await
        .map_err(invalid)?
        .worktree_settings;
    if &latest != expected {
        return Err(invalid("Worktree settings changed; refresh required"));
    }
    Ok(())
}
