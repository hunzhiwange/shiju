//! Command migration is verified through discovery and the actual skill tool.

use crate::config::Config;
use crate::skills::{SkillLoadMode, load_skills_full_with_config};
use crate::tools::skill::SkillTool;
use crate::tools::traits::Tool;
use crate::tools::{ToolUseContext, scope_tool_use_context};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const MANIFEST: &[u8] = include_bytes!("../../../testdata/plugin_compat/t11/plugin.json");
const COMMAND: &[u8] = include_bytes!("../../../testdata/plugin_compat/t11/review.md");
const EXPECTED: &str = include_str!("../../../testdata/plugin_compat/t11/expected-skill.md");

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
        crate::plugins::install_plugin_from_marketplace(
            self.source.to_str().unwrap(),
            &self.workspace.join("plugins"),
            market,
        )
        .unwrap()
        .installed_dir
    }

    fn inventory(&self) -> super::super::skills::PluginSkillInventory {
        super::super::skills::load_plugin_skill_inventory(
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
        Arc::new(crate::security::SecurityPolicy::default()),
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
fn codex_plugin_compat_t11_rejects_invalid_input() {
    let fixture = Fixture::new();
    write(
        &fixture.source.join("commands/missing.md"),
        include_bytes!("../../../testdata/plugin_compat/t11/missing.md"),
    );
    for (name, body) in [
        ("arguments", "$ARGUMENTS"),
        ("position", "$1"),
        ("mustache", "{{value}}"),
        ("shell", "!`touch sentinel`"),
        ("file", "Read @private.txt"),
    ] {
        write(
            &fixture.source.join(format!("commands/{name}.md")),
            format!("---\ndescription: Unsupported template\n---\n{body}\n").as_bytes(),
        );
    }
    fixture.install("first");
    let inventory = fixture.inventory();
    assert_eq!(
        inventory.skills.len(),
        1,
        "retain the supported command and reject unsupported templates"
    );
    assert_eq!(inventory.skills[0].name, "source-command-review");
    assert!(inventory.diagnostics.len() >= 6);
    assert!(!fixture.workspace.join("sentinel").exists());
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
    crate::plugins::set_plugin_enabled(
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

#[test]
fn codex_plugin_compat_t11_uses_explicit_files_and_roots_without_default_fallback() {
    let fixture = Fixture::new();
    write(&fixture.source.join(".claude-plugin/plugin.json"), br#"{"name":"ignored-alternative"}"#);
    write(
        &fixture.source.join(".codex-plugin/plugin.json"),
        br#"{"name":"command-pack","commands":["./extra","./single.md","./extra"]}"#,
    );
    write(&fixture.source.join("extra/nested/review.md"), COMMAND);
    write(
        &fixture.source.join("single.md"),
        b"---\r\ndescription: true\r\n---\r\nA plain template.\r\n",
    );
    fixture.install("first");
    let inventory = fixture.inventory();
    assert!(inventory.diagnostics.is_empty());
    assert_eq!(
        inventory.skills.iter().map(|skill| skill.name.as_str()).collect::<Vec<_>>(),
        ["source-command-nested-review", "source-command-single"]
    );
    assert_eq!(inventory.skills[1].description, "true");
}

#[test]
fn codex_plugin_compat_t11_drops_every_slug_collision_and_keeps_healthy_templates() {
    let fixture = Fixture::new();
    for name in ["same_name", "same-name"] {
        write(&fixture.source.join(format!("commands/{name}.md")), COMMAND);
    }
    write(&fixture.source.join("commands/README.md"), COMMAND);
    write(
        &fixture.source.join("commands/no-description.md"),
        b"---\ndescription: '  '\n---\nSkipped\n",
    );
    write(&fixture.source.join("commands/invalid-yaml.md"), b"---\ndescription: [\n---\nSkipped\n");
    write(
        &fixture.source.join("commands/scalar-shape.md"),
        b"---\ndescription: [one]\n---\nSkipped\n",
    );
    fixture.install("first");
    let inventory = fixture.inventory();
    assert_eq!(inventory.skills.len(), 1);
    assert_eq!(inventory.skills[0].name, "source-command-review");
    assert_eq!(
        inventory
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.message.contains("duplicate migrated command name"))
            .count(),
        2
    );
}

#[test]
fn codex_plugin_compat_t11_detects_collisions_before_rendered_size_filtering() {
    let fixture = Fixture::new();
    write(&fixture.source.join("commands/same-name.md"), COMMAND);
    write(
        &fixture.source.join("commands/same_name.md"),
        format!("---\ndescription: Oversize collision\n---\n{}", "x".repeat(4_000)).as_bytes(),
    );
    fixture.install("first");
    let inventory = fixture.inventory();
    assert_eq!(
        inventory.skills.len(),
        1,
        "an oversized collision must not make the other source uniquely callable"
    );
    assert_eq!(inventory.skills[0].name, "source-command-review");
    assert_eq!(
        inventory
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.message.contains("duplicate migrated command name"))
            .count(),
        2
    );
}

#[test]
fn codex_plugin_compat_t11_rejects_explicit_bad_paths_without_default_fallback() {
    for commands in [
        json!("../outside"),
        json!("./../outside"),
        json!("./missing"),
        json!(42),
        json!(["./commands", 42]),
    ] {
        let fixture = Fixture::new();
        write(
            &fixture.source.join(".codex-plugin/plugin.json"),
            &serde_json::to_vec(&json!({"name":"command-pack", "commands": commands})).unwrap(),
        );
        fixture.install("first");
        let inventory = fixture.inventory();
        assert!(inventory.skills.is_empty());
        assert!(!inventory.diagnostics.is_empty());
        assert!(!fixture.config.config_path.parent().unwrap().join("cache").exists());
    }
    let fixture = Fixture::new();
    write(
        &fixture.source.join(".codex-plugin/plugin.json"),
        br#"{"name":"command-pack","commands":[]}"#,
    );
    fixture.install("first");
    assert!(fixture.inventory().skills.is_empty());
    assert!(fixture.inventory().diagnostics.is_empty());
}

#[test]
fn codex_plugin_compat_t11_only_normalizes_the_profile_document_filename() {
    let fixture = Fixture::new();
    write(&fixture.source.join("commands/review.md"), "---\ndescription: 文档 agents.md\n---\n先读 Agents.md；保留 Claude、Cursor 和 myAGENTS.md。\n".as_bytes());
    fixture.install("first");
    let inventory = fixture.inventory();
    assert_eq!(inventory.skills.len(), 1);
    assert!(inventory.skills[0].description.contains("AGENTS.md"));
    assert!(
        inventory.skills[0].prompts[0]
            .contains("先读 AGENTS.md；保留 Claude、Cursor 和 myAGENTS.md。")
    );
}

#[test]
fn codex_plugin_compat_t11_preserves_multiline_description_rendering() {
    let fixture = Fixture::new();
    write(
        &fixture.source.join("commands/review.md"),
        b"---\ndescription: |\n  First line\n  Second line\n---\nA plain command.\n",
    );
    fixture.install("first");
    let inventory = fixture.inventory();
    assert_eq!(inventory.skills.len(), 1);
    assert!(
        inventory.skills[0].prompts[0].contains("description: \"First line\nSecond line\"\n"),
        "rendered YAML bytes must preserve the reference profile's multiline scalar"
    );
}

#[test]
fn codex_plugin_compat_t11_preserves_logical_root_precedence_despite_external_cache() {
    for (root, command_wins) in [("skills", true), (".a-skills", false)] {
        let fixture = Fixture::new();
        write(
            &fixture.source.join(".codex-plugin/plugin.json"),
            &serde_json::to_vec(&json!({"name":"command-pack", "skills":format!("./{root}")}))
                .unwrap(),
        );
        let regular = "---\nname: source-command-review\ndescription: Explicit package skill\n---\nPackage skill wins only when its declared root sorts first.\n";
        write(&fixture.source.join(root).join("review/SKILL.md"), regular.as_bytes());
        fixture.install("first");
        let inventory = fixture.inventory();
        assert_eq!(inventory.skills.len(), 1);
        assert_eq!(
            inventory.skills[0].prompts,
            [if command_wins { EXPECTED } else { regular }],
            "the external cache path must not alter Codex's sorted logical root precedence"
        );
        assert!(
            inventory
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("duplicate skill name"))
        );
    }
}

#[test]
fn codex_plugin_compat_t11_limits_rendered_bytes_and_name_length() {
    let fixture = Fixture::new();
    let empty = "---\ndescription: Empty command\n---\n";
    write(&fixture.source.join("commands/empty.md"), empty.as_bytes());
    write(&fixture.source.join(format!("commands/{}.md", "x".repeat(50))), COMMAND);
    write(&fixture.source.join(format!("commands/{}.md", "y".repeat(49))), COMMAND);
    write(
        &fixture.source.join("commands/oversize.md"),
        format!("---\ndescription: Too large\n---\n{}", "界".repeat(1_400)).as_bytes(),
    );
    fixture.install("first");
    let inventory = fixture.inventory();
    assert_eq!(inventory.skills.len(), 3);
    assert!(inventory.skills.iter().any(|skill| skill.name.len() == 64));
    assert!(inventory.skills.iter().any(|skill| skill.name == "source-command-empty"
        && skill.prompts[0].contains("No command template body was found.")));
    assert!(
        inventory.diagnostics.iter().any(|diagnostic| diagnostic.message.contains("4000 bytes"))
    );
    assert!(
        inventory.diagnostics.iter().any(|diagnostic| diagnostic.message.contains("64 characters"))
    );
}

#[test]
fn codex_plugin_compat_t11_invalidates_derived_cache_and_rejects_tampering() {
    let fixture = Fixture::new();
    let installed = fixture.install("first");
    let first = fixture.inventory();
    let old_path = first.skills[0].location.as_ref().unwrap();
    fs::write(old_path, "forged cache payload").unwrap();
    let rejected = fixture.inventory();
    assert!(rejected.skills.is_empty());
    assert!(
        rejected
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("contents changed"))
    );
    assert!(
        rejected
            .diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.message.contains("forged cache payload"))
    );
    let changed = String::from_utf8(COMMAND.to_vec())
        .unwrap()
        .replace("Review the change", "Review the updated change");
    fs::write(installed.join("commands/review.md"), changed).unwrap();
    let current = fixture.inventory();
    assert_eq!(current.skills.len(), 1);
    assert_ne!(current.skills[0].location.as_ref(), Some(old_path));
    assert!(current.skills[0].prompts[0].contains("updated change"));
    assert_eq!(fs::read_to_string(old_path).unwrap(), "forged cache payload");
}

