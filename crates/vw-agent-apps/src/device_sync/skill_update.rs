//! Staged update of one SSOT and its existing managed projections.
//! The caller holds the settings transaction until finish or explicit rollback.
use std::{collections::BTreeMap, path::Path};
use crate::skills::{
    copy_deploy::{binding_fingerprint, CopyBinding, CopyDeploy},
    inventory::{compute_dir_hash, get_all_installed},
    link_deploy::SyncMethod,
    local_install::{copy_skill_tree, validate_directory, LocalInstallError, LocalSkill},
};

type Result<T> = std::result::Result<T, LocalInstallError>;

pub struct BatchSkillUpdate {
    // Retain the SSOT lock until application rollback has finished.
    projections: Vec<(String, CopyDeploy)>,
    ssot: CopyDeploy,
    _source: tempfile::TempDir,
    pub skill: LocalSkill,
    pub content_hash: String,
}

impl BatchSkillUpdate {
    /// Prepare a batch under shared physical-root leases, avoiding reacquiring
    /// the same flock through independent file descriptions for each skill.
    pub fn prepare_batch(root: &Path, source: &Path, saved: &[LocalSkill],
        hashes: &BTreeMap<String, String>, bindings: &BTreeMap<String, CopyBinding>) -> Result<Vec<Self>> {
        let mut roots = std::collections::BTreeSet::from([std::fs::canonicalize(root)?]);
        for binding in bindings.values().filter(|b| saved.iter().any(|s| s.directory == b.directory)) {
            roots.insert(std::fs::canonicalize(binding.destination.parent().ok_or(LocalInstallError::UnsafeSource)?)?);
        }
        let mut leases = BTreeMap::new();
        // Stable order across processes. Reject overlaps before acquiring any lock.
        let paths: Vec<_> = roots.into_iter().collect();
        for (i, path) in paths.iter().enumerate() {
            if paths[..i].iter().any(|p| p.starts_with(path) || path.starts_with(p)) {
                return Err(LocalInstallError::UnsafeSource);
            }
        }
        for path in paths {
            let file = crate::providers::paths::open_beneath(&path, &path, true)
                .map_err(|_| LocalInstallError::UnsafeSource)?;
            use std::os::fd::AsRawFd;
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            leases.insert(path, file);
        }
        saved.iter().map(|skill| Self::prepare_locked(root, &source.join(&skill.directory), skill,
            hashes.get(&skill.directory).ok_or(LocalInstallError::Conflict)?, bindings, &leases)).collect()
    }

    fn prepare_locked(root: &Path, source: &Path, saved: &LocalSkill,
        expected_hash: &str, bindings: &BTreeMap<String, CopyBinding>,
        leases: &BTreeMap<std::path::PathBuf, std::fs::File>) -> Result<Self> {
        validate_directory(&saved.directory)?;
        if !root.is_absolute() || !source.is_absolute() { return Err(LocalInstallError::UnsafeSource); }
        let canonical_root = std::fs::canonicalize(root)?;
        let root = canonical_root.as_path();
        let current = root.join(&saved.directory);
        let old = CopyBinding { directory: saved.directory.clone(), destination: current.clone(),
            fingerprint: binding_fingerprint(&current, None)?, link_target: None };
        let stage = tempfile::tempdir()?;
        copy_skill_tree(source, &stage.path().join(&saved.directory))?;
        let ssot = CopyDeploy::prepare_tree_locked(stage.path(), root, &saved.directory, Some(&old), SyncMethod::Copy, leases.get(root))?;
        // Checked under the SSOT lock, after publication staging.
        if compute_dir_hash(&current)? != expected_hash { return Err(LocalInstallError::Conflict); }
        let detail = get_all_installed(stage.path(), std::slice::from_ref(saved))?
            .pop().ok_or(LocalInstallError::InvalidSkill)?;
        let mut skill = detail.skill;
        skill.source = saved.source.clone();
        let content_hash = detail.content_hash.ok_or(LocalInstallError::InvalidSkill)?;
        let mut selected: Vec<_> = bindings.iter().filter(|(_, b)| b.directory == saved.directory).collect();
        selected.sort_by(|a, b| a.1.destination.cmp(&b.1.destination));
        // Each physical root is locked once; aliases/overlaps are rejected.
        let mut roots = vec![std::fs::canonicalize(root)?];
        let mut projections = Vec::new();
        for (key, binding) in selected {
            let parent = binding.destination.parent().ok_or(LocalInstallError::UnsafeSource)?;
            let parent = std::fs::canonicalize(parent)?;
            if roots.iter().any(|p| p.starts_with(&parent) || parent.starts_with(p)) {
                return Err(LocalInstallError::UnsafeSource);
            }
            roots.push(parent.clone());
            // Links retain their stable SSOT target and ownership fingerprint.
            // Stage them from the live root, copies from the new source.
            let source_root = if binding.link_target.is_some() { root } else { stage.path() };
            let projection = CopyDeploy::prepare_tree_locked(source_root, &parent, &saved.directory,
                Some(binding), SyncMethod::Auto, leases.get(&parent))?;
            projections.push((key.clone(), projection));
        }
        Ok(Self { projections, ssot, _source: stage, skill, content_hash })
    }

    pub fn bindings(&self) -> impl Iterator<Item = (&str, &CopyBinding)> {
        self.projections.iter().map(|(key, copy)| (key.as_str(), copy.binding()))
    }

    pub fn publish(&mut self) -> Result<()> {
        self.ssot.publish()?;
        for (_, copy) in &mut self.projections { copy.publish()?; }
        Ok(())
    }

    pub fn rollback(&mut self) -> Result<()> {
        let mut failure = None;
        for (_, copy) in self.projections.iter_mut().rev() {
            if let Err(error) = copy.rollback() { failure = Some(error); }
        }
        if let Err(error) = self.ssot.rollback() { failure = Some(error); }
        match failure { Some(error) => Err(error), None => Ok(()) }
    }

    pub fn finish(self) {
        for (_, copy) in self.projections { copy.finish(); }
        self.ssot.finish();
    }
}

