use super::{app_paths::{AppSkillsPath, distribution}, copy_deploy::CopyDeploy,
    disable_deploy::{DisableDeploy, DisableError}, link_deploy::{prepare_deploy, SyncMethod}};
use std::fs;
use vw_api_types::{agent_apps::AppId, app_providers::paths::ConfigPathState};

fn fixture(app_id: AppId) -> (tempfile::TempDir, std::path::PathBuf, AppSkillsPath) {
    let temp = tempfile::tempdir().unwrap();
    let ssot = temp.path().join("ssot");
    let app = temp.path().join("app");
    fs::create_dir_all(ssot.join("example")).unwrap();
    fs::create_dir(&app).unwrap();
    fs::write(ssot.join("example/SKILL.md"), "# Example").unwrap();
    let target = AppSkillsPath { app_id, directory: app, state: ConfigPathState::Present,
        distribution: distribution(app_id) };
    (temp, ssot, target)
}

#[test]
fn cp12_disable_deploy_success() {
    for method in [SyncMethod::Copy, SyncMethod::Symlink] {
        let (_temp, ssot, target) = fixture(AppId::Pi);
        let mut deploy = prepare_deploy(&ssot, &target, "example", None, method).unwrap();
        deploy.publish().unwrap();
        let binding = deploy.finish();
        let mut disable = DisableDeploy::prepare(&ssot, &target, "example", Some(&binding)).unwrap();
        disable.publish().unwrap();
        assert!(fs::symlink_metadata(&binding.destination).is_err());
        drop(disable); // Failed settings commit restores the copy or link.
        assert!(fs::symlink_metadata(&binding.destination).is_ok());
        let mut disable = DisableDeploy::prepare(&ssot, &target, "example", Some(&binding)).unwrap();
        disable.publish().unwrap();
        disable.finish().unwrap();
        assert!(fs::symlink_metadata(&binding.destination).is_err());
        assert_eq!(fs::read_to_string(ssot.join("example/SKILL.md")).unwrap(), "# Example");
    }
}

#[test]
fn cp12_disable_deploy_rejects_without_side_effects() {
    for app in [AppId::Pi, AppId::Mcode, AppId::Agy, AppId::Kimi, AppId::Dsh, AppId::Omp, AppId::Codex] {
        let (_temp, ssot, mut target) = fixture(app);
        let mut copy = CopyDeploy::prepare(&ssot, &target, "example", None).unwrap();
        copy.publish().unwrap();
        let binding = copy.finish();
        fs::write(binding.destination.join("SKILL.md"), "external edit").unwrap();
        assert!(matches!(DisableDeploy::prepare(&ssot, &target, "example", Some(&binding)), Err(DisableError::Preserved)));
        // An alternate app ID pointing at the same physical root grants no ownership.
        target.app_id = AppId::Claude;
        target.distribution = distribution(target.app_id);
        assert!(matches!(DisableDeploy::prepare(&ssot, &target, "example", None), Err(DisableError::Preserved)));
        assert_eq!(fs::read_to_string(binding.destination.join("SKILL.md")).unwrap(), "external edit");
        fs::remove_dir_all(&binding.destination).unwrap();
        let external = ssot.parent().unwrap().join("missing-external");
        std::os::unix::fs::symlink(&external, &binding.destination).unwrap();
        assert!(DisableDeploy::prepare(&ssot, &target, "example", Some(&binding)).is_err());
        assert_eq!(fs::read_link(&binding.destination).unwrap(), external);
        assert!(DisableDeploy::prepare(&ssot, &target, "../escape", Some(&binding)).is_err());
    }
}

#[test]
fn cp12_disable_deploy_preserves_replacement_after_prepare() {
    let (_temp, ssot, target) = fixture(AppId::Codex);
    let mut copy = CopyDeploy::prepare(&ssot, &target, "example", None).unwrap();
    copy.publish().unwrap();
    let binding = copy.finish();
    let mut disable = DisableDeploy::prepare(&ssot, &target, "example", Some(&binding)).unwrap();
    fs::write(binding.destination.join("SKILL.md"), "external edit").unwrap();
    assert!(disable.publish().is_err());
    drop(disable);
    assert_eq!(fs::read_to_string(binding.destination.join("SKILL.md")).unwrap(), "external edit");
}
