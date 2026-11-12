//! Restore pinned snapshots without checking out through user filters or running setup.
use super::{Manifest, Restored, invalid, registry};
use crate::worktree::Error;
use git2::{Index, Oid, Repository};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use vw_api_types::id::ProjectScope;

/// Restore a durable snapshot into a fresh managed checkout under current tenant settings.
/// Lifecycle locking and a persisted recovery ID reject concurrent and repeated restores.
pub async fn restore(
    scope: &ProjectScope,
    primary: &Path,
    recovery_id: &str,
) -> Result<Restored, Error> {
    let lifecycle = registry::lifecycle_lock(primary)?;
    if registry::was_restored(scope, recovery_id).await? {
        return Err(invalid("Recovery snapshot was already restored"));
    }
    let source = primary.to_owned();
    let owner = scope.clone();
    let id = recovery_id.to_owned();
    let load_lock = lifecycle.try_clone()?;
    let snapshot = tokio::task::spawn_blocking(move || {
        let _lock = load_lock;
        load_snapshot(&owner, &source, &id)
    })
    .await??;
    let project = crate::project::get(scope)
        .await
        .map_err(invalid)?
        .ok_or_else(|| invalid("Recovery project not found"))?;
    let mut settings = crate::config::get_global_for_tenant(scope.tenant_scope())
        .await
        .map_err(invalid)?
        .worktree_settings;
    // Recovery uses pinned local objects; it never fetches or runs initialization commands.
    settings.fetch_before_create = false;
    let root = super::super::creation_policy::prepare(
        &project,
        primary.to_str().ok_or_else(|| invalid("Non-UTF-8 repository path"))?,
        &settings,
    )
    .await?;

    let directory = root.join(format!("recovered-{}", uuid::Uuid::new_v4()));
    super::super::creation_policy::validate_parent(&root, &directory).await?;
    let source = primary.to_owned();
    let create_lock = lifecycle.try_clone()?;
    let (mut rollback, branch, branch_changed, original_id) =
        tokio::task::spawn_blocking(move || {
            create_restore(&source, &root, directory, snapshot, create_lock)
        })
        .await??;
    let record =
        registry::register_recovered_locked(scope, primary, &rollback.directory, recovery_id)
            .await?;
    rollback.armed = false;
    Ok(Restored { record, original_id, branch, branch_changed })
}

pub(super) struct Snapshot {
    pub(super) manifest: Manifest,
    index: Oid,
    worktree: Oid,
    head: Oid,
}

pub(super) fn load_snapshot(
    scope: &ProjectScope,
    primary: &Path,
    recovery_id: &str,
) -> Result<Snapshot, Error> {
    let repo = Repository::open(primary).map_err(invalid)?;
    let reference = recovery_reference(scope, recovery_id)?;
    let commit =
        repo.find_reference(&reference).map_err(invalid)?.peel_to_commit().map_err(invalid)?;
    let tree = commit.tree().map_err(invalid)?;
    let manifest: Manifest = serde_json::from_slice(
        repo.find_blob(
            tree.get_name("manifest.json").ok_or_else(|| invalid("Missing manifest"))?.id(),
        )
        .map_err(invalid)?
        .content(),
    )
    .map_err(invalid)?;
    if manifest.version != 1
        || manifest.recovery_id != recovery_id
        || &manifest.record.scope != scope
    {
        return Err(invalid("Recovery snapshot ownership mismatch"));
    }
    let common = std::fs::canonicalize(repo.commondir())?;
    if common != manifest.record.common_dir
        || registry::identity(&common)? != manifest.record.common_identity
    {
        return Err(invalid("Recovery repository identity changed"));
    }
    let head = Oid::from_str(&manifest.head).map_err(invalid)?;
    repo.find_commit(head).map_err(invalid)?;
    if commit.parent_count() != 1 || commit.parent_id(0).map_err(invalid)? != head {
        return Err(invalid("Recovery HEAD does not match pinned parent"));
    }
    let index_tree = repo
        .find_tree(tree.get_name("index").ok_or_else(|| invalid("Missing index"))?.id())
        .map_err(invalid)?;
    let working_tree = repo
        .find_tree(tree.get_name("worktree").ok_or_else(|| invalid("Missing worktree"))?.id())
        .map_err(invalid)?;
    repo.find_blob(tree.get_name("index.raw").ok_or_else(|| invalid("Missing raw index"))?.id())
        .map_err(invalid)?;
    validate_tree(&repo, &index_tree, 0)?;
    validate_tree(&repo, &working_tree, 0)?;

    Ok(Snapshot { manifest, index: index_tree.id(), worktree: working_tree.id(), head })
}

