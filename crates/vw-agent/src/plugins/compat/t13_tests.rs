//! 安装只展示引导，实际调用仍走普通技能工具与来源隔离。

use crate::config::Config;
use crate::plugins::{
    PluginInstallResult, install_plugin_from_marketplace, install_plugin_from_source,
};
use crate::tools::skill::SkillTool;
use crate::tools::traits::Tool;
use crate::tools::{ToolUseContext, scope_tool_use_context};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, String> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .map(Result::unwrap)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| {
            (
                entry.path().strip_prefix(root).unwrap().to_path_buf(),
                hex::encode(Sha256::digest(fs::read(entry.path()).unwrap())),
            )
        })
        .collect()
}

struct Fixture {
    _temp: tempfile::TempDir,
    source: PathBuf,
    config: Arc<Config>,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        write(&source.join(".codex-plugin/plugin.json"), br#"{"name":"onboarding-pack","extensions":{"com.openai":{"onboardingSkill":"skills/setup/SKILL.md"}}}"#);
        write(
            &source.join("skills/setup/SKILL.md"),
            include_bytes!("../../../testdata/plugin_compat/t13/SKILL.md"),
        );
        write(&source.join("skills/setup/resources/guide.txt"), b"Original package guide.\n");
        write(
            &source.join("skills/setup/scripts/setup.sh"),
            b"#!/bin/sh\nprintf 'must not run' > setup-was-run\n",
        );
        let mut config = Config::default();
        config.workspace_dir = temp.path().join("workspace");
        config.config_path = temp.path().join("config/shiju.json");
        Self { _temp: temp, source, config: Arc::new(config) }
    }

    fn plugins_dir(&self) -> PathBuf {
        self.config.config_path.parent().unwrap().join("plugins")
    }

    fn install(&self) -> PluginInstallResult {
        install_plugin_from_source(self.source.to_str().unwrap(), &self.plugins_dir()).unwrap()
    }

    fn tool(&self) -> SkillTool {
        SkillTool::new_with_runtime_config(
            Arc::new(crate::security::SecurityPolicy::default()),
            "t13".into(),
            false,
            self.config.workspace_dir.clone(),
            self.config.clone(),
        )
    }
}

#[tokio::test]
async fn codex_plugin_compat_t13_invokes_onboarding_only_on_request() {
    let fixture = Fixture::new();
    let before = snapshot(&fixture.source);
    let context = Arc::new(ToolUseContext::new("t13", None).with_full_access_enabled(true));
    let (installed, message) = scope_tool_use_context(context.clone(), async {
        let installed = fixture.install();
        let message = super::super::cli::installation_message(&installed, &fixture.config);
        (installed, message)
    })
    .await;
    assert!(
        message.contains("onboarding-pack:setup"),
        "installation must expose the declared onboarding through the ordinary skill entry"
    );
    assert!(context.take_invoked_skills().is_empty());
    let entry = super::onboarding::entry(&installed.manifest, &installed.identity, &fixture.config)
        .unwrap()
        .unwrap();
    let implicit =
        scope_tool_use_context(context.clone(), fixture.tool().call(json!({"skill":entry})))
            .await
            .unwrap();
    assert_eq!(implicit.data["success"], false, "installing is not explicit skill consent");
    let skills =
        crate::skills::load_skills_full_with_config(&fixture.config.workspace_dir, &fixture.config);
    let request = format!("Please run ${entry} for sandbox-target");
    let selected =
        crate::tools::skill::explicitly_referenced_skill_names(&skills, [request.as_str()]);
    assert!(selected.contains(&entry));
    let explicit = Arc::new(
        ToolUseContext::new("t13", None)
            .with_full_access_enabled(true)
            .with_explicitly_requested_skills(selected),
    );
    let result = scope_tool_use_context(
        explicit.clone(),
        fixture.tool().call(json!({"skill":entry,"args":"sandbox-target"})),
    )
    .await
    .unwrap();
    assert_eq!(result.data["success"], true);
    assert!(result.extra_messages[0].content.as_str().unwrap().contains("sandbox-target"));
    let directory =
        PathBuf::from(result.data["base_dir"].as_str().unwrap().strip_prefix("file://").unwrap());
    assert_eq!(directory.canonicalize().unwrap(), installed.manifest.root.join("skills/setup"));
    assert_eq!(
        fs::read(directory.join("resources/guide.txt")).unwrap(),
        b"Original package guide.\n"
    );
    assert_eq!(explicit.take_invoked_skills()[0].args.as_deref(), Some("sandbox-target"));
    assert!(!fixture.config.workspace_dir.join("setup-was-run").exists());
    assert_eq!(snapshot(&fixture.source), before);
    assert_eq!(snapshot(&installed.installed_dir), before);
}

