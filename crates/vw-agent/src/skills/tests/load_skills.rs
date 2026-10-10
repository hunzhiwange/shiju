//! Skill 加载流程的行为测试。
//!
//! 覆盖 TOML/Markdown 清单解析、缺失或无效 skill 的跳过逻辑，以及工作区、
//! 祖先目录和全局目录之间的发现优先级。测试保持在独立文件中，避免把加载
//! 逻辑和验证场景混在一起。

use super::super::*;
use super::helpers::{EnvVarGuard, open_skills_env_lock};
use crate::app::agent::config::Config;
use crate::app::agent::config::SkillsPromptInjectionMode;
use std::fs;
use std::path::PathBuf;
use vw_config_types::{paths::home_config_dir, skills::SkillsDirectoryProvider};

fn load_workspace_only(workspace_dir: &std::path::Path) -> Vec<Skill> {
    let _lock = open_skills_env_lock().lock().unwrap();
    let _enabled = EnvVarGuard::unset("VIBEWINDOW_OPEN_SKILLS_ENABLED");
    let _dir = EnvVarGuard::unset("VIBEWINDOW_OPEN_SKILLS_DIR");
    load_skills(workspace_dir)
}

#[test]
fn load_empty_skills_dir() {
    let dir = tempfile::tempdir().unwrap();
    let skills = load_workspace_only(dir.path());
    assert!(skills.is_empty());
}

#[test]
fn load_skill_from_toml() {
    let dir = tempfile::tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    let skill_dir = skills_dir.join("test-skill");
    fs::create_dir_all(&skill_dir).unwrap();

    fs::write(
        skill_dir.join("SKILL.toml"),
        r#"
[skill]
name = "test-skill"
description = "A test skill"
version = "1.0.0"
tags = ["test"]

[[tools]]
name = "hello"
description = "Says hello"
kind = "shell"
command = "echo hello"
"#,
    )
    .unwrap();

    let skills = load_workspace_only(dir.path());
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "test-skill");
    assert_eq!(skills[0].tools.len(), 1);
    assert_eq!(skills[0].tools[0].name, "hello");
}

#[test]
fn load_skill_from_md() {
    let dir = tempfile::tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    let skill_dir = skills_dir.join("md-skill");
    fs::create_dir_all(&skill_dir).unwrap();

    fs::write(skill_dir.join("SKILL.md"), "# My Skill\nThis skill does cool things.\n").unwrap();

    let skills = load_workspace_only(dir.path());
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "md-skill");
    assert!(skills[0].description.contains("cool things"));
}

#[test]
fn load_markdown_skill_prefers_frontmatter_description() {
    let dir = tempfile::tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    let skill_dir = skills_dir.join("frontmatter-skill");
    fs::create_dir_all(&skill_dir).unwrap();

    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: Fancy Skill\ndescription: Frontmatter wins\n---\n# Heading\nFallback line\n",
    )
    .unwrap();

    let skills = load_workspace_only(dir.path());
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "frontmatter-skill");
    assert_eq!(skills[0].description, "Frontmatter wins");
}

#[test]
fn load_nonexistent_dir() {
    let dir = tempfile::tempdir().unwrap();
    let fake = dir.path().join("nonexistent");
    let skills = load_workspace_only(&fake);
    assert!(skills.is_empty());
}

#[test]
fn load_ignores_files_in_skills_dir() {
    let dir = tempfile::tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();
    fs::write(skills_dir.join("not-a-skill.txt"), "hello").unwrap();
    let skills = load_workspace_only(dir.path());
    assert!(skills.is_empty());
}

#[test]
fn load_ignores_dir_without_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    let empty_skill = skills_dir.join("empty-skill");
    fs::create_dir_all(&empty_skill).unwrap();
    let skills = load_workspace_only(dir.path());
    assert!(skills.is_empty());
}

#[test]
fn load_multiple_skills() {
    let dir = tempfile::tempdir().unwrap();
    let skills_dir = dir.path().join("skills");

    for name in ["alpha", "beta", "gamma"] {
        let skill_dir = skills_dir.join(name);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), format!("# {name}\nSkill {name} description.\n"))
            .unwrap();
    }

    let skills = load_workspace_only(dir.path());
    for name in ["alpha", "beta", "gamma"] {
        assert!(skills.iter().any(|skill| skill.name == name));
    }
}

