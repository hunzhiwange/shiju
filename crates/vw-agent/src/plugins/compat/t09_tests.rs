//! 通过真实插件发现与技能加载入口验证多根、拒绝和来源隔离。

use crate::app::agent::config::Config;
use crate::app::agent::skills::{Skill, SkillLoadMode};
use std::fs;
use std::path::{Path, PathBuf};

const MANIFEST: &[u8] = include_bytes!("../../../testdata/plugin_compat/t09/plugin.json");
const ALPHA: &str = include_str!("../../../testdata/plugin_compat/t09/alpha/SKILL.md");
const BETA: &str = include_str!("../../../testdata/plugin_compat/t09/beta/SKILL.md");

struct Fixture {
    temp: tempfile::TempDir,
    workspace: PathBuf,
    config: Config,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("workspace");
        let mut config = Config::default();
        config.workspace_dir = workspace.clone();
        config.config_path = temp.path().join("config/vibewindow.json");
        Self { temp, workspace, config }
    }

    fn package(&self, folder: &str) -> PathBuf {
        let root = self.workspace.join("plugins").join(folder);
        write(&root.join(".codex-plugin/plugin.json"), MANIFEST);
        write(&root.join("a-skills/alpha/SKILL.md"), ALPHA.as_bytes());
        write(&root.join("z-skills/beta/SKILL.md"), BETA.as_bytes());
        root
    }

    fn load(&self) -> Vec<Skill> {
        crate::plugins::load_plugin_skills(&self.workspace, &self.config, SkillLoadMode::Full)
    }
}

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn names(skills: &[Skill]) -> Vec<&str> {
    skills.iter().map(|skill| skill.name.as_str()).collect()
}

#[test]
fn codex_plugin_compat_t09_loads_every_skill_root() {
    let fixture = Fixture::new();
    let root = fixture.package("bundle");
    let skills = fixture.load();
    assert_eq!(names(&skills), ["alpha", "beta"], "every audited root must contribute skills");
    assert_eq!(skills[0].prompts, [ALPHA]);
    assert_eq!(skills[1].prompts, [BETA]);
    assert_eq!(
        skills[0].location.as_ref().unwrap(),
        &root.canonicalize().unwrap().join("a-skills/alpha/SKILL.md")
    );
    assert_eq!(fs::read(root.join(".codex-plugin/plugin.json")).unwrap(), MANIFEST);
}

#[test]
fn codex_plugin_compat_t09_rejects_invalid_input() {
    let mut fixture = Fixture::new();
    let root = fixture.package("bundle");
    write(&root.join("z-skills/beta/run.sh"), b"#!/bin/sh\ntrue\n");
    fixture.config.skills.security.reject_script_files = true;
    let skills = fixture.load();
    assert_eq!(
        names(&skills),
        ["alpha"],
        "reject the unsafe skill while retaining the healthy audited root"
    );
    assert_eq!(fs::read(root.join("z-skills/beta/run.sh")).unwrap(), b"#!/bin/sh\ntrue\n");
}

#[test]
fn codex_plugin_compat_t09_preserves_isolation() {
    let fixture = Fixture::new();
    let active = fixture.package("active");
    let disabled = fixture.package("disabled");
    let mut manifest: serde_json::Value = serde_json::from_slice(MANIFEST).unwrap();
    manifest["name"] = "disabled-bundle".into();
    write(&disabled.join(".codex-plugin/plugin.json"), &serde_json::to_vec(&manifest).unwrap());
    write(&fixture.workspace.join("plugins/disabled-bundle.disabled"), b"");
    let external = fixture.temp.path().join("outside.txt");
    fs::write(&external, b"untouched").unwrap();
    let skills = fixture.load();
    assert_eq!(
        names(&skills),
        ["alpha", "beta"],
        "enabled plugin must load all roots even when another plugin is disabled"
    );
    assert!(
        skills.iter().all(|skill| skill
            .location
            .as_ref()
            .unwrap()
            .starts_with(active.canonicalize().unwrap()))
    );
    assert_eq!(fs::read(external).unwrap(), b"untouched");
    assert_eq!(fs::read(disabled.join("a-skills/alpha/SKILL.md")).unwrap(), ALPHA.as_bytes());
}

