//! Host contracts: settings tab assembly, per-domain error banners and desktop message routing.
//! 验证代理设置测试子模块已正确接入。
//! 各标签页的行为由细分测试覆盖，此处仅防止测试模块装配回归。

#[test]
fn agents_view_builds_default_loading_and_error_states() {
    let (mut app, _) = crate::app::App::new();

    let _: iced::Element<'_, crate::app::Message> = super::view(&app);

    app.agents_settings.loading = true;
    app.agents_settings.save_error = Some("unable to save".to_string());
    let _: iced::Element<'_, crate::app::Message> = super::view(&app);
}

#[test]
fn agent_config_tabs_render_each_existing_settings_page() {
    let (mut app, _) = crate::app::App::new();

    for tab in [
        crate::app::state::AgentConfigTab::Agents,
        crate::app::state::AgentConfigTab::Channels,
        crate::app::state::AgentConfigTab::Runtime,
        crate::app::state::AgentConfigTab::Acp,
        crate::app::state::AgentConfigTab::Mcp,
        crate::app::state::AgentConfigTab::Skills,
        crate::app::state::AgentConfigTab::Plugins,
        crate::app::state::AgentConfigTab::Reliability,
        crate::app::state::AgentConfigTab::DialogueFlow,
        crate::app::state::AgentConfigTab::QuickMessages,
        crate::app::state::AgentConfigTab::Heartbeat,
    ] {
        app.agents_settings.active_config_tab = tab;
        let _: iced::Element<'_, crate::app::Message> = super::view(&app);
    }
}

#[test]
fn agents_error_banner_follows_agent_tabs() {
    let (mut app, _) = crate::app::App::new();
    app.agents_settings.active_config_tab = crate::app::state::AgentConfigTab::Agents;

    assert_eq!(super::view(&app).as_widget().children().len(), 2);

    app.agents_settings.save_error = Some("agents save failed".to_string());
    assert_eq!(super::active_tab_save_error(&app), Some("agents save failed"));
    assert_eq!(super::view(&app).as_widget().children().len(), 3);

    app.runtime_settings.save_error = Some("runtime save failed".to_string());
    app.acp_settings.save_error = Some("acp save failed".to_string());
    app.reliability_settings.save_error = Some("reliability save failed".to_string());
    app.heartbeat_settings.save_error = Some("heartbeat save failed".to_string());
    for (tab, expected) in [
        (crate::app::state::AgentConfigTab::Runtime, Some("runtime save failed")),
        (crate::app::state::AgentConfigTab::Acp, Some("acp save failed")),
        (crate::app::state::AgentConfigTab::Reliability, Some("reliability save failed")),
        (crate::app::state::AgentConfigTab::Heartbeat, Some("heartbeat save failed")),
        (crate::app::state::AgentConfigTab::Channels, None),
        (crate::app::state::AgentConfigTab::Mcp, None),
        (crate::app::state::AgentConfigTab::Skills, None),
        (crate::app::state::AgentConfigTab::Plugins, None),
        (crate::app::state::AgentConfigTab::DialogueFlow, None),
        (crate::app::state::AgentConfigTab::QuickMessages, None),
    ] {
        app.agents_settings.active_config_tab = tab;
        assert_eq!(super::active_tab_save_error(&app), expected);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_tabs_capabilities_appearance_agents() {
    use crate::app::components::system_settings_common::tab_appearance_tests;
    use crate::app::message::settings::{AgentConfigTabMessage, AgentsMessage, SettingsMessage};
    use crate::app::state::AgentConfigTab;
    use crate::app::{App, Message};

    tab_appearance_tests::run(
        concat!(module_path!(), "::settings_tabs_capabilities_appearance_agents"),
        || App::new().0,
        |scene, app| {
            app.agents_settings.config_ready = true;
            app.agents_settings.active_config_tab = AgentConfigTab::Agents;
            app.agents_settings.catalog_official = false;
            app.agents_settings.save_error = Some("fixture agent error".into());
            let locale = app.desktop_locale;
            let tr = |label| super::tr(locale, label);

            let catalog =
                scene.check(app, super::view(app), &["已启用智能体", "官方智能体"].map(tr), 1);
            assert!(matches!(
                catalog,
                SettingsMessage::Agents(AgentsMessage::CatalogTabSelected(true))
            ));
            let _ = app.update(Message::Settings(catalog));
            assert!(app.agents_settings.catalog_official);
            assert!(scene.labels(super::view(app)).contains("fixture agent error"));

            let names: Vec<_> = super::AGENT_CONFIG_TABS.iter().map(|tab| tr(tab.label)).collect();
            let config = scene.check(app, super::view(app), &names, 1);
            assert!(matches!(
                config,
                SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(
                    AgentConfigTab::Channels
                ))
            ));
            let _ = app.update(Message::Settings(config));
            let labels = scene.labels(super::view(app));
            assert!(labels.contains(tr("通道配置")));
            assert!(!labels.contains("fixture agent error"));
        },
    )
    .await;
}
