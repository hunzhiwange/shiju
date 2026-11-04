use super::catalog;
use crate::app::state::{AgentsSettingsState, DelegateAgentSettingsEntry};

#[test]
fn agent_catalog_view_shares_enabled_state_across_tabs() {
    let config = catalog_config();
    let mut settings = AgentsSettingsState::default();
    settings.entries = crate::app::state::agent_settings_entries_from_config(&config);
    assert_eq!(
        catalog::entries(&settings, true, "").iter().filter(|e| e.key.starts_with("acp-")).count(),
        22
    );
    let key = "acp-codex";
    settings.entries.iter_mut().find(|e| e.key == key).unwrap().enabled = true;
    let enabled = catalog::entries(&settings, false, "codex");
    let official = catalog::entries(&settings, true, "codex");
    assert!(std::ptr::eq(enabled[0], official[0]));
    settings.entries.iter_mut().find(|e| e.key == key).unwrap().enabled = false;
    assert!(catalog::entries(&settings, false, "codex").is_empty());
    assert!(!catalog::entries(&settings, true, "codex")[0].enabled);
}

#[test]
fn agent_catalog_view_keeps_disabled_custom_entries_manageable() {
    let mut settings = AgentsSettingsState::default();
    let mut custom = DelegateAgentSettingsEntry::from_config("my-agent", None);
    custom.enabled = false;
    custom.label = "中文资料".into();
    settings.entries.push(custom);
    assert_eq!(catalog::entries(&settings, false, "中文资料")[0].key, "my-agent");
    assert!(catalog::entries(&settings, true, "中文资料").is_empty());
    assert!(catalog::entries(&settings, false, "missing").is_empty());
}

fn catalog_config() -> vw_config_types::config::Config {
    // Wire-shaped catalog fixture: official identities come from the gateway, not CLI detection.
    let mut config: vw_config_types::config::Config =
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
        let mut value =
            serde_json::to_value(vw_config_types::agent::DelegateAgentConfig::default()).unwrap();
        let fields = serde_json::json!({"builtin":true, "enabled": backend == "codex", "label":backend,
            "runtime":{"kind":"acp","backend":backend}});
        value.as_object_mut().unwrap().extend(fields.as_object().unwrap().clone());
        config.agents.insert(format!("acp-{backend}"), serde_json::from_value(value).unwrap());
    }
    config
}

fn dispatch(app: &mut crate::app::App, message: crate::app::message::settings::AgentsMessage) {
    let _ = crate::app::message::settings::update(
        app,
        crate::app::message::settings::SettingsMessage::Agents(message),
    );
}

#[test]
fn agent_catalog_view_failed_toggle_preserves_snapshot_and_main() {
    use crate::app::message::settings::AgentsMessage;
    let mut app = crate::app::App::new().0;
    app.agents_settings.config_ready = true;
    app.agents_settings.entries =
        crate::app::state::agent_settings_entries_from_config(&catalog_config());
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
    assert!(catalog::entries(&app.agents_settings, false, "codex")[0].enabled);
    dispatch(&mut app, AgentsMessage::CatalogToggle("acp-codex".into(), false));
    dispatch(
        &mut app,
        AgentsMessage::CatalogToggled { key: "acp-codex".into(), enabled: false, result: Ok(()) },
    );
    assert!(catalog::entries(&app.agents_settings, false, "codex").is_empty());
    assert!(!catalog::entries(&app.agents_settings, true, "codex")[0].enabled);
    assert!(app.agents_settings.entries.iter().find(|entry| entry.key == "main").unwrap().enabled);
}

