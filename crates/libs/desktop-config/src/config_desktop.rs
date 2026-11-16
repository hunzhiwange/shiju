use iced::Task;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet, hash_map::Entry};
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::watch;
use gateway_endpoint::GatewayEndpoint;
use vw_gateway_client::GatewayClient;
use vw_api_transport::id::TenantScope;
use crate::config_project_chat as project_chat;
use crate::config_system_settings::SystemSettingsSection;
pub use project_chat::{ChatModelPreferences, ProjectPreferencesScope};
pub enum PreferencesMessage {
    ProjectChatPreferencesLoaded(ProjectPreferencesScope, u64, Result<Option<ChatModelPreferences>, String>),
    DesktopPreferencesSaved(PreferencesScope, Result<(), String>),
    TaskPetSizeSaved(PreferencesScope, Result<(), String>),
    SystemSettingsSaved(PreferencesScope, SystemSettingsSection, Result<(), String>),
    ProjectChatPreferencesSaved(ProjectPreferencesScope, Result<(), String>),
}
use PreferencesMessage as Message;
pub type PreferencesScope = (String, TenantScope);

pub fn preferences_scope(endpoint: &GatewayEndpoint) -> PreferencesScope {
    (endpoint.base_url().to_ascii_lowercase(), endpoint.tenant_scope().clone())
}

const REASONING_EFFORTS: &str = "session_reasoning_efforts";

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum PatchTarget {
    Desktop,
    System(SystemSettingsSection),
}

struct PreferencesPatch {
    endpoint: GatewayEndpoint,
    fields: Map<String, Value>,
    companion_size_save: bool,
}

#[derive(Default)]
pub struct DesktopPreferencesPersistence {
    // Each entry represents an in-flight PATCH and any edits that arrived after it started.
    in_flight: HashMap<(PreferencesScope, PatchTarget), Option<PreferencesPatch>>,
    pub edited_desktop_fields: HashMap<PreferencesScope, HashSet<String>>,
    pub edited_system_settings: HashMap<PreferencesScope, HashSet<SystemSettingsSection>>,
    pub dialogue_flow_load_revision: u64,
    pub project_chat: project_chat::ProjectChatPreferences,
    #[cfg(not(target_arch = "wasm32"))]
    idle: Option<watch::Sender<bool>>,
}

impl DesktopPreferencesPersistence {
    pub fn enqueue(&mut self, endpoint: GatewayEndpoint, fields: Map<String, Value>) -> Task<Message> {
        self.edited_desktop_fields
            .entry(preferences_scope(&endpoint))
            .or_default()
            .extend(fields.keys().filter(|key| key.as_str() != REASONING_EFFORTS).cloned());
        self.enqueue_patch(endpoint, PatchTarget::Desktop, fields, false)
    }

    pub fn enqueue_system_settings(
        &mut self,
        endpoint: GatewayEndpoint,
        section: SystemSettingsSection,
        fields: Map<String, Value>,
    ) -> Task<Message> {
        self.edited_system_settings
            .entry(preferences_scope(&endpoint))
            .or_default()
            .insert(section);
        self.enqueue_patch(endpoint, PatchTarget::System(section), fields, false)
    }

    pub fn has_system_settings_write(
        &self,
        scope: &PreferencesScope,
        section: SystemSettingsSection,
    ) -> bool {
        self.in_flight.contains_key(&(scope.clone(), PatchTarget::System(section)))
    }

    pub fn enqueue_patch(
        &mut self,
        endpoint: GatewayEndpoint,
        target: PatchTarget,
        mut fields: Map<String, Value>,
        companion_size_save: bool,
    ) -> Task<Message> {
        let scope = preferences_scope(&endpoint);
        match self.in_flight.entry((scope.clone(), target)) {
            Entry::Vacant(entry) => {
                entry.insert(None);
                Self::send(
                    scope,
                    target,
                    PreferencesPatch { endpoint, fields, companion_size_save },
                )
            }
            Entry::Occupied(mut entry) => {
                if let Some(pending) = entry.get_mut() {
                    pending.endpoint = endpoint;
                    pending.companion_size_save |= companion_size_save;
                    if target == PatchTarget::Desktop
                        && let Some(Value::Object(edits)) = fields.get_mut(REASONING_EFFORTS)
                        && let Some(Value::Object(mut earlier)) =
                            pending.fields.remove(REASONING_EFFORTS)
                    {
                        earlier.append(edits);
                        *edits = earlier;
                    }
                    pending.fields.extend(fields);
                } else {
                    entry.insert(Some(PreferencesPatch { endpoint, fields, companion_size_save }));
                }
                Task::none()
            }
        }
    }

