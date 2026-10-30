//! Agent declarations are exercised through the real plugin skill loader.

use crate::config::Config;
use crate::skills::SkillLoadMode;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    config: Config,
}

fn write(path: &Path, content: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
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

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        config.workspace_dir = temp.path().join("workspace");
        config.config_path = temp.path().join("config/shiju.json");
        let root = config.workspace_dir.join("plugins/agent-pack");
        write(&root.join(".codex-plugin/plugin.json"), br#"{"name":"agent-pack"}"#);
        write(
            &root.join("skills/review/SKILL.md"),
            include_bytes!("../../../testdata/plugin_compat/t12/SKILL.md"),
        );
        write(
            &root.join("skills/review/agents/openai.yaml"),
            include_bytes!("../../../testdata/plugin_compat/t12/openai.yaml"),
        );
        write(&root.join("skills/review/assets/icon.svg"), b"<svg/>\n");
        Self { _temp: temp, root, config }
    }

    fn load(&self) -> super::super::skills::PluginSkillInventory {
        super::super::skills::load_plugin_skill_inventory(
            &self.config.workspace_dir,
            &self.config,
            SkillLoadMode::Full,
        )
    }
}

#[test]
fn codex_plugin_compat_t12_matches_baseline_agent_declaration() {
    let fixture = Fixture::new();
    let before = snapshot(&fixture.root);
    let inventory = fixture.load();
    assert_eq!(inventory.skills.len(), 1);
    let skill = &inventory.skills[0];
    let metadata = skill.agent_metadata.as_ref().unwrap();
    assert_eq!(
        metadata.interface.as_ref().and_then(|value| value.display_name.as_deref()),
        Some("Review assistant"),
        "the real skill loader must retain the supported skill-local agent metadata"
    );
    assert_eq!(metadata.dependencies.len(), 1);
    assert_eq!(metadata.dependencies[0].value, "public-review");
    assert_eq!(metadata.dependencies[0].oauth_callback_port, Some(4321));
    assert!(skill.disable_model_invocation, "metadata policy must restrict implicit calls");
    assert!(skill.tools.is_empty() && skill.allowed_tools.is_empty());
    assert_eq!(snapshot(&fixture.root), before);
}

#[test]
fn codex_plugin_compat_t12_rejects_invalid_input() {
    let fixture = Fixture::new();
    write(
        &fixture.root.join("skills/review/agents/openai.yaml"),
        b"interface:\n  display_name: Keep me\n  icon_small: ../../outside.svg\n  brand_color: red\ndependencies:\n  tools:\n    - type: ' '\n      value: rejected\n    - type: mcp\n      value: '  retained   server '\n      command: harmless\n",
    );
    let inventory = fixture.load();
    assert_eq!(inventory.skills.len(), 1, "invalid optional fields must not erase the skill");
    let metadata = inventory.skills[0].agent_metadata.as_ref().unwrap();
    assert_eq!(
        metadata.dependencies.iter().map(|value| value.value.as_str()).collect::<Vec<_>>(),
        ["retained server"],
        "reject only invalid individual dependencies without granting any tools"
    );
    let interface = metadata.interface.as_ref().unwrap();
    assert_eq!(interface.display_name.as_deref(), Some("Keep me"));
    assert!(interface.icon_small.is_none() && interface.brand_color.is_none());
}

#[test]
fn codex_plugin_compat_t12_preserves_isolation() {
    let fixture = Fixture::new();
    write(
        &fixture.root.join("agents/privileged.md"),
        b"---\nname: privileged\ntools: ['shell']\npermissions: full-access\n---\nNever execute.\n",
    );
    write(&fixture.root.join("skills/restricted/SKILL.md"), b"---\nname: restricted\ndescription: Product restricted\n---\nDo not load outside the named product.\n");
    write(
        &fixture.root.join("skills/restricted/agents/openai.yaml"),
        b"policy:\n  products: [CODEX]\n",
    );
    let before = snapshot(&fixture.root);
    let inventory = fixture.load();
    assert_eq!(
        inventory.skills.iter().map(|skill| skill.name.as_str()).collect::<Vec<_>>(),
        ["review"],
        "an unnamed host cannot impersonate Codex to bypass a product restriction"
    );
    assert!(inventory.skills[0].tools.is_empty());
    assert_eq!(snapshot(&fixture.root), before);
}

