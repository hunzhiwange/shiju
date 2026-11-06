#![cfg(target_os = "macos")]
use super::paths::*;
use std::collections::BTreeMap;
use vw_api_types::{agent_apps::AppId, app_providers::paths::ConfigPathState};
#[test]
fn cc04_paths_resolves_macos_override() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir(home.path().join("custom")).unwrap();
    std::fs::write(home.path().join("custom/models.yaml"), "secret: fixture").unwrap();
    let result = read(home.path(), AppId::Omp, Some("~/custom"), &BTreeMap::new()).unwrap();
    assert_eq!(result.directory, home.path().join("custom").to_str().unwrap());
    assert!(
        result
            .files
            .iter()
            .any(|p| p.path.ends_with("models.yaml") && p.state == ConfigPathState::Present)
    );
    assert_eq!(
        std::fs::read_to_string(home.path().join("custom/models.yaml")).unwrap(),
        "secret: fixture"
    );
}
#[test]
fn cc04_paths_rejects_unauthorized_target() {
    let home = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    for value in [outside.path().to_str().unwrap(), "~/../escape", "relative", "~other/root"] {
        assert!(read(home.path(), AppId::Omp, Some(value), &BTreeMap::new()).is_err());
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(outside.path(), home.path().join("escape")).unwrap();
        assert!(read(home.path(), AppId::Omp, Some("~/escape"), &BTreeMap::new()).is_err());
    }
}
#[test]
fn cc04_paths_omp_profiles_and_native_precedence() {
    let home = tempfile::tempdir().unwrap();
    let mut env = BTreeMap::from([
        ("OMP_PROFILE".into(), "work".into()),
        ("PI_PROFILE".into(), "parent".into()),
        ("PI_CONFIG_DIR".into(), ".custom".into()),
        (
            "PI_CODING_AGENT_DIR".into(),
            home.path().join(".custom/profiles/parent/agent").to_str().unwrap().into(),
        ),
    ]);
    let result = read(home.path(), AppId::Omp, None, &env).unwrap();
    assert_eq!(result.directory, home.path().join(".custom/profiles/work/agent").to_str().unwrap());
    env.insert("OMP_PROFILE".into(), "default".into());
    assert_eq!(
        read(home.path(), AppId::Omp, None, &env).unwrap().directory,
        home.path().join(".custom/agent").to_str().unwrap()
    );
    env.insert("OMP_PROFILE".into(), "".into());
    assert_eq!(
        read(home.path(), AppId::Omp, None, &env).unwrap().directory,
        home.path().join(".custom/agent").to_str().unwrap()
    );
    for profile in ["../x", "CON", "nul", "com0", "bad.", "Upper", "a/b"] {
        env.insert("OMP_PROFILE".into(), profile.into());
        assert!(read(home.path(), AppId::Omp, None, &env).is_err());
    }
}

