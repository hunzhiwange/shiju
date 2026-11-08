//! Scope project model reads to the current selection and serialize writes per project.

use super::{PreferencesScope, gateway_client_endpoint, preferences_scope};
use crate::app::{App, Message};
use iced::Task;
use serde_json::{Map, Value, json};
use std::collections::{HashMap, hash_map::Entry};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};

pub type ChatModelPreferences = (String, bool, Option<String>);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ProjectPreferencesScope {
    gateway: PreferencesScope,
    path: String,
}

impl ProjectPreferencesScope {
    fn new(endpoint: &GatewayEndpoint, path: String) -> Self {
        Self { gateway: preferences_scope(endpoint), path }
    }
}

struct ProjectChatWrite {
    latest: ChatModelPreferences,
    pending_endpoint: Option<GatewayEndpoint>,
}

#[derive(Default)]
pub(super) struct ProjectChatPreferences {
    revision: u64,
    global: Option<(PreferencesScope, ChatModelPreferences)>,
    edited_global: bool,
    writes: HashMap<ProjectPreferencesScope, ProjectChatWrite>,
}

impl ProjectChatPreferences {
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn has_pending_writes(&self) -> bool {
        !self.writes.is_empty()
    }

    fn enqueue(
        &mut self,
        scope: ProjectPreferencesScope,
        endpoint: GatewayEndpoint,
        preferences: ChatModelPreferences,
    ) -> Task<Message> {
        match self.writes.entry(scope.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(ProjectChatWrite {
                    latest: preferences.clone(),
                    pending_endpoint: None,
                });
                Self::put(scope, endpoint, preferences)
            }
            Entry::Occupied(mut entry) => {
                let write = entry.get_mut();
                write.latest = preferences;
                write.pending_endpoint = Some(endpoint);
                Task::none()
            }
        }
    }

    fn put(
        scope: ProjectPreferencesScope,
        endpoint: GatewayEndpoint,
        (model, auto_model, acp_agent): ChatModelPreferences,
    ) -> Task<Message> {
        let path = scope.path.clone();
        Task::perform(
            async move {
                GatewayClient::new(endpoint)?
                    .desktop_project_preferences_put(
                        &path,
                        &model,
                        auto_model,
                        acp_agent.as_deref(),
                    )
                    .await
            },
            move |result| Message::ProjectChatPreferencesSaved(scope, result),
        )
    }

    fn finished(&mut self, scope: ProjectPreferencesScope) -> Task<Message> {
        if let Some(write) = self.writes.get_mut(&scope)
            && let Some(endpoint) = write.pending_endpoint.take()
        {
            return Self::put(scope, endpoint, write.latest.clone());
        }
        self.writes.remove(&scope);
        Task::none()
    }
}

impl App {
    fn chat_model_defaults(&mut self, endpoint: &GatewayEndpoint) -> &mut ChatModelPreferences {
        let state = &mut self.desktop_preferences_persistence.project_chat;
        let gateway = preferences_scope(endpoint);
        let (scope, defaults) = state.global.get_or_insert_with(|| {
            (gateway.clone(), (self.model.clone(), self.auto_model, self.acp_agent.clone()))
        });
        if *scope != gateway {
            *scope = gateway;
            *defaults = ("auto".to_string(), true, None);
            state.edited_global = false;
        }
        defaults
    }

    pub(crate) fn invalidate_project_chat_preferences_read(&mut self) {
        let revision = &mut self.desktop_preferences_persistence.project_chat.revision;
        *revision = revision.wrapping_add(1);
    }

    pub(crate) fn reload_project_chat_preferences(
        &mut self,
        project_changed: bool,
    ) -> Task<Message> {
        self.invalidate_project_chat_preferences_read();
        let endpoint = gateway_client_endpoint();
        let defaults = self.chat_model_defaults(&endpoint).clone();
        if project_changed {
            self.apply_project_chat_preferences(defaults.0, defaults.1, defaults.2);
        }

        let Some(path) = self.project_path.clone() else {
            return Task::none();
        };
        let scope = ProjectPreferencesScope::new(&endpoint, path.clone());
        let state = &self.desktop_preferences_persistence.project_chat;
        if let Some(write) = state.writes.get(&scope) {
            // The server may still return the value preceding our latest PUT.
            let (model, auto_model, acp_agent) = write.latest.clone();
            self.apply_project_chat_preferences(model, auto_model, acp_agent);
            return Task::none();
        }
        let revision = state.revision;
        Task::perform(
            async move { GatewayClient::new(endpoint)?.desktop_project_preferences_get(&path).await },
            move |result| Message::ProjectChatPreferencesLoaded(scope, revision, result),
        )
    }

