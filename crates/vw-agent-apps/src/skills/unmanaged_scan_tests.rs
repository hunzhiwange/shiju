use super::{app_paths, local_install::LocalSkill, unmanaged_scan::scan_unmanaged};
use std::{collections::BTreeMap, fs};
use vw_api_types::agent_apps::AppId;

#[test]
fn cp12_unmanaged_scan_success() {
    let temporary = tempfile::tempdir().unwrap();
    let home = temporary.path().canonicalize().unwrap();
    let ssot = home.join("ssot");
    fs::create_dir(&ssot).unwrap();
    let targets: Vec<_> = [AppId::Claude, AppId::Codex].into_iter().map(|app| {
        app_paths::resolve(&home, app, None, &BTreeMap::new(), &[&ssot]).unwrap()
    }).collect();
    for target in &targets {
        fs::create_dir_all(target.directory.join("sample")).unwrap();
        fs::write(target.directory.join("sample/SKILL.md"), "---\nname: Sample\n---\nInstructions").unwrap();
    }
    let rows = scan_unmanaged(&home, &ssot, &[], &targets).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].found_in.len(), 2);
    assert!(!rows[0].managed);
    assert!(!rows[0].conflict);
    fs::create_dir(ssot.join("sample")).unwrap();
    fs::copy(targets[0].directory.join("sample/SKILL.md"), ssot.join("sample/SKILL.md")).unwrap();
    let saved = LocalSkill {
        directory: "sample".into(), name: "Sample".into(), description: None,
        source: ssot.join("sample"),
    };
    let rows = scan_unmanaged(&home, &ssot, &[saved.clone()], &targets).unwrap();
    assert!(rows[0].managed);
    fs::write(targets[1].directory.join("sample/SKILL.md"), "Different instructions").unwrap();
    let rows = scan_unmanaged(&home, &ssot, &[saved], &targets).unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.conflict));
    assert_eq!(rows.iter().filter(|row| row.managed).count(), 1);
}

#[test]
fn cp12_unmanaged_scan_rejects_without_side_effects() {
    let temporary = tempfile::tempdir().unwrap();
    let home = temporary.path().canonicalize().unwrap();
    let ssot = home.join("ssot");
    fs::create_dir(&ssot).unwrap();
    let target = app_paths::resolve(&home, AppId::Claude, None, &BTreeMap::new(), &[&ssot]).unwrap();
    fs::create_dir_all(&target.directory).unwrap();
    let outside = tempfile::tempdir().unwrap();
    let private = outside.path().join("SKILL.md");
    fs::write(&private, "unchanged").unwrap();
    std::os::unix::fs::symlink(outside.path(), target.directory.join("linked")).unwrap();
    let rows = scan_unmanaged(&home, &ssot, &[], &[target.clone()]).unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].external_link);
    assert!(rows[0].content_hash.is_none());
    fs::create_dir(target.directory.join("unsafe")).unwrap();
    std::os::unix::fs::symlink(&private, target.directory.join("unsafe/SKILL.md")).unwrap();
    assert!(scan_unmanaged(&home, &ssot, &[], &[target.clone()]).is_err());
    assert_eq!(fs::read_to_string(private).unwrap(), "unchanged");
    assert_eq!(fs::read_dir(ssot).unwrap().count(), 0);
    assert!(fs::symlink_metadata(target.directory.join("linked")).unwrap().file_type().is_symlink());
}
