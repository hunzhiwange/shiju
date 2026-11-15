use super::config_desktop::{PreferencesScope, preferences_scope, PreferencesMessage as Message};
use iced::Task;
use std::collections::{HashMap, hash_map::Entry};
use gateway_endpoint::GatewayEndpoint;
use vw_gateway_client::GatewayClient;
pub type ChatModelPreferences = (String, bool, Option<String>);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ProjectPreferencesScope {
    pub gateway: PreferencesScope,
    pub path: String,
}

impl ProjectPreferencesScope {
    pub fn new(endpoint: &GatewayEndpoint, path: String) -> Self {
        Self { gateway: preferences_scope(endpoint), path }
    }
}

pub struct ProjectChatWrite {
    pub latest: ChatModelPreferences,
    pending_endpoint: Option<GatewayEndpoint>,
}

#[derive(Default)]
pub struct ProjectChatPreferences {
    pub revision: u64,
    pub global: Option<(PreferencesScope, ChatModelPreferences)>,
    pub edited_global: bool,
    pub selection_override: bool,
    pub writes: HashMap<ProjectPreferencesScope, ProjectChatWrite>,
}

impl ProjectChatPreferences {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn has_pending_writes(&self) -> bool {
        !self.writes.is_empty()
    }

    pub fn enqueue(
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
                gateway_desktop_settings::GatewayDesktopSettingsClient::new(GatewayClient::new(endpoint)?.transport().clone()).desktop_project_preferences_put(
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

    pub fn finished(&mut self, scope: ProjectPreferencesScope) -> Task<Message> {
        if let Some(write) = self.writes.get_mut(&scope)
            && let Some(endpoint) = write.pending_endpoint.take()
        {
            return Self::put(scope, endpoint, write.latest.clone());
        }
        self.writes.remove(&scope);
        Task::none()
    }
}

impl ProjectChatPreferences {
    pub fn defaults(&mut self, endpoint: &GatewayEndpoint, initial: ChatModelPreferences) -> &mut ChatModelPreferences {
        let gateway = preferences_scope(endpoint);
        let (scope, defaults) = self.global.get_or_insert_with(|| (gateway.clone(), initial));
        if *scope != gateway {
            *scope = gateway;
            *defaults = ("auto".to_string(), true, None);
            self.edited_global = false;
        }
        defaults
    }
    pub fn invalidate_read(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.selection_override = true;
    }
    pub fn apply_bootstrap(&mut self, cfg: &serde_json::Value) -> Option<ChatModelPreferences> {
        if self.edited_global { return None; }
        let (_, defaults) = self.global.as_mut().expect("chat model defaults initialized");
        if let Some(model) = cfg.get("model").and_then(serde_json::Value::as_str) { defaults.0 = model.to_string(); }
        if let Some(auto_model) = cfg.get("auto_model").and_then(serde_json::Value::as_bool) { defaults.1 = auto_model; }
        if let Some(acp_agent) = cfg.get("acp_agent").and_then(serde_json::Value::as_str) { defaults.2 = Some(acp_agent.to_string()); }
        (!self.selection_override).then(|| defaults.clone())
    }
}