#[test]
fn toml_skill_with_multiple_tools() {
    let dir = tempfile::tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    let skill_dir = skills_dir.join("multi-tool");
    fs::create_dir_all(&skill_dir).unwrap();

    fs::write(
        skill_dir.join("SKILL.toml"),
        r#"
[skill]
name = "multi-tool"
description = "Has many tools"
version = "2.0.0"
author = "tester"
tags = ["automation", "devops"]

[[tools]]
name = "build"
description = "Build the project"
kind = "shell"
command = "cargo build"

[[tools]]
name = "test"
description = "Run tests"
kind = "shell"
command = "cargo test"

[[tools]]
name = "deploy"
description = "Deploy via HTTP"
kind = "http"
command = "https://api.example.com/deploy"
"#,
    )
    .unwrap();

    let skills = load_workspace_only(dir.path());
    let s = skills
        .iter()
        .find(|skill| skill.name == "multi-tool")
        .expect("multi-tool skill should load");
    assert_eq!(s.name, "multi-tool");
    assert_eq!(s.version, "2.0.0");
    assert_eq!(s.author.as_deref(), Some("tester"));
    assert_eq!(s.tags, vec!["automation", "devops"]);
    assert_eq!(s.tools.len(), 3);
    assert_eq!(s.tools[0].name, "build");
    assert_eq!(s.tools[1].kind, "shell");
    assert_eq!(s.tools[2].kind, "http");
}

#[test]
fn toml_skill_minimal() {
    let dir = tempfile::tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    let skill_dir = skills_dir.join("minimal");
    fs::create_dir_all(&skill_dir).unwrap();

    fs::write(
        skill_dir.join("SKILL.toml"),
        r#"
[skill]
name = "minimal"
description = "Bare minimum"
"#,
    )
    .unwrap();

    let skills = load_workspace_only(dir.path());
    let skill =
        skills.iter().find(|skill| skill.name == "minimal").expect("minimal skill should load");
    assert_eq!(skill.version, "0.1.0");
    assert!(skill.author.is_none());
    assert!(skill.tags.is_empty());
    assert!(skill.tools.is_empty());
}

#[test]
fn toml_skill_invalid_syntax_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    let skill_dir = skills_dir.join("broken");
    fs::create_dir_all(&skill_dir).unwrap();

    fs::write(skill_dir.join("SKILL.toml"), "this is not valid toml {{{{").unwrap();

    let skills = load_workspace_only(dir.path());
    assert!(!skills.iter().any(|skill| skill.name == "broken"));
}

#[test]
fn md_skill_heading_only() {
    let dir = tempfile::tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    let skill_dir = skills_dir.join("heading-only");
    fs::create_dir_all(&skill_dir).unwrap();

    fs::write(skill_dir.join("SKILL.md"), "# Just a Heading\n").unwrap();

    let skills = load_workspace_only(dir.path());
    let skill = skills
        .iter()
        .find(|skill| skill.name == "heading-only")
        .expect("heading-only skill should load");
    assert_eq!(skill.description, "No description");
}

#[test]
fn toml_prefers_over_md() {
    let dir = tempfile::tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    let skill_dir = skills_dir.join("dual");
    fs::create_dir_all(&skill_dir).unwrap();

    fs::write(
        skill_dir.join("SKILL.toml"),
        "[skill]\nname = \"from-toml\"\ndescription = \"TOML wins\"\n",
    )
    .unwrap();
    fs::write(skill_dir.join("SKILL.md"), "# From MD\nMD description\n").unwrap();

    let skills = load_workspace_only(dir.path());
    assert!(skills.iter().any(|skill| skill.name == "from-toml"));
    assert!(!skills.iter().any(|skill| skill.name == "From MD"));
}

#[test]
fn skills_dir_path() {
    let base = std::path::Path::new("/home/user/.vibewindow");
    let dir = skills_dir(base);
    assert_eq!(dir, PathBuf::from("/home/user/.vibewindow/skills"));
}