#[test]
fn codex_plugin_compat_t09_preserves_source_and_relative_resources() {
    let fixture = Fixture::new();
    let root = fixture.package("bundle");
    let resource = root.join("a-skills/alpha/resources/input.txt");
    write(&resource, b"package resource");
    let skills = fixture.load();
    let source = skills[0].plugin_source.as_ref().unwrap();
    assert_eq!(source.plugin_id, "root-bundle");
    assert_eq!(source.namespace, "root-bundle");
    assert_eq!(source.root, root.canonicalize().unwrap());
    assert_eq!(source.relative_skill_path, Path::new("a-skills/alpha/SKILL.md"));
    let resolved =
        skills[0].location.as_ref().unwrap().parent().unwrap().join("resources/input.txt");
    assert_eq!(fs::read(resolved).unwrap(), b"package resource");
    assert_eq!(fs::read(root.join(".codex-plugin/plugin.json")).unwrap(), MANIFEST);
    let metadata = crate::plugins::load_plugin_skills(
        &fixture.workspace,
        &fixture.config,
        SkillLoadMode::MetadataOnly,
    );
    assert_eq!(names(&metadata), ["alpha", "beta"]);
    assert!(metadata.iter().all(|skill| skill.prompts.is_empty() && skill.plugin_source.is_some()));
    let mut serialized = serde_json::to_value(&skills[0]).unwrap();
    assert!(serialized.get("plugin_source").is_none());
    serialized["plugin_source"] = serde_json::json!({"plugin_id":"forged"});
    let decoded: Skill = serde_json::from_value(serialized).unwrap();
    assert!(decoded.plugin_source.is_none());
}

