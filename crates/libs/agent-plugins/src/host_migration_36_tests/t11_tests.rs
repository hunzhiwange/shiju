//! Command migration is verified through discovery and the actual skill tool.

use agent_config::runtime::Config;
use super::skills::{SkillLoadMode, load_skills_full_with_config};
use super::SkillTool;
use tool_types::Tool;
use super::{ToolUseContext, scope_tool_use_context};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const MANIFEST: &[u8] = include_bytes!("../../../../vw-agent/testdata/plugin_compat/t11/plugin.json");
const COMMAND: &[u8] = include_bytes!("../../../../vw-agent/testdata/plugin_compat/t11/review.md");
const EXPECTED: &str = include_str!("../../../../vw-agent/testdata/plugin_compat/t11/expected-skill.md");

struct Fixture {
    _temp: tempfile::TempDir,
    workspace: PathBuf,
    source: PathBuf,
    config: Config,
}

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

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("original");
        let workspace = temp.path().join("workspace");
        write(&source.join(".codex-plugin/plugin.json"), MANIFEST);
        write(&source.join("commands/review.md"), COMMAND);
        let mut config = Config::default();
        config.workspace_dir = workspace.clone();
        config.config_path = temp.path().join("config/shiju.json");
        Self { _temp: temp, source, workspace, config }
    }

    fn install(&self, market: &str) -> PathBuf {
        crate::install_plugin_from_marketplace(
            self.source.to_str().unwrap(),
            &self.workspace.join("plugins"),
            market,
        )
        .unwrap()
        .installed_dir
    }

    fn inventory(&self) -> super::super::skills::PluginSkillInventory {
        super::load_plugin_skill_inventory(
            &self.workspace,
            &self.config,
            SkillLoadMode::Full,
        )
    }
}

#[tokio::test]
async fn codex_plugin_compat_t11_invokes_migrated_command() {
    let fixture = Fixture::new();
    let installed = fixture.install("first");
    let before = snapshot(&installed);
    let tool = SkillTool::new_with_runtime_config(
        Arc::new(agent_security::SecurityPolicy::default()),
        "t11".into(),
        false,
        fixture.workspace.clone(),
        Arc::new(fixture.config.clone()),
    );
    let context = Arc::new(ToolUseContext::new("t11", None).with_full_access_enabled(true));
    let result = scope_tool_use_context(
        context.clone(),
        tool.call(
            json!({"skill":"command-pack@first:source-command-review","args":"literal arguments"}),
        ),
    )
    .await
    .unwrap();
    assert_eq!(result.data["success"], true, "a supported command must become callable");
    assert!(result.extra_messages[0].content.as_str().unwrap().contains(EXPECTED));
    let record = context.take_invoked_skills().pop().unwrap();
    assert_eq!(record.args.as_deref(), Some("literal arguments"));
    assert!(!record.source.starts_with(&installed));
    assert_eq!(fs::read_to_string(record.source).unwrap(), EXPECTED);
    assert_eq!(snapshot(&installed), before);
    assert_eq!(snapshot(&fixture.source), before);
}

#[test]
fn codex_plugin_compat_t11_preserves_isolation() {
    let fixture = Fixture::new();
    let first = fixture.install("first");
    let second = fixture.install("second");
    let before_first = snapshot(&first);
    let before_second = snapshot(&second);
    let skills = load_skills_full_with_config(&fixture.workspace, &fixture.config);
    let commands =
        skills.iter().filter(|skill| skill.name == "source-command-review").collect::<Vec<_>>();
    assert_eq!(commands.len(), 2, "same command in different markets must retain both identities");
    assert_ne!(commands[0].location, commands[1].location);
    for command in commands {
        let source = command.plugin_source.as_ref().unwrap();
        assert_eq!(source.relative_skill_path, Path::new("commands/review.md"));
        assert!(!command.location.as_ref().unwrap().starts_with(&source.root));
    }
    super::set_plugin_enabled(
        &fixture.workspace.join("plugins"),
        "command-pack@first",
        false,
    )
    .unwrap();
    let current = fixture.inventory();
    assert_eq!(current.skills.len(), 1);
    assert_eq!(current.skills[0].plugin_source.as_ref().unwrap().plugin_id, "command-pack@second");
    assert_eq!(snapshot(&first), before_first);
    assert_eq!(snapshot(&second), before_second);
}