#[test]
fn load_skills_discovers_workspace_ancestor_and_global_sources() {
    // HOME 是进程级环境变量，测试需要串行化，避免并发测试互相污染
    // skill 搜索路径。
    let _lock = open_skills_env_lock().lock().unwrap();
    let home = tempfile::tempdir().unwrap();
    let _guard = EnvVarGuard::set("HOME", home.path().to_str().unwrap());

    let repo_root = home.path().join("repo-root");
    let project_dir = repo_root.join("apps").join("demo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(&project_dir).unwrap();

    let global_skill = home_config_dir(home.path()).join("skills").join("global-skill");
    fs::create_dir_all(&global_skill).unwrap();
    fs::write(global_skill.join("SKILL.md"), "# Global\nGlobal skill\n").unwrap();

    let plain_global_skill = home.path().join(".skills").join("plain-global-skill");
    fs::create_dir_all(&plain_global_skill).unwrap();
    fs::write(plain_global_skill.join("SKILL.md"), "# Plain Global\nPlain global skill\n").unwrap();

    let parent_skill = repo_root.join(".vibewindow").join("skills").join("parent-skill");
    fs::create_dir_all(&parent_skill).unwrap();
    fs::write(parent_skill.join("SKILL.md"), "# Parent\nParent skill\n").unwrap();

    let parent_plain_skill = repo_root.join("skills").join("parent-plain-skill");
    fs::create_dir_all(&parent_plain_skill).unwrap();
    fs::write(parent_plain_skill.join("SKILL.md"), "# Parent Plain\nParent plain skill\n").unwrap();

    let parent_agents_skill = repo_root.join(".agents").join("skills").join("parent-agents-skill");
    fs::create_dir_all(&parent_agents_skill).unwrap();
    fs::write(parent_agents_skill.join("SKILL.md"), "# Parent Agents\nParent agents skill\n")
        .unwrap();

    let hidden_skill = project_dir.join(".vibewindow").join("skills").join("hidden-skill");
    fs::create_dir_all(&hidden_skill).unwrap();
    fs::write(hidden_skill.join("SKILL.md"), "# Hidden\nHidden skill\n").unwrap();

    let legacy_skill = project_dir.join("skills").join("legacy-skill");
    fs::create_dir_all(&legacy_skill).unwrap();
    fs::write(legacy_skill.join("SKILL.md"), "# Legacy\nLegacy skill\n").unwrap();

    let agents_skill = project_dir.join(".agents").join("skills").join("agents-skill");
    fs::create_dir_all(&agents_skill).unwrap();
    fs::write(agents_skill.join("SKILL.md"), "# Agents\nAgents skill\n").unwrap();

    let global_shared = home_config_dir(home.path()).join("skills").join("shared-skill");
    fs::create_dir_all(&global_shared).unwrap();
    fs::write(global_shared.join("SKILL.md"), "# Shared\nGlobal shared\n").unwrap();

    let workspace_shared = project_dir.join("skills").join("shared-skill");
    fs::create_dir_all(&workspace_shared).unwrap();
    fs::write(workspace_shared.join("SKILL.md"), "# Shared\nWorkspace wins\n").unwrap();

    let mut skills = load_skills(&project_dir);
    skills.sort_by(|left, right| left.name.cmp(&right.name));

    // 同名 skill 以更靠近当前工作区的来源为准，避免全局配置覆盖项目内
    // 明确提供的能力。
    let names = skills.iter().map(|skill| skill.name.as_str()).collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "agents-skill",
            "global-skill",
            "hidden-skill",
            "legacy-skill",
            "parent-agents-skill",
            "parent-plain-skill",
            "parent-skill",
            "plain-global-skill",
            "shared-skill",
        ]
    );

    let shared = skills.iter().find(|skill| skill.name == "shared-skill").unwrap();
    assert_eq!(shared.description, "Workspace wins");
}

