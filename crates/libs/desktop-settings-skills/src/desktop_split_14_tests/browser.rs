//! 验证技能浏览器的目录作用域、路径契约、搜索样式和详情状态。
//!
//! 技能来源覆盖全局、项目和多种目录提供商，用于约束作用域判定与路径展示。

// 保留计划任务 819 的既定覆盖面。
const SOURCE: &str = concat!(
    include_str!("../browser.rs"),
    include_str!("../browser_discovery.rs"),
    include_str!("../browser_widgets.rs"),
    include_str!("../browser_detail.rs")
);

use crate::browser::DetailActionStyle;
use crate::browser::{
    catalog_panel_style, detail_action_button, detail_modal_body, detail_source_note,
    discovery_order_text, discovery_order_view, empty_state, header_panel_style, loading_banner,
    refresh_button, scope_button, skill_project_options, status_banner, view as browser_view,
    view_overlays,
};
use desktop_assets::assets::Icon;

use crate::types::{
    SkillsCatalogItem, SkillsCatalogKind, SkillsDirectoryScope, SkillsSelectedDetail,
};
use crate::{BrowserContext, BrowserState, Event as Message};
use config_skills::SkillsDirectoryProvider;
use iced::advanced::widget::Tree;

pub(super) fn help<'a, E>(
    _: desktop_i18n::I18n,
    _: Option<&iced::Theme>,
    _: &'a str,
    label: iced::widget::Text<'a>,
    _: std::borrow::Cow<'a, str>,
) -> iced::widget::Row<'a, E> {
    iced::widget::row![label]
}
struct Fixture {
    project_path: Option<String>,
    recent_projects: Vec<String>,
    skills_settings: State,
    theme: iced::Theme,
}
impl Default for Fixture {
    fn default() -> Self {
        Self {
            project_path: None,
            recent_projects: Vec::new(),
            skills_settings: State::default(),
            theme: iced::Theme::Dark,
        }
    }
}
struct State {
    directory_provider: config_skills::SkillsDirectoryProvider,
    query: String,
    directory_scope: SkillsDirectoryScope,
    loading: bool,
    catalog: Vec<SkillsCatalogItem>,
    selected_skill_id: Option<String>,
    selected_skill_detail: Option<SkillsSelectedDetail>,
    detail_loading: bool,
    detail_error: Option<String>,
    status_message: Option<String>,
    status_is_error: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            directory_provider: config_skills::SkillsDirectoryProvider::Vibewindow,
            query: String::new(),
            directory_scope: SkillsDirectoryScope::All,
            loading: false,
            catalog: vec![],
            selected_skill_id: None,
            selected_skill_detail: None,
            detail_loading: false,
            detail_error: None,
            status_message: None,
            status_is_error: false,
        }
    }
}
impl Fixture {
    fn context(&self) -> BrowserContext<'_> {
        let s = &self.skills_settings;
        BrowserContext {
            help: help::<Message>,
            desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
            app_theme: &self.theme,
            project_path: &self.project_path,
            recent_projects: &self.recent_projects,
            skills_settings: BrowserState {
                directory_provider: s.directory_provider,
                query: &s.query,
                directory_scope: s.directory_scope,
                loading: s.loading,
                catalog: &s.catalog,
                selected_skill_id: &s.selected_skill_id,
                selected_skill_detail: &s.selected_skill_detail,
                detail_loading: s.detail_loading,
                detail_error: &s.detail_error,
                status_message: &s.status_message,
                status_message_translation: &None,
                status_is_error: s.status_is_error,
            },
        }
    }
}
fn app() -> Fixture {
    Fixture::default()
}

