//! 验证工具选择器的查询匹配、作用域标签、尺寸限制与主题对比度。
//! 搜索 fixture 混合空白、大小写和多个字段，确保命中规则宽松但预览尺寸受控。

use super::tool_selector::{
    SESSION_SELECTOR_LIST_MAX_HEIGHT, SESSION_SELECTOR_SCROLLBAR_WIDTH,
    SESSION_SELECTOR_SKILL_DESCRIPTION_CHARS, context_list_button_style, context_row_text_color,
    count_badge, ellipsize_text, is_dark_theme, matches_query, search_text_input_style,
    selector_search_bar, session_tool_selector_popover, skill_scope_button, skill_scope_label,
    skill_scope_source_matches, skill_tooltip_content, tab_button,
};
use crate::app::App;
use crate::app::state::{
    DelegateAgentSettingsEntry, SessionToolGroup, SessionToolSelectorTab, SkillsCatalogItem,
    SkillsCatalogKind, SkillsDirectoryScope,
};
use iced::Length;
use iced::advanced::widget::Tree;
use iced::widget::{button, text_input};
use iced::{Background, Color, Theme};

const EPSILON: f32 = 0.000_01;

fn assert_close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= EPSILON, "expected {actual} to equal {expected}");
}

fn assert_color_close(actual: Color, expected: Color) {
    assert_close(actual.r, expected.r);
    assert_close(actual.g, expected.g);
    assert_close(actual.b, expected.b);
    assert_close(actual.a, expected.a);
}

fn skill(
    id: &str,
    title: &str,
    description: &str,
    source: &str,
    source_path: Option<&str>,
    installed: bool,
    enabled: bool,
) -> SkillsCatalogItem {
    SkillsCatalogItem {
        id: id.to_string(),
        title: title.to_string(),
        description: description.to_string(),
        kind: SkillsCatalogKind::Personal,
        resource_count: 0,
        installed,
        enabled,
        source: source.to_string(),
        source_path: source_path.map(ToOwned::to_owned),
    }
}

#[test]
fn selector_scrollbar_width_matches_chat_design_contract() {
    assert_eq!(SESSION_SELECTOR_SCROLLBAR_WIDTH, 4.0);
}

#[test]
fn selector_list_has_positive_height_limit() {
    assert!(SESSION_SELECTOR_LIST_MAX_HEIGHT > 0.0);
}

#[test]
fn skill_description_preview_stays_compact() {
    assert!(SESSION_SELECTOR_SKILL_DESCRIPTION_CHARS <= 28);
}

#[test]
fn ellipsize_text_compacts_whitespace_and_uses_ascii_dots() {
    assert_eq!(ellipsize_text("alpha\n beta   gamma", 12), "alpha bet...");
    assert_eq!(ellipsize_text("short text", 20), "short text");
    assert_eq!(ellipsize_text("abcdef", 3), "...");
    assert_eq!(ellipsize_text("你好世界", 4), "你好世界");
    assert_eq!(ellipsize_text("abcdef", 0), "...");
}

#[test]
fn matches_query_trims_and_matches_any_part_case_insensitively() {
    assert!(matches_query("  ", &["Alpha", "Beta"]));
    assert!(matches_query(" ALP ", &["alpha", "beta"]));
    assert!(matches_query("eta", &["alpha", "BETA"]));
    assert!(!matches_query("gamma", &["alpha", "beta"]));
    assert!(!matches_query("alpha", &[]));
}

#[test]
fn skill_scope_helpers_map_exact_sources_and_labels() {
    let cases = [
        (SkillsDirectoryScope::Project, "workspace", "项目"),
        (SkillsDirectoryScope::Ancestor, "ancestor", "父级"),
        (SkillsDirectoryScope::Global, "global", "全局"),
        (SkillsDirectoryScope::Bundled, "bundled", "内置"),
    ];

    for (scope, source, label) in cases {
        assert!(skill_scope_source_matches(scope, source));
        assert!(!skill_scope_source_matches(scope, "other"));
        assert_eq!(skill_scope_label(scope), label);
    }
    assert!(skill_scope_source_matches(SkillsDirectoryScope::All, "other"));
    assert_eq!(skill_scope_label(SkillsDirectoryScope::All), "全部");
    assert!(!skill_scope_source_matches(SkillsDirectoryScope::Global, "GLOBAL"));
}