    fn send(
        scope: PreferencesScope,
        target: PatchTarget,
        patch: PreferencesPatch,
    ) -> Task<Message> {
        let companion_size_save = patch.companion_size_save;
        Task::perform(
            async move {
                let client = GatewayClient::new(patch.endpoint)?;
                if let PatchTarget::System(_) = target {
                    return gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).desktop_system_settings_patch(&patch.fields).await;
                }
                let mut fields = patch.fields;
                let mut reasoning_result = Ok(());
                if let Some(Value::Object(edits)) = fields.get_mut(REASONING_EFFORTS) {
                    // PATCH replaces this entire object. Preserve all other sessions, including
                    // values this desktop cannot decode.
                    match gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).desktop_preferences_get().await {
                        Ok(mut current) => {
                            if let Some(Value::Object(mut saved)) =
                                current.get_mut(REASONING_EFFORTS).map(Value::take)
                            {
                                saved.append(edits);
                                *edits = saved;
                            }
                        }
                        Err(error) => {
                            fields.remove(REASONING_EFFORTS);
                            reasoning_result = Err(error);
                        }
                    }
                }
                if !fields.is_empty() {
                    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).desktop_preferences_patch(&Value::Object(fields)).await?;
                }
                reasoning_result
            },
            move |result| match target {
                PatchTarget::Desktop if companion_size_save => {
                    Message::TaskPetSizeSaved(scope, result)
                }
                PatchTarget::Desktop => Message::DesktopPreferencesSaved(scope, result),
                PatchTarget::System(section) => {
                    Message::SystemSettingsSaved(scope, section, result)
                }
            },
        )
    }

    pub fn enqueue_companion_size(&mut self, endpoint: GatewayEndpoint, snapshot: Value) -> Task<Message> {
        self.edited_desktop_fields.entry(preferences_scope(&endpoint)).or_default().insert("mini_companion".into());
        self.enqueue_patch(endpoint, PatchTarget::Desktop, Map::from_iter([("mini_companion".into(), snapshot)]), true)
    }

    pub fn save_chat_model_preferences(
        &mut self,
        endpoint: GatewayEndpoint,
        path: Option<String>,
        initial: ChatModelPreferences,
        preferences: ChatModelPreferences,
    ) -> Task<Message> {
        self.project_chat.invalidate_read();
        *self.project_chat.defaults(&endpoint, initial) = preferences.clone();
        self.project_chat.edited_global = true;
        let fields = Map::from_iter([
            ("model".into(), serde_json::json!(preferences.0)),
            ("auto_model".into(), serde_json::json!(preferences.1)),
            ("acp_agent".into(), serde_json::json!(preferences.2)),
        ]);
        let global = self.enqueue(endpoint.clone(), fields);
        let project = if let Some(path) = path {
            self.project_chat.enqueue(
                ProjectPreferencesScope::new(&endpoint, path),
                endpoint,
                preferences,
            )
        } else {
            Task::none()
        };
        Task::batch([global, project])
    }

    pub fn save_automatic_model_preference(
        &mut self,
        endpoint: GatewayEndpoint,
        initial: ChatModelPreferences,
    ) -> Task<Message> {
        self.project_chat.invalidate_read();
        self.project_chat.defaults(&endpoint, initial).1 = true;
        self.project_chat.edited_global = true;
        self.enqueue(endpoint, Map::from_iter([("auto_model".into(), Value::Bool(true))]))
    }

    pub fn system_settings_save_feedback(&self, scope: &PreferencesScope, current: &PreferencesScope, section: SystemSettingsSection, failed: bool, has_save_error: bool) -> SystemSettingsSaveFeedback {
        if section == SystemSettingsSection::GatewayClient {
            return if failed && scope == current && !self.has_system_settings_write(scope, section) && !has_save_error {
                SystemSettingsSaveFeedback::GatewaySyncFailed
            } else { SystemSettingsSaveFeedback::None };
        }
        if section == SystemSettingsSection::DialogueFlow && scope == current {
            return SystemSettingsSaveFeedback::DialogueFlow;
        }
        if failed { SystemSettingsSaveFeedback::Failed } else { SystemSettingsSaveFeedback::None }
    }

    pub fn desktop_preference_edited(&self, scope: &PreferencesScope, key: &str) -> bool {
        self.edited_desktop_fields.get(scope).is_some_and(|fields| fields.contains(key))
    }

    pub fn retain_unedited_desktop_preferences(&self, scope: &PreferencesScope, cfg: &mut Value) {
        if let Some(fields) = cfg.as_object_mut()
            && let Some(edited) = self.edited_desktop_fields.get(scope)
        {
            fields.retain(|key, _| !edited.contains(key));
        }
    }

    pub fn system_settings_edited(
        &self,
        scope: &PreferencesScope,
        section: SystemSettingsSection,
    ) -> bool {
        self.edited_system_settings.get(scope).is_some_and(|sections| sections.contains(&section))
    }

    pub fn enqueue_gateway_client_snapshot(
        &mut self,
        config: config_ui::GatewayClientSystemSettingsConfig,
    ) -> (GatewayEndpoint, Task<Message>) {
        let endpoint = crate::config_gateway::gateway_client_endpoint_from_config(&config);
        let mut snapshot = serde_json::json!(config);
        // Object merges must explicitly clear credentials omitted by serialization.
        for key in ["skey", "bearer_token", "username", "password"] {
            if snapshot.get(key).is_none() {
                snapshot[key] = Value::Null;
            }
        }
        let task = self.enqueue_system_settings(
            endpoint.clone(),
            SystemSettingsSection::GatewayClient,
            Map::from_iter([("gateway_client".to_string(), snapshot)]),
        );
        (endpoint, task)
    }

    pub fn finished(&mut self, scope: PreferencesScope, target: PatchTarget) -> Task<Message> {
        let key = (scope.clone(), target);
        if let Some(pending) = self.in_flight.get_mut(&key).and_then(Option::take) {
            return Self::send(scope, target, pending);
        }
        self.in_flight.remove(&key);
        self.notify_if_idle();
        Task::none()
    }

    pub fn finish_system_settings_save(
        &mut self,
        scope: PreferencesScope,
        section: SystemSettingsSection,
    ) -> Task<Message> {
        self.finished(scope, PatchTarget::System(section))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn is_idle(&self) -> bool {
        self.in_flight.is_empty() && !self.project_chat.has_pending_writes()
    }

    pub fn notify_if_idle(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if self.is_idle()
            && let Some(idle) = self.idle.take()
        {
            idle.send_replace(true);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn wait_for_idle(&mut self) -> Option<watch::Receiver<bool>> {
        if self.is_idle() {
            return None;
        }
        let idle = self.idle.get_or_insert_with(|| watch::channel(false).0);
        Some(idle.subscribe())
    }
}

use crate::access::run_gateway_call;
impl crate::ConfigAccess {
/// 公开函数，执行 load_app_config_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn load_app_config_async(&self) -> Result<serde_json::Value, String> {
    let client = self.gateway_client()?;
    let value = gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).desktop_preferences_get().await?;
    Ok(if value.is_object() { value } else { serde_json::json!({}) })
}
/// 公开函数，执行 save_app_config_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn save_app_config_async(&self, v: serde_json::Value) -> Result<(), String> {
    let patch = if v.is_object() { v } else { serde_json::json!({}) };
    let client = self.gateway_client()?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).desktop_preferences_patch(&patch).await.map(|_| ())
}
/// 公开函数，执行 update_agents_compat_registry_result_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn update_agents_compat_registry_result_async(&self,
    update: impl FnOnce(&mut serde_json::Map<String, serde_json::Value>),
) -> Result<(), String> {
    let mut cfg = self.load_app_config_async().await?;
    if !cfg.is_object() {
        cfg = serde_json::json!({});
    }

    let Some(root) = cfg.as_object_mut() else {
        return Err("desktop preferences root is not an object".to_string());
    };

    let agent_value = root.entry("agent".to_string()).or_insert_with(|| serde_json::json!({}));
    if !agent_value.is_object() {
        *agent_value = serde_json::json!({});
    }

    let Some(agent_map) = agent_value.as_object_mut() else {
        return Err("desktop preferences agent section is not an object".to_string());
    };

    update(agent_map);
    self.save_app_config_async(cfg).await
}
/// 公开函数，执行 load_app_config 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn load_app_config(&self) -> serde_json::Value {
    let result = run_gateway_call(self.load_app_config_async());
    match result {
        Ok(value) if value.is_object() => value,
        Ok(_) => serde_json::json!({}),
        Err(err) => {
            if gateway_http::is_gateway_unavailable_error(&err) {
                tracing::debug!(target: "vw_desktop", error = %err, "failed to load desktop preferences via gateway");
            } else {
                tracing::warn!(target: "vw_desktop", error = %err, "failed to load desktop preferences via gateway");
            }
            serde_json::json!({})
        }
    }
}
/// 公开函数，执行 save_app_config 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn save_app_config(&self, v: &serde_json::Value) {
    if let Err(err) = run_gateway_call(self.save_app_config_async(v.clone())) {
        if gateway_http::is_gateway_unavailable_error(&err) {
            tracing::debug!(target: "vw_desktop", error = %err, "failed to save desktop preferences via gateway");
        } else {
            tracing::warn!(target: "vw_desktop", error = %err, "failed to save desktop preferences via gateway");
        }
    }
}
/// 公开函数，执行 set_config_field 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn set_config_field(&self, key: &str, value: serde_json::Value) {
    let mut cfg = self.load_app_config();
    if let Some(obj) = cfg.as_object_mut() {
        obj.insert(key.to_string(), value);
    } else {
        cfg = serde_json::json!({ key: value });
    }
    self.save_app_config(&cfg);
}
/// 公开函数，执行 load_project_chat_preferences 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub fn load_project_chat_preferences(&self,
    _project_path: &str,
) -> Option<(String, bool, Option<String>)> {
    None
}
/// 公开函数，执行 save_project_chat_preferences 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub fn save_project_chat_preferences(&self,
    _project_path: &str,
    _model: &str,
    _auto_model: bool,
    _acp_agent: Option<&str>,
) {
}
/// 公开函数，执行 load_project_chat_preferences 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub fn load_project_chat_preferences(&self, project_path: &str) -> Option<(String, bool, Option<String>)> {
    let result = run_gateway_call(async {
        let client = self.gateway_client()?;
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).desktop_project_preferences_get(project_path).await
    });
    match result {
        Ok(value) => value,
        Err(err) => {
            if gateway_http::is_gateway_unavailable_error(&err) {
                tracing::debug!(target: "vw_desktop", error = %err, project_path, "failed to load project chat preferences via gateway");
            } else {
                tracing::warn!(target: "vw_desktop", error = %err, project_path, "failed to load project chat preferences via gateway");
            }
            None
        }
    }
}
/// 公开函数，执行 load_project_chat_preferences_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn load_project_chat_preferences_async(&self,
    project_path: &str,
) -> Result<Option<(String, bool, Option<String>)>, String> {
    let client = self.gateway_client()?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).desktop_project_preferences_get(project_path).await
}
/// 公开函数，执行 save_project_chat_preferences 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub fn save_project_chat_preferences(&self,
    project_path: &str,
    model: &str,
    auto_model: bool,
    acp_agent: Option<&str>,
) {
    if let Err(err) = run_gateway_call(async {
        let client = self.gateway_client()?;
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).desktop_project_preferences_put(project_path, model, auto_model, acp_agent).await
    }) {
        tracing::warn!(target: "vw_desktop", error = %err, project_path, "failed to save project chat preferences via gateway");
    }
}
/// 公开函数，执行 save_project_chat_preferences_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn save_project_chat_preferences_async(&self,
    project_path: &str,
    model: &str,
    auto_model: bool,
    acp_agent: Option<&str>,
) -> Result<(), String> {
    let client = self.gateway_client()?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).desktop_project_preferences_put(project_path, model, auto_model, acp_agent).await
}
/// 公开函数，执行 load_json_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub fn load_json_tool_content(&self) -> String {
    String::new()
}
/// 公开函数，执行 save_json_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub fn save_json_tool_content(&self, _content: &str) {}
/// 公开函数，执行 load_sql_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub fn load_sql_tool_content(&self) -> String {
    String::new()
}
/// 公开函数，执行 save_sql_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub fn save_sql_tool_content(&self, _content: &str) {}
/// 公开函数，执行 load_html_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub fn load_html_tool_content(&self) -> String {
    String::new()
}
/// 公开函数，执行 save_html_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub fn save_html_tool_content(&self, _content: &str) {}
/// 公开函数，执行 load_json_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub fn load_json_tool_content(&self) -> String {
    self.load_tool_content("json")
}
/// 公开函数，执行 save_json_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub fn save_json_tool_content(&self, content: &str) {
    self.save_tool_content("json", content);
}
/// 公开函数，执行 save_json_tool_content_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn save_json_tool_content_async(&self, content: &str) -> Result<(), String> {
    self.save_tool_content_async("json", content).await
}
/// 公开函数，执行 load_sql_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub fn load_sql_tool_content(&self) -> String {
    self.load_tool_content("sql")
}
/// 公开函数，执行 save_sql_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub fn save_sql_tool_content(&self, content: &str) {
    self.save_tool_content("sql", content);
}
/// 公开函数，执行 save_sql_tool_content_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn save_sql_tool_content_async(&self, content: &str) -> Result<(), String> {
    self.save_tool_content_async("sql", content).await
}
/// 公开函数，执行 load_html_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub fn load_html_tool_content(&self) -> String {
    self.load_tool_content("html")
}
/// 公开函数，执行 save_html_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub fn save_html_tool_content(&self, content: &str) {
    self.save_tool_content("html", content);
}
/// 公开函数，执行 save_html_tool_content_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn save_html_tool_content_async(&self, content: &str) -> Result<(), String> {
    self.save_tool_content_async("html", content).await
}
#[cfg(not(target_arch = "wasm32"))]
pub fn load_tool_content(&self, tool_type: &str) -> String {
    let result = run_gateway_call(async {
        let client = self.gateway_client()?;
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).desktop_tool_content_get(tool_type).await
    });
    match result {
        Ok(content) => content,
        Err(err) => {
            if gateway_http::is_gateway_unavailable_error(&err) {
                tracing::debug!(target: "vw_desktop", error = %err, tool_type, "failed to load tool content via gateway");
            } else {
                tracing::warn!(target: "vw_desktop", error = %err, tool_type, "failed to load tool content via gateway");
            }
            String::new()
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub fn save_tool_content(&self, tool_type: &str, content: &str) {
    if let Err(err) = run_gateway_call(async {
        let client = self.gateway_client()?;
        gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).desktop_tool_content_put(tool_type, content).await
    }) {
        tracing::warn!(target: "vw_desktop", error = %err, tool_type, "failed to save tool content via gateway");
    }
}
pub async fn save_tool_content_async(&self, tool_type: &str, content: &str) -> Result<(), String> {
    let client = self.gateway_client()?;
    gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).desktop_tool_content_put(tool_type, content).await
}
}

