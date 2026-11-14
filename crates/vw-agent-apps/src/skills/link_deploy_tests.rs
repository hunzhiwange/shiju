use super::{app_paths::{AppSkillsPath, distribution}, link_deploy::{SyncMethod, prepare_deploy}};
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
fn cp12_link_deploy_success() {
    for method in [SyncMethod::Symlink, SyncMethod::Auto] {
        let (_temp, ssot, target) = fixture();
        let mut deployment = prepare_deploy(&ssot, &target, "example", None, method).unwrap();
        deployment.publish().unwrap();
        let binding = deployment.finish();
        assert_eq!(binding.link_target, Some(fs::canonicalize(&ssot).unwrap().join("example")));
        assert_eq!(fs::read_link(&binding.destination).unwrap(), binding.link_target.clone().unwrap());
        fs::write(ssot.join("example/SKILL.md"), "# Changed").unwrap();
        assert_eq!(fs::read_to_string(binding.destination.join("SKILL.md")).unwrap(), "# Changed");
        let mut update = prepare_deploy(&ssot, &target, "example", Some(&binding), method).unwrap();
        update.publish().unwrap();
        update.rollback().unwrap();
        drop(update);
        assert_eq!(fs::read_link(&binding.destination).unwrap(), binding.link_target.unwrap());
    }
}

#[test]
fn cp12_link_deploy_rejects_without_side_effects() {
    for method in [SyncMethod::Symlink, SyncMethod::Auto] {
        let (_temp, ssot, target) = fixture();
        let destination = target.directory.join("example");
        fs::create_dir(&destination).unwrap();
        fs::write(destination.join("SKILL.md"), "external").unwrap();
        assert!(prepare_deploy(&ssot, &target, "example", None, method).is_err());
        assert_eq!(fs::read_to_string(destination.join("SKILL.md")).unwrap(), "external");
        assert!(prepare_deploy(&ssot, &target, "../escape", None, method).is_err());
        fs::remove_dir_all(&destination).unwrap();
        let mut deployment = prepare_deploy(&ssot, &target, "example", None, method).unwrap();
        deployment.publish().unwrap();
        drop(deployment);
        assert!(fs::symlink_metadata(destination).is_err());
    }
}
