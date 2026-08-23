//! 验证插件技能加载、安全策略、覆盖优先级与禁用行为。

use crate::app::agent::config::{Config, SkillsPromptInjectionMode};
use crate::app::agent::skills::{
    SkillLoadMode, load_skills_full_with_config, load_skills_with_config,
};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

fn env_lock() -> &'static Mutex<()> {
    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    ENV_LOCK.get_or_init(|| Mutex::new(()))
}

fn lock_env() -> MutexGuard<'static, ()> {
    env_lock().lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct EnvVarGuard {
    key: &'static str,
    original: Option<String>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let original = std::env::var(key).ok();
        unsafe { std::env::set_var(key, value) };
        Self { key, original }
    }

    fn unset(key: &'static str) -> Self {
        let original = std::env::var(key).ok();
        unsafe { std::env::remove_var(key) };
        Self { key, original }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.original {
            unsafe { std::env::set_var(self.key, value) };
        } else {
            unsafe { std::env::remove_var(self.key) };
        }
    }
}

fn runtime_config(workspace_dir: &Path, config_dir: &Path) -> Config {
    let mut config = Config::default();
    config.workspace_dir = workspace_dir.to_path_buf();
    config.config_path = config_dir.join("vibewindow.json");
    config.skills.open_skills_enabled = false;
    config
}

fn write_plugin_manifest(plugin_root: &Path, name: &str, skills_path: &str) {
    let manifest_dir = plugin_root.join(".vibewindow-plugin");
    fs::create_dir_all(&manifest_dir).expect("manifest dir");
    fs::write(
        manifest_dir.join("plugin.json"),
        serde_json::to_vec_pretty(&json!({
            "name": name,
            "version": "0.1.0",
            "description": format!("{name} test plugin"),
            "skills": skills_path,
        }))
        .expect("manifest json"),
    )
    .expect("write manifest");
}

fn write_skill(skills_dir: &Path, name: &str, description: &str) -> PathBuf {
    let skill_dir = skills_dir.join(name);
    fs::create_dir_all(&skill_dir).expect("skill dir");
    fs::write(skill_dir.join("SKILL.md"), format!("# {name}\n{description}\n"))
        .expect("write skill");
    skill_dir
}

fn skill_names(skills: &[crate::app::agent::skills::Skill]) -> Vec<&str> {
    skills.iter().map(|skill| skill.name.as_str()).collect()
}

#[test]
fn plugin_skills_are_loaded_from_manifest_component_path() {
    let _lock = lock_env();
    let dir = tempfile::tempdir().expect("temp dir");
    let _home = EnvVarGuard::set("HOME", dir.path().join("home").to_str().unwrap());
    let _open_enabled = EnvVarGuard::unset("VIBEWINDOW_OPEN_SKILLS_ENABLED");
    let _open_dir = EnvVarGuard::unset("VIBEWINDOW_OPEN_SKILLS_DIR");

    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/plugin-pack");
    write_plugin_manifest(&plugin_root, "plugin-pack", "./components/skills");
    write_skill(&plugin_root.join("components/skills"), "plugin-skill", "Plugin skill loaded");

    let mut config = runtime_config(&workspace_dir, &config_dir);
    config.skills.prompt_injection_mode = SkillsPromptInjectionMode::Compact;

    let compact = load_skills_with_config(&workspace_dir, &config);
    let full = load_skills_full_with_config(&workspace_dir, &config);

    println!("plugin skill compact order: {:?}", skill_names(&compact));
    println!("plugin skill full order: {:?}", skill_names(&full));
    assert_eq!(skill_names(&compact), vec!["plugin-skill"]);
    assert!(compact[0].prompts.is_empty());
    assert_eq!(skill_names(&full), vec!["plugin-skill"]);
    assert_eq!(full[0].prompts, vec!["# plugin-skill\nPlugin skill loaded\n"]);
}

