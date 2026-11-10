//! 验证代理设置主视图的标签切换、状态组合与空态路径。
//! 主入口测试组合基础、身份、技能和工具页，检查共享状态传递。

use crate::app::App;
use crate::app::state::{AgentSettingsEntryKind, ProviderSummary};

fn root_child_count(app: &App) -> usize {
    super::agents_view(app).as_widget().children().len()
}

#[test]
fn agents_view_loading_state_adds_status_row() {
    let (mut app, _) = App::new();

    assert_eq!(root_child_count(&app), 4);

    app.agents_settings.config_ready = true;
    app.agents_settings.loading = true;

    assert_eq!(root_child_count(&app), 5);
}

#[test]
fn agents_view_builds_cards_for_main_enabled_disabled_and_model_variants() {
    let (mut app, _) = App::new();
    app.agents_settings.config_ready = true;
    let mut main = app
        .agents_settings
        .entries
        .iter()
        .find(|entry| entry.key == "main")
        .cloned()
        .expect("default settings should contain the main agent");
    main.model = "   ".to_string();

    let mut enabled_worker = app
        .agents_settings
        .entries
        .iter()
        .find(|entry| entry.key != "main")
        .cloned()
        .expect("default settings should contain a worker agent");
    enabled_worker.enabled = true;
    enabled_worker.model = "worker-model".to_string();

    let mut disabled_custom = enabled_worker.clone();
    disabled_custom.key = "custom-disabled".to_string();
    disabled_custom.label = "Disabled custom".to_string();
    disabled_custom.kind = AgentSettingsEntryKind::Custom;
    disabled_custom.enabled = false;
    disabled_custom.model.clear();

    app.agents_settings.entries = vec![main, enabled_worker, disabled_custom];
    app.agents_settings.selected_agent = "custom-disabled".to_string();

    assert_eq!(root_child_count(&app), 5);
}

#[test]
fn agents_view_missing_selection_uses_first_entry_and_unknown_tab_falls_back() {
    let (mut app, _) = App::new();
    app.agents_settings.config_ready = true;
    let first_key = app
        .agents_settings
        .entries
        .first()
        .expect("default settings should contain agents")
        .key
        .clone();
    app.agents_settings.selected_agent = "missing-agent".to_string();
    app.agents_settings.active_detail_tab = "future-detail-tab".to_string();
    app.agents_settings.providers = vec![ProviderSummary {
        id: "provider-a".to_string(),
        name: "Provider A".to_string(),
        source_label: "test".to_string().into(),
        connected: true,
    }];

    let selected = super::shared::selected_entry(&app)
        .expect("a missing selection should fall back to the first entry");
    assert_eq!(selected.key, first_key);
    assert_eq!(root_child_count(&app), 5);
}

#[derive(Default)]
struct CatalogText(Vec<(String, iced::Rectangle)>);
impl iced::advanced::widget::Operation for CatalogText {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation)) {
        operate(self);
    }
    fn text(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        text: &str,
    ) {
        self.0.push((text.into(), bounds));
    }
}