#[test]
fn codex_plugin_compat_t09_loads_default_recursive_root_and_frontmatter() {
    let fixture = Fixture::new();
    let root = fixture.workspace.join("plugins/default");
    write(&root.join(".codex-plugin/plugin.json"), br#"{"name":"default-pack"}"#);
    let content = "---\nname: '  explicit   name '\ndescription: |\n  Read the\n  default root.\nmetadata:\n  short-description: ' Compact   label '\n---\nOriginal body.\n";
    write(&root.join("skills/nested/group/SKILL.md"), content.as_bytes());
    let skills = fixture.load();
    assert_eq!(names(&skills), ["explicit name"]);
    assert_eq!(skills[0].description, "Read the default root.");
    assert_eq!(skills[0].short_description.as_deref(), Some("Compact label"));
    assert_eq!(skills[0].prompts, [content]);
    assert!(!root.join(".vibewindow-plugin").exists());
}

#[test]
fn codex_plugin_compat_t09_limits_portable_discovery_to_direct_markdown_children() {
    let fixture = Fixture::new();
    let root = fixture.workspace.join("plugins/portable");
    write(&root.join("plugin.json"), br#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"portable-pack"}"#);
    write(&root.join("skills/direct/SKILL.md"), ALPHA.as_bytes());
    write(
        &root.join("skills/direct/SKILL.toml"),
        b"[skill]\nname='toml-shadow'\ndescription='ignored native manifest'\n",
    );
    write(&root.join("skills/group/deep/SKILL.md"), BETA.as_bytes());
    write(&root.join("skills/SKILL.md"), BETA.as_bytes());
    write(
        &root.join("skills/only-toml/SKILL.toml"),
        b"[skill]\nname='toml-only'\ndescription='not a portable skill'\n",
    );
    let skills = fixture.load();
    assert_eq!(names(&skills), ["alpha"]);
    assert_eq!(skills[0].prompts, [ALPHA]);
    let empty = fixture.workspace.join("plugins/without-skills");
    write(&empty.join("plugin.json"), br#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"without-skills"}"#);
    assert_eq!(names(&fixture.load()), ["alpha"]);
    assert!(!empty.join("skills").exists());
}

#[test]
fn codex_plugin_compat_t09_reports_conflicts_and_deduplicates_overlapping_roots() {
    let fixture = Fixture::new();
    let root = fixture.package("bundle");
    write(&root.join("z-skills/beta/SKILL.md"), ALPHA.as_bytes());
    let inventory = super::super::skills::load_plugin_skill_inventory(
        &fixture.workspace,
        &fixture.config,
        SkillLoadMode::Full,
    );
    assert_eq!(names(&inventory.skills), ["alpha"]);
    let collision =
        inventory.diagnostics.iter().find(|d| d.message.contains("duplicate skill name")).unwrap();
    assert_eq!(collision.plugin_id.as_deref(), Some("root-bundle"));
    assert_eq!(collision.path, root.canonicalize().unwrap().join("z-skills/beta/SKILL.md"));
    assert!(collision.message.contains("a-skills/alpha/SKILL.md"));
    write(
        &root.join(".codex-plugin/plugin.json"),
        br#"{"name":"root-bundle","skills":["./a-skills/alpha","./a-skills","./a-skills"]}"#,
    );
    let overlap = super::super::skills::load_plugin_skill_inventory(
        &fixture.workspace,
        &fixture.config,
        SkillLoadMode::Full,
    );
    assert_eq!(names(&overlap.skills), ["alpha"]);
    assert!(overlap.diagnostics.is_empty());
}

#[test]
fn codex_plugin_compat_t09_rejects_bad_metadata_without_dropping_healthy_skills() {
    let fixture = Fixture::new();
    let root = fixture.package("bundle");
    for (folder, body) in [
        ("missing", "# Unstructured\nMissing frontmatter\n".to_owned()),
        ("empty", "---\ndescription: ' '\n---\nNo description.\n".to_owned()),
        ("shape", "---\ndescription: [one, two]\n---\nInvalid shape.\n".to_owned()),
        ("long", format!("---\nname: {}\ndescription: too long\n---\n", "界".repeat(65))),
        ("broken", "---\nname: okay\ndescription: okay\nmetadata: [\n---\n".to_owned()),
        (
            "sensitive",
            "---\ndescription: okay\nmetadata: private-fixture-payload\n---\n".to_owned(),
        ),
    ] {
        write(&root.join(format!("a-skills/{folder}/SKILL.md")), body.as_bytes());
    }
    let inventory = super::super::skills::load_plugin_skill_inventory(
        &fixture.workspace,
        &fixture.config,
        SkillLoadMode::Full,
    );
    assert_eq!(names(&inventory.skills), ["alpha", "beta"]);
    assert_eq!(inventory.diagnostics.len(), 6);
    assert!(inventory.diagnostics.iter().all(|d| !d.message.contains("private-fixture-payload")));
    assert!(inventory.diagnostics.iter().all(|d| d.plugin_id.as_deref() == Some("root-bundle")
        && d.path.starts_with(root.canonicalize().unwrap())));
}

#[test]
fn codex_plugin_compat_t09_repairs_scalar_prose_and_preserves_block_bodies() {
    let fixture = Fixture::new();
    let root = fixture.workspace.join("plugins/scalars");
    write(&root.join(".codex-plugin/plugin.json"), br#"{"name":"scalars"}"#);
    let content = "---\ndescription: Build for AWS: ECS # comment\nargument-hint: <duration: e.g. 7d>\nmetadata:\n  short-description: |\n    Keep original: yes\n    second line\n---\nDo not rewrite me.\n";
    write(&root.join("skills/fallback/SKILL.md"), content.as_bytes());
    let skills = fixture.load();
    assert_eq!(names(&skills), ["fallback"]);
    assert_eq!(skills[0].description, "Build for AWS: ECS");
    assert_eq!(skills[0].short_description.as_deref(), Some("Keep original: yes second line"));
    assert_eq!(skills[0].prompts, [content]);
    assert_eq!(fs::read_to_string(root.join("skills/fallback/SKILL.md")).unwrap(), content);
}

#[test]
fn codex_plugin_compat_t09_preserves_disabled_skills_and_rejects_file_roots() {
    let fixture = Fixture::new();
    let root = fixture.package("bundle");
    write(&root.join("a-skills/alpha/SKILL.disabled"), b"");
    assert_eq!(names(&fixture.load()), ["beta"]);
    write(
        &root.join(".codex-plugin/plugin.json"),
        br#"{"name":"root-bundle","skills":"./a-skills/alpha/SKILL.md"}"#,
    );
    let inventory = super::super::skills::load_plugin_skill_inventory(
        &fixture.workspace,
        &fixture.config,
        SkillLoadMode::Full,
    );
    assert!(inventory.skills.is_empty());
    assert!(inventory.diagnostics.iter().any(|d| d.message.contains("must be a directory")));
}

#[cfg(unix)]
#[test]
fn codex_plugin_compat_t09_rejects_symlinks_without_following_resource_cycles() {
    use std::os::unix::fs::symlink;
    let mut fixture = Fixture::new();
    let root = fixture.package("bundle");
    let external = fixture.temp.path().join("outside");
    write(&external.join("SKILL.md"), BETA.as_bytes());
    symlink(&external, root.join("z-skills/linked")).unwrap();
    symlink(root.join("a-skills/alpha"), root.join("a-skills/alpha/cycle")).unwrap();
    fixture.config.skills.security.reject_symlinks = true;
    let inventory = super::super::skills::load_plugin_skill_inventory(
        &fixture.workspace,
        &fixture.config,
        SkillLoadMode::Full,
    );
    assert_eq!(names(&inventory.skills), ["beta"]);
    assert!(inventory.diagnostics.iter().any(|d| d.message.contains("symlink")));
    assert_eq!(fs::read(external.join("SKILL.md")).unwrap(), BETA.as_bytes());
    // 默认策略仍不得在审计时进入循环或外部资源；不改变用户配置的准入规则。
    fixture.config.skills.security.reject_symlinks = false;
    let permissive = fixture.load();
    assert_eq!(names(&permissive), ["alpha", "beta"]);
    assert!(
        permissive.iter().all(|skill| skill
            .location
            .as_ref()
            .unwrap()
            .starts_with(root.canonicalize().unwrap()))
    );
}

#[test]
fn codex_plugin_compat_t09_preserves_same_name_marketplace_provenance_in_runtime_merge() {
    let fixture = Fixture::new();
    let package = fixture.package("source");
    let source = fixture.temp.path().join("original");
    fs::rename(package, &source).unwrap();
    let install_dir = fixture.workspace.join("plugins");
    for market in ["first", "second"] {
        crate::plugins::install_plugin_from_marketplace(
            source.to_str().unwrap(),
            &install_dir,
            market,
        )
        .unwrap();
    }
    let skills = crate::app::agent::skills::load_skills_full_with_config(
        &fixture.workspace,
        &fixture.config,
    );
    let plugins = skills.iter().filter(|skill| skill.plugin_source.is_some()).collect::<Vec<_>>();
    assert_eq!(plugins.len(), 4);
    for market in ["first", "second"] {
        let key = format!("root-bundle@{market}");
        let owned = plugins
            .iter()
            .filter(|skill| skill.plugin_source.as_ref().unwrap().plugin_id == key)
            .collect::<Vec<_>>();
        assert_eq!(
            owned.iter().map(|skill| skill.name.as_str()).collect::<Vec<_>>(),
            ["alpha", "beta"]
        );
        assert!(owned.iter().all(|skill| skill.plugin_source.as_ref().unwrap().root
            == install_dir.join(&key).canonicalize().unwrap()));
    }
    assert_eq!(fs::read(source.join(".codex-plugin/plugin.json")).unwrap(), MANIFEST);
}

#[cfg(unix)]
#[test]
fn codex_plugin_compat_t09_rejects_literal_backslash_aliases_before_loading_body() {
    let fixture = Fixture::new();
    let root = fixture.workspace.join("plugins/portable");
    write(&root.join("plugin.json"), br#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"portable-pack"}"#);
    write(&root.join("skills/odd/alias/SKILL.md"), ALPHA.as_bytes());
    write(&root.join("skills/odd\\alias/SKILL.md"), BETA.as_bytes());
    let inventory = super::super::skills::load_plugin_skill_inventory(
        &fixture.workspace,
        &fixture.config,
        SkillLoadMode::Full,
    );
    assert!(inventory.skills.is_empty());
    assert!(
        inventory
            .diagnostics
            .iter()
            .any(|d| d.message.contains("using '/'") && d.path.ends_with("odd\\alias/SKILL.md"))
    );
    assert_eq!(fs::read(root.join("skills/odd\\alias/SKILL.md")).unwrap(), BETA.as_bytes());
}