#[test]
fn context_colors_preserve_contrast_across_light_and_dark_themes() {
    assert!(!is_dark_theme(&Theme::Light));
    assert!(is_dark_theme(&Theme::Dark));

    assert_color_close(
        context_row_text_color(&Theme::Light, true, 0.42),
        Color::from_rgba8(28, 32, 40, 0.42),
    );
    assert_color_close(
        context_row_text_color(&Theme::Light, false, 0.42),
        Theme::Light.palette().text.scale_alpha(0.42),
    );
    assert_color_close(
        context_row_text_color(&Theme::Dark, true, 0.42),
        Theme::Dark.palette().text.scale_alpha(0.42),
    );
}

#[test]
fn context_button_style_overrides_selected_text_color() {
    let selected = context_list_button_style(&Theme::Light, button::Status::Active, true);
    let unselected = context_list_button_style(&Theme::Light, button::Status::Active, false);

    assert_color_close(selected.text_color, context_row_text_color(&Theme::Light, true, 0.96));
    assert_color_close(unselected.text_color, Theme::Light.palette().text.scale_alpha(0.88));

    for status in [button::Status::Hovered, button::Status::Pressed, button::Status::Disabled] {
        let style = context_list_button_style(&Theme::Dark, status, true);
        assert_color_close(style.text_color, context_row_text_color(&Theme::Dark, true, 0.96));
    }
}

#[test]
fn search_input_style_mutes_only_disabled_values() {
    let active = search_text_input_style(&Theme::Dark, text_input::Status::Active);
    let disabled = search_text_input_style(&Theme::Dark, text_input::Status::Disabled);

    match active.background {
        Background::Color(background) => assert_color_close(background, Color::TRANSPARENT),
        other => panic!("expected transparent color background, got {other:?}"),
    }
    assert_close(active.border.width, 0.0);
    assert_color_close(active.value, Theme::Dark.palette().text);
    assert_color_close(disabled.value, Theme::Dark.palette().text.scale_alpha(0.50));
    assert_color_close(active.icon, Theme::Dark.palette().text.scale_alpha(0.65));
    assert_color_close(active.placeholder, Theme::Dark.palette().text.scale_alpha(0.48));
    assert_color_close(active.selection, Theme::Dark.palette().primary.scale_alpha(0.22));
}

#[test]
fn selector_helpers_build_all_discrete_widget_variants() {
    let badges = [count_badge("0 / 2".to_string(), false), count_badge("1 / 2".to_string(), true)];
    let tabs = [
        tab_button(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            SessionToolSelectorTab::Agent,
            true,
        ),
        tab_button(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            SessionToolSelectorTab::Tools,
            false,
        ),
        tab_button(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            SessionToolSelectorTab::Skills,
            false,
        ),
    ];
    let scopes = [
        SkillsDirectoryScope::Project,
        SkillsDirectoryScope::Ancestor,
        SkillsDirectoryScope::Global,
        SkillsDirectoryScope::Bundled,
        SkillsDirectoryScope::All,
    ]
    .map(|scope| {
        skill_scope_button(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            scope,
            SkillsDirectoryScope::Project,
        )
    });
    let _ = selector_search_bar(
        crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
        "read".to_string(),
    );
    let _ = skill_tooltip_content(
        crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
        "Review".to_string(),
        "review".to_string(),
        "workspace".to_string(),
        String::new(),
    );
    let _ = skill_tooltip_content(
        crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
        "Review".to_string(),
        "review".to_string(),
        "workspace".to_string(),
        "Checks risky changes".to_string(),
    );

    assert_eq!(badges.len(), 2);
    assert_eq!(tabs.len(), 3);
    assert_eq!(scopes.len(), 5);
}

