use crate::browser::{
    scope_description, scope_source_matches, scope_title,
};
use crate::types::SkillsDirectoryScope;
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
fn scope_descriptions_include_standard_paths() {
    let global = format!("~/{}/skills, ~/.skills", config_runtime_paths::HOME_CONFIG_DIR_NAME);
    let global_description = format!("查看当前用户 {global} 中的全局技能。");
    let cases = [
        (
            SkillsDirectoryScope::Project,
            "查看当前项目 .shiju-harness/skills, skills, .agents/skills 目录命中的技能。",
        ),
        (
            SkillsDirectoryScope::Ancestor,
            "查看从项目父级向上发现的 .shiju-harness/skills, skills, .agents/skills 技能。",
        ),
        (
            SkillsDirectoryScope::Global,
            global_description.as_str(),
        ),
        (SkillsDirectoryScope::Bundled, "查看随程序提供并默认可用的内置技能。"),
        (
            SkillsDirectoryScope::All,
            "全部结果包含当前项目、父级目录、全局目录以及内置技能，便于核对最终命中来源。",
        ),
    ];

    for (scope, expected) in cases {
        assert_eq!(
            scope_description(config_ui::DesktopLocale::SimplifiedChinese, scope),
            expected
        );
    }
}
