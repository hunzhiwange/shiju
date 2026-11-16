//! Agent declarations are exercised through the real plugin skill loader.

use agent_config::runtime::Config;
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
            include_bytes!("../../../../vw-agent/testdata/plugin_compat/t12/SKILL.md"),
        );
        write(
            &root.join("skills/review/agents/openai.yaml"),
            include_bytes!("../../../../vw-agent/testdata/plugin_compat/t12/openai.yaml"),
        );
        write(&root.join("skills/review/assets/icon.svg"), b"<svg/>\n");
        Self { _temp: temp, root, config }
    }


}

#[tokio::test]
async fn codex_plugin_compat_t12_enforces_policy_in_real_invocation_and_replay_admission() {
    use super::skills::{SkillListingSessionState, load_skills_full_with_config};
    use super::SkillTool;
    use tool_types::Tool;
    use super::{ToolUseContext, scope_tool_use_context};
    use serde_json::json;
    use std::sync::Arc;

    let fixture = Fixture::new();
    let before = snapshot(&fixture.root);
    let tool = SkillTool::new_with_runtime_config(
        Arc::new(agent_security::SecurityPolicy::default()),
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
        include_bytes!("../../../../vw-agent/testdata/plugin_compat/t12/openai.yaml"),
    );
    assert_eq!(snapshot(&fixture.root), before);
    assert!(!fixture.config.workspace_dir.join("sentinel").exists());
}
