//! Host configuration integration for skill installation.
use super::install_skill_from_source;

fn write_skill(parent: &std::path::Path, name: &str, markdown: &str) -> std::path::PathBuf {
    let dir = parent.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("SKILL.md"), markdown).unwrap();
    dir
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn configured_skill_security_controls_loading_and_install_audit() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    let load_sources = workspace.join(".shiju/skills");
    let install_sources = dir.path().join("install-sources");
    let installs = dir.path().join("installs");
    std::fs::create_dir_all(&load_sources).unwrap();
    std::fs::create_dir_all(&install_sources).unwrap();
    std::fs::create_dir_all(&installs).unwrap();

    let load_source = write_skill(&load_sources, "load-risk", "# Load risk\nSafe manifest\n");
    std::fs::write(load_source.join("run.sh"), "#!/bin/sh\ntrue\n").unwrap();
    let mut config = crate::app::agent::config::Config::default();
    config.workspace_dir = workspace.clone();
    config.config_path = dir.path().join("config/vibewindow.json");

    let loaded = crate::app::agent::skills::load_skills_with_config(&workspace, &config);
    assert!(loaded.iter().any(|skill| skill.name == "load-risk"));

    let default_security = config.skills.security.clone();
    let reject_scripts = config_skills::SkillSecurityConfig {
        reject_script_files: true,
        ..Default::default()
    };
    config.skills.security = reject_scripts.clone();
    assert!(
        crate::app::agent::skills::load_skills_with_config(&workspace, &config)
            .iter()
            .all(|skill| skill.name != "load-risk")
    );

    let install_default =
        write_skill(&install_sources, "install-default", "# Install default\nSafe manifest\n");
    std::fs::write(install_default.join("run.sh"), "#!/bin/sh\ntrue\n").unwrap();
    install_skill_from_source(install_default.to_str().unwrap(), &installs, &default_security)
        .unwrap();
    assert!(installs.join("install-default/run.sh").is_file());

    let install_rejected =
        write_skill(&install_sources, "install-rejected", "# Install rejected\nSafe manifest\n");
    std::fs::write(install_rejected.join("run.sh"), "#!/bin/sh\ntrue\n").unwrap();
    let error = match install_skill_from_source(
        install_rejected.to_str().unwrap(),
        &installs,
        &reject_scripts,
    ) {
        Ok(_) => panic!("script-rejecting policy must fail installation"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("Skill security audit failed"));
    assert!(!installs.join("install-rejected").exists());
}
