//! Cross-domain startup assembly; domain states own their local transformations.
use crate::app::state::RuntimePlatform;
use crate::app::App;

impl App {
    pub(in crate::app) fn apply_external_apps_state(
        &mut self,
        platform: Option<RuntimePlatform>,
        apps: Vec<(String, bool)>,
    ) {
        desktop_navigation::external_apps_update::apply_external_apps_state(
            &mut self.open_external_platform,
            &mut self.external_navigation.open_external_exists,
            &mut self.external_navigation.open_external_app,
            platform,
            apps,
        );
    }

    pub(crate) fn apply_project_chat_preferences(
        &mut self,
        model: String,
        auto_model: bool,
        acp_agent: Option<String>,
    ) {
        self.session_state.apply_project_defaults(model.clone(), auto_model, acp_agent.clone());

        // Existing sessions keep their own model. Updating the draft defaults must also preserve
        // text and task settings entered while the project GET was in flight.
        let runtime = self.get_session_runtime_mut("__empty__");
        desktop_session::session::runtime::apply_draft_preferences(
            runtime, model, auto_model, acp_agent,
        );
    }

    pub(in crate::app) fn apply_bootstrap_app_config(&mut self, mut cfg: serde_json::Value) {
        self.retain_unedited_desktop_preferences(&mut cfg);
        self.apply_task_pet_preferences(&cfg);
        self.desktop_update.restore_preferences(&cfg);
        self.session_state.restore_access_preferences(&cfg);
        self.session_state.reasoning_effort = self.current_session_runtime().reasoning_effort;
        self.apply_bootstrap_chat_preferences(&cfg);
        desktop_navigation::app_init::apply_preferences(
            &mut self.navigation_panels,
            &mut self.navigation_mode,
            &mut self.navigation_sidebar,
            &mut self.external_navigation,
            &mut self.layout_drag,
            &cfg,
        );
        desktop_settings_providers::app_init::apply_input_preferences(
            &mut self.model_settings,
            &cfg,
        );
        self.file_manager.apply_preferences(&cfg);
        if let Some(visible) = cfg.get("show_terminal").and_then(|v| v.as_bool()) {
            self.terminal.set_visible(visible);
        }
        self.workspace_state.apply_preferences(&cfg);
        self.session_state.restore_history_preferences(&cfg);
        if cfg.get("web_bookmarks").is_some() {
            self.web_navigation.web_bookmarks = super::load_web_bookmarks(&cfg);
        }
        self.git.apply_preferences(&cfg);
        self.task_board.apply_preferences(&cfg);
    }
}

#[cfg(test)]
use crate::app::Message;
