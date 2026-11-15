use crate::help::help_text;
#[test]
fn help_text_documents_skill_page_capabilities() {
    let text = help_text(config_ui::DesktopLocale::SimplifiedChinese);

    for expected in
        ["项目目录 / 全部目录", "SKILL.md", "SKILL.toml", "启用、禁用、删除", "安装到当前项目目录"]
    {
        assert!(text.contains(expected), "help text should contain {expected}");
    }
}

#[test]
fn help_text_documents_configuration_fields_and_defaults() {
    let text = help_text(config_ui::DesktopLocale::SimplifiedChinese);

    for expected in [
        "auto_load_project_skills",
        "prompt_injection_mode",
        "compact（默认）",
        "full：注入完整技能文本",
    ] {
        assert!(text.contains(expected), "help text should contain {expected}");
    }

    assert!(text.contains(
        r#""auto_load_project_skills": true,
    "prompt_injection_mode": "compact""#
    ));

    assert!(!text.contains("社区仓库"));
    assert!(!text.contains("仓库目录"));
}

#[test]
fn help_text_documents_local_skill_disable_marker_behavior() {
    let text = help_text(config_ui::DesktopLocale::SimplifiedChinese);

    assert!(text.contains("SKILL.disabled"));
    assert!(text.contains("禁用后仍会在目录页显示"));
    assert!(text.contains("运行时会跳过加载"));
}