/// 独立解码每个会话，避免未知值影响其它会话的偏好恢复。
pub fn session_reasoning_efforts_from_config(
    cfg: &serde_json::Value,
) -> std::collections::HashMap<String, Option<vw_api_chat_identity::ReasoningEffortDto>>
{
    cfg.get(REASONING_EFFORTS)
        .and_then(serde_json::Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(id, value)| {
            serde_json::from_value(value.clone()).ok().map(|effort| (id.clone(), effort))
        })
        .collect()
}

/// Cross-domain feedback selected after the queued save has advanced.
pub enum SystemSettingsSaveFeedback { None, GatewaySyncFailed, DialogueFlow, Failed }

impl crate::ConfigAccess {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn update_agents_compat_registry_async(&self, update: impl FnOnce(&mut Map<String, Value>) + Send + 'static) -> Task<crate::ConfigSaved> {
        let access = self.clone();
        crate::spawn_gateway_task("agents_compat", async move { access.update_agents_compat_registry_result_async(update).await })
    }
    #[cfg(target_arch = "wasm32")]
    pub fn update_agents_compat_registry_async(&self, update: impl FnOnce(&mut Map<String, Value>) + 'static) -> Task<crate::ConfigSaved> {
        let access = self.clone();
        crate::spawn_gateway_task("agents_compat", async move { access.update_agents_compat_registry_result_async(update).await })
    }
}