#[test]
fn codex_plugin_compat_t11_metadata_and_security_use_the_existing_skill_boundary() {
    let mut fixture = Fixture::new();
    fixture.install("first");
    let skills = crate::plugins::load_plugin_skills(
        &fixture.workspace,
        &fixture.config,
        SkillLoadMode::MetadataOnly,
    );
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].description, "Review a change safely.");
    assert!(skills[0].prompts.is_empty());
    assert!(skills[0].allowed_tools.is_empty(), "command metadata must not add tool privileges");
    fixture.config.skills.security.require_manifest = true;
    assert_eq!(
        fixture.inventory().skills.len(),
        1,
        "derived SKILL.md must satisfy the configured manifest audit"
    );
}

#[test]
fn codex_plugin_compat_t11_does_not_migrate_portable_or_native_commands() {
    let fixture = Fixture::new();
    for (folder, manifest_path, manifest) in [
        (
            "portable",
            "plugin.json",
            r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"portable"}"#,
        ),
        (
            "native",
            ".vibewindow-plugin/plugin.json",
            r#"{"name":"native","version":"1.0.0","description":"Native package"}"#,
        ),
    ] {
        let root = fixture.workspace.join("plugins").join(folder);
        write(&root.join(manifest_path), manifest.as_bytes());
        write(&root.join("commands/review.md"), COMMAND);
    }
    let inventory = fixture.inventory();
    assert!(inventory.skills.is_empty());
    assert!(inventory.diagnostics.is_empty());
    assert!(!fixture.config.config_path.parent().unwrap().join("cache").exists());
}