#[test]
fn codex_plugin_compat_t13_rejects_invalid_input() {
    let fixture = Fixture::new();
    write(&fixture.source.join("notes.txt"), b"Not a skill.\n");
    write(&fixture.source.join(".codex-plugin/plugin.json"), br#"{"name":"onboarding-pack","extensions":{"com.openai":{"onboardingSkill":"notes.txt"}}}"#);
    let installed = fixture.install();
    assert!(
        super::onboarding::entry(&installed.manifest, &installed.identity, &fixture.config)
            .is_err(),
        "a contained ordinary file must not be presented as an onboarding skill"
    );
}

#[tokio::test]
async fn codex_plugin_compat_t13_preserves_isolation() {
    let fixture = Fixture::new();
    let first = install_plugin_from_marketplace(
        fixture.source.to_str().unwrap(),
        &fixture.plugins_dir(),
        "first",
    )
    .unwrap();
    let second = install_plugin_from_marketplace(
        fixture.source.to_str().unwrap(),
        &fixture.plugins_dir(),
        "second",
    )
    .unwrap();
    let entry =
        super::onboarding::entry(&second.manifest, &second.identity, &fixture.config).unwrap();
    assert_eq!(
        entry.as_deref(),
        Some("onboarding-pack@second:setup"),
        "onboarding entry must retain the exact marketplace identity"
    );
    let first_entry = super::onboarding::entry(&first.manifest, &first.identity, &fixture.config)
        .unwrap()
        .unwrap();
    let context = Arc::new(
        ToolUseContext::new("t13", None)
            .with_full_access_enabled(true)
            .with_explicitly_requested_skills(std::collections::BTreeSet::from([first_entry])),
    );
    let result = scope_tool_use_context(
        context.clone(),
        fixture.tool().call(json!({"skill":entry.unwrap()})),
    )
    .await
    .unwrap();
    assert_eq!(result.data["success"], false);
    assert!(context.take_invoked_skills().is_empty());
    crate::plugins::set_plugin_enabled(&fixture.plugins_dir(), "onboarding-pack@second", false)
        .unwrap();
    assert!(super::onboarding::entry(&second.manifest, &second.identity, &fixture.config).is_err());
    let context = Arc::new(
        ToolUseContext::new("t13", None)
            .with_full_access_enabled(true)
            .with_explicitly_requested_skills(["onboarding-pack@second:setup"]),
    );
    let result = scope_tool_use_context(
        context.clone(),
        fixture.tool().call(json!({"skill":"onboarding-pack@second:setup"})),
    )
    .await
    .unwrap();
    assert_eq!(
        result.data["success"], false,
        "a previously displayed entry does not bypass current plugin admission"
    );
    assert!(context.take_invoked_skills().is_empty());
}

#[test]
fn codex_plugin_compat_t13_reads_both_manifest_positions_and_legacy_alternatives() {
    for format in ["codex", "claude", "cursor", "portable-inline", "portable-overlay"] {
        for raw in ["skills/setup/SKILL.md", "./skills/setup/SKILL.md"] {
            let fixture = Fixture::new();
            fs::remove_file(fixture.source.join(".codex-plugin/plugin.json")).unwrap();
            let legacy = json!({"name":"onboarding-pack","extensions":{"com.openai":{"onboardingSkill":raw}}});
            if format.starts_with("portable") {
                let mut portable = json!({"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"onboarding-pack"});
                if format == "portable-inline" {
                    portable["extensions"] = json!({"com.openai":{"onboardingSkill":raw}});
                } else {
                    write(
                        &fixture.source.join(".codex-plugin/plugin.json"),
                        &serde_json::to_vec(&legacy).unwrap(),
                    );
                }
                write(&fixture.source.join("plugin.json"), &serde_json::to_vec(&portable).unwrap());
            } else {
                write(
                    &fixture.source.join(format!(".{format}-plugin/plugin.json")),
                    &serde_json::to_vec(&legacy).unwrap(),
                );
            }
            let before = snapshot(&fixture.source);
            let installed = fixture.install();
            assert_eq!(
                super::onboarding::entry(&installed.manifest, &installed.identity, &fixture.config)
                    .unwrap()
                    .as_deref(),
                Some("onboarding-pack:setup"),
                "{format}: {raw}"
            );
            assert_eq!(snapshot(&installed.installed_dir), before);
        }
    }
}

#[test]
fn codex_plugin_compat_t13_does_not_add_roots_or_select_name_collision_losers() {
    for (path, extra) in [
        ("skills/setup", None),
        ("other/setup/SKILL.md", Some("other/setup/SKILL.md")),
        ("skills/z/SKILL.md", Some("skills/z/SKILL.md")),
    ] {
        let fixture = Fixture::new();
        if let Some(extra) = extra {
            write(
                &fixture.source.join(extra),
                include_bytes!("../../../testdata/plugin_compat/t13/SKILL.md"),
            );
        }
        write(&fixture.source.join(".codex-plugin/plugin.json"), &serde_json::to_vec(&json!({"name":"onboarding-pack","extensions":{"com.openai":{"onboardingSkill":path}}})).unwrap());
        let installed = fixture.install();
        assert!(
            super::onboarding::entry(&installed.manifest, &installed.identity, &fixture.config)
                .is_err(),
            "{path} is not a selected package skill entry"
        );
        let message = super::super::cli::installation_message(&installed, &fixture.config);
        assert!(
            message.contains("Plugin installed:") && message.contains("Onboarding unavailable:")
        );
        assert!(!message.contains("Onboarding available on request:"));
    }
}

#[tokio::test]
async fn codex_plugin_compat_t13_preserves_user_invocation_and_security_restrictions() {
    let mut fixture = Fixture::new();
    let installed = fixture.install();
    let entry = super::onboarding::entry(&installed.manifest, &installed.identity, &fixture.config)
        .unwrap()
        .unwrap();
    Arc::make_mut(&mut fixture.config).skills.security.reject_script_files = true;
    assert!(
        super::onboarding::entry(&installed.manifest, &installed.identity, &fixture.config)
            .is_err()
    );
    Arc::make_mut(&mut fixture.config).skills.security.reject_script_files = false;
    let content =
        String::from_utf8(include_bytes!("../../../testdata/plugin_compat/t13/SKILL.md").to_vec())
            .unwrap()
            .replace("argument-hint:", "user-invocable: false\nargument-hint:");
    write(&installed.installed_dir.join("skills/setup/SKILL.md"), content.as_bytes());
    assert!(
        super::onboarding::entry(&installed.manifest, &installed.identity, &fixture.config)
            .is_err()
    );
    let context = Arc::new(
        ToolUseContext::new("t13", None)
            .with_full_access_enabled(true)
            .with_explicitly_requested_skills([entry.clone()]),
    );
    let result =
        scope_tool_use_context(context.clone(), fixture.tool().call(json!({"skill":entry})))
            .await
            .unwrap();
    assert_eq!(result.data["success"], false);
    assert!(context.take_invoked_skills().is_empty());
}

#[test]
fn codex_plugin_compat_t13_keeps_absent_or_replaced_overlay_inert() {
    for declaration in
        [json!({}), json!({"onboardingSkill":false}), json!({"onboardingSkill":null})]
    {
        let fixture = Fixture::new();
        write(&fixture.source.join("plugin.json"), &serde_json::to_vec(&json!({"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"onboarding-pack","extensions":{"com.openai":declaration}})).unwrap());
        let installed = fixture.install();
        assert!(
            installed.manifest.onboarding_skill.is_none(),
            "inline extension replaces the whole legacy overlay"
        );
        assert!(
            super::onboarding::entry(&installed.manifest, &installed.identity, &fixture.config)
                .unwrap()
                .is_none()
        );
        assert!(
            !super::super::cli::installation_message(&installed, &fixture.config)
                .contains("Onboarding")
        );
    }
}

#[test]
fn codex_plugin_compat_t13_does_not_borrow_a_shadowing_installation() {
    let fixture = Fixture::new();
    let installed = fixture.install();
    let workspace_root = fixture.config.workspace_dir.join("plugins/onboarding-pack");
    write(
        &workspace_root.join(".codex-plugin/plugin.json"),
        &fs::read(fixture.source.join(".codex-plugin/plugin.json")).unwrap(),
    );
    write(
        &workspace_root.join("skills/setup/SKILL.md"),
        include_bytes!("../../../testdata/plugin_compat/t13/SKILL.md"),
    );
    assert!(
        super::onboarding::entry(&installed.manifest, &installed.identity, &fixture.config)
            .is_err(),
        "same identity from another selected root must not satisfy the installed package's onboarding declaration"
    );
    assert!(
        !super::super::cli::installation_message(&installed, &fixture.config)
            .contains("Onboarding available on request:")
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
    let error = super::onboarding::entry(&installed.manifest, &installed.identity, &fixture.config)
        .unwrap_err();
    assert!(!error.to_string().contains("private-fixture-payload"));
    assert_eq!(fs::read(outside).unwrap(), b"private-fixture-payload\n");
}

#[test]
fn codex_plugin_compat_t13_uses_final_skill_admission_after_local_shadowing() {
    for local_name in ["setup", "onboarding-pack:setup"] {
        let fixture = Fixture::new();
        let installed = fixture.install();
        write(
            &fixture.config.workspace_dir.join("skills/local/SKILL.toml"),
            format!(
                "[skill]\nname = '{local_name}'\ndescription = 'Existing local setup skill.'\n"
            )
            .as_bytes(),
        );
        let admitted = crate::skills::load_skills_full_with_config(
            &fixture.config.workspace_dir,
            &fixture.config,
        );
        assert!(
            admitted.iter().any(|skill| skill.name == local_name && skill.plugin_source.is_none())
        );
        if local_name == "setup" {
            assert!(admitted.iter().all(|skill| {
                skill
                    .plugin_source
                    .as_ref()
                    .is_none_or(|source| source.plugin_id != "onboarding-pack")
            }));
        } else {
            assert!(admitted.iter().any(|skill| skill.plugin_source.is_some()));
        }
        assert!(
            super::onboarding::entry(&installed.manifest, &installed.identity, &fixture.config)
                .is_err(),
            "onboarding cannot advertise a plugin skill hidden or ambiguous in ordinary runtime admission"
        );
    }
}
