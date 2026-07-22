// Tests for plan6 task 820.
const SOURCE: &str = include_str!("catalog.rs");

use super::catalog::{
    catalog_group_section, catalog_item, catalog_matches_query, catalog_skill_initials,
    section_card_style, section_copy, skill_badge, source_label,
};
use crate::app::state::{
    SkillsCatalogItem as CatalogSkillMeta, SkillsCatalogKind as CatalogSkillKind,
};

fn skill(
    id: &str,
    title: &str,
    description: &str,
    source: &str,
    source_path: Option<&str>,
) -> CatalogSkillMeta {
    CatalogSkillMeta {
        id: id.to_string(),
        title: title.to_string(),
        description: description.to_string(),
        kind: CatalogSkillKind::Personal,
        resource_count: 0,
        installed: true,
        enabled: true,
        source: source.to_string(),
        source_path: source_path.map(ToOwned::to_owned),
    }
}

fn source_declares_symbol(name: &str) -> bool {
    let needles = [
        format!("fn {name}"),
        format!("pub fn {name}"),
        format!("struct {name}"),
        format!("pub struct {name}"),
        format!("enum {name}"),
        format!("pub enum {name}"),
        format!("type {name}"),
        format!("pub type {name}"),
        format!("const {name}"),
        format!("pub const {name}"),
        format!("static {name}"),
        format!("pub static {name}"),
        format!("impl {name}"),
    ];

    needles.iter().any(|needle| SOURCE.contains(needle))
}

fn skill_with_kind(
    id: &str,
    title: &str,
    description: &str,
    source: &str,
    source_path: Option<&str>,
    kind: CatalogSkillKind,
    resource_count: usize,
    installed: bool,
    enabled: bool,
) -> CatalogSkillMeta {
    CatalogSkillMeta {
        id: id.to_string(),
        title: title.to_string(),
        description: description.to_string(),
        kind,
        resource_count,
        installed,
        enabled,
        source: source.to_string(),
        source_path: source_path.map(ToOwned::to_owned),
    }
}

#[test]
fn catalog_matches_query_searches_all_visible_fields_case_insensitively() {
    let item = skill(
        "rust-helper",
        "Code Review",
        "Finds risky changes",
        "workspace",
        Some("/repo/.vibewindow/skills/review"),
    );

    for query in ["", "RUST", "review", "RISKY", "WORKSPACE", "VIBEWINDOW"] {
        assert!(catalog_matches_query(&item, query), "query should match: {query}");
    }

    assert!(!catalog_matches_query(&item, "browser"));
}

#[test]
fn catalog_matches_query_handles_missing_source_path() {
    let item = skill("id", "Title", "Description", "bundled", None);

    assert!(catalog_matches_query(&item, "bundle"));
    assert!(!catalog_matches_query(&item, "missing/path"));
}

#[test]
fn catalog_skill_initials_prefers_title_words_and_falls_back_to_id() {
    let titled = skill("fallback-id", "agent_builder daily", "", "global", None);
    let untitled = skill("xy-tool", "", "", "global", None);

    assert_eq!(catalog_skill_initials(&titled), "AD");
    assert_eq!(catalog_skill_initials(&untitled), "XY");
}

#[test]
fn source_labels_and_section_copy_cover_known_and_unknown_sources() {
    let cases = [
        ("workspace", "本目录", "项目目录"),
        ("ancestor", "父级目录", "父级目录"),
        ("global", "全局目录", "全局目录"),
        ("bundled", "内置技能", "内置技能"),
        ("other", "来源", "其他来源"),
    ];

    for (source, label, title) in cases {
        let (_, section_title, subtitle) = section_copy(source);
        assert_eq!(source_label(source), label);
        assert_eq!(section_title, title);
        assert!(!subtitle.is_empty());
    }
}

#[test]
fn catalog_widgets_build_for_badges_items_groups_and_styles() {
    let _ = section_card_style(&iced::Theme::Dark);
    let _ = skill_badge("Enabled", true);
    let item = skill("local", "Local Skill", "Description", "workspace", Some("/tmp/skill"));
    let _ = catalog_item(item.clone(), true);
    let _ = catalog_group_section("workspace", vec![item], Some("local"));
}

