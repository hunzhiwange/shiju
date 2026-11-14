//! Skill-category migration. Paths and enabled applications require separate host
//! authorization; source rows never grant filesystem or execution permissions.
use super::{
    preview::{self, Decision, ImportPreview, PreviewCandidate},
    source::{SourceCategory, SourceScope, SourceSnapshot},
};
use crate::skills::{
    bulk_sync::SkillSyncTarget,
    copy_deploy::{CopyBinding, CopyDeploy, binding_fingerprint},
    disable_deploy::DisableDeploy,
    inventory::get_all_installed,
    link_deploy::SyncMethod,
    local_install::{LocalSkill, copy_skill_tree, validate_directory},
    repo_install::RepoSkillSource,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io,
    os::fd::AsRawFd,
    path::Path,
};
use uuid::Uuid;

type Result<T> = io::Result<T>;
fn invalid() -> io::Error {
    io::Error::other("invalid, stale or unauthorized skill migration")
}
fn checked<T, E>(value: std::result::Result<T, E>) -> Result<T> {
    value.map_err(|_| invalid())
}

/// Preserved provenance and enable intent; application truth remains copy_bindings.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigratedSkill {
    pub id: String,
    pub directory: String,
    pub name: String,
    pub description: Option<String>,
    pub repository: Option<RepoSkillSource>,
    pub readme_url: Option<String>,
    pub installed_at: i64,
    pub updated_at: i64,
    pub content_hash: String,
    pub enabled: BTreeMap<String, bool>,
}
impl std::fmt::Debug for MigratedSkill {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MigratedSkill([redacted])")
    }
}

/// Kept server-side between preview and execution. The resource guard owns a
/// bounded, link-free private copy, so later source changes cannot alter consent.
pub struct SkillsPreview {
    preview: ImportPreview,
    records: Vec<MigratedSkill>,
    resources: tempfile::TempDir,
}

