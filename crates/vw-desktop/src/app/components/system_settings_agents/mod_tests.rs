//! Host contracts: settings tab assembly, per-domain error banners and desktop message routing.
//! 验证代理设置测试子模块已正确接入。
//! 各标签页的行为由细分测试覆盖，此处仅防止测试模块装配回归。

use super::AgentManagementPage;
use crate::app::state::{AgentConfigTab, WorkspaceMode};
use crate::app::{App, Message, Screen, message::ViewMessage};

#[test]
fn agents_view_builds_default_loading_and_error_states() {
    let (mut app, _) = crate::app::App::new();

    let _: iced::Element<'_, Message> = super::management_view(&app, AgentManagementPage::Agents);

    app.agents_settings.loading = true;
    app.agents_settings.save_error = Some("unable to save".to_string());
    let _: iced::Element<'_, Message> = super::management_view(&app, AgentManagementPage::Agents);
}

#[test]
fn agent_config_tabs_render_each_existing_settings_page() {
    let (mut app, _) = crate::app::App::new();

    for mode in [WorkspaceMode::Work, WorkspaceMode::Code] {
        app.navigation_mode.workspace_mode = mode;
        let _ = app.update(Message::View(ViewMessage::OpenAgentManagement(None)));
        assert_eq!(app.navigation_tabs.screen, Screen::AgentManagement(AgentManagementPage::Agents));

        for page in AgentManagementPage::ALL {
            let _ = app.update(Message::View(ViewMessage::OpenAgentManagement(Some(page))));
            assert_eq!(app.navigation_tabs.screen, Screen::AgentManagement(page));
            assert!(app.navigation_tabs.screen.shows_workspace_session_sidebar());
            assert!(!app.navigation_panels.show_system_settings);
            assert_eq!(app.agents_settings.active_config_tab, AgentConfigTab::DialogueFlow);
            let _: iced::Element<'_, Message> = app.view();
        }
        assert_eq!(
            app.navigation_tabs.open_tabs.iter().filter(|tab| {
                matches!(tab.screen, Screen::AgentManagement(_))
            }).count(),
            1,
        );
    }
}

#[test]
fn agents_error_banner_follows_agent_tabs() {
    let (mut app, _) = crate::app::App::new();
    app.agents_settings.save_error = Some("agents save failed".to_string());
    assert_eq!(super::tab_save_error(&app, AgentConfigTab::Agents), Some("agents save failed"));

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
        (crate::app::state::AgentConfigTab::Plugins, None),
        (crate::app::state::AgentConfigTab::DialogueFlow, None),
        (crate::app::state::AgentConfigTab::QuickMessages, None),
    ] {
        assert_eq!(super::tab_save_error(&app, tab), expected);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_tabs_capabilities_appearance_agents() {
    use crate::app::components::system_settings_common::tab_appearance_tests;
    use crate::app::message::settings::{AgentConfigTabMessage, AgentsMessage, SettingsMessage};
    tab_appearance_tests::run(
        concat!(module_path!(), "::settings_tabs_capabilities_appearance_agents"),
        || App::new().0,
        |scene, app| {
            app.agents_settings.config_ready = true;
            app.agents_settings.catalog_official = false;
            app.agents_settings.save_error = Some("fixture agent error".into());
            let locale = app.general_settings.desktop_locale;
            let tr = |label| super::tr(locale, label);

            let catalog =
                scene.check(
                    app,
                    super::management_view(app, AgentManagementPage::Agents),
                    &["已启用智能体", "官方智能体"].map(tr),
                    1,
                );
            assert!(matches!(
                catalog,
                SettingsMessage::Agents(AgentsMessage::CatalogTabSelected(true))
            ));
            let _ = app.update(Message::Settings(catalog));
            assert!(app.agents_settings.catalog_official);
            let labels = scene.labels(super::management_view(app, AgentManagementPage::Agents));
            assert!(labels.contains("fixture agent error"));
            for page in AgentManagementPage::ALL {
                assert!(labels.contains(tr(page.label())));
            }

            let names: Vec<_> = super::AGENT_CONFIG_TABS.iter()
                .filter(|tab| crate::app::message::settings::management_page(tab.tab).is_none())
                .map(|tab| tr(tab.label))
                .collect();
            let config = scene.check(app, super::view(app), &names, 1);
            assert!(matches!(
                config,
                SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(
                    AgentConfigTab::Runtime
                ))
            ));
            let _ = app.update(Message::Settings(config));
            let labels = scene.labels(super::view(app));
            assert_eq!(app.agents_settings.active_config_tab, AgentConfigTab::Runtime);
            assert!(!labels.contains("fixture agent error"));
        },
    )
    .await;
}
