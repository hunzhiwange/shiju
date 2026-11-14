//! Backup-first removal on the authorized execution host. Settings owners must
//! publish inside their transaction, roll back on failure, then finish on commit.
use std::{collections::BTreeMap, fs, io::{self, Read}, path::{Path, PathBuf}};
use serde::{Deserialize, Serialize};
use super::{
    app_paths::AppSkillsPath,
    copy_deploy::{binding_fingerprint, CopyBinding},
    disable_deploy::{DisableDeploy, DisableError},
    local_install::{copy_skill_tree, validate_directory, LocalInstallError, LocalSkill},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillUninstallResult {
    pub backup_id: String,
    pub backup_path: PathBuf,
    pub preserved_paths: Vec<PathBuf>,
    pub cleanup_incomplete: bool,
}

#[derive(Serialize)]
struct BackupMetadata<'a> {
    schema_version: u32,
    backup_created_at: u64,
    skill: &'a LocalSkill,
    bindings: &'a BTreeMap<String, CopyBinding>,
    fingerprint: &'a str,
}

pub struct SkillUninstall {
    projections: Vec<DisableDeploy>,
    // Drop projections before releasing the SSOT lock.
    ssot: DisableDeploy,
    result: SkillUninstallResult,
    published: bool,
    fingerprint: String,
    metadata: Vec<u8>,
}

