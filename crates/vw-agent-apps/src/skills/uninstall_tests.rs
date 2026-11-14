use std::{collections::BTreeMap, fs, path::PathBuf};
use super::{app_paths::{distribution, AppSkillsPath}, copy_deploy::CopyBinding,
    link_deploy::{prepare_deploy, SyncMethod}, local_install::LocalSkill, uninstall::SkillUninstall};
use vw_api_types::{agent_apps::AppId, app_providers::paths::ConfigPathState};

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    backups: PathBuf,
    saved: LocalSkill,
    bindings: BTreeMap<String, CopyBinding>,
    targets: BTreeMap<String, AppSkillsPath>,
}

fn fixture(method: SyncMethod) -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("ssot");
    let backups = temp.path().join("backups");
    let app = temp.path().join("app");
    fs::create_dir_all(root.join("example")).unwrap();
    fs::create_dir(&backups).unwrap();
    fs::create_dir(&app).unwrap();
    fs::write(root.join("example/SKILL.md"), "# Example").unwrap();
    fs::write(root.join("example/.private-resource"), "included in backup").unwrap();
    let target = AppSkillsPath { app_id: AppId::Pi, directory: app,
        state: ConfigPathState::Present, distribution: distribution(AppId::Pi) };
    let mut deployment = prepare_deploy(&root, &target, "example", None, method).unwrap();
    deployment.publish().unwrap();
    let binding = deployment.finish();
    let saved = LocalSkill { directory: "example".into(), name: "Example".into(),
        description: None, source: temp.path().join("original") };
    Fixture { _temp: temp, root, backups, saved,
        bindings: BTreeMap::from([("pi".into(), binding)]),
        targets: BTreeMap::from([("pi".into(), target)]) }
}

#[test]
fn cp12_uninstall_success() {
    for method in [SyncMethod::Copy, SyncMethod::Symlink] {
        let f = fixture(method);
        let mut uninstall = SkillUninstall::prepare(&f.root, &f.backups, &f.saved, &f.bindings, &f.targets).unwrap();
        uninstall.publish().unwrap();
        let result = uninstall.finish().unwrap();
        assert!(!result.cleanup_incomplete);
        assert!(result.preserved_paths.is_empty());
        assert_eq!(result.backup_path.file_name().unwrap().to_str().unwrap(), result.backup_id);
        assert_eq!(fs::read_to_string(result.backup_path.join("skill/SKILL.md")).unwrap(), "# Example");
        assert_eq!(fs::read_to_string(result.backup_path.join("skill/.private-resource")).unwrap(), "included in backup");
        assert!(result.backup_path.join("meta.json").is_file());
        assert!(!f.root.join("example").exists());
        assert!(fs::symlink_metadata(&f.bindings["pi"].destination).is_err());
        assert!(f.root.is_dir());
        assert!(f.targets["pi"].directory.is_dir());
    }
}

#[test]
fn cp12_uninstall_rejects_without_side_effects() {
    let f = fixture(SyncMethod::Copy);
    let invalid_backup = f.root.join("example");
    assert!(SkillUninstall::prepare(&f.root, &invalid_backup, &f.saved, &f.bindings, &f.targets).is_err());
    let missing_backup = f.backups.join("missing");
    assert!(SkillUninstall::prepare(&f.root, &missing_backup, &f.saved, &f.bindings, &f.targets).is_err());
    assert_eq!(fs::read_to_string(f.root.join("example/SKILL.md")).unwrap(), "# Example");
    assert_eq!(fs::read_to_string(f.bindings["pi"].destination.join("SKILL.md")).unwrap(), "# Example");
    fs::remove_dir_all(f.root.join("example")).unwrap();
    assert!(SkillUninstall::prepare(&f.root, &f.backups, &f.saved, &f.bindings, &f.targets).is_err());
    assert_eq!(fs::read_dir(&f.backups).unwrap().count(), 0);
    assert!(f.bindings["pi"].destination.is_dir());
}

#[test]
fn cp12_uninstall_preserves_external_edits_and_unresolved_bindings() {
    for resolved in [true, false] {
        let f = fixture(SyncMethod::Copy);
        fs::write(f.bindings["pi"].destination.join("SKILL.md"), "external edit").unwrap();
        let targets = if resolved { f.targets.clone() } else { BTreeMap::new() };
        let mut uninstall = SkillUninstall::prepare(&f.root, &f.backups, &f.saved, &f.bindings, &targets).unwrap();
        uninstall.publish().unwrap();
        let result = uninstall.finish().unwrap();
        assert!(result.cleanup_incomplete);
        assert!(result.preserved_paths.contains(&f.bindings["pi"].destination));
        assert_eq!(fs::read_to_string(f.bindings["pi"].destination.join("SKILL.md")).unwrap(), "external edit");
        assert_eq!(fs::read_to_string(result.backup_path.join("skill/SKILL.md")).unwrap(), "# Example");
    }
}