    pub(crate) fn project_chat_preferences_loaded(
        &mut self,
        scope: ProjectPreferencesScope,
        revision: u64,
        result: Result<Option<ChatModelPreferences>, String>,
    ) -> Task<Message> {
        if self.desktop_preferences_persistence.project_chat.revision != revision
            || self.project_path.as_deref() != Some(scope.path.as_str())
            || scope.gateway != preferences_scope(&gateway_client_endpoint())
        {
            return Task::none();
        }
        match result {
            Ok(Some((model, auto_model, acp_agent))) => {
                self.apply_project_chat_preferences(model, auto_model, acp_agent);
                Task::none()
            }
            Ok(None) => Task::none(),
            Err(error) => {
                tracing::warn!(target: "vw_desktop", %error, project_path = %scope.path, "failed to load project chat preferences");
                Task::none()
            }
        }
    }

    pub(crate) fn save_chat_model_preferences(
        &mut self,
        model: String,
        auto_model: bool,
    ) -> Task<Message> {
        self.invalidate_project_chat_preferences_read();
        let endpoint = gateway_client_endpoint();
        let preferences = (model, auto_model, self.acp_agent.clone());
        *self.chat_model_defaults(&endpoint) = preferences.clone();
        let fields = Map::from_iter([
            ("model".to_owned(), json!(preferences.0)),
            ("auto_model".to_owned(), json!(preferences.1)),
            ("acp_agent".to_owned(), json!(preferences.2)),
        ]);
        let persistence = &mut self.desktop_preferences_persistence;
        persistence.project_chat.edited_global = true;
        let global = persistence.enqueue(endpoint.clone(), fields);
        let project = if let Some(path) = self.project_path.clone() {
            let scope = ProjectPreferencesScope::new(&endpoint, path);
            persistence.project_chat.enqueue(scope, endpoint, preferences)
        } else {
            Task::none()
        };
        Task::batch([global, project])
    }

    pub(crate) fn save_automatic_model_preference(&mut self) -> Task<Message> {
        self.invalidate_project_chat_preferences_read();
        self.auto_model = true;
        self.get_session_runtime_mut("__empty__").auto_model = true;
        let endpoint = gateway_client_endpoint();
        self.chat_model_defaults(&endpoint).1 = true;
        self.desktop_preferences_persistence.project_chat.edited_global = true;
        self.desktop_preferences_persistence
            .enqueue(endpoint, Map::from_iter([("auto_model".to_owned(), Value::Bool(true))]))
    }

    pub(crate) fn apply_bootstrap_chat_preferences(&mut self, cfg: &Value) {
        let endpoint = gateway_client_endpoint();
        self.chat_model_defaults(&endpoint);
        let state = &mut self.desktop_preferences_persistence.project_chat;
        if state.edited_global {
            return;
        }
        let (_, defaults) = state.global.as_mut().expect("chat model defaults initialized");
        if let Some(model) = cfg.get("model").and_then(Value::as_str) {
            defaults.0 = model.to_string();
        }
        if let Some(auto_model) = cfg.get("auto_model").and_then(Value::as_bool) {
            defaults.1 = auto_model;
        }
        if let Some(acp_agent) = cfg.get("acp_agent").and_then(Value::as_str) {
            defaults.2 = Some(acp_agent.to_string());
        }
        if self.project_path.is_none() {
            let (model, auto_model, acp_agent) = defaults.clone();
            self.apply_project_chat_preferences(model, auto_model, acp_agent);
        }
    }

    pub(crate) fn project_chat_preferences_saved(
        &mut self,
        scope: ProjectPreferencesScope,
        result: Result<(), String>,
    ) -> Task<Message> {
        let persistence = &mut self.desktop_preferences_persistence;
        let next = persistence.project_chat.finished(scope);
        persistence.notify_if_idle();
        match result {
            Ok(()) => next,
            Err(error) => {
                Task::batch([next, self.show_error_toast(format!("项目聊天偏好未保存：{error}"))])
            }
        }
    }
}
