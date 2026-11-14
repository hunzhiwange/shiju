#![cfg(target_os = "macos")]
use super::app_paths::*;
use std::{collections::BTreeMap, fs};
use vw_api_types::{agent_apps::AppId, app_providers::paths::ConfigPathState};

#[test]
fn cp12_app_paths_success() {
    use AppId::*;
    let home = tempfile::tempdir().unwrap();
    let cases = [
        (Claude, ".claude/skills", SkillDistribution::ManagedBinding),
        (ClaudeDesktop, ".claude-desktop/skills", SkillDistribution::NoDirectorySync),
        (Codex, ".codex/skills", SkillDistribution::ManagedBinding),
        (Gemini, ".gemini/skills", SkillDistribution::ManagedBinding),
        (GrokBuild, ".grok/skills", SkillDistribution::ManagedBinding),
        (OpenCode, ".config/opencode/skills", SkillDistribution::ManagedBinding),
        (OpenClaw, ".openclaw/skills", SkillDistribution::PathOnly),
        (Hermes, ".hermes/skills", SkillDistribution::ManagedBinding),
        (Pi, ".pi/agent/skills", SkillDistribution::NativeDirectory),
        (Mcode, ".minimax/skills", SkillDistribution::ManagedBinding),
        (Kimi, ".kimi-code/skills", SkillDistribution::ManagedBinding),
        (Dsh, ".dsh/skills", SkillDistribution::ManagedBinding),
        (Agy, ".gemini/config/skills", SkillDistribution::ManagedBinding),
        (Omp, ".omp/agent/skills", SkillDistribution::NativeDirectory),
    ];
    assert_eq!(cases.len(), crate::apps::catalog::catalog().len());
    for (app, suffix, capability) in cases {
        let target = resolve(home.path(), app, None, &BTreeMap::new(), &[]).unwrap();
        assert_eq!(target.directory, home.path().join(suffix));
        assert_eq!(target.state, ConfigPathState::Absent);
        assert_eq!(target.distribution, capability);
        assert_eq!(target.require_distribution().is_ok(), !matches!(app, ClaudeDesktop | OpenClaw));
    }
    assert_eq!(fs::read_dir(home.path()).unwrap().count(), 0);
}

#[test]
fn cp12_app_paths_overrides_and_execution_environment() {
    use AppId::*;
    let home = tempfile::tempdir().unwrap();
    for app in [Claude, Codex, Gemini, GrokBuild, OpenCode, OpenClaw, Hermes, Pi, Omp] {
        let target = resolve(home.path(), app, Some("~/custom"), &BTreeMap::new(), &[]).unwrap();
        assert_eq!(target.directory, home.path().join("custom/skills"));
    }
    for (app, variable) in [
        (Hermes, "HERMES_HOME"), (Pi, "PI_CODING_AGENT_DIR"),
        (Mcode, "MINIMAX_DATA_DIR"), (Mcode, "MAVIS_DATA_DIR"),
        (Kimi, "KIMI_CODE_HOME"), (Dsh, "DSH_HOME"),
    ] {
        let environment = BTreeMap::from([
            (variable.into(), home.path().join("native").to_str().unwrap().into()),
        ]);
        assert_eq!(resolve(home.path(), app, None, &environment, &[]).unwrap().directory,
            home.path().join("native/skills"));
    }
    let environment = BTreeMap::from([
        ("OMP_PROFILE".into(), "work".into()),
        ("PI_PROFILE".into(), "parent".into()),
        ("PI_CONFIG_DIR".into(), ".omp-custom".into()),
        ("PI_CODING_AGENT_DIR".into(),
            home.path().join(".omp-custom/profiles/parent/agent").to_str().unwrap().into()),
    ]);
    assert_eq!(resolve(home.path(), Omp, None, &environment, &[]).unwrap().directory,
        home.path().join(".omp-custom/profiles/work/agent/skills"));
    assert_eq!(resolve(home.path(), Omp, Some("~/custom"), &environment, &[]).unwrap().directory,
        home.path().join("custom/skills"));
}

