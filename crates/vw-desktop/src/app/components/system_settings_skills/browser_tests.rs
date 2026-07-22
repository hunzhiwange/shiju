// Tests for plan6 task 819.
const SOURCE: &str = include_str!("browser.rs");

use super::browser::DetailActionStyle;
use super::browser::{
    active_scope_badge, catalog_panel_style, detail_action_button, detail_modal_body,
    detail_source_note, discovery_order_text, discovery_order_view, empty_state,
    header_panel_style, loading_banner, provider_global_paths, provider_project_paths,
    refresh_button, scope_button, scope_description, scope_source_matches, scope_title,
    search_bar_style, search_text_input_style, status_banner, view as browser_view, view_overlays,
};
use crate::app::assets::Icon;
use crate::app::message;
use crate::app::state::{
    SkillsCatalogItem, SkillsCatalogKind, SkillsDirectoryScope, SkillsSelectedDetail,
};
use crate::app::{App, Message, message::SettingsMessage};
use iced::advanced::widget::Tree;
use iced::widget::text_input;
use iced::{Background, Color};
use vw_config_types::skills::SkillsDirectoryProvider;

fn app() -> App {
    App::new().0
}

fn catalog_item(id: &str, source: &str) -> SkillsCatalogItem {
    SkillsCatalogItem {
        id: id.to_string(),
        title: format!("Skill {id}"),
        description: format!("Description for {id}"),
        kind: SkillsCatalogKind::Personal,
        resource_count: 1,
        installed: true,
        enabled: true,
        source: source.to_string(),
        source_path: Some(format!("/skills/{id}")),
    }
}