#[test]
fn plugin_skills_follow_configured_security_policy() {
    let _lock = lock_env();
    let dir = tempfile::tempdir().expect("temp dir");
    let _home = EnvVarGuard::set("HOME", dir.path().join("home").to_str().unwrap());
    let _open_enabled = EnvVarGuard::unset("VIBEWINDOW_OPEN_SKILLS_ENABLED");
    let _open_dir = EnvVarGuard::unset("VIBEWINDOW_OPEN_SKILLS_DIR");

    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/plugin-pack");
    write_plugin_manifest(&plugin_root, "plugin-pack", "./skills");
    let skill = write_skill(&plugin_root.join("skills"), "scripted-plugin", "Plugin script");
    fs::write(skill.join("run.sh"), "#!/bin/sh\ntrue\n").expect("write plugin script");

    let mut config = runtime_config(&workspace_dir, &config_dir);
    let default_loaded = load_skills_full_with_config(&workspace_dir, &config);
    assert!(default_loaded.iter().any(|skill| skill.name == "scripted-plugin"));

    config.skills.security.reject_script_files = true;
    let rejected = load_skills_full_with_config(&workspace_dir, &config);
    assert!(rejected.iter().all(|skill| skill.name != "scripted-plugin"));
}

#[test]
fn plugin_skills_workspace_skill_overrides_plugin_skill() {
    let _lock = lock_env();
    let dir = tempfile::tempdir().expect("temp dir");
    let _home = EnvVarGuard::set("HOME", dir.path().join("home").to_str().unwrap());
    let _open_enabled = EnvVarGuard::unset("VIBEWINDOW_OPEN_SKILLS_ENABLED");
    let _open_dir = EnvVarGuard::unset("VIBEWINDOW_OPEN_SKILLS_DIR");

    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let local_skills = workspace_dir.join(".vibewindow/skills");
    write_skill(&local_skills, "shared-skill", "Workspace skill wins");

    let plugin_root = workspace_dir.join(".vibewindow/plugins/plugin-pack");
    write_plugin_manifest(&plugin_root, "plugin-pack", "./skills");
    write_skill(&plugin_root.join("skills"), "shared-skill", "Plugin skill loses");
    write_skill(&plugin_root.join("skills"), "plugin-only", "Plugin skill remains");

    let config = runtime_config(&workspace_dir, &config_dir);
    let skills = load_skills_full_with_config(&workspace_dir, &config);

    println!("plugin conflict skill order: {:?}", skill_names(&skills));
    assert_eq!(skill_names(&skills), vec!["shared-skill", "plugin-only"]);
    assert_eq!(skills[0].description, "Workspace skill wins");
}

#[test]
fn plugin_skills_disabled_plugin_skills_are_skipped() {
    let _lock = lock_env();
    let dir = tempfile::tempdir().expect("temp dir");
    let _home = EnvVarGuard::set("HOME", dir.path().join("home").to_str().unwrap());
    let _open_enabled = EnvVarGuard::unset("VIBEWINDOW_OPEN_SKILLS_ENABLED");
    let _open_dir = EnvVarGuard::unset("VIBEWINDOW_OPEN_SKILLS_DIR");

    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let plugin_source = workspace_dir.join(".vibewindow/plugins");

    let active_root = plugin_source.join("active-plugin");
    write_plugin_manifest(&active_root, "active-plugin", "./skills");
    write_skill(&active_root.join("skills"), "active-skill", "Active plugin skill");

    let muted_root = plugin_source.join("muted-plugin");
    write_plugin_manifest(&muted_root, "muted-plugin", "./skills");
    write_skill(&muted_root.join("skills"), "muted-skill", "Muted plugin skill");
    fs::write(plugin_source.join("muted-plugin.disabled"), "").expect("disabled marker");

    let config = runtime_config(&workspace_dir, &config_dir);
    let skills = load_skills_full_with_config(&workspace_dir, &config);

    println!("plugin enabled skill order: {:?}", skill_names(&skills));
    assert_eq!(skill_names(&skills), vec!["active-skill"]);
}

#[test]
fn plugin_skills_invalid_plugin_source_returns_empty() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    fs::create_dir_all(workspace_dir.join(".vibewindow")).expect("workspace config dir");
    fs::write(workspace_dir.join(".vibewindow/plugins"), "not a directory")
        .expect("invalid plugin source");

    let mut config = runtime_config(&workspace_dir, &config_dir);
    config.config_path.clear();
    let skills = super::load_plugin_skills(&workspace_dir, &config, SkillLoadMode::Full);

    assert!(skills.is_empty());
}