impl SkillsPreview {
    pub fn preview(&self) -> &ImportPreview {
        &self.preview
    }
    /// Normalize selected SQLite rows. JSON skills shapes are explicitly rejected
    /// until their versioned domain layout is supported, rather than silently lost.
    pub fn prepare(
        scope: &SourceScope,
        source: &SourceSnapshot,
        revision: String,
        resource_root: &Path,
        existing: &BTreeMap<String, MigratedSkill>,
    ) -> Result<Self> {
        checked(crate::providers::paths::open_beneath(resource_root, resource_root, true))?;
        let resources = tempfile::tempdir()?;
        let mut records = Vec::new();
        let mut names = BTreeSet::new();
        let mut ids = BTreeSet::new();
        for row in
            checked(source.records(scope))?.iter().filter(|r| r.category == SourceCategory::Skills)
        {
            if row.origin == "skill_repos" {
                continue;
            } // References are embedded per installed skill.
            if row.origin != "skills" {
                return Err(invalid());
            }
            let fields = row.fields.as_object().ok_or_else(invalid)?;
            let string = |key: &str| -> Result<String> {
                let value = fields.get(key).and_then(|v| v.as_str()).ok_or_else(invalid)?;
                if value.trim().is_empty() || value.len() > 4096 {
                    return Err(invalid());
                }
                Ok(value.to_owned())
            };
            let directory = string("directory")?;
            checked(validate_directory(&directory))?;
            let id = string("id")?;
            if !names.insert(directory.to_ascii_lowercase()) || !ids.insert(id.clone()) {
                return Err(invalid());
            }
            let mut enabled = BTreeMap::new();
            for (key, value) in fields.iter().filter(|(key, _)| key.starts_with("enabled_")) {
                let flag = match value {
                    serde_json::Value::Bool(flag) => *flag,
                    v if v.as_i64() == Some(0) => false,
                    v if v.as_i64() == Some(1) => true,
                    _ => return Err(invalid()),
                };
                enabled.insert(key.trim_start_matches("enabled_").to_owned(), flag);
            }
            let repository = match fields.get("repo_owner").filter(|v| !v.is_null()) {
                None => {
                    if ["repo_name", "repo_branch"]
                        .iter()
                        .any(|k| fields.get(*k).is_some_and(|v| !v.is_null()))
                    {
                        return Err(invalid());
                    }
                    None
                }
                Some(_) => {
                    let owner = string("repo_owner")?;
                    let name = string("repo_name")?;
                    let branch = string("repo_branch")?;
                    checked(crate::skills::repos::validate_repo_ref(&owner, &name, &branch))?;
                    if branch.eq_ignore_ascii_case("HEAD") {
                        return Err(invalid());
                    }
                    Some(RepoSkillSource { owner, name, branch, directory: directory.clone() })
                }
            };
            checked(crate::providers::paths::open_beneath(
                resource_root,
                &resource_root.join(&directory),
                true,
            ))?;
            checked(copy_skill_tree(
                &resource_root.join(&directory),
                &resources.path().join(&directory),
            ))?;
            check_resource_budget(resources.path())?;
            let skill = LocalSkill {
                directory: directory.clone(),
                name: string("name")?,
                description: None,
                source: resources.path().join(&directory),
            };
            let detail = checked(get_all_installed(resources.path(), &[skill]))?
                .pop()
                .ok_or_else(invalid)?;
            let hash = detail.content_hash.ok_or_else(invalid)?;
            if let Some(expected) =
                fields.get("content_hash").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
            {
                if expected != hash {
                    return Err(invalid());
                }
            }
            let description = match fields.get("description") {
                None | Some(serde_json::Value::Null) => None,
                Some(serde_json::Value::String(s)) if s.len() <= 65536 => Some(s.clone()),
                _ => return Err(invalid()),
            };
            if detail.skill.name != string("name")? || detail.skill.description != description {
                return Err(invalid());
            }
            records.push(MigratedSkill {
                id,
                directory,
                name: detail.skill.name,
                description,
                repository,
                readme_url: match fields.get("readme_url") {
                    None | Some(serde_json::Value::Null) => None,
                    Some(serde_json::Value::String(s)) if s.len() <= 4096 => Some(s.clone()),
                    _ => return Err(invalid()),
                },
                installed_at: fields
                    .get("installed_at")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(invalid)?,
                updated_at: fields.get("updated_at").and_then(|v| v.as_i64()).unwrap_or(0),
                content_hash: hash,
                enabled,
            });
        }
        if records.is_empty() {
            return Err(invalid());
        }
        // Only digests reach generic portable preview. Enable intent remains in
        // the private typed plan and still requires explicit target authorization.
        let identity = |record: &MigratedSkill| -> Result<serde_json::Value> {
            let bytes = checked(serde_json::to_vec(record))?;
            Ok(serde_json::json!({"name": format!("{:x}", Sha256::digest(bytes))}))
        };
        let candidates: Vec<_> = records
            .iter()
            .map(|record| {
                Ok(PreviewCandidate {
                    incoming: identity(record)?,
                    existing: existing.get(&record.directory).map(&identity).transpose()?,
                    references_valid: true,
                })
            })
            .collect::<Result<_>>()?;
        let mut candidates = Some(candidates);
        let preview = checked(preview::build_preview(
            scope,
            source,
            &[SourceCategory::Skills],
            revision,
            |_, _| Ok(candidates.take().unwrap_or_default()),
        ))?;
        Ok(Self { preview, records, resources })
    }

    pub fn selected(
        &self,
        scope: &SourceScope,
        source: &SourceSnapshot,
        revision: &str,
        decisions: &[(Uuid, Decision)],
    ) -> Result<Vec<&MigratedSkill>> {
        let decisions = checked(self.preview.resolve(scope, source, revision, decisions))?;
        Ok(self
            .records
            .iter()
            .zip(self.preview.entries())
            .filter_map(|(record, entry)| {
                (decisions.get(&entry.id) == Some(&Decision::Replace)).then_some(record)
            })
            .collect())
    }
}

