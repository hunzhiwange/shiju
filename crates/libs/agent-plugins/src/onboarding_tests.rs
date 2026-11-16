//! Onboarding admission and path reauditing through real plugin discovery.
use agent_config::runtime::Config;
use std::fs;
use std::path::{Path, PathBuf};

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

struct Fixture {
    _temp: tempfile::TempDir,
    source: PathBuf,
    config: Config,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        write(&source.join(".codex-plugin/plugin.json"), br#"{"name":"onboarding-pack","extensions":{"com.openai":{"onboardingSkill":"skills/setup/SKILL.md"}}}"#);
        write(&source.join("skills/setup/SKILL.md"), include_bytes!("../../../vw-agent/testdata/plugin_compat/t13/SKILL.md"));
        write(&source.join("skills/setup/resources/guide.txt"), b"Original package guide.\n");
        write(&source.join("skills/setup/scripts/setup.sh"), b"#!/bin/sh\nprintf 'must not run' > setup-was-run\n");
        let mut config = Config::default();
        config.workspace_dir = temp.path().join("workspace");
        config.config_path = temp.path().join("config/shiju.json");
        Self { _temp: temp, source, config }
    }

    fn install(&self) -> crate::PluginInstallResult {
        crate::install_plugin_from_source(
            self.source.to_str().unwrap(),
            &self.config.config_path.parent().unwrap().join("plugins"),
        ).unwrap()
    }

    fn entry(&self, installed: &crate::PluginInstallResult) -> anyhow::Result<Option<String>> {
        let skills = agent_skills::catalog::load_skills_with_config(
            &self.config.workspace_dir,
            &self.config.skills,
            true,
            |mode| crate::skills::load_plugin_skills(
                &self.config.workspace_dir, &self.config, mode, &crate::discover_plugins,
            ),
        );
        super::entry(&installed.manifest, &installed.identity, &skills)
    }
}

#[test]
fn codex_plugin_compat_t13_rejects_invalid_input() {
    let fixture = Fixture::new();
    write(&fixture.source.join("notes.txt"), b"Not a skill.\n");
    write(&fixture.source.join(".codex-plugin/plugin.json"), br#"{"name":"onboarding-pack","extensions":{"com.openai":{"onboardingSkill":"notes.txt"}}}"#);
    let installed = fixture.install();
    assert!(
        fixture.entry(&installed).is_err(),
        "a contained ordinary file must not be presented as an onboarding skill"
    );
}

#[cfg(unix)]
#[test]
fn codex_plugin_compat_t13_reaudits_mutated_onboarding_paths_without_reading_outside() {
    let fixture = Fixture::new();
    let installed = fixture.install();
    let outside = fixture._temp.path().join("outside.md");
    write(&outside, b"private-fixture-payload\n");
    let entry = installed.installed_dir.join("skills/setup/SKILL.md");
    fs::remove_file(&entry).unwrap();
    std::os::unix::fs::symlink(&outside, &entry).unwrap();
    let error = fixture.entry(&installed).unwrap_err();
    assert!(!error.to_string().contains("private-fixture-payload"));
    assert_eq!(fs::read(outside).unwrap(), b"private-fixture-payload\n");
}