#[test]
fn codex_plugin_compat_t12_malformed_optional_metadata_keeps_audited_base_skill() {
    let fixture = Fixture::new();
    for yaml in [
        "interface:\n  display_name: [private-fixture-payload]\n",
        "policy:\n  allow_implicit_invocation: 'private-fixture-payload'\n",
        "policy:\n  products: [private-fixture-payload]\n",
        "dependencies:\n  tools:\n    - type: mcp\n      value: known\n      oauth: {callbackPort: 65536}\n",
    ] {
        write(&fixture.root.join("skills/review/agents/openai.yaml"), yaml.as_bytes());
        let before = snapshot(&fixture.root);
        let inventory = fixture.load();
        assert_eq!(inventory.skills.len(), 1);
        assert!(inventory.skills[0].agent_metadata.is_none());
        assert!(inventory.diagnostics.iter().any(|value| value.message.contains("malformed")));
        assert!(
            inventory
                .diagnostics
                .iter()
                .all(|value| !value.message.contains("private-fixture-payload"))
        );
        assert_eq!(snapshot(&fixture.root), before);
    }
}

#[test]
fn codex_plugin_compat_t12_resolves_local_and_shared_assets_without_escaping() {
    let fixture = Fixture::new();
    write(&fixture.root.join("assets/shared.svg"), b"<svg/>\n");
    let metadata = fixture.load().skills.remove(0).agent_metadata.unwrap();
    let local = metadata.interface.unwrap().icon_small.unwrap();
    assert_eq!(local, fixture.root.canonicalize().unwrap().join("skills/review/assets/icon.svg"));
    for (raw, expected) in [
        ("../../assets/shared.svg", Some("assets/shared.svg")),
        ("./assets/not-generated.svg", Some("skills/review/assets/not-generated.svg")),
        ("../../other/icon.svg", None),
        ("../assets/icon.svg", None),
        ("/outside/icon.svg", None),
        ("assets\\icon.svg", None),
        ("assets/C:icon.svg", None),
        ("../../../../assets/icon.svg", None),
    ] {
        write(
            &fixture.root.join("skills/review/agents/openai.yaml"),
            format!("interface:\n  display_name: Retained\n  icon_small: '{}'\n", raw).as_bytes(),
        );
        let skill = fixture.load().skills.remove(0);
        let interface = skill.agent_metadata.unwrap().interface.unwrap();
        assert_eq!(interface.display_name.as_deref(), Some("Retained"));
        assert_eq!(
            interface.icon_small,
            expected.map(|path| fixture.root.canonicalize().unwrap().join(path)),
            "{raw}"
        );
    }
}

#[test]
fn codex_plugin_compat_t12_normalizes_fields_by_character_limits_and_dependency_entries() {
    let fixture = Fixture::new();
    let yaml = format!(
        "interface:\n  display_name: '{}'\n  short_description: '{}'\n  default_prompt: '{}'\ndependencies:\n  tools:\n    - type: '{}'\n      value: '{}'\n      command: '{}'\n      oauth: {{callbackPort: 65535}}\n    - value: missing-kind\n    - type: mcp\n      value: '{}'\npolicy:\n  allow_implicit_invocation: true\n",
        "界".repeat(64),
        "界".repeat(1024),
        "界".repeat(1025),
        "界".repeat(64),
        "界".repeat(1024),
        "x".repeat(1025),
        "界".repeat(1025),
    );
    write(&fixture.root.join("skills/review/agents/openai.yaml"), yaml.as_bytes());
    let skill = fixture.load().skills.remove(0);
    let metadata = skill.agent_metadata.unwrap();
    let interface = metadata.interface.unwrap();
    assert_eq!(interface.display_name, Some("界".repeat(64)));
    assert_eq!(interface.short_description, Some("界".repeat(1024)));
    assert!(interface.default_prompt.is_none());
    assert_eq!(metadata.dependencies.len(), 1);
    assert_eq!(metadata.dependencies[0].value.chars().count(), 1024);
    assert!(metadata.dependencies[0].command.is_none());
    assert_eq!(metadata.dependencies[0].oauth_callback_port, Some(65535));
    assert!(!skill.disable_model_invocation);
}

#[test]
fn codex_plugin_compat_t12_metadata_is_runtime_projection_not_a_skill_file_grant() {
    let fixture = Fixture::new();
    let original = fixture.load().skills.remove(0);
    let value = serde_json::to_value(&original).unwrap();
    assert_eq!(value["agent_metadata"]["interface"]["display_name"], "Review assistant");
    let restored: crate::skills::Skill = serde_json::from_value(value).unwrap();
    assert!(restored.agent_metadata.is_none());
    assert!(restored.plugin_source.is_none());
    assert!(original.tools.is_empty() && original.allowed_tools.is_empty());
    assert!(original.model.is_none() && original.context.is_none());
}

#[test]
fn codex_plugin_compat_t12_reads_portable_metadata_in_both_load_modes_and_honors_disabling() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.root.join(".codex-plugin/plugin.json")).unwrap();
    write(&fixture.root.join("plugin.json"), br#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"agent-pack","agents":"./agents"}"#);
    let full = fixture.load().skills.remove(0);
    let compact = super::super::skills::load_plugin_skill_inventory(
        &fixture.config.workspace_dir,
        &fixture.config,
        SkillLoadMode::MetadataOnly,
    )
    .skills
    .remove(0);
    assert_eq!(full.agent_metadata, compact.agent_metadata);
    assert!(compact.prompts.is_empty());
    assert!(compact.disable_model_invocation);
    write(&fixture.config.workspace_dir.join("plugins/agent-pack.disabled"), b"");
    assert!(fixture.load().skills.is_empty());
}

