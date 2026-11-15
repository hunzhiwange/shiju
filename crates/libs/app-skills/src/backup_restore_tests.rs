use super::{backup_restore::SkillRestore, local_install::LocalInstall, uninstall::SkillUninstall};
use std::{collections::BTreeMap, fs};

#[test]
fn cp12_backup_restore_success() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("ssot");
    let backups = temp.path().join("backups");
    let source = temp.path().join("source");
    for path in [&root, &backups, &source] { fs::create_dir(path).unwrap(); }
    fs::write(source.join("SKILL.md"), "---\nname: example\n---\nbody").unwrap();
    let mut install = LocalInstall::prepare(&root, &source, "example").unwrap();
    install.publish().unwrap();
    let skill = install.finish();
    let mut uninstall = SkillUninstall::prepare(&root, &backups, &skill, &BTreeMap::new(), &BTreeMap::new()).unwrap();
    uninstall.publish().unwrap();
    let backup = uninstall.finish().unwrap();
    let mut restore = SkillRestore::prepare(&root, &backups, &backup.backup_id, &BTreeMap::new()).unwrap();
    let applied = restore.publish().unwrap();
    assert_eq!(applied.skill.source, root.join("example"));
    restore.finish();
    assert!(root.join("example/SKILL.md").is_file());
    assert!(backup.backup_path.join("skill/SKILL.md").is_file());
    assert!(SkillRestore::prepare(&root, &backups, &backup.backup_id, &BTreeMap::new()).is_err());
}

#[test]
fn cp12_backup_restore_rejects_without_side_effects() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("ssot");
    let backups = temp.path().join("backups");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&backups).unwrap();
    assert!(SkillRestore::prepare(&root, &backups, "../outside", &BTreeMap::new()).is_err());
    assert_eq!(fs::read_dir(root).unwrap().count(), 0);
    assert_eq!(fs::read_dir(backups).unwrap().count(), 0);
}
