use super::skills::{
    discovery_order_text, enabled_skill_ids, scope_description, scope_source_matches,
    skill_card_button_style, skill_source_label, view,
};
use crate::app::state::{SkillsCatalogItem, SkillsCatalogKind, SkillsDirectoryScope};
use iced::advanced::widget::Tree;
use iced::widget::button;
use iced::{Background, Theme};

#[test]
fn skill_source_label_maps_known_and_unknown_sources() {
    assert_eq!(skill_source_label("workspace"), "项目");
    assert_eq!(skill_source_label("ancestor"), "父级");
    assert_eq!(skill_source_label("global"), "全局");
    assert_eq!(skill_source_label("bundled"), "内置");
    assert_eq!(skill_source_label("remote"), "其他");
}

#[test]
fn scope_source_matches_each_directory_scope() {
    assert!(scope_source_matches(SkillsDirectoryScope::Project, "workspace"));
    assert!(!scope_source_matches(SkillsDirectoryScope::Project, "global"));
    assert!(scope_source_matches(SkillsDirectoryScope::Ancestor, "ancestor"));
    assert!(scope_source_matches(SkillsDirectoryScope::Global, "global"));
    assert!(scope_source_matches(SkillsDirectoryScope::Bundled, "bundled"));
    assert!(scope_source_matches(SkillsDirectoryScope::All, "anything"));
}

#[test]
fn scope_description_covers_all_scopes() {
    for scope in [
        SkillsDirectoryScope::Project,
        SkillsDirectoryScope::Ancestor,
        SkillsDirectoryScope::Global,
        SkillsDirectoryScope::Bundled,
        SkillsDirectoryScope::All,
    ] {
        assert!(!scope_description(scope).is_empty());
    }
}

#[test]
fn enabled_skill_ids_filters_disabled_items_preserving_order() {
    let enabled = skill("a", true, "workspace");
    let disabled = skill("b", false, "global");
    let also_enabled = skill("c", true, "bundled");
    let items = vec![&enabled, &disabled, &also_enabled];

    assert_eq!(enabled_skill_ids(&items), vec!["a".to_string(), "c".to_string()]);
}

#[test]
fn skill_card_button_style_covers_selected_hovered_and_pressed_states() {
    let selected = skill_card_button_style(&Theme::Light, button::Status::Active, true);
    let hovered = skill_card_button_style(&Theme::Light, button::Status::Hovered, false);
    let pressed_dark = skill_card_button_style(&Theme::Dark, button::Status::Pressed, false);

    assert!(matches!(selected.background, Some(Background::Color(_))));
    assert!(matches!(hovered.background, Some(Background::Color(_))));
    assert!(matches!(pressed_dark.background, Some(Background::Color(_))));
    assert_eq!(selected.border.radius.top_left, 16.0);
}

#[test]
fn discovery_order_text_distinguishes_open_project_and_global_context() {
    let (mut app, _) = crate::app::App::new();
    let global_order = discovery_order_text(&app);

    assert!(global_order.ends_with(" -> 内置技能"));
    assert!(!global_order.contains("父级 skills、.agents/skills"));

    app.project_path = Some("/workspace/demo".to_string());
    let project_order = discovery_order_text(&app);

    assert!(project_order.starts_with("/workspace/demo/skills"));
    assert!(
        project_order.contains("/workspace/demo/.agents/skills -> 父级 skills、.agents/skills")
    );
}

#[test]
fn skills_view_renders_empty_loading_and_filtered_catalog_states() {
    let (mut app, _) = crate::app::App::new();
    let entry = app.agents_settings.entries[0].clone();
    app.skills_settings.directory_scope = SkillsDirectoryScope::Project;
    app.skills_settings.loading = true;

    let loading_tree = Tree::new(view(&app, &entry).as_widget());
    assert_eq!(loading_tree.children.len(), 3);
    assert_eq!(loading_tree.children[2].children.len(), 2);

    app.skills_settings.loading = false;
    app.skills_settings.catalog = vec![
        skill("project-enabled", true, "workspace"),
        skill("project-disabled", false, "workspace"),
        skill("global", true, "global"),
    ];
    app.skills_settings.directory_scope = SkillsDirectoryScope::All;

    let catalog_tree = Tree::new(view(&app, &entry).as_widget());
    assert_eq!(catalog_tree.children.len(), 3);
    assert_eq!(catalog_tree.children[2].children.len(), 2);
    assert_eq!(catalog_tree.children[2].children[1].children.len(), 3);
}

#[test]
fn unselected_skill_card_style_covers_dark_hover_and_light_press() {
    let dark_hovered = skill_card_button_style(&Theme::Dark, button::Status::Hovered, false);
    let light_pressed = skill_card_button_style(&Theme::Light, button::Status::Pressed, false);
    let dark_active = skill_card_button_style(&Theme::Dark, button::Status::Active, false);

    assert!(matches!(dark_hovered.background, Some(Background::Color(_))));
    assert!(matches!(light_pressed.background, Some(Background::Color(_))));
    assert!(matches!(dark_active.background, Some(Background::Color(_))));
}

fn skill(id: &str, enabled: bool, source: &str) -> SkillsCatalogItem {
    SkillsCatalogItem {
        id: id.to_string(),
        title: id.to_string(),
        description: format!("{id} description"),
        kind: SkillsCatalogKind::System,
        resource_count: 0,
        installed: true,
        enabled,
        source: source.to_string(),
        source_path: None,
    }
}