#[test]
fn common_project_skill_roots_load_without_selecting_codex_provider() {
    let _lock = open_skills_env_lock().lock().unwrap();
    let home = tempfile::tempdir().unwrap();
    let _guard = EnvVarGuard::set("HOME", home.path().to_str().unwrap());

    let project_dir = home.path().join("project");
    fs::create_dir_all(project_dir.join(".git")).unwrap();
    for (relative_root, skill_name) in
        [("skills", "plain-skill"), (".agents/skills", "agents-skill")]
    {
        let skill_dir = project_dir.join(relative_root).join(skill_name);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), format!("# {skill_name}\n")).unwrap();
    }

    let mut config = Config::default();
    config.skills.directory_provider = SkillsDirectoryProvider::Claude;

    let names = load_skills_with_config(&project_dir, &config)
        .into_iter()
        .map(|skill| skill.name)
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["plain-skill", "agents-skill"]);
}

#[test]
fn load_skills_uses_configured_directory_provider() {
    let _lock = open_skills_env_lock().lock().unwrap();
    let home = tempfile::tempdir().unwrap();
    let _guard = EnvVarGuard::set("HOME", home.path().to_str().unwrap());
    let _enabled = EnvVarGuard::unset("VIBEWINDOW_OPEN_SKILLS_ENABLED");
    let _dir = EnvVarGuard::unset("VIBEWINDOW_OPEN_SKILLS_DIR");

    let project_dir = home.path().join("project");
    fs::create_dir_all(&project_dir).unwrap();

    let vibewindow_skill = project_dir.join(".vibewindow").join("skills").join("vw-skill");
    fs::create_dir_all(&vibewindow_skill).unwrap();
    fs::write(vibewindow_skill.join("SKILL.md"), "# VibeWindow\nVibeWindow skill\n").unwrap();

    let codex_skill = project_dir.join(".codex").join("skills").join("codex-skill");
    fs::create_dir_all(&codex_skill).unwrap();
    fs::write(codex_skill.join("SKILL.md"), "# Codex\nCodex skill\n").unwrap();

    let mut config = Config::default();
    config.skills.directory_provider = SkillsDirectoryProvider::Codex;

    let names = load_skills_with_config(&project_dir, &config)
        .iter()
        .map(|skill| skill.name.clone())
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["codex-skill"]);
}

#[test]
fn workspace_skill_roots_follow_directory_provider_contract() {
    let workspace = std::path::Path::new("/workspace");

    assert_eq!(
        workspace_skills_dir(workspace, SkillsDirectoryProvider::Vibewindow),
        PathBuf::from("/workspace/.vibewindow/skills")
    );
    assert_eq!(
        workspace_skills_dir(workspace, SkillsDirectoryProvider::Codex),
        PathBuf::from("/workspace/.codex/skills")
    );
    assert_eq!(
        workspace_skills_dir(workspace, SkillsDirectoryProvider::Claude),
        PathBuf::from("/workspace/.claude/skills")
    );
    assert_eq!(
        workspace_skills_dir(workspace, SkillsDirectoryProvider::Cursor),
        PathBuf::from("/workspace/.cursor/skills")
    );
}

#[test]
fn configured_prompt_mode_controls_loaded_skill_detail() {
    let _lock = open_skills_env_lock().lock().unwrap();
    let _enabled = EnvVarGuard::unset("VIBEWINDOW_OPEN_SKILLS_ENABLED");
    let _dir = EnvVarGuard::unset("VIBEWINDOW_OPEN_SKILLS_DIR");

    let dir = tempfile::tempdir().unwrap();
    let skill_dir = dir.path().join(".vibewindow").join("skills").join("detailed");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.toml"),
        r#"
        prompts = ["Full instructions"]

        [skill]
        name = "detailed"
        description = "Detailed skill"

        [[tools]]
        name = "run"
        description = "Run it"
        kind = "shell"
        command = "echo run"
        "#,
    )
    .unwrap();

    let mut config = Config::default();
    config.skills.prompt_injection_mode = SkillsPromptInjectionMode::Compact;

    let compact = load_skills_with_config(dir.path(), &config);
    assert_eq!(compact.len(), 1);
    assert!(compact[0].tools.is_empty());
    assert!(compact[0].prompts.is_empty());

    let full = load_skills_full_with_config(dir.path(), &config);
    assert_eq!(full.len(), 1);
    assert_eq!(full[0].tools.len(), 1);
    assert_eq!(full[0].prompts, vec!["Full instructions"]);
}
