//! 验证代理技能列表、筛选、启停状态与操作消息。
//! 未安装、禁用和搜索无结果状态用于确认操作按钮不会越权出现。

use super::{discovery_order_text, skills_view as view};
use crate::skills::skill_card_button_style;
use crate::state::{SkillsCatalogItem, SkillsDirectoryScope};
use desktop_settings_skills::types::SkillsCatalogKind;
use iced::advanced::widget::Tree;
use iced::widget::button;
use iced::{Background, Theme};

#[test]
fn discovery_order_text_distinguishes_open_project_and_global_context() {
    let mut app = crate::agents_state::AgentsSettingsState::default();
    let global_order = discovery_order_text(&app);

    assert!(!global_order.contains("内置技能"));
    assert!(!global_order.contains("父级 skills、.agents/skills"));

    app.skill_directory = Some("/workspace/demo".to_string());
    let project_order = discovery_order_text(&app);

    assert!(project_order.starts_with("发现顺序：/workspace/demo/skills"));
    assert!(
        project_order.contains("/workspace/demo/.agents/skills -> 父级 skills、.agents/skills")
    );
}

#[test]
fn skills_view_renders_empty_loading_and_filtered_catalog_states() {
    let mut app = crate::agents_state::AgentsSettingsState::default();
    let entry = app.entries[0].clone();
    app.skill_directory_scope = SkillsDirectoryScope::Project;
    app.skill_catalog_loading = true;

    let loading_tree = Tree::new(view(&app, &entry).as_widget());
    assert_eq!(loading_tree.children.len(), 4);
    assert_eq!(loading_tree.children[3].children.len(), 2);

    app.skill_catalog_loading = false;
    app.skill_catalog = vec![
        skill("project-enabled", true, "workspace"),
        skill("project-disabled", false, "workspace"),
        skill("global", true, "global"),
    ];
    app.skill_directory_scope = SkillsDirectoryScope::All;

    let catalog_tree = Tree::new(view(&app, &entry).as_widget());
    assert_eq!(catalog_tree.children.len(), 4);
    assert_eq!(catalog_tree.children[3].children.len(), 2);
    assert_eq!(catalog_tree.children[3].children[1].children.len(), 3);
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

#[test]
fn agent_config_cleanup_skills_inherit_and_fixed_are_distinct() {
    use config_agent_profile::AgentDefault;
    let mut app = crate::agents_state::AgentsSettingsState::default();
    app.skill_catalog = vec![skill("allowed", true, "global"), skill("other", true, "global")];
    let mut entry = app.entries[0].clone();
    entry.allowed_skills = vec!["allowed".into()];
    entry.defaults.skills = AgentDefault::Inherit;
    assert_eq!(crate::skills::selected_skill_ids(&entry), &["allowed"]);
    entry.defaults.skills = AgentDefault::Fixed(vec![]);
    assert!(crate::skills::selected_skill_ids(&entry).is_empty());
    entry.defaults.skills = AgentDefault::Fixed(vec!["other".into()]);
    assert_eq!(crate::skills::selected_skill_ids(&entry), &["other"]);
}