impl SkillUninstall {
    /// `saved` and bindings must come from the scoped persisted snapshot; targets
    /// must be resolved and authorized on this host. Unresolved bindings are kept
    /// as preserved paths, never interpreted as permission to access their roots.
    pub fn prepare(root: &Path, backup_root: &Path, saved: &LocalSkill,
        bindings: &BTreeMap<String, CopyBinding>, targets: &BTreeMap<String, AppSkillsPath>)
        -> Result<Self, DisableError> {
        validate_directory(&saved.directory)?;
        if !root.is_absolute() || !backup_root.is_absolute() {
            return Err(DisableError::Preserved);
        }
        // Missing SSOT is an error: an application directory cannot stand in as
        // the backup source or prove ownership of external data.
        crate::providers::paths::open_beneath(root, root, true)
            .map_err(|_| DisableError::Preserved)?;
        crate::providers::paths::open_beneath(backup_root, backup_root, true)
            .map_err(|_| DisableError::Preserved)?;
        let root = fs::canonicalize(root)?;
        let backup_root = fs::canonicalize(backup_root)?;
        if overlaps(&root, &backup_root) { return Err(DisableError::Preserved); }
        let backup_dir = crate::providers::paths::open_beneath(&backup_root, &backup_root, true)
            .map_err(|_| DisableError::Preserved)?;
        let source = root.join(&saved.directory);
        let fingerprint = binding_fingerprint(&source, None)?;
        let owned = CopyBinding { directory: saved.directory.clone(), destination: source.clone(),
            fingerprint: fingerprint.clone(), link_target: None };
        let ssot = DisableDeploy::prepare_tree(&root, &owned)?;
        let mut selected: Vec<_> = bindings.iter()
            .filter(|(_, binding)| binding.directory == saved.directory).collect();
        selected.sort_by(|a, b| a.1.destination.cmp(&b.1.destination));
        let mut projections = Vec::new();
        let mut preserved_paths = Vec::new();
        let mut roots = vec![root.clone(), backup_root.clone()];
        for (key, binding) in selected {
            let Some(target) = targets.get(key) else {
                preserved_paths.push(binding.destination.clone());
                continue;
            };
            let physical = fs::canonicalize(&target.directory);
            let physical = match physical {
                Ok(path) => path,
                Err(_) => { preserved_paths.push(binding.destination.clone()); continue; }
            };
            // Reject overlap with backups even for a stale or unowned binding.
            if overlaps(&physical, &backup_root) { return Err(DisableError::Preserved); }
            // Aliases are locked only once. In particular an app root alias of
            // SSOT never grants permission to treat that root as a projection.
            if roots.iter().any(|root| overlaps(root, &physical)) {
                preserved_paths.push(binding.destination.clone());
                continue;
            }
            roots.push(physical);
            match DisableDeploy::prepare(&root, target, &saved.directory, Some(binding)) {
                Ok(deployment) => projections.push(deployment),
                Err(_) => preserved_paths.push(binding.destination.clone()),
            }
        }
        let backup = tempfile::Builder::new().prefix("skill-uninstall-").tempdir_in(&backup_root)?;
        let tree = backup.path().join("skill");
        copy_skill_tree(&source, &tree)?;
        if binding_fingerprint(&source, None)? != fingerprint
            || binding_fingerprint(&tree, None)? != fingerprint {
            return Err(DisableError::Preserved);
        }
        let saved_bindings = bindings.iter().filter(|(_, b)| b.directory == saved.directory)
            .map(|(key, binding)| (key.clone(), binding.clone())).collect();
        let metadata = serde_json::to_vec_pretty(&BackupMetadata {
            schema_version: 1,
            backup_created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| DisableError::Preserved)?.as_millis()
                .try_into().map_err(|_| DisableError::Preserved)?,
            skill: saved, bindings: &saved_bindings, fingerprint: &fingerprint,
        }).map_err(|_| LocalInstallError::InvalidSkill)?;
        fs::write(backup.path().join("meta.json"), &metadata)?;
        sync_tree(backup.path(), backup.path())?;
        let backup_id = backup.path().file_name().and_then(|s| s.to_str())
            .ok_or(DisableError::Preserved)?.to_owned();
        let backup_path = backup.keep();
        backup_dir.sync_all()?;
        Ok(Self { projections, ssot, result: SkillUninstallResult {
            backup_id, backup_path, cleanup_incomplete: !preserved_paths.is_empty(), preserved_paths,
        }, published: false, fingerprint, metadata })
    }

    pub fn publish(&mut self) -> Result<(), DisableError> {
        if self.published { return Err(DisableError::Preserved); }
        // Backup must remain intact through publication, not just preparation.
        let backup = &self.result.backup_path;
        let source = backup.join("skill");
        let mut metadata = Vec::new();
        crate::providers::paths::open_beneath(backup, &backup.join("meta.json"), false)
            .map_err(|_| DisableError::Preserved)?
            .take(self.metadata.len() as u64 + 1).read_to_end(&mut metadata)?;
        if metadata != self.metadata { return Err(DisableError::Preserved); }
        if binding_fingerprint(&source, None)? != self.fingerprint {
            return Err(DisableError::Preserved);
        }
        for projection in &mut self.projections { projection.publish()?; }
        self.ssot.publish()?;
        self.published = true;
        Ok(())
    }

    pub fn rollback(&mut self) -> Result<(), DisableError> {
        let mut failure = None;
        if let Err(error) = self.ssot.rollback() { failure = Some(error); }
        for projection in self.projections.iter_mut().rev() {
            if let Err(error) = projection.rollback() { failure = Some(error); }
        }
        self.published = false;
        match failure { Some(error) => Err(error), None => Ok(()) }
    }

    /// Only after settings commit. Cleanup failures retain their quarantine and
    /// are returned with the successful backup, rather than losing recovery data.
    pub fn finish(mut self) -> Result<SkillUninstallResult, DisableError> {
        if !self.published { return Err(DisableError::Preserved); }
        for deployment in self.projections.into_iter().chain(std::iter::once(self.ssot)) {
            let recovery = deployment.recovery_path();
            if deployment.finish().is_err() {
                self.result.cleanup_incomplete = true;
                if let Some(path) = recovery { self.result.preserved_paths.push(path); }
            }
        }
        self.result.preserved_paths.sort();
        self.result.preserved_paths.dedup();
        Ok(self.result)
    }
}

fn overlaps(a: &Path, b: &Path) -> bool { a.starts_with(b) || b.starts_with(a) }

fn sync_tree(root: &Path, path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.is_dir() {
        for entry in fs::read_dir(path)? { sync_tree(root, &entry?.path())?; }
    } else if !metadata.is_file() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "unsafe uninstall backup"));
    }
    crate::providers::paths::open_beneath(root, path, metadata.is_dir())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "unsafe uninstall backup"))?
        .sync_all()
}
