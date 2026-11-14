use super::{
    app_paths, unmanaged_scan::{scan_unmanaged, SkillLocation},
    unmanaged_import::{prepare_import, PreparedImport, UnmanagedSelection},
};
use std::{collections::BTreeMap, fs};
use vw_api_types::agent_apps::AppId;

#[test]
fn cp12_unmanaged_import_success() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().canonicalize().unwrap();
    let ssot = home.join("ssot");
    fs::create_dir(&ssot).unwrap();
    let targets: Vec<_> = [AppId::Claude, AppId::Codex].into_iter().map(|app| {
        app_paths::resolve(&home, app, None, &BTreeMap::new(), &[&ssot]).unwrap()
    }).collect();
    for target in &targets {
        fs::create_dir_all(target.directory.join("sample")).unwrap();
        fs::write(target.directory.join("sample/SKILL.md"), "Instructions").unwrap();
    }
    let rows = scan_unmanaged(&home, &ssot, &[], &targets).unwrap();
    let selection = UnmanagedSelection {
        directory: "sample".into(), content_hash: rows[0].content_hash.clone().unwrap(),
        sources: vec![SkillLocation { app_id: targets[0].app_id,
            path: targets[0].directory.join("sample") }],
    };
    let PreparedImport::New(mut import) = prepare_import(&home, &ssot, &[], &targets, &selection).unwrap()
        else { panic!("expected new import") };
    import.publish().unwrap();
    let saved = import.finish();
    assert_eq!(fs::read_to_string(ssot.join("sample/SKILL.md")).unwrap(), "Instructions");
    assert!(matches!(prepare_import(&home, &ssot, &[saved], &targets, &selection).unwrap(),
        PreparedImport::Existing(_)));
    for target in &targets {
        assert_eq!(fs::read_to_string(target.directory.join("sample/SKILL.md")).unwrap(), "Instructions");
    }
}

#[test]
fn cp12_unmanaged_import_rejects_without_side_effects() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().canonicalize().unwrap();
    let ssot = home.join("ssot");
    fs::create_dir(&ssot).unwrap();
    let target = app_paths::resolve(&home, AppId::Claude, None, &BTreeMap::new(), &[&ssot]).unwrap();
    fs::create_dir_all(target.directory.join("sample")).unwrap();
    let document = target.directory.join("sample/SKILL.md");
    fs::write(&document, "Original").unwrap();
    let targets = [target];
    let rows = scan_unmanaged(&home, &ssot, &[], &targets).unwrap();
    let mut selection = UnmanagedSelection {
        directory: "sample".into(), content_hash: rows[0].content_hash.clone().unwrap(),
        sources: rows[0].found_in.clone(),
    };
    fs::write(&document, "Changed").unwrap();
    assert!(prepare_import(&home, &ssot, &[], &targets, &selection).is_err());
    selection.sources.clear();
    assert!(prepare_import(&home, &ssot, &[], &targets, &selection).is_err());
    assert_eq!(fs::read_dir(&ssot).unwrap().count(), 0);
    assert_eq!(fs::read_to_string(document).unwrap(), "Changed");
}
