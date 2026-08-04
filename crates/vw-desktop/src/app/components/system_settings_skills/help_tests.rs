use super::help::{help_text, view_overlays};
use crate::app::{App, Message};
use iced::Element;
use iced::widget::text;

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn help_text_documents_skill_page_capabilities() {
    let text = help_text();

    for expected in
        ["项目目录 / 全部目录", "SKILL.md", "SKILL.toml", "启用、禁用、删除", "安装到当前项目目录"]
    {
        assert!(text.contains(expected), "help text should contain {expected}");
    }
}

#[test]
fn help_text_documents_configuration_fields_and_defaults() {
    let text = help_text();

    for expected in [
        "auto_load_project_skills",
        "open_skills_enabled",
        "open_skills_dir",
        "prompt_injection_mode",
        "$HOME/open-skills",
        "compact（默认）",
        "full：注入完整技能文本",
    ] {
        assert!(text.contains(expected), "help text should contain {expected}");
    }

    assert!(text.contains(
        r#""auto_load_project_skills": true,
    "open_skills_enabled": false,
    "open_skills_dir": null,
    "prompt_injection_mode": "compact""#
    ));
}

#[test]
fn help_text_documents_local_skill_disable_marker_behavior() {
    let text = help_text();

    assert!(text.contains("SKILL.disabled"));
    assert!(text.contains("禁用后仍会在目录页显示"));
    assert!(text.contains("运行时会跳过加载"));
}

#[test]
fn view_overlays_builds_hidden_and_visible_help_states() {
    let mut app = App::new().0;

    keep_element(view_overlays(&app, text("dialog without help").into()));

    app.skills_settings.show_help_modal = true;
    keep_element(view_overlays(&app, text("dialog behind help").into()));
}
