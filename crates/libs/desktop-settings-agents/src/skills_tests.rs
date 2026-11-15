//! 验证代理技能列表、筛选、启停状态与操作消息。
//! 未安装、禁用和搜索无结果状态用于确认操作按钮不会越权出现。

use super::skills::{
    agent_skill_matches_scope, agent_skills_scope, capability_policy_label,
    capability_policy_tooltip, enabled_skill_ids, scope_description,
    scope_source_matches, skill_card_button_style, skill_policy_message, skill_source_label,
};
use crate::message::Event as Message;
use crate::message::AgentsMessage;
use desktop_settings_skills::types::{SkillsCatalogItem, SkillsCatalogKind, SkillsDirectoryScope};
use iced::widget::button;
use iced::{Background, Theme};
use config_agent::DelegateCapabilityPolicy;

#[test]
fn skill_source_label_maps_known_and_unknown_sources() {
    assert_eq!(skill_source_label("workspace"), "项目");
    assert_eq!(skill_source_label("ancestor"), "父级");
    assert_eq!(skill_source_label("global"), "全局");
    assert_eq!(skill_source_label("bundled"), "待安装");
    assert_eq!(skill_source_label("remote"), "其他");
}

#[test]
fn skill_capability_policy_controls_use_typed_messages_and_expected_labels() {
    assert_eq!(capability_policy_label(DelegateCapabilityPolicy::Strict), "严格限制");
    assert_eq!(capability_policy_label(DelegateCapabilityPolicy::Unrestricted), "不限制");
    assert!(matches!(
        skill_policy_message("main".to_string(), DelegateCapabilityPolicy::Unrestricted),
        Message::Agents(AgentsMessage::SkillPolicyChanged(
            ref agent,
            DelegateCapabilityPolicy::Unrestricted,
        )) if agent == "main"
    ));
}

#[test]
fn skill_capability_policy_tooltips_distinguish_fixed_and_temporary_selection() {
    assert!(capability_policy_tooltip(DelegateCapabilityPolicy::Strict).contains("不允许"));
    assert!(capability_policy_tooltip(DelegateCapabilityPolicy::Unrestricted).contains("临时选择"));
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
fn agent_skill_filter_excludes_uninstalled_catalog_candidates() {
    let installed = skill("installed", true, "global");
    let mut candidate = skill("candidate", false, "bundled");
    candidate.installed = false;

    assert!(agent_skill_matches_scope(&installed, SkillsDirectoryScope::All));
    assert!(!agent_skill_matches_scope(&candidate, SkillsDirectoryScope::All));
    assert_eq!(agent_skills_scope(SkillsDirectoryScope::Bundled), SkillsDirectoryScope::All);
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