/// Owns every publication until the settings transaction commits. All root locks
/// are acquired in canonical order and shared via dup, avoiding recursive flock.
pub struct SkillMigration {
    copies: Vec<CopyDeploy>,
    removals: Vec<DisableDeploy>,
    pub skills: Vec<LocalSkill>,
    pub bindings: BTreeMap<String, CopyBinding>,
}
impl SkillMigration {
    pub fn prepare(
        root: &Path,
        plan: &SkillsPreview,
        selected: &[&MigratedSkill],
        saved: &[LocalSkill],
        bindings: &BTreeMap<String, CopyBinding>,
        targets: &BTreeMap<String, SkillSyncTarget>,
    ) -> Result<Self> {
        let mut roots = BTreeMap::<_, File>::new();
        let root = std::fs::canonicalize(root)?;
        roots.insert(
            root.clone(),
            checked(crate::providers::paths::open_beneath(&root, &root, true))?,
        );
        for (app, target) in targets {
            if checked(serde_json::to_value(target.path.app_id))?.as_str() != Some(app.as_str()) {
                return Err(invalid());
            }
            checked(target.identity().validate())?;
            checked(target.path.require_distribution())?;
            let path = std::fs::canonicalize(&target.path.directory)?;
            if roots.keys().any(|old| path.starts_with(old) || old.starts_with(&path)) {
                return Err(invalid());
            }
            roots.insert(
                path,
                checked(crate::providers::paths::open_beneath(
                    &target.path.directory,
                    &target.path.directory,
                    true,
                ))?,
            );
        }
        for file in roots.values() {
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        let mut result = Self {
            copies: Vec::new(),
            removals: Vec::new(),
            skills: saved.to_vec(),
            bindings: bindings.clone(),
        };
        for record in selected {
            let previous =
                saved.iter().find(|s| s.directory.eq_ignore_ascii_case(&record.directory));
            if previous.is_some_and(|s| s.directory != record.directory) {
                return Err(invalid());
            }
            let old = previous
                .map(|_| -> Result<CopyBinding> {
                    Ok(CopyBinding {
                        directory: record.directory.clone(),
                        destination: root.join(&record.directory),
                        fingerprint: checked(binding_fingerprint(
                            &root.join(&record.directory),
                            None,
                        ))?,
                        link_target: None,
                    })
                })
                .transpose()?;
            result.copies.push(checked(CopyDeploy::prepare_tree_locked(
                plan.resources.path(),
                &root,
                &record.directory,
                old.as_ref(),
                SyncMethod::Copy,
                roots.get(&root),
            ))?);
            result.skills.retain(|s| s.directory != record.directory);
            result.skills.push(LocalSkill {
                directory: record.directory.clone(),
                name: record.name.clone(),
                description: record.description.clone(),
                source: root.join(&record.directory),
            });
            // Existing bindings must all be covered by the explicit host selection.
            for (key, _) in bindings.iter().filter(|(_, b)| b.directory == record.directory) {
                if !targets.values().any(|t| {
                    serde_json::to_string(&(&t.installation_id, t.path.app_id, &record.directory))
                        .ok()
                        .as_ref()
                        == Some(key)
                }) {
                    return Err(invalid());
                }
            }
            for (app, enabled) in &record.enabled {
                if *enabled && !targets.contains_key(app) {
                    return Err(invalid());
                }
            }
            for (app, target) in targets {
                let key = checked(serde_json::to_string(&(
                    &target.installation_id,
                    target.path.app_id,
                    &record.directory,
                )))?;
                let destination = std::fs::canonicalize(&target.path.directory)?;
                if record.enabled.get(app).copied().unwrap_or(false) {
                    let copy = checked(CopyDeploy::prepare_tree_locked(
                        plan.resources.path(),
                        &destination,
                        &record.directory,
                        bindings.get(&key),
                        SyncMethod::Copy,
                        roots.get(&destination),
                    ))?;
                    result.bindings.insert(key, copy.binding().clone());
                    result.copies.push(copy);
                } else if let Some(old) = bindings.get(&key) {
                    result.removals.push(checked(DisableDeploy::prepare_tree_locked(
                        &destination,
                        old,
                        roots.get(&destination),
                    ))?);
                    result.bindings.remove(&key);
                } else if std::fs::symlink_metadata(destination.join(&record.directory)).is_ok() {
                    return Err(invalid());
                }
            }
        }
        result.skills.sort_by(|a, b| a.directory.cmp(&b.directory));
        Ok(result)
    }
    pub fn publish(&mut self) -> Result<()> {
        for copy in &mut self.copies {
            checked(copy.publish())?;
        }
        for removal in &mut self.removals {
            checked(removal.publish())?;
        }
        Ok(())
    }
    pub fn rollback(&mut self) -> Result<()> {
        for removal in self.removals.iter_mut().rev() {
            checked(removal.rollback())?;
        }
        for copy in self.copies.iter_mut().rev() {
            checked(copy.rollback())?;
        }
        Ok(())
    }
    pub fn finish(self) -> Result<()> {
        // Disarm copies even when quarantine cleanup needs recovery after commit.
        for copy in self.copies {
            copy.finish();
        }
        let mut failure = None;
        for removal in self.removals {
            if removal.finish().is_err() {
                failure = Some(invalid());
            }
        }
        match failure {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

/// Complete bounded owner preimage; called while the migration owns root locks.
pub fn capture_skill_resources(
    paths: BTreeSet<std::path::PathBuf>,
    state: &[u8],
) -> Result<Vec<u8>> {
    use std::io::Read;
    let mut directories = Vec::new();
    let mut files = Vec::new();
    let mut links = Vec::new();
    let mut remaining = 32 * 1024 * 1024usize;
    let mut count = 0usize;
    for root in paths {
        if std::fs::symlink_metadata(&root)?.file_type().is_symlink() {
            links.push((root.clone(), std::fs::read_link(&root)?));
            continue;
        }
        let mut pending = vec![root.clone()];
        while let Some(path) = pending.pop() {
            count += 1;
            if count > 30000 || path.components().count() > root.components().count() + 128 {
                return Err(invalid());
            }
            let metadata = std::fs::symlink_metadata(&path)?;
            if metadata.is_dir() {
                directories.push(path.clone());
                checked(crate::providers::paths::open_beneath(&root, &path, true))?;
                for entry in std::fs::read_dir(&path)? {
                    pending.push(entry?.path());
                }
            } else {
                if !metadata.is_file() {
                    return Err(invalid());
                }
                let file = checked(crate::providers::paths::open_beneath(&root, &path, false))?;
                let mut bytes = Vec::new();
                file.take(remaining as u64 + 1).read_to_end(&mut bytes)?;
                remaining = remaining.checked_sub(bytes.len()).ok_or_else(invalid)?;
                files.push((path, bytes));
            }
        }
    }
    checked(serde_json::to_vec(&(state, directories, files, links)))
}

fn check_resource_budget(root: &Path) -> Result<()> {
    let mut pending = vec![root.to_owned()];
    let mut entries = 0usize;
    let mut bytes = 0u64;
    while let Some(path) = pending.pop() {
        entries += 1;
        let metadata = std::fs::symlink_metadata(&path)?;
        if metadata.is_dir() {
            for entry in std::fs::read_dir(path)? {
                pending.push(entry?.path());
            }
        } else if metadata.is_file() {
            bytes = bytes.checked_add(metadata.len()).ok_or_else(invalid)?;
        } else {
            return Err(invalid());
        }
        if entries > crate::skills::archive::MAX_ARCHIVE_ENTRIES
            || bytes > crate::skills::archive::MAX_ARCHIVE_TOTAL_BYTES
        {
            return Err(invalid());
        }
    }
    Ok(())
}