fn create_restore(
    primary: &Path,
    root: &Path,
    directory: std::path::PathBuf,
    snapshot: Snapshot,
    lifecycle: std::fs::File,
) -> Result<(RestoreRollback, String, bool, String), Error> {
    if std::fs::canonicalize(root)? != root
        || std::fs::symlink_metadata(root)?.file_type().is_symlink()
    {
        return Err(invalid("Recovery root identity changed"));
    }
    let repo = Repository::open(primary).map_err(invalid)?;
    let common = std::fs::canonicalize(repo.commondir())?;
    if common != snapshot.manifest.record.common_dir
        || registry::identity(&common)? != snapshot.manifest.record.common_identity
    {
        return Err(invalid("Recovery repository identity changed before creation"));
    }

    let head = snapshot.head;
    let working_tree = repo.find_tree(snapshot.worktree).map_err(invalid)?;
    let index_tree = repo.find_tree(snapshot.index).map_err(invalid)?;
    let original_branch = snapshot
        .manifest
        .branch
        .as_deref()
        .and_then(|b| b.strip_prefix("refs/heads/"))
        .filter(|branch| !branch.starts_with('-'));
    let available = original_branch.filter(|branch| {
        repo.find_branch(branch, git2::BranchType::Local).is_ok_and(|b| {
            b.get().target() == Some(head)
                && !b.is_head()
                && repo.worktrees().is_ok_and(|names| {
                    names.iter().all(|name| {
                        name.ok()
                            .flatten()
                            .and_then(|n| repo.find_worktree(n).ok())
                            .and_then(|wt| Repository::open(wt.path()).ok())
                            .and_then(|r| {
                                r.head().ok().and_then(|h| h.name().ok().map(str::to_owned))
                            })
                            .as_deref()
                            != Some(&format!("refs/heads/{branch}"))
                    })
                })
        })
    });
    let branch_changed = available.is_none();
    let branch = available
        .map(str::to_owned)
        .unwrap_or_else(|| format!("shiju/recovered-{}", uuid::Uuid::new_v4()));
    // Reserve a fresh destination before Git is allowed to use it.
    std::fs::create_dir(&directory)?;
    let mut rollback = RestoreRollback {
        primary: primary.to_owned(),
        directory: directory.clone(),
        branch: None,
        registered: false,
        armed: true,
        identity: registry::identity(&directory)?,
        _lifecycle: lifecycle,
    };
    if branch_changed {
        repo.branch(&branch, &repo.find_commit(head).map_err(invalid)?, false).map_err(invalid)?;
        rollback.branch = Some((branch.clone(), head));
    }
    let output = crate::shell::git_std_command()
        .arg("-C")
        .arg(primary)
        .args(["worktree", "add", "--no-checkout", "--"])
        .arg(&directory)
        .arg(&branch)
        .output()?;
    if !output.status.success() {
        return Err(invalid("Failed to create recovery worktree"));
    }
    rollback.registered = true;
    let (_, _, git_dir, _) = registry::checkout(primary, &directory)?;
    let checkout = Repository::open(&directory).map_err(invalid)?;
    let checkout_head = checkout.head().map_err(invalid)?;
    if checkout_head.target() != Some(head)
        || checkout_head.name().map_err(invalid)? != format!("refs/heads/{branch}")
    {
        return Err(invalid("Recovery branch changed during creation"));
    }
    write_tree(&repo, &working_tree, &directory)?;
    // Rebuild stat caches for the new checkout rather than trusting old inode/timestamps.
    let mut index = Index::open(&git_dir.join("index")).map_err(invalid)?;
    index.read_tree(&index_tree).map_err(invalid)?;
    index.write().map_err(invalid)?;

    Ok((rollback, branch, branch_changed, snapshot.manifest.record.id))
}

