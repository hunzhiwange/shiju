//! Catalog transactions and page layout over real domain state.
use super::profile_inputs as inputs;
use super::profile_update::{ProfileFixture, dispatch};

fn catalog_config() -> config_aggregate::Config {
    // Wire-shaped catalog fixture: official identities come from the gateway, not CLI detection.
    let mut config: config_aggregate::Config =
        serde_json::from_value(serde_json::json!({})).unwrap();
    for backend in [
        "auggie",
        "claude",
        "codex",
        "codebuddy",
        "goose",
        "vibe",
        "hermes",
        "snow",
        "copilot",
        "cursor",
        "droid",
        "gemini",
        "iflow",
        "kilocode",
        "kimi",
        "kiro",
        "opencode",
        "openclaw",
        "pi",
        "qoder",
        "qwen",
        "trae",
    ] {
        let mut value = serde_json::to_value(config_agent::DelegateAgentConfig::default()).unwrap();
        let fields = serde_json::json!({"builtin":true, "enabled": backend == "codex", "label":backend,
            "runtime":{"kind":"acp","backend":backend}});
        value.as_object_mut().unwrap().extend(fields.as_object().unwrap().clone());
        config.agents.insert(format!("acp-{backend}"), serde_json::from_value(value).unwrap());
    }
    config
}

#[test]
fn agent_catalog_view_failed_toggle_preserves_snapshot_and_main() {
    use crate::message::AgentsMessage;
    let mut app = ProfileFixture::default();
    app.agents_settings.config_ready = true;
    app.agents_settings.entries =
        crate::state::agent_settings_entries_from_config(&catalog_config());
    dispatch(&mut app, AgentsMessage::CatalogToggle("main".into(), false));
    assert!(!app.agents_settings.catalog_busy);
    dispatch(&mut app, AgentsMessage::CatalogToggle("acp-codex".into(), false));
    assert!(app.agents_settings.catalog_busy);
    dispatch(
        &mut app,
        AgentsMessage::CatalogToggled {
            key: "acp-codex".into(),
            enabled: false,
            result: Err("controlled failure".into()),
        },
    );
    assert!(crate::catalog::entries(&app.agents_settings.entries, false, "codex")[0].enabled);
    dispatch(&mut app, AgentsMessage::CatalogToggle("acp-codex".into(), false));
    dispatch(
        &mut app,
        AgentsMessage::CatalogToggled { key: "acp-codex".into(), enabled: false, result: Ok(()) },
    );
    assert!(crate::catalog::entries(&app.agents_settings.entries, false, "codex").is_empty());
    assert!(!crate::catalog::entries(&app.agents_settings.entries, true, "codex")[0].enabled);
    assert!(app.agents_settings.entries.iter().find(|entry| entry.key == "main").unwrap().enabled);
}

#[test]
fn agent_catalog_view_copy_keeps_profile_with_new_custom_identity() {
    use crate::message::AgentsMessage;
    let mut app = ProfileFixture::default();
    app.agents_settings.config_ready = true;
    app.agents_settings.entries =
        crate::state::agent_settings_entries_from_config(&catalog_config());
    dispatch(&mut app, AgentsMessage::CatalogCopy("acp-codex".into()));
    let mut copied = serde_json::to_value(&catalog_config().agents["acp-codex"]).unwrap();
    copied["key"] = "custom-fixture".into();
    copied["builtin"] = false.into();
    dispatch(&mut app, AgentsMessage::CatalogCopied(Ok(copied)));
    assert!(!app.agents_settings.catalog_busy);
    assert_eq!(app.agents_settings.selected_agent, "custom-fixture");
    let copied = crate::catalog::entries(&app.agents_settings.entries, false, "custom-fixture")[0];
    assert!(!copied.builtin);
    assert!(
        matches!(&copied.runtime, config_agent_profile::AgentRuntime::Acp { backend } if backend == "codex")
    );
    dispatch(&mut app, AgentsMessage::ProfileOpen("acp-codex".into()));
    dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("claude".into()));
    dispatch(&mut app, AgentsMessage::ProfileDelete);
    assert!(!app.agents_settings.profile_saving);
    assert!(matches!(&app.agents_settings.profile_modal.as_ref().unwrap().runtime,
        config_agent_profile::AgentRuntime::Acp { backend } if backend == "codex"));
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_catalog_view_renders_long_chinese_cards_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
    for (theme, name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        let mut app = ProfileFixture::default();
        app.agents_settings.config_ready = true;
        app.agents_settings.entries =
            crate::state::agent_settings_entries_from_config(&catalog_config());
        app.agents_settings.catalog_official = true;
        app.agents_settings.entries[0].label = "中文智能体长名称 English".repeat(8);
        app.agents_settings.entries[0].description = "中文长描述，默认配置与运行引擎。".repeat(16);
        let mut element = inputs::agents(&app);
        let mut renderer =
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                .await
                .unwrap();
        let bounds = Size::new(1400.0, 1000.0);
        let mut tree = Tree::new(element.as_widget());
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
        let pixels = renderer.screenshot(Size::new(1400, 1000), 1.0, Color::TRANSPARENT);
        assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
        image::save_buffer(
            format!("/tmp/shiju-agent-catalog-{name}.png"),
            &pixels,
            1400,
            1000,
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_catalog_keeps_two_columns_and_scrollbar_gutter() {
    use iced::advanced::{layout, renderer::Headless, widget::Tree};
    use iced::{Font, Pixels, Size};
    let mut app = ProfileFixture::default();
    app.agents_settings.config_ready = true;
    app.agents_settings.loading = false;
    app.agents_settings.entries =
        crate::state::agent_settings_entries_from_config(&catalog_config());
    app.agents_settings.catalog_official = true;
    app.agents_settings
        .entries
        .retain(|entry| matches!(entry.key.as_str(), "acp-claude" | "acp-codex" | "acp-gemini"));
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .unwrap();
    for width in [1100.0, 760.0, 640.0] {
        let mut element = inputs::agents(&app);
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(width, 5000.0)),
        );
        let scrollable = &node.children()[2];
        let content = &scrollable.children()[0];
        let grid = &content.children()[0];
        assert_eq!(grid.bounds().x, 7.0);
        assert_eq!(grid.bounds().width, width - 14.0);
        let cards = grid.children();
        assert_eq!(cards.len(), 3);
        let column_width = (grid.bounds().width - 12.0) / 2.0;
        for card in cards {
            assert_eq!(card.bounds().width, column_width);
        }
        assert_eq!(cards[0].bounds().x, 0.0);
        assert_eq!(cards[1].bounds().x, column_width + 12.0);
        assert_eq!(cards[0].bounds().y, cards[1].bounds().y);
        assert_eq!(cards[2].bounds().x, 0.0);
        assert!(cards[2].bounds().y > cards[1].bounds().y);
    }
}