#[test]
fn cc04_paths_defaults_and_file_states() {
    use AppId::*;
    let home = tempfile::tempdir().unwrap();
    for (app, suffix) in [
        (Claude, ".claude/settings.json"),
        (Codex, ".codex/config.toml"),
        (Gemini, ".gemini/.env"),
        (GrokBuild, ".grok/config.toml"),
        (OpenCode, ".config/opencode/opencode.json"),
        (OpenClaw, ".openclaw/openclaw.json"),
        (Hermes, ".hermes/config.yaml"),
        (Pi, ".pi/agent/models.json"),
        (Omp, ".omp/agent/models.yml"),
        (Mcode, ".minimax/config.yaml"),
        (Kimi, ".kimi-code/config.toml"),
        (Dsh, ".dsh/profiles/web/cordis.patch.yml"),
        (Agy, ".gemini/antigravity-cli/settings.json"),
        (ClaudeDesktop, "Library/Application Support/Claude/claude_desktop_config.json"),
    ] {
        let result = read(home.path(), app, None, &BTreeMap::new()).unwrap();
        assert_eq!(result.files[0].path, home.path().join(suffix).to_str().unwrap());
        assert_eq!(result.files[0].state, ConfigPathState::Absent);
    }
    std::fs::create_dir(home.path().join(".claude")).unwrap();
    std::fs::write(home.path().join(".claude/claude.json"), "fixture").unwrap();
    assert!(
        read(home.path(), Claude, None, &BTreeMap::new()).unwrap().files[0]
            .path
            .ends_with("claude.json")
    );
    std::fs::create_dir(home.path().join(".claude/settings.json")).unwrap();
    let result = read(home.path(), Claude, None, &BTreeMap::new()).unwrap();
    assert!(result.files[0].path.ends_with("settings.json"));
    assert_eq!(result.files[0].state, ConfigPathState::Inaccessible);
}
#[test]
fn cc04_paths_omp_xdg_and_yaml_precedence() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join(".omp/profiles/work/agent")).unwrap();
    std::fs::create_dir_all(home.path().join("data/omp/profiles/work")).unwrap();
    let env = BTreeMap::from([
        ("PI_PROFILE".into(), "work".into()),
        ("XDG_DATA_HOME".into(), home.path().join("data").to_str().unwrap().into()),
    ]);
    let result = read(home.path(), AppId::Omp, None, &env).unwrap();
    assert_eq!(result.data_directory, home.path().join("data/omp/profiles/work").to_str().unwrap());
    std::fs::write(home.path().join(".omp/profiles/work/agent/models.yaml"), "fixture").unwrap();
    assert!(
        read(home.path(), AppId::Omp, None, &env).unwrap().files[0].path.ends_with("models.yaml")
    );
    std::fs::write(home.path().join(".omp/profiles/work/agent/models.yml"), "fixture").unwrap();
    assert!(
        read(home.path(), AppId::Omp, None, &env).unwrap().files[0].path.ends_with("models.yml")
    );
    let overridden = read(home.path(), AppId::Omp, Some("~/custom"), &env).unwrap();
    assert_eq!(overridden.directory, overridden.data_directory);
    assert!(!home.path().join("custom").exists());
}

#[test]
fn cc04_paths_environment_precedence_and_inaccessible_files() {
    let home = tempfile::tempdir().unwrap();
    let environment = BTreeMap::from([
        ("HERMES_HOME".into(), format!(" {} ", home.path().join("hermes").display())),
        ("PI_CODING_AGENT_DIR".into(), "~/pi-agent".into()),
        ("MINIMAX_DATA_DIR".into(), " ".into()),
        ("MAVIS_DATA_DIR".into(), home.path().join("mavis").to_str().unwrap().into()),
    ]);
    assert_eq!(
        read(home.path(), AppId::Hermes, None, &environment).unwrap().directory,
        home.path().join("hermes").to_str().unwrap()
    );
    assert_eq!(
        read(home.path(), AppId::Pi, None, &environment).unwrap().directory,
        home.path().join("pi-agent").to_str().unwrap()
    );
    assert_eq!(
        read(home.path(), AppId::Mcode, None, &environment).unwrap().directory,
        home.path().join("mavis").to_str().unwrap()
    );
    assert_eq!(
        read(home.path(), AppId::Hermes, Some("~/override"), &environment).unwrap().directory,
        home.path().join("override").to_str().unwrap()
    );
    let invalid = BTreeMap::from([("HERMES_HOME".into(), "~/.hermes".into())]);
    assert!(read(home.path(), AppId::Hermes, None, &invalid).is_err());
    #[cfg(unix)]
    {
        std::fs::create_dir(home.path().join(".codex")).unwrap();
        let outside = tempfile::NamedTempFile::new().unwrap();
        std::os::unix::fs::symlink(outside.path(), home.path().join(".codex/config.toml")).unwrap();
        let result = read(home.path(), AppId::Codex, None, &BTreeMap::new()).unwrap();
        assert_eq!(result.files[0].state, ConfigPathState::Inaccessible);
    }
}