#[test]
fn catalog_matches_query_matches_each_field_individually() {
    let item = skill_with_kind(
        "rust-helper",
        "Code Review",
        "Finds risky changes",
        "workspace",
        Some("/repo/.vibewindow/skills/review"),
        CatalogSkillKind::Personal,
        0,
        true,
        true,
    );

    assert!(catalog_matches_query(&item, "rust"));
    assert!(catalog_matches_query(&item, "code"));
    assert!(catalog_matches_query(&item, "risky"));
    assert!(catalog_matches_query(&item, "workspace"));
    assert!(catalog_matches_query(&item, "review"));
    assert!(catalog_matches_query(&item, ""));
    assert!(!catalog_matches_query(&item, "no-match"));
}

#[test]
fn section_card_style_sets_expected_properties() {
    let style = section_card_style(&iced::Theme::Dark);
    assert_eq!(style.border.radius, 14.0.into());
}

#[test]
fn skill_badge_non_emphasized_delegates_to_settings_value_badge() {
    let _ = skill_badge("text", false);
}

#[test]
fn skill_badge_emphasized_creates_styled_container() {
    let _ = skill_badge("text", true);
}

#[test]
fn catalog_skill_initials_handles_hyphen_and_single_word() {
    let hyphen = skill("id", "agent-builder", "", "global", None);
    assert_eq!(catalog_skill_initials(&hyphen), "AB");

    let single = skill("id", "Agent", "", "global", None);
    assert_eq!(catalog_skill_initials(&single), "A");
}

#[test]
fn catalog_skill_initials_skips_empty_parts_and_limits_fallback_to_two_characters() {
    let separated = skill("unused", "  agent--builder daily", "", "global", None);
    assert_eq!(catalog_skill_initials(&separated), "AB");

    let fallback = skill("xyz-tool", " - ", "", "global", None);
    assert_eq!(catalog_skill_initials(&fallback), "XY");
}

#[test]
fn section_card_style_resets_shadow_for_light_and_dark_themes() {
    for theme in [iced::Theme::Light, iced::Theme::Dark] {
        let style = section_card_style(&theme);
        assert_eq!(style.border.radius, 14.0.into());
        assert_eq!(style.shadow, iced::Shadow::default());
    }
}

#[test]
fn catalog_item_variants_cover_all_kinds_resources_and_states() {
    let recommended = skill_with_kind(
        "rec-1",
        "Recommended Skill",
        "A featured skill",
        "workspace",
        None,
        CatalogSkillKind::Recommended,
        5,
        true,
        true,
    );
    let _ = catalog_item(recommended, false);

    let system = skill_with_kind(
        "sys-1",
        "System Skill",
        "A built-in skill",
        "global",
        Some("/path"),
        CatalogSkillKind::System,
        0,
        true,
        false,
    );
    let _ = catalog_item(system, false);

    let personal_not_installed = skill_with_kind(
        "per-1",
        "Personal Skill",
        "A local skill",
        "ancestor",
        None,
        CatalogSkillKind::Personal,
        3,
        false,
        false,
    );
    let _ = catalog_item(personal_not_installed, true);
}

#[test]
fn catalog_group_section_variants_cover_sources_and_empty_skills() {
    let skill_ws = skill_with_kind(
        "ws-1",
        "WS Skill",
        "desc",
        "workspace",
        None,
        CatalogSkillKind::Recommended,
        0,
        true,
        true,
    );

    let _ = catalog_group_section("bundled", vec![], None);
    let _ = catalog_group_section("global", vec![skill_ws], Some("ws-1"));
    let _ = catalog_group_section("ancestor", vec![], None);
    let _ = catalog_group_section("unknown", vec![], None);
}

#[test]
fn catalog_tests_keeps_planned_coverage_targets() {
    for name in [
        "catalog_matches_query",
        "section_card_style",
        "skill_badge",
        "catalog_skill_initials",
        "section_copy",
        "source_label",
        "catalog_item",
        "catalog_group_section",
    ] {
        assert!(source_declares_symbol(name), "expected source to declare coverage target {name}");
    }
}