#[test]
fn selector_popover_renders_default_agent_tab() {
    let app = App::new().0;

    let popover = session_tool_selector_popover(&app);
    assert_eq!(popover.as_widget().size().width, Length::Fixed(360.0));

    let tree = Tree::new(popover.as_widget());
    assert_eq!(tree.children.len(), 6);
    assert_eq!(tree.children[3].children.len(), 3);
}

#[test]
fn selector_popover_renders_empty_and_filtered_agent_states() {
    let mut app = App::new().0;
    app.agents_settings.entries.clear();

    let _ = session_tool_selector_popover(&app);

    app.current_session_runtime_mut().tool_selector.set_query("missing agent".to_string());
    let _ = session_tool_selector_popover(&app);
}

#[test]
fn selector_popover_keeps_selected_disabled_agent_visible_with_empty_model() {
    let mut app = App::new().0;
    let mut selected = DelegateAgentSettingsEntry::from_config("disabled-worker", None);
    selected.enabled = false;
    selected.label = "Disabled Worker".to_string();
    selected.model.clear();
    selected.allowed_tools.clear();
    app.agents_settings.entries = vec![selected];
    app.current_session_runtime_mut().agent = Some("disabled-worker".to_string());

    let popover = session_tool_selector_popover(&app);
    let tree = Tree::new(popover.as_widget());

    assert_eq!(tree.children.len(), 6);
}

#[test]
fn selector_popover_renders_tools_tab_with_empty_filtered_and_grouped_tools() {
    let mut app = App::new().0;
    app.current_session_runtime_mut().tool_selector.select_tab(SessionToolSelectorTab::Tools);

    let _ = session_tool_selector_popover(&app);

    app.agents_settings.available_tools = vec![
        " bash ".to_string(),
        "read".to_string(),
        "read".to_string(),
        "web_search".to_string(),
        "memory_store".to_string(),
        "custom_action".to_string(),
        "   ".to_string(),
    ];
    {
        let selector = &mut app.current_session_runtime_mut().tool_selector;
        selector.toggle_manual_tool("read");
        selector.toggle_group_collapsed(SessionToolGroup::Files);
    }

    let _ = session_tool_selector_popover(&app);

    app.current_session_runtime_mut().tool_selector.set_query("missing tool".to_string());

    let _ = session_tool_selector_popover(&app);
}

#[test]
fn selector_popover_renders_expanded_selected_tool_row() {
    let mut app = App::new().0;
    app.agents_settings.available_tools = vec!["read".to_string()];
    let selector = &mut app.current_session_runtime_mut().tool_selector;
    selector.select_tab(SessionToolSelectorTab::Tools);
    selector.toggle_manual_tool("read");

    let _ = session_tool_selector_popover(&app);
}

#[test]
fn selector_popover_renders_skills_tab_loading_and_empty_scope_states() {
    let mut app = App::new().0;
    {
        let runtime = app.current_session_runtime_mut();
        runtime.tool_selector.select_tab(SessionToolSelectorTab::Skills);
        runtime.tool_selector.select_skill_directory_scope(SkillsDirectoryScope::Global);
    }
    app.skills_settings.loading = true;

    let _ = session_tool_selector_popover(&app);

    app.skills_settings.loading = false;
    let _ = session_tool_selector_popover(&app);

    app.skills_settings.catalog = vec![
        skill("global-review", "Global Review", "", "global", None, true, true),
        skill(
            "workspace-rust",
            "Rust Workspace",
            "Reviews a workspace and reports deterministic findings for the current project",
            "workspace",
            Some("/workspace/.agents/skills/rust"),
            true,
            true,
        ),
        skill("bundled-off", "Bundled Off", "Disabled", "bundled", None, false, true),
        skill("ancestor-off", "Ancestor Off", "Disabled", "ancestor", None, true, false),
    ];
    app.current_session_runtime_mut().tool_selector.toggle_manual_skill("global-review");

    let _ = session_tool_selector_popover(&app);

    app.current_session_runtime_mut().tool_selector.set_query("missing skill".to_string());
    let _ = session_tool_selector_popover(&app);

    {
        let selector = &mut app.current_session_runtime_mut().tool_selector;
        selector.select_skill_directory_scope(SkillsDirectoryScope::Bundled);
        selector.set_query(String::new());
    }
    let _ = session_tool_selector_popover(&app);

    app.current_session_runtime_mut()
        .tool_selector
        .select_skill_directory_scope(SkillsDirectoryScope::All);
    let _ = session_tool_selector_popover(&app);
}