#[test]
fn cp12_uninstall_failed_commit_restores_sources() {
    let f = fixture(SyncMethod::Symlink);
    let mut uninstall = SkillUninstall::prepare(&f.root, &f.backups, &f.saved, &f.bindings, &f.targets).unwrap();
    uninstall.publish().unwrap();
    uninstall.rollback().unwrap();
    drop(uninstall);
    assert_eq!(fs::read_to_string(f.root.join("example/SKILL.md")).unwrap(), "# Example");
    assert_eq!(fs::read_link(&f.bindings["pi"].destination).unwrap(), f.root.join("example"));
}

#[test]
fn cp12_uninstall_rechecks_backup_before_removal() {
    let f = fixture(SyncMethod::Copy);
    let mut uninstall = SkillUninstall::prepare(&f.root, &f.backups, &f.saved, &f.bindings, &f.targets).unwrap();
    let backup = fs::read_dir(&f.backups).unwrap().next().unwrap().unwrap().path();
    fs::write(backup.join("skill/SKILL.md"), "damaged backup").unwrap();
    assert!(uninstall.publish().is_err());
    drop(uninstall);
    assert!(f.root.join("example/SKILL.md").is_file());
    assert!(f.bindings["pi"].destination.is_dir());
}

#[test]
fn cp12_uninstall_copy_failure_keeps_managed_trees() {
    let f = fixture(SyncMethod::Copy);
    fs::write(f.root.join("example/SKILL.md"), "").unwrap();
    assert!(SkillUninstall::prepare(&f.root, &f.backups, &f.saved, &f.bindings, &f.targets).is_err());
    assert_eq!(fs::read_to_string(f.root.join("example/SKILL.md")).unwrap(), "");
    assert_eq!(fs::read_to_string(f.bindings["pi"].destination.join("SKILL.md")).unwrap(), "# Example");
    assert_eq!(fs::read_dir(&f.backups).unwrap().count(), 0);
}

#[test]
fn cp12_uninstall_preserves_alias_root_and_external_link() {
    let mut f = fixture(SyncMethod::Copy);
    let alias = f._temp.path().join("alias");
    std::os::unix::fs::symlink(&f.targets["pi"].directory, &alias).unwrap();
    let mut target = f.targets["pi"].clone();
    target.directory = alias.clone();
    f.targets.insert("alias".into(), target);
    f.bindings.insert("alias".into(), f.bindings["pi"].clone());
    let external = f._temp.path().join("external");
    fs::create_dir(&external).unwrap();
    fs::write(external.join("SKILL.md"), "external").unwrap();
    fs::remove_dir_all(&f.bindings["pi"].destination).unwrap();
    std::os::unix::fs::symlink(&external, &f.bindings["pi"].destination).unwrap();
    let mut uninstall = SkillUninstall::prepare(&f.root, &f.backups, &f.saved, &f.bindings, &f.targets).unwrap();
    uninstall.publish().unwrap();
    let result = uninstall.finish().unwrap();
    assert!(result.cleanup_incomplete);
    assert!(fs::symlink_metadata(&alias).unwrap().file_type().is_symlink());
    assert_eq!(fs::read_link(&f.bindings["pi"].destination).unwrap(), external);
    assert_eq!(fs::read_to_string(external.join("SKILL.md")).unwrap(), "external");
}

#[test]
fn cp12_uninstall_rejects_external_replacement_after_backup() {
    let f = fixture(SyncMethod::Copy);
    let mut uninstall = SkillUninstall::prepare(&f.root, &f.backups, &f.saved, &f.bindings, &f.targets).unwrap();
    fs::write(f.bindings["pi"].destination.join("SKILL.md"), "external replacement").unwrap();
    assert!(uninstall.publish().is_err());
    uninstall.rollback().unwrap();
    drop(uninstall);
    assert!(f.root.join("example/SKILL.md").is_file());
    assert_eq!(fs::read_to_string(f.bindings["pi"].destination.join("SKILL.md")).unwrap(), "external replacement");
}