#[test]
fn codex_plugin_compat_t11_refuses_cache_inside_original_package_before_creating_it() {
    let mut fixture = Fixture::new();
    let installed = fixture.install("first");
    let before = snapshot(&installed);
    fixture.config.config_path = installed.join("missing/config/shiju.json");
    let inventory = fixture.inventory();
    assert!(inventory.skills.is_empty());
    assert!(
        inventory
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("outside original package"))
    );
    assert!(!installed.join("missing").exists());
    assert_eq!(snapshot(&installed), before);
}

#[cfg(unix)]
#[test]
fn codex_plugin_compat_t11_rejects_symlink_entries_and_cache_redirection() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    let installed = fixture.install("first");
    let outside = fixture._temp.path().join("external");
    fs::create_dir(&outside).unwrap();
    symlink(&outside, installed.join("commands/link")).unwrap();
    symlink(installed.join("commands/review.md"), installed.join("commands/alias.md")).unwrap();
    let inventory = fixture.inventory();
    assert_eq!(inventory.skills.len(), 1);
    assert_eq!(inventory.diagnostics.len(), 2);
    let cache = fixture.config.config_path.parent().unwrap().join("cache/plugin-commands");
    // Redirect only a separate fixture's empty host cache; never remove a live cache tree.
    let mut redirected = fixture.config.clone();
    redirected.config_path = fixture._temp.path().join("redirected/shiju.json");
    fs::create_dir_all(redirected.config_path.parent().unwrap().join("cache")).unwrap();
    symlink(&outside, redirected.config_path.parent().unwrap().join("cache/plugin-commands"))
        .unwrap();
    let denied = super::super::skills::load_plugin_skill_inventory(
        &fixture.workspace,
        &redirected,
        SkillLoadMode::Full,
    );
    assert!(denied.skills.is_empty());
    assert!(
        denied
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("cache directory must not be a symlink"))
    );
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
    assert!(cache.is_dir());
}
