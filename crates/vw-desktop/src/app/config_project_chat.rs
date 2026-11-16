//! Scope project model reads to the current selection and serialize writes per project.

use super::gateway_client_endpoint;
use super::{ChatModelPreferences, ProjectPreferencesScope};
use crate::app::{App, Message};
use gateway_endpoint::GatewayEndpoint;
use iced::Task;
use serde_json::Value;
impl App {
    fn chat_model_defaults(&mut self, endpoint: &GatewayEndpoint) -> &mut ChatModelPreferences {
        self.desktop_preferences_persistence
            .project_chat
            .defaults(endpoint, (self.model.clone(), self.auto_model, self.acp_agent.clone()))
    }

    pub(crate) fn invalidate_project_chat_preferences_read(&mut self) {
        self.desktop_preferences_persistence.project_chat.invalidate_read();
    }

    pub(crate) fn reload_project_chat_preferences(
        &mut self,
        project_changed: bool,
    ) -> Task<Message> {
        let read = self.desktop_preferences_persistence.project_chat.reload(
            gateway_client_endpoint(),
            self.navigation_tabs.project_path.clone(),
            project_changed,
            (self.model.clone(), self.auto_model, self.acp_agent.clone()),
        );
        for (model, auto_model, acp_agent) in read.selections {
            self.apply_project_chat_preferences(model, auto_model, acp_agent);
        }
        read.task.map(super::desktop::preferences_message)
    }

    pub(crate) fn project_chat_preferences_loaded(
        &mut self,
        scope: ProjectPreferencesScope,
        revision: u64,
        result: Result<Option<ChatModelPreferences>, String>,
    ) -> Task<Message> {
        if let Some((model, auto_model, acp_agent)) =
            self.desktop_preferences_persistence.project_chat.loaded(
                &gateway_client_endpoint(),
                self.navigation_tabs.project_path.as_deref(),
                scope,
                revision,
                result,
            )
        {
            self.apply_project_chat_preferences(model, auto_model, acp_agent);
        }
        Task::none()
    }

    pub(crate) fn save_chat_model_preferences(
        &mut self,
        model: String,
        auto_model: bool,
    ) -> Task<Message> {
        self.desktop_preferences_persistence
            .save_chat_model_preferences(
                gateway_client_endpoint(),
                self.navigation_tabs.project_path.clone(),
                (self.model.clone(), self.auto_model, self.acp_agent.clone()),
                (model, auto_model, self.acp_agent.clone()),
            )
            .map(super::desktop::preferences_message)
    }

    pub(crate) fn save_automatic_model_preference(&mut self) -> Task<Message> {
        self.auto_model = true;
        self.get_session_runtime_mut("__empty__").auto_model = true;
        self.desktop_preferences_persistence
            .save_automatic_model_preference(
                gateway_client_endpoint(),
                (self.model.clone(), self.auto_model, self.acp_agent.clone()),
            )
            .map(super::desktop::preferences_message)
    }

    pub(crate) fn apply_bootstrap_chat_preferences(&mut self, cfg: &Value) {
        let endpoint = gateway_client_endpoint();
        self.chat_model_defaults(&endpoint);
        if let Some((model, auto_model, acp_agent)) =
            self.desktop_preferences_persistence.project_chat.apply_bootstrap(cfg)
        {
            self.apply_project_chat_preferences(model, auto_model, acp_agent);
        }
    }

    pub(crate) fn project_chat_preferences_saved(
        &mut self,
        scope: ProjectPreferencesScope,
        result: Result<(), String>,
    ) -> Task<Message> {
        let persistence = &mut self.desktop_preferences_persistence;
        let next =
            persistence.project_chat.finished(scope).map(super::desktop::preferences_message);
        persistence.notify_if_idle();
        match result {
            Ok(()) => next,
            Err(error) => {
                Task::batch([next, self.show_error_toast(format!("项目聊天偏好未保存：{error}"))])
            }
        }
    }
}