#[test]
fn cp12_app_paths_rejects_without_side_effects() {
    let home = tempfile::tempdir().unwrap();
    let environment = BTreeMap::new();
    fs::write(home.path().join("sentinel"), "unchanged").unwrap();
    for raw in ["~/../escape", "relative", "~other/root"] {
        assert!(resolve(home.path(), AppId::Claude, Some(raw), &environment, &[]).is_err());
    }
    for app in [AppId::ClaudeDesktop, AppId::Mcode, AppId::Kimi, AppId::Dsh, AppId::Agy] {
        assert!(resolve(home.path(), app, Some("~/custom"), &environment, &[]).is_err());
    }
    for excluded in [".claude/skills", ".claude", ".claude/skills/backup"] {
        assert_eq!(resolve(home.path(), AppId::Claude, None, &environment,
            &[&home.path().join(excluded)]), Err(AppPathError::OverlappingRoots));
    }
    let bad_profile = BTreeMap::from([("OMP_PROFILE".into(), "../escape".into())]);
    assert!(resolve(home.path(), AppId::Omp, None, &bad_profile, &[]).is_err());
    assert_eq!(fs::read_to_string(home.path().join("sentinel")).unwrap(), "unchanged");
    assert_eq!(fs::read_dir(home.path()).unwrap().count(), 1);
}

#[test]
fn cp12_app_paths_rejects_root_links_and_non_directories() {
    use std::os::unix::fs::symlink;
    let home = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::create_dir(home.path().join("real")).unwrap();
    for destination in [home.path().join("real"), outside.path().to_path_buf(), home.path().join("missing")] {
        symlink(destination, home.path().join(".claude")).unwrap();
        assert!(resolve(home.path(), AppId::Claude, None, &BTreeMap::new(), &[]).is_err());
        fs::remove_file(home.path().join(".claude")).unwrap();
    }
    fs::create_dir(home.path().join(".claude")).unwrap();
    fs::write(home.path().join(".claude/skills"), "not a directory").unwrap();
    assert!(resolve(home.path(), AppId::Claude, None, &BTreeMap::new(), &[]).is_err());
    assert_eq!(fs::read_to_string(home.path().join(".claude/skills")).unwrap(), "not a directory");
}

#[test]
fn cp12_app_paths_native_state_tracks_real_directories() {
    use std::os::unix::fs::symlink;
    let home = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    for app in [AppId::Pi, AppId::Omp] {
        let target = resolve(home.path(), app, None, &BTreeMap::new(), &[]).unwrap();
        assert!(!native_enabled(home.path(), &target, "fixture").unwrap());
        fs::create_dir_all(target.directory.join("fixture")).unwrap();
        assert!(native_enabled(home.path(), &target, "fixture").unwrap());
        fs::remove_dir(target.directory.join("fixture")).unwrap();
        let managed = home.path().join("ssot/fixture");
        fs::create_dir_all(&managed).unwrap();
        symlink(&managed, target.directory.join("fixture")).unwrap();
        assert!(native_enabled(home.path(), &target, "fixture").unwrap());
        fs::remove_dir(&managed).unwrap();
        assert_eq!(native_enabled(home.path(), &target, "fixture"), Err(AppPathError::UnsafeDirectory));
        fs::remove_file(target.directory.join("fixture")).unwrap();
        symlink(outside.path(), target.directory.join("fixture")).unwrap();
        assert_eq!(native_enabled(home.path(), &target, "fixture"), Err(AppPathError::UnsafeDirectory));
        for name in ["", ".", "..", "../escape", "a/b", "a\\b"] {
            assert_eq!(native_enabled(home.path(), &target, name), Err(AppPathError::InvalidSkillDirectory));
        }
    }
    let claude = resolve(home.path(), AppId::Claude, None, &BTreeMap::new(), &[]).unwrap();
    assert_eq!(native_enabled(home.path(), &claude, "fixture"), Err(AppPathError::BindingStateRequired));
    let desktop = resolve(home.path(), AppId::ClaudeDesktop, None, &BTreeMap::new(), &[]).unwrap();
    assert_eq!(native_enabled(home.path(), &desktop, "fixture"), Err(AppPathError::NoDirectorySync));
}
