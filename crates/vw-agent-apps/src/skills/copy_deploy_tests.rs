use super::{app_paths::{AppSkillsPath, distribution}, copy_deploy::CopyDeploy};
use std::fs;
use vw_api_types::{agent_apps::AppId, app_providers::paths::ConfigPathState};

fn fixture() -> (tempfile::TempDir, std::path::PathBuf, AppSkillsPath) {
    let temp = tempfile::tempdir().unwrap();
    let ssot = temp.path().join("ssot");
    let app = temp.path().join("app");
    fs::create_dir_all(ssot.join("example")).unwrap();
    fs::create_dir(&app).unwrap();
    fs::write(ssot.join("example/SKILL.md"), "# Example").unwrap();
    let target = AppSkillsPath { app_id: AppId::Codex, directory: app,
        state: ConfigPathState::Present, distribution: distribution(AppId::Codex) };
    (temp, ssot, target)
}

#[test]
fn cp12_copy_deploy_success() {
    let (_temp, ssot, target) = fixture();
    let mut copy = CopyDeploy::prepare(&ssot, &target, "example", None).unwrap();
    assert!(!target.directory.join("example").exists());
    copy.publish().unwrap();
    let binding = copy.finish();
    assert_eq!(fs::read_to_string(binding.destination.join("SKILL.md")).unwrap(), "# Example");
    fs::write(ssot.join("example/SKILL.md"), "# Updated").unwrap();
    let mut update = CopyDeploy::prepare(&ssot, &target, "example", Some(&binding)).unwrap();
    update.publish().unwrap();
    assert_eq!(fs::read_to_string(binding.destination.join("SKILL.md")).unwrap(), "# Updated");
    // A settings failure restores the previous committed copy.
    update.rollback().unwrap();
    drop(update);
    assert_eq!(fs::read_to_string(binding.destination.join("SKILL.md")).unwrap(), "# Example");
}

#[test]
fn cp12_copy_deploy_rejects_without_side_effects() {
    let (_temp, ssot, target) = fixture();
    fs::create_dir(target.directory.join("example")).unwrap();
    fs::write(target.directory.join("example/SKILL.md"), "external").unwrap();
    assert!(CopyDeploy::prepare(&ssot, &target, "example", None).is_err());
    assert_eq!(fs::read_to_string(target.directory.join("example/SKILL.md")).unwrap(), "external");
    fs::remove_dir_all(target.directory.join("example")).unwrap();
    let mut copy = CopyDeploy::prepare(&ssot, &target, "example", None).unwrap();
    copy.publish().unwrap();
    let binding = copy.finish();
    fs::write(binding.destination.join("SKILL.md"), "user edit").unwrap();
    assert!(CopyDeploy::prepare(&ssot, &target, "example", Some(&binding)).is_err());
    assert_eq!(fs::read_to_string(binding.destination.join("SKILL.md")).unwrap(), "user edit");
    assert!(CopyDeploy::prepare(&ssot, &target, "../escape", None).is_err());
}