fn recovery_reference(scope: &ProjectScope, id: &str) -> Result<String, Error> {
    let id = uuid::Uuid::parse_str(id).map_err(invalid)?;
    let encode =
        |value: &str| value.as_bytes().iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    Ok(format!(
        "refs/shiju-worktree-recovery/{}-{}-{}/{}",
        encode(scope.platform_id().as_ref()),
        encode(scope.company_id().as_ref()),
        encode(scope.project_id().as_ref()),
        id
    ))
}

fn validate_tree(repo: &Repository, tree: &git2::Tree<'_>, depth: usize) -> Result<(), Error> {
    if depth > 128 {
        return Err(invalid("Recovery tree nesting exceeds safety limit"));
    }
    for entry in tree.iter() {
        let name = entry.name().map_err(invalid)?;
        if name.is_empty()
            || name == "."
            || name == ".."
            || name.contains(['/', '\\', ':'])
            || name.trim_end_matches([' ', '.']).eq_ignore_ascii_case(".git")
            || name.to_ascii_lowercase().starts_with("git~")
        {
            return Err(invalid("Unsafe recovery tree path"));
        }
        match entry.filemode() {
            0o040000 => {
                validate_tree(repo, &repo.find_tree(entry.id()).map_err(invalid)?, depth + 1)?
            }
            0o100644 | 0o100755 | 0o120000 => {
                repo.find_blob(entry.id()).map_err(invalid)?;
            }
            _ => return Err(invalid("Unsupported recovery tree mode")),
        }
    }
    Ok(())
}

fn write_tree(repo: &Repository, tree: &git2::Tree<'_>, root: &Path) -> Result<(), Error> {
    for entry in tree.iter() {
        let path = root.join(entry.name().map_err(invalid)?);
        if entry.filemode() == 0o040000 {
            std::fs::create_dir(&path)?;
            write_tree(repo, &repo.find_tree(entry.id()).map_err(invalid)?, &path)?;
        } else {
            let blob = repo.find_blob(entry.id()).map_err(invalid)?;
            if entry.filemode() == 0o120000 {
                #[cfg(unix)]
                {
                    use std::os::unix::ffi::OsStrExt;
                    std::os::unix::fs::symlink(std::ffi::OsStr::from_bytes(blob.content()), &path)?;
                }
                #[cfg(windows)]
                std::os::windows::fs::symlink_file(
                    std::str::from_utf8(blob.content()).map_err(invalid)?,
                    &path,
                )?;
            } else {
                let mut file = OpenOptions::new().write(true).create_new(true).open(&path)?;
                file.write_all(blob.content())?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    file.set_permissions(std::fs::Permissions::from_mode(
                        entry.filemode() as u32 & 0o777,
                    ))?;
                }
            }
        }
    }
    Ok(())
}

struct RestoreRollback {
    primary: std::path::PathBuf,
    directory: std::path::PathBuf,
    branch: Option<(String, Oid)>,
    registered: bool,
    armed: bool,
    identity: String,
    _lifecycle: std::fs::File,
}
impl Drop for RestoreRollback {
    fn drop(&mut self) {
        if !self.armed || registry::identity(&self.directory).ok().as_ref() != Some(&self.identity)
        {
            return;
        }
        if self.registered {
            let removed = crate::shell::git_std_command()
                .arg("-C")
                .arg(&self.primary)
                .args(["worktree", "remove", "--force"])
                .arg(&self.directory)
                .output();
            if !removed.is_ok_and(|output| output.status.success()) {
                return;
            }
        } else if std::fs::remove_dir(&self.directory).is_err() {
            return;
        }
        if let Some((branch, head)) = &self.branch
            && let Ok(repo) = Repository::open(&self.primary)
            && let Ok(mut branch) = repo.find_branch(branch, git2::BranchType::Local)
            && branch.get().target() == Some(*head)
        {
            let _ = branch.delete();
        }
    }
}
