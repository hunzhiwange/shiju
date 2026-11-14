use super::{update_apply::SkillUpdate, inventory::compute_dir_hash, local_install::LocalSkill};
use std::{collections::BTreeMap, fs};

#[test]
fn cp12_update_apply_success() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("ssot");
    let source = temp.path().join("remote");
    fs::create_dir_all(root.join("example")).unwrap();
    fs::create_dir(&source).unwrap();
    fs::write(root.join("example/SKILL.md"), "# Old").unwrap();
    fs::write(source.join("SKILL.md"), "---\nname: New\n---\n# New").unwrap();
    let saved = LocalSkill { directory: "example".into(), name: "Old".into(), description: None, source: source.clone() };
    let hash = compute_dir_hash(&root.join("example")).unwrap();
    let mut update = SkillUpdate::prepare(&root, &source, &saved, &hash, &BTreeMap::new()).unwrap();
    assert_eq!(update.skill.name, "New");
    update.publish().unwrap();
    assert!(fs::read_to_string(root.join("example/SKILL.md")).unwrap().contains("# New"));
    update.rollback().unwrap();
    drop(update);
    assert_eq!(compute_dir_hash(&root.join("example")).unwrap(), hash);
    let mut update = SkillUpdate::prepare(&root, &source, &saved, &hash, &BTreeMap::new()).unwrap();
    update.publish().unwrap();
    update.finish();
    assert!(fs::read_to_string(root.join("example/SKILL.md")).unwrap().contains("# New"));
}

#[test]
fn cp12_update_apply_rejects_without_side_effects() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("ssot");
    let source = temp.path().join("remote");
    fs::create_dir_all(root.join("example")).unwrap();
    fs::create_dir(&source).unwrap();
    fs::write(root.join("example/SKILL.md"), "# Local edit").unwrap();
    fs::write(source.join("SKILL.md"), "# New").unwrap();
    let saved = LocalSkill { directory: "example".into(), name: "Old".into(), description: None, source: source.clone() };
    assert!(SkillUpdate::prepare(&root, &source, &saved, "stale", &BTreeMap::new()).is_err());
    assert_eq!(fs::read_to_string(root.join("example/SKILL.md")).unwrap(), "# Local edit");
}

#[test]
fn cp12_update_apply_projection_failure_restores_ssot_and_prior_copy() {
    use super::{app_paths::{AppSkillsPath, distribution}, copy_deploy::CopyDeploy};
    use vw_api_types::{agent_apps::AppId, app_providers::paths::ConfigPathState};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("ssot");
    let source = temp.path().join("remote");
    fs::create_dir_all(root.join("example")).unwrap();
    fs::create_dir(&source).unwrap();
    fs::write(root.join("example/SKILL.md"), "# Old").unwrap();
    fs::write(source.join("SKILL.md"), "# New").unwrap();
    let mut bindings = BTreeMap::new();
    for name in ["app-a", "app-b"] {
        let directory = temp.path().join(name);
        fs::create_dir(&directory).unwrap();
        let target = AppSkillsPath { app_id: AppId::Codex, directory,
            state: ConfigPathState::Present, distribution: distribution(AppId::Codex) };
        let mut copy = CopyDeploy::prepare(&root, &target, "example", None).unwrap();
        copy.publish().unwrap();
        bindings.insert(name.to_owned(), copy.finish());
    }
    let saved = LocalSkill { directory: "example".into(), name: "Old".into(), description: None, source: source.clone() };
    let hash = compute_dir_hash(&root.join("example")).unwrap();
    let mut update = SkillUpdate::prepare(&root, &source, &saved, &hash, &bindings).unwrap();
    // An outside writer ignores our advisory lock after staging.
    fs::write(temp.path().join("app-b/example/SKILL.md"), "# Outside edit").unwrap();
    assert!(update.publish().is_err());
    update.rollback().unwrap();
    drop(update);
    assert_eq!(compute_dir_hash(&root.join("example")).unwrap(), hash);
    assert_eq!(fs::read_to_string(temp.path().join("app-a/example/SKILL.md")).unwrap(), "# Old");
    assert_eq!(fs::read_to_string(temp.path().join("app-b/example/SKILL.md")).unwrap(), "# Outside edit");
}
