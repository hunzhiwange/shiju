//! Persistent ownership of managed checkouts; Git registration alone is not authorization.
use super::Error;
use crate::storage;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use vw_api_types::id::ProjectScope;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Lifecycle {
    Ready,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub scope: ProjectScope,
    pub directory: PathBuf,
    pub common_dir: PathBuf,
    pub common_identity: String,
    pub git_dir: PathBuf,
    pub git_identity: String,
    pub checkout_identity: String,
    pub created_at_ms: u64,
    pub last_used_at_ms: u64,
    pub state: Lifecycle,
    /// Durable receipt preventing repeated restoration of the same snapshot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovered_from: Option<String>,
}

fn invalid(error: impl std::fmt::Display) -> Error {
    Error::Invalid(error.to_string())
}
fn key(scope: &ProjectScope) -> String {
    let hex = |s: &str| s.as_bytes().iter().map(|b| format!("{b:02x}")).collect::<String>();
    format!(
        "{}-{}-{}",
        hex(scope.platform_id().as_ref()),
        hex(scope.company_id().as_ref()),
        hex(scope.project_id().as_ref())
    )
}

/// A nonblocking OS lock protects Git mutations and registry changes across processes.
/// Keep the file in place: unlinking a lock file would allow competing inode locks.
pub fn lifecycle_lock(primary: &Path) -> Result<std::fs::File, Error> {
    let repo = git2::Repository::open(primary).map_err(invalid)?;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(repo.commondir().join("shiju-worktree-lifecycle.lock"))?;
    file.try_lock().map_err(|_| invalid("Worktree lifecycle is busy in another operation"))?;
    Ok(file)
}

pub(super) fn identity(path: &Path) -> Result<String, Error> {
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(invalid("Symlink checkout is not allowed"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(format!("{}:{}", metadata.dev(), metadata.ino()))
    }
    #[cfg(not(unix))]
    {
        Ok(format!("{:?}", metadata.created()?))
    }
}

/// Verify both directions of Git's linked-worktree registration, without repository discovery.
pub(crate) fn checkout(
    primary: &Path,
    directory: &Path,
) -> Result<(PathBuf, PathBuf, PathBuf, String), Error> {
    let checkout_identity = identity(directory)?;
    let directory = std::fs::canonicalize(directory)?;
    let root = git2::Repository::open(primary).map_err(invalid)?;
    let common = std::fs::canonicalize(root.commondir())?;
    let main = git2::Repository::open(&common).map_err(invalid)?;
    if main.workdir().is_some_and(|p| std::fs::canonicalize(p).ok().as_ref() == Some(&directory)) {
        return Err(invalid("Cannot delete the primary workspace"));
    }
    let repo = git2::Repository::open_ext(
        &directory,
        git2::RepositoryOpenFlags::NO_SEARCH,
        std::iter::empty::<&Path>(),
    )
    .map_err(invalid)?;
    if !repo.is_worktree() || std::fs::canonicalize(repo.commondir())? != common {
        return Err(invalid("Checkout belongs to another Git repository"));
    }
    let git_dir = std::fs::canonicalize(repo.path())?;
    let backlink = std::fs::read_to_string(git_dir.join("gitdir"))?;
    if std::fs::canonicalize(backlink.trim())? != std::fs::canonicalize(directory.join(".git"))? {
        return Err(invalid("Git checkout backlink mismatch"));
    }
    for name in main.worktrees().map_err(invalid)?.iter() {
        let Some(name) = name.map_err(invalid)? else {
            continue;
        };
        let wt = main.find_worktree(name).map_err(invalid)?;
        if std::fs::canonicalize(wt.path()).ok().as_ref() == Some(&directory)
            && wt.validate().is_ok()
        {
            return Ok((directory, common, git_dir, checkout_identity));
        }
    }
    Err(invalid("Checkout is not registered in Git"))
}

fn scope_lock(scope: &ProjectScope) -> Result<std::fs::File, Error> {
    let root = vw_config_types::paths::home_config_dir(&crate::global::paths().home)
        .join("storage")
        .join("managed-worktrees");
    std::fs::create_dir_all(&root)?;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(root.join(format!("{}.lock", key(scope))))?;
    file.try_lock().map_err(|_| invalid("Managed registry is busy in another operation"))?;
    Ok(file)
}

async fn records(scope: &ProjectScope) -> Result<Vec<Record>, Error> {
    match storage::read(&["managed-worktrees", &key(scope)]).await {
        Ok(records) => Ok(records),
        Err(storage::Error::NotFound(_)) => Ok(vec![]),
        Err(error) => Err(invalid(error)),
    }
}
async fn save(scope: &ProjectScope, records: &[Record]) -> Result<(), Error> {
    storage::write_atomic(&["managed-worktrees", &key(scope)], &records).await.map_err(invalid)
}

/// Call while holding the common repository lifecycle lock.
pub(crate) async fn register_locked(
    scope: &ProjectScope,
    primary: &Path,
    directory: &Path,
) -> Result<Record, Error> {
    register_with_recovery(scope, primary, directory, None).await
}

pub(crate) async fn register_recovered_locked(
    scope: &ProjectScope,
    primary: &Path,
    directory: &Path,
    recovery_id: &str,
) -> Result<Record, Error> {
    register_with_recovery(scope, primary, directory, Some(recovery_id)).await
}

pub(crate) async fn was_restored(scope: &ProjectScope, recovery_id: &str) -> Result<bool, Error> {
    Ok(records(scope).await?.iter().any(|r| r.recovered_from.as_deref() == Some(recovery_id)))
}

async fn register_with_recovery(
    scope: &ProjectScope,
    primary: &Path,
    directory: &Path,
    recovered_from: Option<&str>,
) -> Result<Record, Error> {
    let (directory, common_dir, git_dir, checkout_identity) = checkout(primary, directory)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let _scope_lock = scope_lock(scope)?;
    let mut records = records(scope).await?;
    if recovered_from
        .is_some_and(|id| records.iter().any(|r| r.recovered_from.as_deref() == Some(id)))
    {
        return Err(invalid("Recovery snapshot was already restored"));
    }
    if records.iter().any(|r| r.directory == directory && r.state == Lifecycle::Ready) {
        return Err(invalid("Checkout already managed"));
    }
    let record = Record {
        id: uuid::Uuid::new_v4().to_string(),
        scope: scope.clone(),
        directory,
        common_identity: identity(&common_dir)?,
        git_identity: identity(&git_dir)?,
        common_dir,
        git_dir,
        checkout_identity,
        created_at_ms: now,
        last_used_at_ms: now,
        state: Lifecycle::Ready,
        recovered_from: recovered_from.map(str::to_owned),
    };
    let marker = record.git_dir.join("shiju-managed.json");
    let bytes = serde_json::to_vec(&record).map_err(invalid)?;
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&marker)?;
    file.write_all(&bytes)?;
    records.push(record.clone());
    if let Err(error) = save(scope, &records).await {
        let _ = std::fs::remove_file(marker);
        return Err(error);
    }
    Ok(record)
}

