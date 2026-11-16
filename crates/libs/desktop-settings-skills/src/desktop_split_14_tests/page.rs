use super::browser::help;
use crate::page::*;
use crate::types::SkillsSettingsTab;
use config_skills::{SkillsDirectoryProvider, SkillsPromptInjectionMode};
use iced::{Element, advanced::widget::Tree, widget::text};
fn context() -> ConfigContext<'static> {
    ConfigContext {
        help: help::<Message>,
        desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
        app_theme: &iced::Theme::Dark,
        directory_provider: SkillsDirectoryProvider::Vibewindow,
        auto_load_project_skills: true,
        prompt_injection_mode: SkillsPromptInjectionMode::Compact,
    }
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
    keep_element(field_row(context(), "字段", "字段说明", text("字段值")));

    for (label, tab) in skills_tab_labels() {
        let inactive_tab = if *tab == SkillsSettingsTab::SystemConfig {
            SkillsSettingsTab::Skills
        } else {
            SkillsSettingsTab::SystemConfig
        };
        keep_element(skills_tab_button(label, *tab, *tab));
        keep_element(skills_tab_button(label, *tab, inactive_tab));
    }
}

#[test]
fn system_config_view_builds_default_and_populated_settings() {
    let app = context();
    keep_element(system_config_view(app));

    let mut app = context();
    app.auto_load_project_skills = false;
    app.prompt_injection_mode = SkillsPromptInjectionMode::Full;

    for provider in SkillsDirectoryProvider::ALL {
        app.directory_provider = provider;
        keep_element(system_config_view(app));
    }
}

#[test]
fn system_config_view_keeps_three_config_rows_in_one_panel() {
    let app = context();
    let element = system_config_view(app);
    let tree = Tree::new(element.as_widget());

    assert_eq!(tree.children.len(), 2);
    // The field rows have controls; separators have no children.
    let fields = tree.children[1].children.iter().filter(|child| !child.children.is_empty());
    assert_eq!(fields.count(), 3);
}