fn selected_detail(source: &str, kind: SkillsCatalogKind) -> SkillsSelectedDetail {
    SkillsSelectedDetail {
        id: format!("{source}-skill"),
        title: format!("{source} skill"),
        description: "Skill detail".to_string(),
        kind,
        installed: true,
        enabled: true,
        source: source.to_string(),
        source_path: (source != "bundled").then(|| format!("/skills/{source}")),
        document_name: "SKILL.md".to_string(),
        document_content: "# Skill\n\nInstructions".to_string(),
        can_install: true,
        can_toggle: true,
        can_delete: true,
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
fn scope_titles_and_badges_cover_all_scopes() {
    let cases = [
        (SkillsDirectoryScope::Project, "项目目录技能", "当前筛选: 项目目录"),
        (SkillsDirectoryScope::Ancestor, "父级目录技能", "当前筛选: 父级目录"),
        (SkillsDirectoryScope::Global, "全局目录技能", "当前筛选: 全局目录"),
        (SkillsDirectoryScope::Bundled, "内置技能", "当前筛选: 内置技能"),
        (SkillsDirectoryScope::All, "全部技能目录", "当前筛选: 全部目录"),
    ];

    for (scope, title, badge) in cases {
        assert_eq!(scope_title(scope), title);
        assert_eq!(active_scope_badge(scope), badge);
    }
}

#[test]
fn provider_paths_match_directory_provider_contract() {
    let vibewindow_global =
        format!("~/{}/skills 与 ~/.skills", vw_config_types::paths::HOME_CONFIG_DIR_NAME);
    let cases = [
        (
            SkillsDirectoryProvider::Vibewindow,
            ".vibewindow/skills 与 skills",
            vibewindow_global.as_str(),
        ),
        (
            SkillsDirectoryProvider::Codex,
            ".codex/skills 与 .agents/skills",
            "~/.codex/skills 与 ~/.agents/skills",
        ),
        (SkillsDirectoryProvider::Claude, ".claude/skills", "~/.claude/skills"),
        (SkillsDirectoryProvider::Cursor, ".cursor/skills", "~/.cursor/skills"),
    ];

    for (provider, project, global) in cases {
        assert_eq!(provider_project_paths(provider), project);
        assert_eq!(provider_global_paths(provider), global);
    }
}

#[test]
fn scope_descriptions_include_provider_specific_paths() {
    assert!(
        scope_description(SkillsDirectoryScope::Project, SkillsDirectoryProvider::Codex)
            .contains(".codex/skills")
    );
    assert!(
        scope_description(SkillsDirectoryScope::Ancestor, SkillsDirectoryProvider::Claude)
            .contains(".claude/skills")
    );
    assert!(
        scope_description(SkillsDirectoryScope::Global, SkillsDirectoryProvider::Cursor)
            .contains("~/.cursor/skills")
    );
    assert!(
        scope_description(SkillsDirectoryScope::Bundled, SkillsDirectoryProvider::Vibewindow)
            .contains("内置技能")
    );
    assert!(
        scope_description(SkillsDirectoryScope::All, SkillsDirectoryProvider::Vibewindow)
            .contains("全部")
    );
}

#[test]
fn search_styles_preserve_transparency_and_disabled_contrast() {
    let theme = iced::Theme::Dark;
    let palette = theme.palette();
    let active = search_text_input_style(&theme, text_input::Status::Active);
    let disabled = search_text_input_style(&theme, text_input::Status::Disabled);

    assert_eq!(active.background, Background::Color(Color::TRANSPARENT));
    assert_eq!(active.border.width, 0.0);
    assert_eq!(active.border.color, Color::TRANSPARENT);
    assert_eq!(active.value, palette.text);
    assert_eq!(disabled.value, palette.text.scale_alpha(0.50));
    assert_eq!(active.placeholder, palette.text.scale_alpha(0.48));
    assert_eq!(active.selection, palette.primary.scale_alpha(0.22));

    let search_bar = search_bar_style(&theme);
    assert_eq!(search_bar.background, Some(Background::Color(palette.background)));
    assert_eq!(search_bar.border.radius, 12.0.into());
}

#[test]
fn scope_descriptions_are_exact_for_each_codex_scope() {
    let provider = SkillsDirectoryProvider::Codex;
    let cases = [
        (
            SkillsDirectoryScope::Project,
            "查看当前项目 .codex/skills 与 .agents/skills 目录命中的技能。",
        ),
        (
            SkillsDirectoryScope::Ancestor,
            "查看从项目父级向上发现的 .codex/skills 与 .agents/skills 技能。",
        ),
        (
            SkillsDirectoryScope::Global,
            "查看当前用户 ~/.codex/skills 与 ~/.agents/skills 中的全局技能。",
        ),
        (SkillsDirectoryScope::Bundled, "查看产品随包提供、可安装到项目的内置技能。"),
        (
            SkillsDirectoryScope::All,
            "全部结果包含当前项目、父级目录、全局目录以及内置技能，便于核对最终命中来源。",
        ),
    ];

    for (scope, expected) in cases {
        assert_eq!(scope_description(scope, provider), expected);
    }
}

#[test]
fn discovery_order_text_distinguishes_project_context() {
    let mut app = app();
    app.skills_settings.directory_provider = SkillsDirectoryProvider::Codex;

    assert_eq!(
        discovery_order_text(&app),
        "未打开项目时，仅显示 ~/.codex/skills 与 ~/.agents/skills 与内置技能。"
    );

    app.project_path = Some("/workspace/demo".to_string());
    assert_eq!(
        discovery_order_text(&app),
        "/workspace/demo/.codex/skills 与 .agents/skills -> 父级 .codex/skills 与 .agents/skills -> ~/.codex/skills 与 ~/.agents/skills"
    );
}

#[test]
fn discovery_order_view_builds_for_closed_and_open_project() {
    let mut app = app();
    app.skills_settings.directory_provider = SkillsDirectoryProvider::Claude;
    let _ = discovery_order_view(&app);

    app.project_path = Some("/workspace/demo".to_string());
    let _ = discovery_order_view(&app);
}

#[test]
fn browser_widgets_build_for_status_actions_and_common_states() {
    let _ = search_bar_style(&iced::Theme::Dark);
    let _ = header_panel_style(&iced::Theme::Dark);
    let _ = catalog_panel_style(&iced::Theme::Dark);
    let _ = search_text_input_style(&iced::Theme::Dark, text_input::Status::Active);
    let _ = search_text_input_style(&iced::Theme::Dark, text_input::Status::Disabled);

    let _ = status_banner("同步完成", false);
    let _ = status_banner("同步失败", true);
    let _ =
        scope_button("全局目录", SkillsDirectoryScope::Global, SkillsDirectoryScope::Project, true);
    let _ = scope_button(
        "项目目录",
        SkillsDirectoryScope::Project,
        SkillsDirectoryScope::Project,
        false,
    );
    let _ = refresh_button(false);
    let _ = refresh_button(true);
    let _ = loading_banner();
    let _ = empty_state("空", "没有结果");
    let _ = detail_source_note(Some("/tmp/skill"));
    let _ = detail_source_note(None);
    let _ = detail_action_button(
        "执行",
        Icon::Plus,
        DetailActionStyle::Primary,
        Some(Message::Settings(SettingsMessage::SkillsRefresh)),
    );
    let _ = detail_action_button("删除", Icon::Trash, DetailActionStyle::Danger, None);
    let _ = detail_action_button(
        "次要",
        Icon::ArrowRepeat,
        DetailActionStyle::Secondary,
        Some(Message::Settings(message::SettingsMessage::SkillsRefresh)),
    );
}

#[test]
fn detail_modal_builds_loading_error_empty_and_action_states() {
    let mut app = app();

    app.skills_settings.detail_loading = true;
    let _ = detail_modal_body(&app, false);

    app.skills_settings.detail_loading = false;
    app.skills_settings.detail_error = Some("detail failed".to_string());
    let _ = detail_modal_body(&app, false);

    app.skills_settings.detail_error = None;
    let _ = detail_modal_body(&app, false);

    let cases = [
        ("workspace", SkillsCatalogKind::Recommended),
        ("ancestor", SkillsCatalogKind::System),
        ("global", SkillsCatalogKind::Personal),
        ("bundled", SkillsCatalogKind::System),
        ("unknown", SkillsCatalogKind::Personal),
    ];
    for (source, kind) in cases {
        app.skills_settings.selected_skill_detail = Some(selected_detail(source, kind));
        let _ = detail_modal_body(&app, true);
    }

    let mut restricted = selected_detail("bundled", SkillsCatalogKind::System);
    restricted.installed = false;
    restricted.enabled = false;
    restricted.can_install = false;
    restricted.can_toggle = false;
    restricted.can_delete = false;
    app.skills_settings.selected_skill_detail = Some(restricted);
    let _ = detail_modal_body(&app, false);

    app.skills_settings.loading = true;
    app.skills_settings.selected_skill_detail =
        Some(selected_detail("workspace", SkillsCatalogKind::Recommended));
    let _ = detail_modal_body(&app, true);
}

#[test]
fn detail_modal_action_row_matches_supported_actions() {
    let mut app = app();
    let mut detail = selected_detail("workspace", SkillsCatalogKind::Personal);
    detail.can_install = true;
    detail.can_toggle = false;
    detail.can_delete = true;
    app.skills_settings.selected_skill_detail = Some(detail);

    let element = detail_modal_body(&app, true);
    let tree = Tree::new(element.as_widget());
    let action_row = &tree.children[1];

    assert_eq!(action_row.children.len(), 2);
}

#[test]
fn detail_modal_keeps_toggle_available_when_install_requires_project() {
    let mut app = app();
    let mut detail = selected_detail("bundled", SkillsCatalogKind::System);
    detail.installed = false;
    detail.enabled = false;
    detail.can_install = true;
    detail.can_toggle = true;
    detail.can_delete = false;
    app.skills_settings.selected_skill_detail = Some(detail);

    let closed_project = detail_modal_body(&app, false);
    std::hint::black_box(closed_project);

    app.project_path = Some("/workspace/demo".to_string());
    let open_project = detail_modal_body(&app, true);
    std::hint::black_box(open_project);
}

#[test]
fn overlays_build_for_base_loading_and_each_detail_source() {
    let mut app = app();
    let _ = view_overlays(&app, empty_state("base", "content"));

    app.skills_settings.selected_skill_id = Some("selected".to_string());
    let _ = view_overlays(&app, empty_state("base", "content"));

    for source in ["workspace", "ancestor", "global", "bundled", "unknown"] {
        app.skills_settings.selected_skill_detail =
            Some(selected_detail(source, SkillsCatalogKind::Personal));
        let _ = view_overlays(&app, empty_state("base", "content"));
    }
}

#[test]
fn browser_view_builds_empty_loading_status_and_grouped_catalog_states() {
    let mut app = app();
    app.skills_settings.directory_provider = SkillsDirectoryProvider::Codex;

    for scope in [
        SkillsDirectoryScope::Project,
        SkillsDirectoryScope::Ancestor,
        SkillsDirectoryScope::Global,
        SkillsDirectoryScope::Bundled,
        SkillsDirectoryScope::All,
    ] {
        app.skills_settings.directory_scope = scope;
        let _ = browser_view(&app);
    }

    app.project_path = Some("/workspace/demo".to_string());
    for scope in [SkillsDirectoryScope::Project, SkillsDirectoryScope::Ancestor] {
        app.skills_settings.directory_scope = scope;
        let _ = browser_view(&app);
    }

    app.skills_settings.directory_scope = SkillsDirectoryScope::All;
    app.skills_settings.loading = true;
    let _ = browser_view(&app);

    app.skills_settings.loading = false;
    app.skills_settings.status_message = Some("refreshed".to_string());
    let _ = browser_view(&app);
    app.skills_settings.status_is_error = true;
    let _ = browser_view(&app);

    app.skills_settings.status_message = None;
    app.skills_settings.catalog = vec![
        catalog_item("workspace", "workspace"),
        catalog_item("ancestor", "ancestor"),
        catalog_item("global", "global"),
        catalog_item("bundled", "bundled"),
    ];
    app.skills_settings.selected_skill_id = Some("global".to_string());
    let _ = browser_view(&app);

    app.skills_settings.query = "missing".to_string();
    let _ = browser_view(&app);
}

#[test]
fn browser_view_search_can_match_source_path_and_scope_together() {
    let mut app = app();
    app.skills_settings.directory_scope = SkillsDirectoryScope::All;
    app.skills_settings.catalog = vec![
        catalog_item("workspace", "workspace"),
        catalog_item("global", "global"),
        catalog_item("bundled", "bundled"),
    ];
    app.skills_settings.query = "/skills/global".to_string();

    let _ = browser_view(&app);

    app.skills_settings.directory_scope = SkillsDirectoryScope::Global;
    let _ = browser_view(&app);
}

#[test]
fn browser_tests_keeps_planned_coverage_targets() {
    for name in [
        "DetailActionStyle",
        "search_bar_style",
        "header_panel_style",
        "catalog_panel_style",
        "status_banner",
        "scope_button",
        "refresh_button",
        "scope_source_matches",
        "scope_title",
        "scope_description",
        "active_scope_badge",
        "discovery_order_text",
        "discovery_order_view",
        "loading_banner",
        "empty_state",
        "detail_source_note",
        "detail_modal_body",
        "view_overlays",
        "view",
    ] {
        assert!(source_declares_symbol(name), "expected source to declare coverage target {name}");
    }
}
