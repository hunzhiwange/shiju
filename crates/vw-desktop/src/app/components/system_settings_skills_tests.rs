use super::*;
use crate::app::{App, Message};
use iced::Element;
use iced::advanced::widget::Tree;
use iced::widget::text;

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn skills_tab_labels_keep_expected_order_and_unique_tabs() {
    let labels = skills_tab_labels();

    assert_eq!(
        labels,
        &[
            ("技能", SkillsSettingsTab::Skills),
            ("顺序", SkillsSettingsTab::DiscoveryOrder),
            ("系统配置", SkillsSettingsTab::SystemConfig),
        ]
    );
    for (_, expected_tab) in labels {
        assert_eq!(labels.iter().filter(|(_, tab)| tab == expected_tab).count(), 1);
    }
}

#[test]
fn field_row_and_tab_buttons_build_active_and_inactive_states() {
    keep_element(field_row("字段", "字段说明", text("字段值")));

    for (label, tab) in skills_tab_labels() {
        let inactive_tab = if *tab == SkillsSettingsTab::SystemConfig {
            SkillsSettingsTab::Skills
        } else {
            SkillsSettingsTab::SystemConfig
        };
        keep_element(settings_tab_button(label, *tab, *tab));
        keep_element(settings_tab_button(label, *tab, inactive_tab));
    }
}

#[test]
fn system_config_view_builds_default_and_populated_settings() {
    let app = test_app();
    keep_element(system_config_view(&app));

    let mut app = test_app();
    app.skills_settings.auto_load_project_skills = false;
    app.skills_settings.open_skills_enabled = true;
    app.skills_settings.open_skills_dir_input = "/tmp/open-skills-fixture".to_string();
    app.skills_settings.prompt_injection_mode = SkillsPromptInjectionMode::Full;

    for provider in SkillsDirectoryProvider::ALL {
        app.skills_settings.directory_provider = provider;
        keep_element(system_config_view(&app));
    }
}

#[test]
fn system_config_view_keeps_five_config_rows_in_one_panel() {
    let app = test_app();
    let element = system_config_view(&app);
    let tree = Tree::new(element.as_widget());

    assert_eq!(tree.children.len(), 2);
    assert_eq!(tree.children[1].children.len(), 5);
}

#[test]
fn view_builds_every_skills_settings_tab() {
    let mut app = test_app();

    for (_, tab) in skills_tab_labels() {
        app.skills_settings.active_tab = *tab;
        keep_element(view(&app));
        assert_eq!(app.skills_settings.active_tab, *tab);
    }
}

#[test]
fn view_overlays_builds_with_help_closed_and_open() {
    let app = test_app();
    keep_element(view_overlays(&app, text("基础对话框").into()));

    let mut app = test_app();
    app.skills_settings.show_help_modal = true;
    keep_element(view_overlays(&app, text("基础对话框").into()));
}
