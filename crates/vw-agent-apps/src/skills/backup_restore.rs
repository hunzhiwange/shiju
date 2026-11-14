//! Restore on an authorized host; retain the backup even after settings commit.
use std::{collections::BTreeMap, fs, io::Read, path::{Path, PathBuf}};
use serde::{Deserialize, Serialize};
use super::{app_paths::AppSkillsPath, copy_deploy::{binding_fingerprint, CopyBinding, CopyDeploy},
    local_install::{LocalInstall, LocalInstallError, LocalSkill}};
use crate::providers::paths::open_beneath;

type Result<T> = std::result::Result<T, LocalInstallError>;
#[derive(Deserialize)]
struct Metadata {
    schema_version: u32,
    skill: LocalSkill,
    fingerprint: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRestoreResult {
    pub skill: LocalSkill,
    pub bindings: BTreeMap<String, CopyBinding>,
}

pub struct SkillRestore {
    projections: BTreeMap<String, CopyDeploy>,
    install: Option<LocalInstall>,
    root: PathBuf,
    backup: PathBuf,
    metadata: Vec<u8>,
    fingerprint: String,
    targets: BTreeMap<String, AppSkillsPath>,
    published: bool,
}
impl SkillRestore {
    /// Targets are an explicit authorized selection, never paths from backup metadata.
    /// Keys use the persisted (installation ID, app ID, directory) binding identity.
    pub fn prepare(root: &Path, backup_root: &Path, id: &str,
        targets: &BTreeMap<String, AppSkillsPath>) -> Result<Self> {
        if !root.is_absolute() || !backup_root.is_absolute() {
            return Err(LocalInstallError::UnsafeSource);
        }
        open_beneath(root, root, true).map_err(|_| LocalInstallError::UnsafeSource)?;
        open_beneath(backup_root, backup_root, true).map_err(|_| LocalInstallError::UnsafeSource)?;
        let root = fs::canonicalize(root)?;
        let backups = fs::canonicalize(backup_root)?;
        let mut roots = vec![root.clone(), backups.clone()];
        if overlaps(&root, &backups) { return Err(LocalInstallError::UnsafeSource); }
        for target in targets.values() {
            target.require_distribution().map_err(|_| LocalInstallError::UnsafeSource)?;
            open_beneath(&target.directory, &target.directory, true)
                .map_err(|_| LocalInstallError::UnsafeSource)?;
            let physical = fs::canonicalize(&target.directory)?;
            if roots.iter().any(|other| overlaps(other, &physical)) {
                return Err(LocalInstallError::UnsafeSource);
            }
            roots.push(physical);
        }
        // Catalog validates the ID and schema without interpreting saved destinations.
        let entry = super::backup_catalog::list_backups(&backups)?.into_iter()
            .find(|entry| entry.backup_id == id).ok_or(LocalInstallError::InvalidSkill)?;
        let backup = entry.backup_path;
        let metadata = read_metadata(&backup)?;
        let meta: Metadata = serde_json::from_slice(&metadata).map_err(|_| LocalInstallError::InvalidSkill)?;
        if meta.schema_version != 1 || binding_fingerprint(&backup.join("skill"), None)? != meta.fingerprint {
            return Err(LocalInstallError::InvalidSkill);
        }
        let install = LocalInstall::prepare(&root, &backup.join("skill"), &meta.skill.directory)?;
        if install.skill().name != meta.skill.name || install.skill().description != meta.skill.description {
            return Err(LocalInstallError::InvalidSkill);
        }
        // Source is the restored SSOT, never the old source path embedded in metadata.
        Ok(Self { projections: BTreeMap::new(), install: Some(install), root, backup,
            metadata, fingerprint: meta.fingerprint, targets: targets.clone(), published: false })
    }
    pub fn skill(&self) -> &LocalSkill { self.install.as_ref().expect("active restore").skill() }
    pub fn publish(&mut self) -> Result<SkillRestoreResult> {
        if self.published { return Err(LocalInstallError::Conflict); }
        if read_metadata(&self.backup)? != self.metadata
            || binding_fingerprint(&self.backup.join("skill"), None)? != self.fingerprint {
            return Err(LocalInstallError::InvalidSkill);
        }
        let directory = self.skill().directory.clone();
        let result = (|| {
            self.install.as_mut().ok_or(LocalInstallError::Conflict)?.publish()?;
            if binding_fingerprint(&self.root.join(&directory), None)? != self.fingerprint {
                return Err(LocalInstallError::InvalidSkill);
            }
            for (key, target) in &self.targets {
                let mut deploy = CopyDeploy::prepare(&self.root, target, &directory, None)?;
                deploy.publish()?;
                self.projections.insert(key.clone(), deploy);
            }
            Ok(())
        })();
        if let Err(error) = result { self.rollback()?; return Err(error); }
        self.published = true;
        let mut skill = self.skill().clone();
        skill.source = self.root.join(&directory);
        Ok(SkillRestoreResult { skill, bindings: self.projections.iter()
            .map(|(key, deploy)| (key.clone(), deploy.binding().clone())).collect() })
    }
    pub fn rollback(&mut self) -> Result<()> {
        // Keep SSOT if a projection cannot safely be removed.
        for deploy in self.projections.values_mut().rev() { deploy.rollback()?; }
        self.projections.clear();
        if let Some(install) = self.install.as_mut() { install.rollback()?; }
        self.published = false;
        Ok(())
    }
    /// Disarm only after the settings transaction commits.
    pub fn finish(mut self) {
        for (_, deploy) in std::mem::take(&mut self.projections) { deploy.finish(); }
        if let Some(install) = self.install.take() { install.finish(); }
    }
}
fn overlaps(a: &Path, b: &Path) -> bool { a.starts_with(b) || b.starts_with(a) }
fn read_metadata(backup: &Path) -> Result<Vec<u8>> {
    let file = open_beneath(backup, &backup.join("meta.json"), false)
        .map_err(|_| LocalInstallError::UnsafeSource)?;
    if !file.metadata()?.is_file() { return Err(LocalInstallError::InvalidSkill); }
    let mut bytes = Vec::new();
    file.take(1024 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 1024 * 1024 { return Err(LocalInstallError::LimitExceeded); }
    Ok(bytes)
}
