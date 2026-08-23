//! 验证新版技能发现的来源优先级、去重和目录过滤规则。

use super::*;
use crate::app::agent::skills::tests::helpers::{EnvVarGuard, open_skills_env_lock};
use std::fs;
use std::path::Path;

fn write_skill(dir: &Path, contents: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join("SKILL.md"), contents).unwrap();
}

#[test]
fn skill_discovery_parses_claude_invocation_frontmatter() {
    let root = tempfile::tempdir().unwrap();
    let skill_dir = root.path().join("skills").join("review");
    write_skill(
        &skill_dir,
        r#"---
name: review
description: Review a proposed change.
when_to_use: Before accepting a code change.
allowed-tools: [Read, Grep]
model: sonnet
disable-model-invocation: true
user-invocable: false
hooks:
  PreToolUse:
    - matcher: Bash
context: fork
paths: ["src/**/*.rs"]
effort: high
argument-hint: "[revision]"
---
# Review
"#,
    );

    let skills = load_skills(root.path());
    let skill = skills.iter().find(|skill| skill.name == "review").unwrap();

    assert_eq!(skill.when_to_use.as_deref(), Some("Before accepting a code change."));
    assert_eq!(skill.allowed_tools, vec!["Read", "Grep"]);
    assert_eq!(skill.model.as_deref(), Some("sonnet"));
    assert!(skill.disable_model_invocation);
    assert!(!skill.user_invocable);
    assert!(skill.hooks.is_some());
    assert_eq!(skill.context.as_deref(), Some("fork"));
    assert_eq!(skill.paths, vec!["src/**/*.rs"]);
    assert_eq!(skill.effort.as_deref(), Some("high"));
    assert_eq!(skill.argument_hint.as_deref(), Some("[revision]"));
}

#[test]
fn skill_discovery_applies_source_precedence_and_dynamic_paths() {
    let _lock = open_skills_env_lock().lock().unwrap();
    let home = tempfile::tempdir().unwrap();
    let _home = EnvVarGuard::set("HOME", home.path().to_str().unwrap());
    let _enabled = EnvVarGuard::unset("VIBEWINDOW_OPEN_SKILLS_ENABLED");
    let _dir = EnvVarGuard::unset("VIBEWINDOW_OPEN_SKILLS_DIR");

    let repo = home.path().join("repo");
    let workspace = repo.join("apps").join("demo");
    fs::create_dir_all(repo.join(".git")).unwrap();
    fs::create_dir_all(&workspace).unwrap();

    write_skill(
        &repo.join(".vibewindow").join("skills").join("shared"),
        "---\ndescription: Ancestor description\n---\n# Shared\n",
    );
    write_skill(
        &workspace.join(".vibewindow").join("skills").join("shared"),
        "---\ndescription: Workspace description\n---\n# Shared\n",
    );
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(
            workspace.join(".vibewindow").join("skills"),
            workspace.join("skills"),
        )
        .unwrap();
        let workspace_sources = discover_local_skill_source_dirs(Some(&workspace))
            .into_iter()
            .filter(|source| source.kind == LocalSkillSourceKind::Workspace)
            .collect::<Vec<_>>();
        assert_eq!(workspace_sources.len(), 1, "real paths must only be discovered once");
    }
    write_skill(&workspace.join(".vibewindow").join("skills").join("disabled"), "# Disabled\n");
    fs::write(
        workspace.join(".vibewindow").join("skills").join("disabled").join("SKILL.disabled"),
        "",
    )
    .unwrap();

    let nested = workspace
        .join(".vibewindow")
        .join("skills")
        .join("collections")
        .join("rust")
        .join("nested");
    write_skill(&nested, "---\ndescription: Nested skill\npaths: [\"src/*.rs\"]\n---\n# Nested\n");

    let initial = load_skills(&workspace);
    let shared = initial.iter().find(|skill| skill.name == "shared").unwrap();
    assert_eq!(shared.description, "Workspace description");
    assert!(!initial.iter().any(|skill| skill.name == "disabled"));

    let nested = initial.iter().find(|skill| skill.name == "nested").unwrap();
    assert!(nested.applies_to_path(Path::new("src/lib.rs")));
    assert!(!nested.applies_to_path(Path::new("docs/readme.md")));

    write_skill(
        &workspace.join(".vibewindow").join("skills").join("collections").join("newly-added"),
        "---\ndescription: Added after first discovery\n---\n# New\n",
    );
    let rediscovered = load_skills(&workspace);
    assert!(rediscovered.iter().any(|skill| skill.name == "newly-added"));
}