#[test]
fn agent_catalog_view_copy_keeps_profile_with_new_custom_identity() {
    use crate::app::message::settings::AgentsMessage;
    let mut app = crate::app::App::new().0;
    app.agents_settings.config_ready = true;
    app.agents_settings.entries =
        crate::app::state::agent_settings_entries_from_config(&catalog_config());
    dispatch(&mut app, AgentsMessage::CatalogCopy("acp-codex".into()));
    let mut copied = serde_json::to_value(&catalog_config().agents["acp-codex"]).unwrap();
    copied["key"] = "custom-fixture".into();
    copied["builtin"] = false.into();
    dispatch(&mut app, AgentsMessage::CatalogCopied(Ok(copied)));
    assert!(!app.agents_settings.catalog_busy);
    assert_eq!(app.agents_settings.selected_agent, "custom-fixture");
    let copied = catalog::entries(&app.agents_settings, false, "custom-fixture")[0];
    assert!(!copied.builtin);
    assert!(
        matches!(&copied.runtime, vw_config_types::agent::AgentRuntime::Acp { backend } if backend == "codex")
    );
    dispatch(&mut app, AgentsMessage::ProfileOpen("acp-codex".into()));
    dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("claude".into()));
    dispatch(&mut app, AgentsMessage::ProfileDelete);
    assert!(!app.agents_settings.profile_saving);
    assert!(matches!(&app.agents_settings.profile_modal.as_ref().unwrap().runtime,
        vw_config_types::agent::AgentRuntime::Acp { backend } if backend == "codex"));
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_catalog_view_renders_long_chinese_cards_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
    for (theme, name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        let mut app = crate::app::App::new().0;
        app.agents_settings.config_ready = true;
        app.agents_settings.entries =
            crate::app::state::agent_settings_entries_from_config(&catalog_config());
        app.agents_settings.catalog_official = true;
        app.agents_settings.entries[0].label = "中文智能体长名称 English".repeat(8);
        app.agents_settings.entries[0].description = "中文长描述，默认配置与运行引擎。".repeat(16);
        let mut element = super::view(&app);
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

#[test]
fn agent_catalog_view_launch_preserves_selected_stable_identity_without_acp_picker() {
    use crate::app::message::settings::AgentsMessage;
    let mut app = crate::app::App::new().0;
    app.agents_settings.config_ready = true;
    app.agents_settings.entries =
        crate::app::state::agent_settings_entries_from_config(&catalog_config());
    app.project_path = Some("/tmp/catalog-project".into());
    app.agents_settings.project_path = app.project_path.clone();
    app.acp_agent = None;
    dispatch(&mut app, AgentsMessage::CatalogLaunch("acp-codex".into()));
    assert!(app.agents_settings.catalog_busy);
    let info = serde_json::from_value(serde_json::json!({
        "id":"catalog-session", "slug":"catalog-session", "projectID":"project",
        "directory":"/tmp/catalog-project", "title":"Catalog", "version":"1",
        "time":{"created":1,"updated":1}
    }))
    .unwrap();
    dispatch(
        &mut app,
        AgentsMessage::CatalogSessionCreated { key: "acp-codex".into(), result: Ok(info) },
    );
    assert_eq!(app.active_session_id.as_deref(), Some("catalog-session"));
    assert_eq!(app.current_session_runtime().agent.as_deref(), Some("acp-codex"));
    assert!(!app.show_settings);
    assert!(!app.agents_settings.catalog_busy);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_config_cleanup_catalog_keeps_three_columns_and_scrollbar_gutter() {
    use iced::advanced::{layout, renderer::Headless, widget::Tree};
    use iced::{Font, Pixels, Size};
    let mut app = crate::app::App::new().0;
    app.agents_settings.config_ready = true;
    app.agents_settings.loading = false;
    app.agents_settings.entries =
        crate::app::state::agent_settings_entries_from_config(&catalog_config());
    app.agents_settings.catalog_official = true;
    app.agents_settings.entries.retain(|entry| entry.key != "acp-trae");
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .unwrap();
    for width in [1100.0, 760.0] {
        let mut element = super::agents_view(&app);
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(width, 5000.0)),
        );
        let scrollable = &node.children()[3];
        let content = &scrollable.children()[0];
        let grid = &content.children()[0];
        assert_eq!(grid.bounds().x, 7.0);
        assert_eq!(grid.bounds().width, width - 14.0);
        let rows = grid.children();
        assert!(rows.len() > 1);
        let expected = rows[0].children()[0].bounds().width;
        for row in rows {
            assert_eq!(row.children().len(), 3, "last row must keep empty slots");
            for card in row.children() {
                assert!((card.bounds().width - expected).abs() < 0.01);
            }
        }
    }
}
