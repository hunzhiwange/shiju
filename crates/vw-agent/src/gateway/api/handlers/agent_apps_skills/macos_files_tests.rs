//! Host-local lifecycle acceptance through the production settings adapter.
//! ACP permission revocation remains covered by acp::mounts_tests; distribution
//! must never add session selections or full_access grants.
use std::{collections::BTreeMap, fs, path::PathBuf};
use anyhow::Result;
use vw_agent_apps::skills::{app_paths, bulk_sync::SkillSyncTarget, link_deploy::SyncMethod};
use vw_api_types::{agent_apps::AppId, id::{ProjectScope, TenantScope}};
use crate::{global_database::GlobalDatabase, gateway::agent_apps_skills_store::SkillInstallsStore};

struct Fixture {
    _home: tempfile::TempDir,
    home: PathBuf,
    root: PathBuf,
    destination: PathBuf,
    backups: PathBuf,
    store: SkillInstallsStore,
    scope: ProjectScope,
    targets: Vec<SkillSyncTarget>,
}
impl Fixture {
    fn new() -> Result<Self> {
        let temporary = tempfile::tempdir()?;
        // Canonicalize macOS /var aliases without changing process HOME.
        let home = temporary.path().canonicalize()?;
        let root = home.join("runtime");
        let destination = home.join(".agents/skills");
        let backups = home.join("backups");
        for path in [&root, &destination, &backups] { fs::create_dir_all(path)?; }
        let source = home.join("source");
        fs::create_dir(&source)?;
        fs::write(source.join("SKILL.md"), "---\nname: Probe\ndescription: Lifecycle\n---\nInstructions")?;
        let store = SkillInstallsStore::new(GlobalDatabase::for_home(&home)?)?;
        let scope = ProjectScope::new(TenantScope::new("p", "c")?, "project".into())?;
        store.install_local(&scope, "host", 0, &root, &source, "probe")?;
        let mut targets = Vec::new();
        for (installation, app) in [("copy", AppId::Claude), ("link", AppId::Codex)] {
            let path = app_paths::resolve(&home, app, None, &BTreeMap::new(), &[&root, &backups])?;
            fs::create_dir_all(&path.directory)?;
            targets.push(SkillSyncTarget { installation_id: installation.into(), path });
        }
        Ok(Self { _home: temporary, home, root, destination, backups, store, scope, targets })
    }
    fn bindings(&self) -> Result<BTreeMap<String, app_paths::AppSkillsPath>> {
        self.targets.iter().map(|target| Ok((serde_json::to_string(&(
            &target.installation_id, target.path.app_id, "probe"))?, target.path.clone()))).collect()
    }
}

#[test]
fn cp12_macos_files_success() -> Result<()> {
    let f = Fixture::new()?;
    for (index, method) in [SyncMethod::Copy, SyncMethod::Symlink].into_iter().enumerate() {
        let target = &f.targets[index];
        f.store.enable_with_method(&f.scope, "host", 1 + index as u64, &f.root,
            &target.installation_id, &target.path, "probe", method)?;
    }
    let copy = f.targets[0].path.directory.join("probe");
    let link = f.targets[1].path.directory.join("probe");
    assert!(!fs::symlink_metadata(&copy)?.file_type().is_symlink());
    assert_eq!(fs::read_link(&link)?, f.root.join("probe"));
    fs::write(copy.join("external.txt"), "keep external edit")?;
    let moved = f.store.migrate_storage(&f.scope, "host", 3, &f.root, &f.destination,
        &f.home, &f.root, &f.targets)?;
    assert!(moved.switched);
    assert!(moved.errors.is_empty());
    assert_eq!(fs::read_link(&link)?, f.destination.join("probe"));
    assert!(f.root.join("probe/SKILL.md").is_file());
    let targets = f.bindings()?;
    let (removed, backup) = f.store.uninstall_skill(&f.scope, "host", 4,
        &f.destination, &f.backups, "probe", &targets)?;
    assert_eq!(removed.version, 5);
    assert!(removed.skills.is_empty());
    assert!(backup.preserved_paths.contains(&copy));
    assert_eq!(fs::read(copy.join("external.txt"))?, b"keep external edit");
    assert!(fs::symlink_metadata(&link).is_err());
    assert!(!f.destination.join("probe").exists());
    assert!(backup.backup_path.join("skill/SKILL.md").is_file());
    // Restore only the removed projection; the externally edited copy is foreign.
    let restore_targets = targets.into_iter().filter(|(_, target)| target.app_id == AppId::Codex).collect();
    let (restored, _) = f.store.restore_skill_backup(&f.scope, "host", 5,
        &f.destination, &f.backups, &backup.backup_id, &restore_targets)?;
    assert_eq!(restored.version, 6);
    assert_eq!(restored.skills.len(), 1);
    assert_eq!(fs::read(link.join("SKILL.md"))?, fs::read(f.destination.join("probe/SKILL.md"))?);
    assert_eq!(fs::read(copy.join("external.txt"))?, b"keep external edit");
    assert!(backup.backup_path.join("skill/SKILL.md").is_file());
    assert_eq!(f.store.get(&f.scope, "host", &f.destination)?, restored);
    Ok(())
}

#[test]
fn cp12_macos_files_rejects_without_side_effects() -> Result<()> {
    let f = Fixture::new()?;
    let before = f.store.get(&f.scope, "host", &f.root)?;
    // An arbitrary existing host directory is not an authorized SSOT choice.
    assert!(f.store.migrate_storage(&f.scope, "host", 1, &f.root, &f.backups,
        &f.home, &f.root, &[]).is_err());
    assert!(f.store.uninstall_skill(&f.scope, "host", 0, &f.root,
        &f.backups, "probe", &BTreeMap::new()).is_err());
    assert!(f.store.restore_skill_backup(&f.scope, "host", 1, &f.root,
        &f.backups, "../escape", &BTreeMap::new()).is_err());
    assert_eq!(f.store.get(&f.scope, "host", &f.root)?, before);
    assert!(f.root.join("probe/SKILL.md").is_file());
    assert_eq!(fs::read_dir(&f.backups)?.count(), 0);
    assert_eq!(fs::read_dir(&f.destination)?.count(), 0);
    for target in &f.targets { assert_eq!(fs::read_dir(&target.path.directory)?.count(), 0); }
    Ok(())
}