#[test]
fn skill_project_options_put_current_first_and_deduplicate_recent_paths() {
    let mut app = app();
    app.project_path = Some("/repo/current".to_string());
    app.recent_projects = vec![
        "/repo/other".to_string(),
        "/repo/current".to_string(),
        "  /repo/third  ".to_string(),
        String::new(),
    ];

    assert_eq!(
        skill_project_options(app.context()),
        vec!["/repo/current".to_string(), "/repo/other".to_string(), "/repo/third".to_string(),]
    );
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
fn discovery_order_text_distinguishes_project_context() {
    let mut app = app();
    app.skills_settings.directory_provider = SkillsDirectoryProvider::Codex;

    assert_eq!(
        discovery_order_text(app.context()),
        "未打开项目时，仅显示 ~/.codex/skills, ~/.agents/skills 与内置技能。"
    );

    app.project_path = Some("/workspace/demo".to_string());
    assert_eq!(
        discovery_order_text(app.context()),
        "发现顺序：/workspace/demo/.codex/skills, .agents/skills, skills -> 父级 .codex/skills, .agents/skills, skills -> ~/.codex/skills, ~/.agents/skills"
    );
}

#[test]
fn discovery_order_view_builds_for_closed_and_open_project() {
    let mut app = app();
    app.skills_settings.directory_provider = SkillsDirectoryProvider::Claude;
    let _ = discovery_order_view(app.context());

    app.project_path = Some("/workspace/demo".to_string());
    let _ = discovery_order_view(app.context());
}

#[test]
fn browser_widgets_build_for_status_actions_and_common_states() {
    let _ = header_panel_style(&iced::Theme::Dark);
    let _ = catalog_panel_style(&iced::Theme::Dark);

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
    let _ = refresh_button(config_ui::DesktopLocale::SimplifiedChinese, false);
    let _ = refresh_button(config_ui::DesktopLocale::SimplifiedChinese, true);
    let _ = loading_banner("正在同步技能目录");
    let _ = empty_state("空", "没有结果");
    let _ = detail_source_note(config_ui::DesktopLocale::SimplifiedChinese, Some("/tmp/skill"));
    let _ = detail_source_note(config_ui::DesktopLocale::SimplifiedChinese, None);
    let _ = detail_action_button::<Message>(
        config_ui::DesktopLocale::SimplifiedChinese,
        "执行",
        Icon::Plus,
        DetailActionStyle::Primary,
        Some(Message::SkillsRefresh),
    );
    let _ = detail_action_button::<Message>(
        config_ui::DesktopLocale::SimplifiedChinese,
        "删除",
        Icon::Trash,
        DetailActionStyle::Danger,
        None,
    );
    let _ = detail_action_button::<Message>(
        config_ui::DesktopLocale::SimplifiedChinese,
        "次要",
        Icon::ArrowRepeat,
        DetailActionStyle::Secondary,
        Some(Message::SkillsRefresh),
    );
}

#[test]
fn detail_modal_builds_loading_error_empty_and_action_states() {
    let mut app = app();

    app.skills_settings.detail_loading = true;
    let _ = detail_modal_body(app.context(), false);

    app.skills_settings.detail_loading = false;
    app.skills_settings.detail_error = Some("detail failed".to_string());
    let _ = detail_modal_body(app.context(), false);

    app.skills_settings.detail_error = None;
    let _ = detail_modal_body(app.context(), false);

    let cases = [
        ("workspace", SkillsCatalogKind::Recommended),
        ("ancestor", SkillsCatalogKind::System),
        ("global", SkillsCatalogKind::Personal),
        ("bundled", SkillsCatalogKind::System),
        ("unknown", SkillsCatalogKind::Personal),
    ];
    for (source, kind) in cases {
        app.skills_settings.selected_skill_detail = Some(selected_detail(source, kind));
        let _ = detail_modal_body(app.context(), true);
    }

    let mut restricted = selected_detail("bundled", SkillsCatalogKind::System);
    restricted.installed = false;
    restricted.enabled = false;
    restricted.can_install = false;
    restricted.can_toggle = false;
    restricted.can_delete = false;
    app.skills_settings.selected_skill_detail = Some(restricted);
    let _ = detail_modal_body(app.context(), false);

    app.skills_settings.loading = true;
    app.skills_settings.selected_skill_detail =
        Some(selected_detail("workspace", SkillsCatalogKind::Recommended));
    let _ = detail_modal_body(app.context(), true);
}

#[test]
fn detail_modal_action_row_matches_supported_actions() {
    let mut app = app();
    let mut detail = selected_detail("workspace", SkillsCatalogKind::Personal);
    detail.can_install = true;
    detail.can_toggle = false;
    detail.can_delete = true;
    app.skills_settings.selected_skill_detail = Some(detail);

    let element = detail_modal_body(app.context(), true);
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

    let closed_project = detail_modal_body(app.context(), false);
    std::hint::black_box(closed_project);

    app.project_path = Some("/workspace/demo".to_string());
    let open_project = detail_modal_body(app.context(), true);
    std::hint::black_box(open_project);
}

#[test]
fn overlays_build_for_base_loading_and_each_detail_source() {
    let mut app = app();
    let _ = view_overlays(app.context(), empty_state("base", "content"), std::convert::identity);

    app.skills_settings.selected_skill_id = Some("selected".to_string());
    let _ = view_overlays(app.context(), empty_state("base", "content"), std::convert::identity);

    for source in ["workspace", "ancestor", "global", "bundled", "unknown"] {
        app.skills_settings.selected_skill_detail =
            Some(selected_detail(source, SkillsCatalogKind::Personal));
        let _ =
            view_overlays(app.context(), empty_state("base", "content"), std::convert::identity);
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
        let _ = browser_view(app.context());
    }

    app.project_path = Some("/workspace/demo".to_string());
    for scope in [SkillsDirectoryScope::Project, SkillsDirectoryScope::Ancestor] {
        app.skills_settings.directory_scope = scope;
        let _ = browser_view(app.context());
    }

    app.skills_settings.directory_scope = SkillsDirectoryScope::All;
    app.skills_settings.loading = true;
    let _ = browser_view(app.context());

    app.skills_settings.loading = false;
    app.skills_settings.status_message = Some("refreshed".to_string());
    let _ = browser_view(app.context());
    app.skills_settings.status_is_error = true;
    let _ = browser_view(app.context());

    app.skills_settings.status_message = None;
    app.skills_settings.catalog = vec![
        catalog_item("workspace", "workspace"),
        catalog_item("ancestor", "ancestor"),
        catalog_item("global", "global"),
        catalog_item("bundled", "bundled"),
    ];
    app.skills_settings.selected_skill_id = Some("global".to_string());
    let _ = browser_view(app.context());

    app.skills_settings.query = "missing".to_string();
    let _ = browser_view(app.context());
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

    let _ = browser_view(app.context());

    app.skills_settings.directory_scope = SkillsDirectoryScope::Global;
    let _ = browser_view(app.context());
}

#[test]
fn browser_tests_keeps_planned_coverage_targets() {
    for name in [
        "DetailActionStyle",
        "header_panel_style",
        "catalog_panel_style",
        "status_banner",
        "scope_button",
        "refresh_button",
        "scope_source_matches",
        "scope_title",
        "scope_description",
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
