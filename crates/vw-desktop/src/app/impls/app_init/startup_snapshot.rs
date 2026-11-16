//! Cross-domain startup assembly; domain states own their local transformations.
use crate::app::state::RuntimePlatform;
use crate::app::{App, Message, Screen, message};
use iced::Task;

impl App {
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) fn load_startup_sessions(&mut self) -> Task<Message> {
        if self.navigation_tabs.screen != Screen::Home
            || self.navigation_tabs.project_path.is_some()
            || self.session_state.active_session_id.is_some()
            || !self.session_state.sessions.is_empty()
        {
            return Task::none();
        }
        let Some(generation) = self.startup_configuration.begin_sessions() else {
            return Task::none();
        };
        self.reload_sessions_for_project(None).map(move |message| match message {
            Message::Project(result) => {
                Message::StartupSessionsLoaded(generation, Box::new(result))
            }
            message => message,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) fn apply_startup_gateway_snapshot(
        &mut self,
        generation: u64,
        snapshot: crate::app::impls::StartupGatewaySnapshot,
    ) -> Task<Message> {
        if !self.startup_configuration.accept(
            generation,
            &snapshot,
            &crate::app::config::gateway_client_endpoint(),
        ) {
            return Task::none();
        }
        let mut tasks = Vec::new();
        if let Some(Ok(system)) = snapshot.system {
            self.apply_system_settings_snapshot(system, false);
        }
        let scope =
            crate::app::config::preferences_scope(&crate::app::config::gateway_client_endpoint());
        let agents_loaded = matches!(&snapshot.agent, Some(Ok(_)));
        if let Some(result) = snapshot.agent {
            match result {
                Ok(agent) => {
                    self.apply_startup_agent_config(&agent.config, &agent.gateway);
                    if !self.startup_configuration.setting_edited(&scope, "acp_agents") {
                        self.session_state.acp_agents = agent.acp_agents;
                    }
                }
                Err(_) => {
                    tracing::debug!(target: "vw_desktop", "agent configuration unavailable during startup")
                }
            }
        }
        if let Some(result) = snapshot.catalog
            && !self.startup_configuration.setting_edited(&scope, "agents_settings")
            && (result.is_ok() || !self.agents_settings.config_ready)
        {
            tasks.push(message::settings::update(
                self,
                message::SettingsMessage::Agents(message::settings::AgentsMessage::ConfigLoaded {
                    project_path: None,
                    result,
                }),
            ));
        }
        if let Some(Ok(tools)) = snapshot.tools {
            tasks.push(message::settings::update(
                self,
                message::SettingsMessage::Agents(message::settings::AgentsMessage::ToolsLoaded(
                    Ok(tools),
                )),
            ));
        }
        let preferences_loaded = matches!(&snapshot.preferences, Some(Ok(_)));
        if let Some(Ok(mut preferences)) = snapshot.preferences {
            self.startup_configuration
                .prepare_preferences_agent(&mut preferences, &self.session_state.acp_agents);
            tasks.push(self.update(Message::StartupAppConfigLoaded(Ok(preferences))));
        } else if agents_loaded
            && let Some(agent) =
                self.startup_configuration.take_pending_agent(&self.session_state.acp_agents)
        {
            self.apply_bootstrap_chat_preferences(&serde_json::json!({"acp_agent": agent}));
        }
        if let Some(Ok(apps)) = snapshot.external_apps {
            let platform = apps.platform.as_deref().and_then(RuntimePlatform::from_gateway_str);
            let selection = self.external_navigation.open_external_app;
            self.apply_external_apps_state(platform, apps.apps);
            if self.desktop_preference_edited(&scope, "open_external_app") {
                self.external_navigation.open_external_app = selection;
            }
        }
        if (agents_loaded || preferences_loaded)
            && self.navigation_tabs.project_path.is_none()
            && self.session_state.active_session_id.is_none()
            && !self.task_board.task_board_create_modal_open
            && !self.task_board.show_task_board
        {
            self.task_board.task_board_last_acp_agent.clone_from(&self.session_state.acp_agent);
            if !self.task_board.task_board_bulk_executor_popover {
                self.task_board.task_board_bulk_acp_agent.clone_from(&self.session_state.acp_agent);
            }
        }
        Task::batch(tasks)
    }
}