#[cfg(test)]
pub async fn register(
    scope: &ProjectScope,
    primary: &Path,
    directory: &Path,
) -> Result<Record, Error> {
    let _lock = lifecycle_lock(primary)?;
    register_locked(scope, primary, directory).await
}
fn validate(record: &Record, scope: &ProjectScope, primary: &Path) -> Result<(), Error> {
    if &record.scope != scope || record.state != Lifecycle::Ready {
        return Err(invalid("Managed checkout ownership mismatch"));
    }
    let (path, common, git, identity) = checkout(primary, &record.directory)?;
    let marker = ownership(&git)?.ok_or_else(|| invalid("Managed checkout credential missing"))?;
    if marker.id != record.id || marker.scope != record.scope {
        return Err(invalid("Managed checkout credential mismatch"));
    }
    if path != record.directory
        || common != record.common_dir
        || git != record.git_dir
        || self::identity(&common)? != record.common_identity
        || self::identity(&git)? != record.git_identity
        || identity != record.checkout_identity
    {
        return Err(invalid("Managed checkout identity changed"));
    }
    Ok(())
}
pub async fn list(scope: &ProjectScope, primary: &Path) -> Result<Vec<Record>, Error> {
    let _lock = lifecycle_lock(primary)?;
    Ok(records(scope).await?.into_iter().filter(|r| validate(r, scope, primary).is_ok()).collect())
}
pub async fn authorize(scope: &ProjectScope, primary: &Path, id: &str) -> Result<Record, Error> {
    let record = records(scope)
        .await?
        .into_iter()
        .find(|r| r.id == id)
        .ok_or_else(|| invalid("Managed checkout not found"))?;
    validate(&record, scope, primary)?;
    Ok(record)
}
pub(crate) async fn record_for_path(
    scope: &ProjectScope,
    directory: &Path,
) -> Result<Option<Record>, Error> {
    Ok(records(scope)
        .await?
        .into_iter()
        .find(|r| r.directory == directory && r.state == Lifecycle::Ready))
}
pub(crate) async fn mark_deleted(scope: &ProjectScope, id: &str) -> Result<(), Error> {
    let _scope_lock = scope_lock(scope)?;
    let mut records = records(scope).await?;
    let record = records
        .iter_mut()
        .find(|r| r.id == id)
        .ok_or_else(|| invalid("Managed checkout not found"))?;
    record.state = Lifecycle::Deleted;
    save(scope, &records).await
}

pub(crate) fn ownership(git_dir: &Path) -> Result<Option<Record>, Error> {
    match std::fs::read(git_dir.join("shiju-managed.json")) {
        Ok(bytes) => serde_json::from_slice(&bytes).map(Some).map_err(invalid),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Resolve a managed ID under the same lock held through the complete removal.
pub async fn remove_managed(
    scope: &ProjectScope,
    primary: &Path,
    id: &str,
    force: bool,
) -> Result<bool, Error> {
    let _lock = lifecycle_lock(primary)?;
    let record = authorize(scope, primary, id).await?;
    super::ops::remove_locked(
        super::RemoveInput { directory: record.directory.to_string_lossy().into_owned(), force },
        scope,
        primary,
    )
    .await
}
