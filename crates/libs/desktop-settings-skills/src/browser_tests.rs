use crate::browser::{
    provider_global_paths, provider_project_paths, scope_description, scope_source_matches,
    scope_title,
};
use crate::types::SkillsDirectoryScope;
use config_skills::SkillsDirectoryProvider;
#[test]
fn scope_source_matching_is_explicit_for_each_scope() {
    assert!(scope_source_matches(SkillsDirectoryScope::Project, "workspace"));
    assert!(scope_source_matches(SkillsDirectoryScope::Ancestor, "ancestor"));
    assert!(scope_source_matches(SkillsDirectoryScope::Global, "global"));
    assert!(scope_source_matches(SkillsDirectoryScope::Bundled, "bundled"));
    assert!(scope_source_matches(SkillsDirectoryScope::All, "anything"));

    assert!(!scope_source_matches(SkillsDirectoryScope::Project, "global"));
    assert!(!scope_source_matches(SkillsDirectoryScope::Bundled, "workspace"));
}
#[test]
fn scope_titles_cover_all_scopes() {
    let cases = [
        (SkillsDirectoryScope::Project, "项目目录技能"),
        (SkillsDirectoryScope::Ancestor, "父级目录技能"),
        (SkillsDirectoryScope::Global, "全局目录技能"),
        (SkillsDirectoryScope::Bundled, "内置技能"),
        (SkillsDirectoryScope::All, "全部技能目录"),
    ];

    for (scope, title) in cases {
        assert_eq!(scope_title(scope), title);
    }
}
#[test]
fn provider_paths_match_directory_provider_contract() {
    let vibewindow_global =
        format!("~/{}/skills, ~/.skills", config_runtime_paths::HOME_CONFIG_DIR_NAME);
    let cases = [
        (
            SkillsDirectoryProvider::Vibewindow,
            ".shiju/skills, skills, .agents/skills",
            vibewindow_global.as_str(),
        ),
        (
            SkillsDirectoryProvider::Codex,
            ".codex/skills, .agents/skills, skills",
            "~/.codex/skills, ~/.agents/skills",
        ),
        (
            SkillsDirectoryProvider::Claude,
            ".claude/skills, skills, .agents/skills",
            "~/.claude/skills",
        ),
        (
            SkillsDirectoryProvider::Cursor,
            ".cursor/skills, skills, .agents/skills",
            "~/.cursor/skills",
        ),
    ];

    for (provider, project, global) in cases {
        assert_eq!(provider_project_paths(provider), project);
        assert_eq!(provider_global_paths(provider), global);
    }
}
#[test]
fn scope_descriptions_include_provider_specific_paths() {
    assert!(
        scope_description(
            config_ui::DesktopLocale::SimplifiedChinese,
            SkillsDirectoryScope::Project,
            SkillsDirectoryProvider::Codex
        )
        .contains(".codex/skills")
    );
    assert!(
        scope_description(
            config_ui::DesktopLocale::SimplifiedChinese,
            SkillsDirectoryScope::Ancestor,
            SkillsDirectoryProvider::Claude
        )
        .contains(".claude/skills")
    );
    assert!(
        scope_description(
            config_ui::DesktopLocale::SimplifiedChinese,
            SkillsDirectoryScope::Global,
            SkillsDirectoryProvider::Cursor
        )
        .contains("~/.cursor/skills")
    );
    assert!(
        scope_description(
            config_ui::DesktopLocale::SimplifiedChinese,
            SkillsDirectoryScope::Bundled,
            SkillsDirectoryProvider::Vibewindow
        )
        .contains("内置技能")
    );
    assert!(
        scope_description(
            config_ui::DesktopLocale::SimplifiedChinese,
            SkillsDirectoryScope::All,
            SkillsDirectoryProvider::Vibewindow
        )
        .contains("全部")
    );
}
#[test]
fn scope_descriptions_are_exact_for_each_codex_scope() {
    let provider = SkillsDirectoryProvider::Codex;
    let cases = [
        (
            SkillsDirectoryScope::Project,
            "查看当前项目 .codex/skills, .agents/skills, skills 目录命中的技能。",
        ),
        (
            SkillsDirectoryScope::Ancestor,
            "查看从项目父级向上发现的 .codex/skills, .agents/skills, skills 技能。",
        ),
        (
            SkillsDirectoryScope::Global,
            "查看当前用户 ~/.codex/skills, ~/.agents/skills 中的全局技能。",
        ),
        (SkillsDirectoryScope::Bundled, "查看产品随包提供、可安装到项目的内置技能。"),
        (
            SkillsDirectoryScope::All,
            "全部结果包含当前项目、父级目录、全局目录以及内置技能，便于核对最终命中来源。",
        ),
    ];

    for (scope, expected) in cases {
        assert_eq!(
            scope_description(config_ui::DesktopLocale::SimplifiedChinese, scope, provider),
            expected
        );
    }
}