#[tokio::test]
async fn codex_plugin_compat_t12_enforces_policy_in_real_invocation_and_replay_admission() {
    use crate::skills::{SkillListingSessionState, load_skills_full_with_config};
    use crate::tools::skill::SkillTool;
    use crate::tools::traits::Tool;
    use crate::tools::{ToolUseContext, scope_tool_use_context};
    use serde_json::json;
    use std::sync::Arc;

    let fixture = Fixture::new();
    let before = snapshot(&fixture.root);
    let tool = SkillTool::new_with_runtime_config(
        Arc::new(crate::security::SecurityPolicy::default()),
        "t12".into(),
        false,
        fixture.config.workspace_dir.clone(),
        Arc::new(fixture.config.clone()),
    );
    let implicit = Arc::new(ToolUseContext::new("t12", None).with_full_access_enabled(true));
    let result =
        scope_tool_use_context(implicit.clone(), tool.call(json!({"skill":"agent-pack:review"})))
            .await
            .unwrap();
    assert_eq!(result.data["success"], false, "full access does not bypass declaration policy");
    assert!(implicit.take_invoked_skills().is_empty());
    let explicit = Arc::new(
        ToolUseContext::new("t12", None)
            .with_full_access_enabled(true)
            .with_explicitly_requested_skills(["agent-pack:review"]),
    );
    let result =
        scope_tool_use_context(explicit.clone(), tool.call(json!({"skill":"agent-pack:review"})))
            .await
            .unwrap();
    assert_eq!(result.data["success"], true);
    assert!(result.context_updates.iter().all(|value| {
        !["skill.allowed_tools", "skill.model", "skill.context"].contains(&value.key.as_str())
    }));
    let mut state = SkillListingSessionState::default();
    state.register_invoked("main", explicit.take_invoked_skills().remove(0));
    let skills = load_skills_full_with_config(&fixture.config.workspace_dir, &fixture.config);
    state.validate_invoked_sources("main", &skills).unwrap();
    write(
        &fixture.root.join("skills/review/agents/openai.yaml"),
        b"policy:\n  products: [chatgpt]\n",
    );
    let restricted = load_skills_full_with_config(&fixture.config.workspace_dir, &fixture.config);
    assert!(state.validate_invoked_sources("main", &restricted).is_err());
    write(
        &fixture.root.join("skills/review/agents/openai.yaml"),
        include_bytes!("../../../testdata/plugin_compat/t12/openai.yaml"),
    );
    assert_eq!(snapshot(&fixture.root), before);
    assert!(!fixture.config.workspace_dir.join("sentinel").exists());
}

#[cfg(unix)]
#[test]
fn codex_plugin_compat_t12_refuses_metadata_and_asset_symlinks_without_reading_outside() {
    use std::os::unix::fs::symlink;
    let mut fixture = Fixture::new();
    // Exercise the plugin metadata boundary even when the surrounding optional audit allows links.
    fixture.config.skills.security.reject_symlinks = false;
    let outside = fixture._temp.path().join("outside");
    write(&outside.join("openai.yaml"), b"interface:\n  display_name: outside-payload\n");
    fs::remove_dir_all(fixture.root.join("skills/review/agents")).unwrap();
    symlink(&outside, fixture.root.join("skills/review/agents")).unwrap();
    let inventory = fixture.load();
    assert_eq!(inventory.skills.len(), 1);
    assert!(inventory.skills[0].agent_metadata.is_none());
    assert!(
        inventory.diagnostics.iter().any(|value| value.message.contains("unsafe agent metadata"))
    );
    assert!(inventory.diagnostics.iter().all(|value| !value.message.contains("outside-payload")));
    fs::remove_file(fixture.root.join("skills/review/agents")).unwrap();
    write(
        &fixture.root.join("skills/review/agents/openai.yaml"),
        b"interface:\n  display_name: retained\n  icon_small: ../../assets/linked.svg\n",
    );
    write(&outside.join("icon.svg"), b"outside-icon");
    fs::create_dir_all(fixture.root.join("assets")).unwrap();
    symlink(outside.join("icon.svg"), fixture.root.join("assets/linked.svg")).unwrap();
    let metadata = fixture.load().skills.remove(0).agent_metadata.unwrap();
    assert!(metadata.interface.unwrap().icon_small.is_none());
    assert_eq!(fs::read(outside.join("icon.svg")).unwrap(), b"outside-icon");
}