#[test]
fn agent_unified_selection_lists_enabled_cli_without_promoting_delegate_roles() {
    use crate::app::state::DelegateAgentSettingsEntry;
    let cli = DelegateAgentSettingsEntry::from_config(
        "acp-codex",
        Some(vw_config_types::agent::AgentDefinitionConfig {
            runtime: vw_config_types::agent::AgentRuntime::Acp { backend: "codex".into() },
            ..Default::default()
        }),
    );
    assert!(cli.can_start_session(), "CLI installation must not hide its profile");
    let mut disabled = cli.clone();
    disabled.enabled = false;
    assert!(!disabled.can_start_session());
    let delegate = DelegateAgentSettingsEntry::from_config("explore", None);
    assert!(!delegate.can_start_session());
}

fn selector_icon_rows(layout: iced::advanced::Layout<'_>) -> Vec<iced::Rectangle> {
    let children = layout.children().collect::<Vec<_>>();
    let mut rows = Vec::new();
    if children.len() == 3
        && (children[0].bounds().width - 16.0).abs() < EPSILON
        && (children[2].bounds().width - 14.0).abs() < EPSILON
    {
        let icon = children[0].bounds();
        let title = children[1].bounds();
        let check = children[2].bounds();
        assert!(icon.x + icon.width <= title.x);
        assert!(title.x + title.width <= check.x);
        assert!((icon.center_y() - check.center_y()).abs() < EPSILON);
        rows.push(icon);
    }
    for child in children {
        rows.extend(selector_icon_rows(child));
    }
    rows
}

