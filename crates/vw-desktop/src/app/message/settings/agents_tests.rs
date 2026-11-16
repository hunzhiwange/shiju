//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use super::update;
use crate::app::{
    App,
    message::settings::{AgentsMessage, SettingsMessage},
};

fn test_app() -> App {
    let mut app = App::new().0;
    app.agents_settings.loading = false;
    app.agents_settings.config_ready = true;
    app
}

#[test]
fn agent_unified_selection_catalog_launch_selects_profile_runtime() {
    let mut app = test_app();
    app.navigation_tabs.project_path = Some("/tmp/unified-agent-fixture".into());
    app.agents_settings.entries.push(crate::app::state::DelegateAgentSettingsEntry::from_config(
        "acp-codex",
        Some(config_agent::AgentDefinitionConfig {
            runtime: config_agent_profile::AgentRuntime::Acp { backend: "codex".into() },
            ..Default::default()
        }),
    ));
    let _ =
        update(&mut app, SettingsMessage::Agents(AgentsMessage::CatalogLaunch("acp-codex".into())));
    assert!(app.session_state.active_session_id.is_none());
    assert!(app.session_state.sessions.is_empty());
    assert_eq!(app.current_session_runtime().agent.as_deref(), Some("acp-codex"));
    assert_eq!(app.current_session_runtime().acp_agent.as_deref(), Some("codex"));
}

#[test]
fn session_entry_catalog_launch_preserves_profile_runtime() {
    use config_agent::AgentDefinitionConfig;
    use config_agent_profile::AgentRuntime;
    for runtime in [AgentRuntime::Native, AgentRuntime::Acp { backend: "codex".into() }] {
        let mut app = test_app();
        app.navigation_panels.show_settings = true;
        app.navigation_panels.show_system_settings = true;
        app.navigation_tabs.screen = crate::app::Screen::Usage;
        app.session_state.active_session_id = Some("catalog-old-session".into());
        app.current_session_runtime_mut().agent = Some("old-profile".into());
        let old_id = app.session_state.active_session_id.clone();
        let old_runtime = app.current_session_runtime().clone();
        app.agents_settings.entries.push(
            crate::app::state::DelegateAgentSettingsEntry::from_config(
                "fixture-profile",
                Some(AgentDefinitionConfig { runtime: runtime.clone(), ..Default::default() }),
            ),
        );
        app.navigation_tabs.project_path = None;
        let _ = update(
            &mut app,
            SettingsMessage::Agents(AgentsMessage::CatalogLaunch("fixture-profile".into())),
        );
        assert!(!app.agents_settings.catalog_busy);
        assert!(app.toast_state.active_toast.as_ref().unwrap().message.contains("请先打开项目"));
        assert!(app.navigation_panels.show_settings);
        assert!(app.navigation_panels.show_system_settings);
        assert_eq!(app.navigation_tabs.screen, crate::app::Screen::Usage);
        assert_eq!(app.session_state.active_session_id, old_id);
        assert_eq!(app.current_session_runtime().agent, old_runtime.agent);
        app.navigation_tabs.project_path = Some("/tmp/catalog-fixture".into());
        let _ = update(
            &mut app,
            SettingsMessage::Agents(AgentsMessage::CatalogLaunch("fixture-profile".into())),
        );
        assert!(app.session_state.active_session_id.is_none());
        assert!(app.session_state.sessions.is_empty());
        assert_eq!(app.current_session_runtime().agent.as_deref(), Some("fixture-profile"));
        assert_eq!(
            app.current_session_runtime().acp_agent.as_deref(),
            match runtime {
                AgentRuntime::Native => None,
                AgentRuntime::Acp { .. } => Some("codex"),
            }
        );
        assert!(!app.navigation_panels.show_settings);
        assert!(!app.navigation_panels.show_system_settings);
        assert_eq!(app.navigation_tabs.screen, crate::app::Screen::Project);
        assert_eq!(
            app.session_state.session_runtime_states.get("catalog-old-session").unwrap().agent,
            old_runtime.agent
        );
        assert!(app.session_state.input_text.is_empty());
        assert!(!app.agents_settings.catalog_busy);
    }
}
