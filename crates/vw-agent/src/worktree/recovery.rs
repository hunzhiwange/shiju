//! Durable, scoped snapshots for managed checkouts. Ignored files are never backed up.
//! Publication is one ref update: the commit contains both the manifest and all saved state.
use super::{Error, registry};
use git2::{Index, IndexEntry, IndexTime, Oid, Repository};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use vw_api_types::id::ProjectScope;

fn invalid(error: impl std::fmt::Display) -> Error {
    Error::Invalid(error.to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,
    pub recovery_id: String,
    pub record: registry::Record,
    pub head: String,
    pub branch: Option<String>,
    /// Ignored files are excluded even for a manually requested snapshot.
    pub has_ignored_files: bool,
}

/// Keep this guard alive through removal under the existing lifecycle lock.
/// Revalidate immediately before deleting; user processes do not honor our lifecycle lock.
pub struct PreparedSnapshot {
    pub reference: String,
    pub manifest: Manifest,
    state: Captured,
    commit: Oid,
    _lifecycle: File,
}

impl PreparedSnapshot {
    pub async fn revalidate(&self, primary: &Path) -> Result<(), Error> {
        let record =
            registry::authorize(&self.manifest.record.scope, primary, &self.manifest.record.id)
                .await?;
        if serde_json::to_vec(&record).map_err(invalid)?
            != serde_json::to_vec(&self.manifest.record).map_err(invalid)?
        {
            return Err(invalid("Managed checkout identity changed after recovery snapshot"));
        }
        let state = self.state.clone();
        let reference = self.reference.clone();
        let expected_commit = self.commit;
        tokio::task::spawn_blocking(move || {
            let repo = Repository::open(&record.directory).map_err(invalid)?;
            if capture(&repo, &record)? != state {
                return Err(invalid("Checkout changed after recovery snapshot; deletion refused"));
            }
            let commit = repo
                .find_reference(&reference)
                .map_err(invalid)?
                .peel_to_commit()
                .map_err(invalid)?;
            if commit.id() != expected_commit {
                return Err(invalid("Recovery reference changed; deletion refused"));
            }
            Ok(())
        })
        .await??;
        // Recheck filesystem and registry identities after the potentially long file scan.
        let current =
            registry::authorize(&self.manifest.record.scope, primary, &self.manifest.record.id)
                .await?;
        if serde_json::to_vec(&current).map_err(invalid)?
            != serde_json::to_vec(&self.manifest.record).map_err(invalid)?
        {
            return Err(invalid("Managed checkout identity changed during recovery verification"));
        }
        Ok(())
    }
}

/// Resolve ownership and publish a snapshot without modifying HEAD, branches, index or stash.
/// Automatic deletion is refused if ignored files exist. Manual callers must disclose their loss.
pub async fn prepare(
    scope: &ProjectScope,
    primary: &Path,
    id: &str,
    automatic: bool,
) -> Result<PreparedSnapshot, Error> {
    let lifecycle = registry::lifecycle_lock(primary)?;
    prepare_locked(scope, primary, id, automatic, lifecycle).await
}

pub(super) async fn prepare_locked(
    scope: &ProjectScope,
    primary: &Path,
    id: &str,
    automatic: bool,
    lifecycle: File,
) -> Result<PreparedSnapshot, Error> {
    let record = registry::authorize(scope, primary, id).await?;
    let prepared =
        tokio::task::spawn_blocking(move || publish(record, lifecycle, automatic)).await??;
    prepared.revalidate(primary).await?;
    Ok(prepared)
}

fn publish(
    record: registry::Record,
    lifecycle: File,
    automatic: bool,
) -> Result<PreparedSnapshot, Error> {
    let scope = record.scope.clone();
    let repo = Repository::open(&record.directory).map_err(invalid)?;
    // A handle-local overlay makes libgit2 flush writable object/ref handles before
    // making objects read-only, including on Windows. Never change the user's config.
    let mut durability = tempfile::NamedTempFile::new()?;
    durability.write_all(b"[core]\nfsyncObjectFiles = true\n")?;
    durability.as_file().sync_all()?;
    let mut config = repo.config().map_err(invalid)?;
    config.add_file(durability.path(), git2::ConfigLevel::App, true).map_err(invalid)?;
    repo.set_config(&config).map_err(invalid)?;
    let state = capture(&repo, &record)?;
    if automatic && state.has_ignored_files {
        return Err(invalid("Ignored files are not backed up; automatic deletion refused"));
    }
    let manifest = Manifest {
        version: 1,
        recovery_id: uuid::Uuid::new_v4().to_string(),
        record,
        head: state.head.to_string(),
        branch: state.branch.clone(),
        has_ignored_files: state.has_ignored_files,
    };
    // Scope components are encoded, never interpreted as Git ref or filesystem syntax.
    let encode =
        |value: &str| value.as_bytes().iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    let reference = format!(
        "refs/shiju-worktree-recovery/{}-{}-{}/{}",
        encode(scope.platform_id().as_ref()),
        encode(scope.company_id().as_ref()),
        encode(scope.project_id().as_ref()),
        manifest.recovery_id
    );
    let manifest_blob =
        repo.blob(&serde_json::to_vec(&manifest).map_err(invalid)?).map_err(invalid)?;
    let mut builder = repo.treebuilder(None).map_err(invalid)?;
    builder.insert("index", state.index, 0o040000).map_err(invalid)?;
    builder.insert("worktree", state.worktree, 0o040000).map_err(invalid)?;
    builder.insert("index.raw", state.raw_index, 0o100644).map_err(invalid)?;
    builder.insert("manifest.json", manifest_blob, 0o100644).map_err(invalid)?;
    let tree = repo.find_tree(builder.write().map_err(invalid)?).map_err(invalid)?;
    let signature =
        git2::Signature::now("ShiJu recovery", "recovery@shiju.local").map_err(invalid)?;
    let parent = repo.find_commit(state.head).map_err(invalid)?;
    let commit = repo
        .commit(None, &signature, &signature, "Managed worktree recovery", &tree, &[&parent])
        .map_err(invalid)?;
    // Sync newly created loose objects before publishing their root. Existing packed objects
    // are already owned by the repository. A failure never grants a deletion credential.
    #[cfg(unix)]
    {
        sync_objects(&repo, tree.id())?;
        sync_loose(&repo, commit)?;
    }
    sync_directory(&repo.commondir().join("objects"))?;
    if capture(&repo, &manifest.record)? != state {
        return Err(invalid("Checkout changed during recovery snapshot; deletion refused"));
    }
    repo.reference(&reference, commit, false, "Publish managed worktree recovery")
        .map_err(invalid)?;
    let ref_path = repo.commondir().join(&reference);
    #[cfg(unix)]
    File::open(&ref_path)?.sync_all()?;
    let mut directory = ref_path.parent();
    while let Some(path) = directory {
        sync_directory(path)?;
        if path == repo.commondir() {
            break;
        }
        directory = path.parent();
    }
    // A published snapshot remains available on any subsequent error; it is never auto-pruned.
    Ok(PreparedSnapshot { reference, manifest, state, commit, _lifecycle: lifecycle })
}

#[derive(Clone, PartialEq, Eq)]
struct Captured {
    head: Oid,
    branch: Option<String>,
    index: Oid,
    raw_index: Oid,
    worktree: Oid,
    has_ignored_files: bool,
}

fn capture(repo: &Repository, record: &registry::Record) -> Result<Captured, Error> {
    if repo.state() != git2::RepositoryState::Clean {
        return Err(invalid("In-progress Git operation cannot be safely recovered"));
    }
    let head = repo.head().map_err(invalid)?;
    let head_id = head.peel_to_commit().map_err(invalid)?.id();
    let branch = if repo.head_detached().map_err(invalid)? {
        None
    } else {
        Some(head.name().map_err(invalid)?.to_owned())
    };
    let index_path = record.git_dir.join("index");
    let raw = std::fs::read(&index_path)?;
    let mut index = Index::open(&index_path).map_err(invalid)?;
    if index.has_conflicts() {
        return Err(invalid("Conflicted index cannot be safely recovered"));
    }
    let mut tracked = HashSet::new();
    for entry in index.iter() {
        std::str::from_utf8(&entry.path)
            .map_err(|_| invalid("Non-UTF-8 file path is unsupported"))?;
        if entry.mode == 0o160000 {
            return Err(invalid("Submodule checkout recovery is unsupported; deletion refused"));
        }
        if entry.flags_extended != 0 {
            return Err(invalid("Sparse or intent-to-add index cannot be safely recovered"));
        }
        tracked.insert(entry.path);
    }
    let index_tree = index.write_tree_to(repo).map_err(invalid)?;
    let mut working = Index::new().map_err(invalid)?;
    let mut has_ignored_files = false;
    visit(repo, &record.directory, Path::new(""), &tracked, &mut working, &mut has_ignored_files)?;
    // Reading or writing an independent index must never refresh the user's index.
    if std::fs::read(&index_path)? != raw {
        return Err(invalid("Index changed during recovery snapshot"));
    }
    Ok(Captured {
        head: head_id,
        branch,
        index: index_tree,
        raw_index: repo.blob(&raw).map_err(invalid)?,
        worktree: working.write_tree_to(repo).map_err(invalid)?,
        has_ignored_files,
    })
}

fn visit(
    repo: &Repository,
    root: &Path,
    relative: &Path,
    tracked: &HashSet<Vec<u8>>,
    working: &mut Index,
    has_ignored_files: &mut bool,
) -> Result<(), Error> {
    for entry in std::fs::read_dir(root.join(relative))? {
        let entry = entry?;
        let path = relative.join(entry.file_name());
        if relative.as_os_str().is_empty() && entry.file_name() == ".git" {
            continue;
        }
        // Never recurse into symlinks. Nested repositories (including ignored ones) must
        // not silently degrade to ordinary files or an unrecoverable gitlink.
        let metadata = std::fs::symlink_metadata(entry.path())?;
        if metadata.is_dir() {
            if entry.path().join(".git").symlink_metadata().is_ok()
                || (entry.path().join("HEAD").is_file()
                    && entry.path().join("objects").is_dir()
                    && entry.path().join("refs").is_dir())
            {
                return Err(invalid("Nested Git repository cannot be safely recovered"));
            }
            if repo.status_should_ignore(&path).map_err(invalid)? {
                *has_ignored_files = true;
            }
            visit(repo, root, &path, tracked, working, has_ignored_files)?;
            continue;
        }
        let path_text = path
            .to_str()
            .ok_or_else(|| invalid("Non-UTF-8 file path is unsupported"))?
            .replace(std::path::MAIN_SEPARATOR, "/");
        if !tracked.contains(path_text.as_bytes())
            && repo.status_should_ignore(&path).map_err(invalid)?
        {
            *has_ignored_files = true;
            continue;
        }
        let (data, mode) = if metadata.file_type().is_symlink() {
            (std::fs::read_link(entry.path())?.as_os_str().as_encoded_bytes().to_vec(), 0o120000)
        } else if metadata.is_file() {
            let mut options = OpenOptions::new();
            options.read(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
            }
            let mut file = options.open(entry.path())?;
            if !file.metadata()?.is_file() {
                return Err(invalid("Entry changed during recovery snapshot"));
            }
            let mut data = Vec::new();
            file.read_to_end(&mut data)?;
            #[cfg(unix)]
            let executable = {
                use std::os::unix::fs::PermissionsExt;
                metadata.permissions().mode() & 0o111 != 0
            };
            #[cfg(not(unix))]
            let executable = repo
                .index()
                .map_err(invalid)?
                .get_path(&path, 0)
                .is_some_and(|e| e.mode == 0o100755);
            (data, if executable { 0o100755 } else { 0o100644 })
        } else {
            return Err(invalid("Special filesystem entry cannot be safely recovered"));
        };
        working
            .add(&IndexEntry {
                ctime: IndexTime::new(0, 0),
                mtime: IndexTime::new(0, 0),
                dev: 0,
                ino: 0,
                mode,
                uid: 0,
                gid: 0,
                file_size: 0,
                id: repo.blob(&data).map_err(invalid)?,
                flags: 0,
                flags_extended: 0,
                path: path_text.into_bytes(),
            })
            .map_err(invalid)?;
    }
    Ok(())
}

#[cfg(unix)]
fn sync_loose(repo: &Repository, oid: Oid) -> Result<(), Error> {
    let hex = oid.to_string();
    let dir = repo.commondir().join("objects").join(&hex[..2]);
    match File::open(dir.join(&hex[2..])) {
        Ok(file) => {
            file.sync_all()?;
            sync_directory(&dir)?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            repo.odb().map_err(invalid)?.read(oid).map_err(invalid)?;
        }
        Err(error) => return Err(error.into()),
    }
    Ok(())
}
#[cfg(unix)]
fn sync_objects(repo: &Repository, oid: Oid) -> Result<(), Error> {
    let tree = repo.find_tree(oid).map_err(invalid)?;
    for entry in tree.iter() {
        if entry.kind() == Some(git2::ObjectType::Tree) {
            sync_objects(repo, entry.id())?;
        } else {
            sync_loose(repo, entry.id())?;
        }
    }
    sync_loose(repo, oid)
}
fn sync_directory(path: &Path) -> Result<(), Error> {
    #[cfg(unix)]
    File::open(path)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[derive(Debug)]
pub struct Restored {
    pub record: registry::Record,
    pub original_id: String,
    pub branch: String,
    pub branch_changed: bool,
}

#[path = "recovery_restore.rs"]
mod restoration;
pub use restoration::restore;

/// Preflight uses the same content scan as recovery, without publishing a snapshot.
pub(super) fn content_fingerprint(
    record: &registry::Record,
) -> Result<(String, bool, bool), Error> {
    use sha2::{Digest, Sha256};
    let repo = Repository::open(&record.directory).map_err(invalid)?;
    let state = capture(&repo, record)?;
    let mut hash = Sha256::new();
    for oid in [state.head, state.index, state.raw_index, state.worktree] {
        hash.update(oid.as_bytes());
    }
    hash.update(state.branch.as_deref().unwrap_or_default().as_bytes());
    hash.update([state.has_ignored_files as u8]);
    let head = repo.find_commit(state.head).map_err(invalid)?.tree_id();
    Ok((
        hex::encode(hash.finalize()),
        state.index != head || state.worktree != state.index,
        state.has_ignored_files,
    ))
}

/// Enumerate only snapshots which pass the restore loader's scope and object checks.
pub async fn list(
    scope: &ProjectScope,
    primary: &Path,
) -> Result<Vec<vw_api_types::worktree::WorktreeSnapshotSummary>, Error> {
    let _lock = registry::lifecycle_lock(primary)?;
    let records = registry::records(scope).await?;
    let repo = Repository::open(primary).map_err(invalid)?;
    let mut items = vec![];
    for reference in repo.references_glob("refs/shiju-worktree-recovery/*/*").map_err(invalid)? {
        let reference = reference.map_err(invalid)?;
        let Some(id) = reference.name().ok().and_then(|n| n.rsplit('/').next()) else { continue };
        let snapshot = match restoration::load_snapshot(scope, primary, id) {
            Ok(snapshot) => snapshot,
            Err(_) => continue,
        };
        let manifest = snapshot.manifest;
        items.push(vw_api_types::worktree::WorktreeSnapshotSummary {
            id: id.to_owned(),
            original_worktree_id: manifest.record.id,
            directory: manifest.record.directory.to_string_lossy().into_owned(),
            branch: manifest.branch,
            has_ignored_files: manifest.has_ignored_files,
            restored: records.iter().any(|r| r.recovered_from.as_deref() == Some(id)),
        });
    }
    items.sort_by(|a, b| a.id.cmp(&b.id));
    items.dedup_by(|a, b| a.id == b.id);
    Ok(items)
}