async fn render_selector_icons(theme: Theme, click: bool) {
    use crate::app::{Message, message};
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, Shell, layout, mouse, renderer};
    use iced::{Event, Font, Pixels, Rectangle, Size};
    use vw_config_types::agent::{AgentDefinitionConfig, AgentRuntime};

    let mut app = App::new().0;
    app.agents_settings.entries = [
        ("main", AgentRuntime::Native),
        ("known", AgentRuntime::Acp { backend: "codex".into() }),
        ("unknown", AgentRuntime::Acp { backend: "unknown-fixture".into() }),
    ]
    .into_iter()
    .map(|(key, runtime)| {
        let mut entry = DelegateAgentSettingsEntry::from_config(
            key,
            Some(AgentDefinitionConfig { runtime, ..Default::default() }),
        );
        entry.enabled = true;
        entry.label = "中文长标题用于检查窄窗口图文布局与选中标记".into();
        entry.allowed_tools = vec!["memory_store".into()];
        entry
    })
    .collect();
    app.agents_settings.available_tools = vec!["memory_store".into()];
    app.skills_settings.catalog =
        vec![skill("review", "中文长标题用于检查技能图标", "说明", "global", None, true, true)];
    app.current_session_runtime_mut().tool_selector.toggle_manual_tool("memory_store");
    app.current_session_runtime_mut().tool_selector.toggle_manual_skill("review");
    for (tab, expected) in [
        (SessionToolSelectorTab::Agent, 3),
        (SessionToolSelectorTab::Tools, 1),
        (SessionToolSelectorTab::Skills, 1),
    ] {
        // Selecting an agent resets context; seed each tab independently.
        if !app.current_session_runtime().tool_selector.is_manual_tool_selected("memory_store") {
            app.current_session_runtime_mut().tool_selector.toggle_manual_tool("memory_store");
        }
        if !app.current_session_runtime().tool_selector.is_manual_skill_selected("review") {
            app.current_session_runtime_mut().tool_selector.toggle_manual_skill("review");
        }
        app.current_session_runtime_mut().tool_selector.select_tab(tab);
        app.current_session_runtime_mut()
            .tool_selector
            .select_skill_directory_scope(SkillsDirectoryScope::All);
        app.current_session_runtime_mut().tool_selector.set_query(
            match tab {
                SessionToolSelectorTab::Tools => "memory_store",
                SessionToolSelectorTab::Skills => "review",
                SessionToolSelectorTab::Agent => "中文",
            }
            .into(),
        );
        let mut renderer =
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                .await
                .expect("headless renderer");
        let mut element = session_tool_selector_popover(&app);
        let mut tree = Tree::new(element.as_widget());
        let bounds = Size::new(360.0, 700.0);
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, bounds),
        );
        element.as_widget().draw(
            &tree,
            &mut renderer,
            &theme,
            &renderer::Style { text_color: theme.palette().text },
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &Rectangle::with_size(bounds),
        );
        let pixels = renderer.screenshot(Size::new(360, 700), 1.0, Color::TRANSPARENT);
        let rows = selector_icon_rows(Layout::new(&node));
        assert_eq!(rows.len(), expected, "missing identity icons for {tab:?}");
        for icon in &rows {
            assert!(
                (icon.y as u32..(icon.y + icon.height) as u32).any(|y| (icon.x as u32
                    ..(icon.x + icon.width) as u32)
                    .any(|x| pixels
                        [((y * 360 + x) * 4) as usize..((y * 360 + x) * 4 + 3) as usize]
                        != pixels[((y * 360 + icon.x as u32 - 2) * 4) as usize
                            ..((y * 360 + icon.x as u32 - 2) * 4 + 3) as usize])),
                "icon must draw pixels"
            );
        }
        let directory = std::env::temp_dir().join("shiju-session-selector-icons");
        std::fs::create_dir_all(&directory).unwrap();
        image::save_buffer(
            directory.join(format!("{theme:?}-{tab:?}.png")),
            &pixels,
            360,
            700,
            image::ColorType::Rgba8,
        )
        .unwrap();
        if click {
            let position = rows[0].center();
            let mut messages = Vec::new();
            for event in [
                mouse::Event::ButtonPressed(mouse::Button::Left),
                mouse::Event::ButtonReleased(mouse::Button::Left),
            ] {
                element.as_widget_mut().update(
                    &mut tree,
                    &Event::Mouse(event),
                    Layout::new(&node),
                    mouse::Cursor::Available(position),
                    &renderer,
                    &mut iced::advanced::clipboard::Null,
                    &mut Shell::new(&mut messages),
                    &Rectangle::with_size(bounds),
                );
            }
            assert_eq!(messages.len(), 1);
            assert!(matches!(
                (&messages[0], tab),
                (
                    Message::Chat(message::ChatMessage::SessionAgentSelected(None)),
                    SessionToolSelectorTab::Agent
                ) | (
                    Message::Chat(message::ChatMessage::SessionManualToolSelected(_)),
                    SessionToolSelectorTab::Tools
                ) | (
                    Message::Chat(message::ChatMessage::SessionSkillSelected(_)),
                    SessionToolSelectorTab::Skills
                )
            ));
            drop(element);
            let Message::Chat(selection) = messages.remove(0) else {
                panic!("expected selector message");
            };
            let _ = message::chat::update(&mut app, selection);
            let runtime = app.current_session_runtime();
            match tab {
                SessionToolSelectorTab::Agent => assert!(runtime.agent.is_none()),
                SessionToolSelectorTab::Tools => {
                    assert!(!runtime.tool_selector.is_manual_tool_selected("memory_store"));
                }
                SessionToolSelectorTab::Skills => {
                    assert!(!runtime.tool_selector.is_manual_skill_selected("review"));
                }
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn session_entry_selector_icons_preserve_selection() {
    render_selector_icons(Theme::Light, true).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn session_entry_selector_icons_render_both_themes() {
    render_selector_icons(Theme::Light, false).await;
    render_selector_icons(Theme::Dark, false).await;
}
