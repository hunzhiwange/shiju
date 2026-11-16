//! 验证工具选择器的查询匹配、作用域标签、尺寸限制与主题对比度。
//! 搜索 fixture 混合空白、大小写和多个字段，确保命中规则宽松但预览尺寸受控。

use super::selector_fixture::SelectorFixture as App;
use super::selector_fixture::session_tool_selector_popover;
use desktop_chat_tools::state::{SessionToolGroup, SessionToolSelectorTab};
use desktop_settings_agents::state::DelegateAgentSettingsEntry;
use desktop_settings_skills::types::{SkillsCatalogItem, SkillsCatalogKind, SkillsDirectoryScope};
use iced::Length;
use iced::advanced::widget::Tree;
use iced::{Color, Theme};

const EPSILON: f32 = 0.000_01;

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
fn selector_popover_renders_default_agent_tab() {
    let app = App::new().0;

    let popover = session_tool_selector_popover(&app);
    assert_eq!(popover.as_widget().size().width, Length::Fixed(360.0));

    let tree = Tree::new(popover.as_widget());
    assert_eq!(tree.children.len(), 6);
    assert_eq!(tree.children[3].children.len(), 3);
}

#[test]
fn selector_popover_hides_tools_when_acp_session_has_stale_tools_tab() {
    let mut app = App::new().0;
    app.agents_settings.available_tools = vec!["read".into()];
    let runtime = app.current_session_runtime_mut();
    runtime.acp_agent = Some("codex".into());
    runtime.tool_selector.select_tab(SessionToolSelectorTab::Tools);

    let popover = session_tool_selector_popover(&app);
    let tree = Tree::new(popover.as_widget());
    assert_eq!(tree.children[3].children.len(), 2);
    assert_eq!(
        app.current_session_runtime().visible_session_tool_selector_tab(),
        SessionToolSelectorTab::Agent,
    );
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
    use desktop_settings_agents::state::DelegateAgentSettingsEntry;
    let cli = DelegateAgentSettingsEntry::from_config(
        "acp-codex",
        Some(config_agent::AgentDefinitionConfig {
            runtime: config_agent_profile::AgentRuntime::Acp { backend: "codex".into() },
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
    use config_agent::AgentDefinitionConfig;
    use config_agent_profile::AgentRuntime;
    use desktop_chat_tools::selector_input::Message;
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, Shell, layout, mouse, renderer};
    use iced::{Event, Font, Pixels, Rectangle, Size};

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
                (Message::SessionAgentSelected(None), SessionToolSelectorTab::Agent)
                    | (Message::SessionManualToolSelected(_), SessionToolSelectorTab::Tools)
                    | (Message::SessionSkillSelected(_), SessionToolSelectorTab::Skills)
            ));
            drop(element);
            let selection = messages.remove(0);
            app.select(selection);
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