async fn render_catalog(theme: iced::Theme, click: bool) {
    use crate::app::{
        Message,
        message::settings::{AgentsMessage, SettingsMessage},
    };
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, Shell, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Event, Font, Pixels, Rectangle, Size};
    use vw_config_types::agent::{AgentDefinitionConfig, AgentRuntime};
    let mut app = App::new().0;
    app.agents_settings.entries = [
        AgentRuntime::Native,
        AgentRuntime::Acp { backend: "codex".into() },
        AgentRuntime::Acp { backend: "unknown-fixture".into() },
    ]
    .into_iter()
    .enumerate()
    .map(|(i, runtime)| {
        let mut entry = crate::app::state::DelegateAgentSettingsEntry::from_config(
            &format!("fixture-{i}"),
            Some(AgentDefinitionConfig { runtime, ..Default::default() }),
        );
        entry.builtin = true;
        entry.enabled = true;
        entry.label = "中文长名称检查卡片换行与运行引擎图标".into();
        entry
    })
    .collect();
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .unwrap();
    for official in [false, true] {
        for scenario in ["ready", "busy", "creating", "disabled", "worker"] {
            let busy = scenario == "busy";
            let blocked = busy || scenario == "creating";
            let available = !matches!(scenario, "disabled" | "worker");
            app.new_session_creating = scenario == "creating";
            for entry in &mut app.agents_settings.entries {
                entry.enabled = scenario != "disabled";
                entry.builtin = official || available;
                entry.kind = if scenario == "worker" {
                    AgentSettingsEntryKind::BuiltinWorker
                } else {
                    AgentSettingsEntryKind::Custom
                };
            }
            app.agents_settings.catalog_official = official;
            app.agents_settings.catalog_busy = busy;
            let mut element = super::catalog::grid(&app);
            let mut tree = Tree::new(element.as_widget());
            let width = if click { 840 } else { 660 };
            let bounds = Size::new(width as f32, 800.0);
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, bounds),
            );
            let mut texts = CatalogText::default();
            element.as_widget_mut().operate(&mut tree, Layout::new(&node), &renderer, &mut texts);
            let starts = texts.0.iter().filter(|(text, _)| text == "开始会话").collect::<Vec<_>>();
            assert_eq!(
                starts.len(),
                if available { 3 } else { 0 },
                "only eligible cards expose 开始会话"
            );
            let cards = Layout::new(&node).children().collect::<Vec<_>>();
            assert_eq!(cards.len(), 3);
            for card in cards {
                let title = card.children().next().unwrap().children().next().unwrap();
                let header = title.children().next().unwrap();
                let tile = header.children().next().unwrap();
                let icon = tile.children().next().unwrap().bounds();
                assert_eq!(tile.bounds().size(), Size::new(44.0, 44.0));
                assert_eq!(icon.size(), Size::new(26.0, 26.0), "engine identity icon");
                assert_eq!(icon.center(), tile.bounds().center());
            }
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &renderer::Style { text_color: theme.palette().text },
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &Rectangle::with_size(bounds),
            );
            let pixels = renderer.screenshot(Size::new(width, 800), 1.0, Color::TRANSPARENT);
            let directory = std::env::temp_dir().join("shiju-session-catalog");
            std::fs::create_dir_all(&directory).unwrap();
            image::save_buffer(
                directory.join(format!("{theme:?}-{official}-{scenario}-{width}.png")),
                &pixels,
                width,
                800,
                image::ColorType::Rgba8,
            )
            .unwrap();
            if click {
                for (index, (_, bounds)) in starts.iter().enumerate() {
                    let mut messages = Vec::new();
                    for event in [
                        mouse::Event::ButtonPressed(mouse::Button::Left),
                        mouse::Event::ButtonReleased(mouse::Button::Left),
                    ] {
                        element.as_widget_mut().update(
                            &mut tree,
                            &Event::Mouse(event),
                            Layout::new(&node),
                            mouse::Cursor::Available(bounds.center()),
                            &renderer,
                            &mut iced::advanced::clipboard::Null,
                            &mut Shell::new(&mut messages),
                            &Rectangle::with_size(Size::new(840.0, 800.0)),
                        );
                    }
                    if blocked {
                        assert!(messages.is_empty());
                    } else {
                        assert_eq!(messages.len(), 1);
                        assert!(
                            matches!(&messages[0], Message::Settings(SettingsMessage::Agents(AgentsMessage::CatalogLaunch(key))) if key == &format!("fixture-{index}"))
                        );
                    }
                }
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn session_entry_catalog_cards_show_icons_and_start_session() {
    render_catalog(iced::Theme::Light, true).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn session_entry_catalog_cards_render_both_themes() {
    render_catalog(iced::Theme::Light, false).await;
    render_catalog(iced::Theme::Dark, false).await;
}
