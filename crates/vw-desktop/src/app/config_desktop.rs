//! 通过网关读写桌面偏好配置。
//! 本模块把历史本地配置调用收敛到显式的桌面偏好接口，并在网关失败时提供安全空配置回退。

use super::gateway::{
    gateway_client_endpoint,
};
use crate::app::{App, Message};
use iced::Task;
use serde_json::{Map, Value};
pub use desktop_config::config_desktop::{PatchTarget, DesktopPreferencesPersistence, PreferencesScope, preferences_scope, ChatModelPreferences, ProjectPreferencesScope};
#[path = "config_project_chat.rs"]
mod project_chat;
pub(super) fn preferences_message(message: desktop_config::config_desktop::PreferencesMessage) -> Message {
    use desktop_config::config_desktop::PreferencesMessage as Local;
    match message {
        Local::ProjectChatPreferencesLoaded(scope, revision, result) => Message::ProjectChatPreferencesLoaded(scope, revision, result),
        Local::DesktopPreferencesSaved(scope, result) => Message::DesktopPreferencesSaved(scope, result),
        Local::TaskPetSizeSaved(scope, result) => Message::View(crate::app::message::ViewMessage::TaskPetSizeSaved(scope, result)),
        Local::SystemSettingsSaved(scope, section, result) => Message::SystemSettingsSaved(scope, section, result),
        Local::ProjectChatPreferencesSaved(scope, result) => Message::ProjectChatPreferencesSaved(scope, result),
    }
}
impl App {
    pub(crate) fn set_task_pet_scale(&mut self, scale: f32) -> Result<(), &'static str> {
        desktop_task_board::pet_state::set_task_pet_scale(&mut self.pet_state.task_pet_scale, scale)
    }

    pub(crate) fn apply_task_pet_preferences(&mut self, cfg: &Value) {
        desktop_task_board::pet_state::apply_task_pet_preferences(
            cfg,
            &mut self.pet_state.task_pet_avatar_kind,
            &mut self.pet_state.task_pet_scale,
            &mut self.companion_navigation.task_pet_visible,
        );
    }

    pub(crate) fn save_task_pet_size(&mut self, scale: f32) -> Task<Message> {
        let endpoint = gateway_client_endpoint();
        self.task_pet_size_pending = Some(preferences_scope(&endpoint));
        self.desktop_preferences_persistence.enqueue_companion_size(endpoint,
            serde_json::json!({ "avatar": self.pet_state.task_pet_avatar_kind, "scale": scale, "visible": self.companion_navigation.task_pet_visible }))
            .map(super::desktop::preferences_message)
    }

    pub(crate) fn save_task_pet_preferences(&mut self) -> Task<Message> {
        self.save_desktop_preference(
            "mini_companion",
            serde_json::json!({
                "avatar": self.pet_state.task_pet_avatar_kind,
                "scale": if self.task_pet_size_pending.is_some() { self.task_pet_preview_scale() } else { self.pet_state.task_pet_scale },
                "visible": self.companion_navigation.task_pet_visible,
            }),
        )
    }

    pub(crate) fn save_desktop_preference(&mut self, key: &str, value: Value) -> Task<Message> {
        self.save_desktop_preferences(Map::from_iter([(key.to_owned(), value)]))
    }

    pub(crate) fn save_desktop_preferences(&mut self, fields: Map<String, Value>) -> Task<Message> {
        if fields.is_empty() {
            return Task::none();
        }
        // Bind the destination before yielding to Iced. A server switch must not redirect queued edits.
        self.desktop_preferences_persistence.enqueue(gateway_client_endpoint(), fields).map(super::desktop::preferences_message)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn desktop_preference_edited(&self, scope: &PreferencesScope, key: &str) -> bool {
        self.desktop_preferences_persistence.desktop_preference_edited(scope, key)
    }

    pub(crate) fn retain_unedited_desktop_preferences(&self, cfg: &mut Value) {
        self.desktop_preferences_persistence.retain_unedited_desktop_preferences(
            &preferences_scope(&gateway_client_endpoint()), cfg);
    }

    pub(crate) fn desktop_preferences_saved(
        &mut self,
        scope: PreferencesScope,
        result: Result<(), String>,
    ) -> Task<Message> {
        let next = self.desktop_preferences_persistence.finished(scope, PatchTarget::Desktop).map(super::desktop::preferences_message);
        match result {
            Ok(()) => next,
            Err(error) => {
                Task::batch([next, self.show_error_toast(format!("桌面偏好未保存：{error}"))])
            }
        }
    }
}


/// 公开函数，执行 load_app_config_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn load_app_config_async() -> Result<serde_json::Value, String> {
    config_access().load_app_config_async().await
}

/// 公开函数，执行 save_app_config_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn save_app_config_async(v: serde_json::Value) -> Result<(), String> {
    config_access().save_app_config_async(v).await
}

/// 公开函数，执行 update_agents_compat_registry_result_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn update_agents_compat_registry_result_async(
    update: impl FnOnce(&mut serde_json::Map<String, serde_json::Value>),
) -> Result<(), String> {
    config_access().update_agents_compat_registry_result_async(update).await
}

/// 公开函数，执行 update_agents_compat_registry_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub fn update_agents_compat_registry_async(
    update: impl FnOnce(&mut serde_json::Map<String, serde_json::Value>) + 'static,
) -> Task<Message> {
    config_access().update_agents_compat_registry_async(update).map(|saved|
        Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: saved.tag, result: saved.result }))
}

/// 公开函数，执行 update_agents_compat_registry_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub fn update_agents_compat_registry_async(
    update: impl FnOnce(&mut serde_json::Map<String, serde_json::Value>) + Send + 'static,
) -> Task<Message> {
    config_access().update_agents_compat_registry_async(update).map(|saved|
        Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: saved.tag, result: saved.result }))
}

/// 公开函数，执行 load_app_config 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn load_app_config() -> serde_json::Value {
    config_access().load_app_config()
}

/// 公开函数，执行 save_app_config 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn save_app_config(v: &serde_json::Value) {
    config_access().save_app_config(v)
}

/// 公开函数，执行 set_config_field 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn set_config_field(key: &str, value: serde_json::Value) {
    config_access().set_config_field(key, value)
}

/// 公开函数，执行 load_project_chat_preferences 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub fn load_project_chat_preferences(
    _project_path: &str,
) -> Option<(String, bool, Option<String>)> {
    config_access().load_project_chat_preferences(_project_path)
}

/// 公开函数，执行 save_project_chat_preferences 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub fn save_project_chat_preferences(
    _project_path: &str,
    _model: &str,
    _auto_model: bool,
    _acp_agent: Option<&str>,
) {
    config_access().save_project_chat_preferences(_project_path, _model, _auto_model, _acp_agent)
}

/// 公开函数，执行 load_project_chat_preferences 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub fn load_project_chat_preferences(project_path: &str) -> Option<(String, bool, Option<String>)> {
    config_access().load_project_chat_preferences(project_path)
}

/// 公开函数，执行 load_project_chat_preferences_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn load_project_chat_preferences_async(
    project_path: &str,
) -> Result<Option<(String, bool, Option<String>)>, String> {
    config_access().load_project_chat_preferences_async(project_path).await
}

/// 公开函数，执行 save_project_chat_preferences 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub fn save_project_chat_preferences(
    project_path: &str,
    model: &str,
    auto_model: bool,
    acp_agent: Option<&str>,
) {
    config_access().save_project_chat_preferences(project_path, model, auto_model, acp_agent)
}

/// 公开函数，执行 save_project_chat_preferences_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn save_project_chat_preferences_async(
    project_path: &str,
    model: &str,
    auto_model: bool,
    acp_agent: Option<&str>,
) -> Result<(), String> {
    config_access().save_project_chat_preferences_async(project_path, model, auto_model, acp_agent).await
}

/// 公开函数，执行 load_json_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub fn load_json_tool_content() -> String {
    config_access().load_json_tool_content()
}

/// 公开函数，执行 save_json_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub fn save_json_tool_content(_content: &str) {
    config_access().save_json_tool_content(_content)
}

/// 公开函数，执行 load_sql_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub fn load_sql_tool_content() -> String {
    config_access().load_sql_tool_content()
}

/// 公开函数，执行 save_sql_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub fn save_sql_tool_content(_content: &str) {
    config_access().save_sql_tool_content(_content)
}

/// 公开函数，执行 load_html_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub fn load_html_tool_content() -> String {
    config_access().load_html_tool_content()
}

/// 公开函数，执行 save_html_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub fn save_html_tool_content(_content: &str) {
    config_access().save_html_tool_content(_content)
}

/// 公开函数，执行 load_json_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub fn load_json_tool_content() -> String {
    config_access().load_json_tool_content()
}

/// 公开函数，执行 save_json_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub fn save_json_tool_content(content: &str) {
    config_access().save_json_tool_content(content)
}

/// 公开函数，执行 save_json_tool_content_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn save_json_tool_content_async(content: &str) -> Result<(), String> {
    config_access().save_json_tool_content_async(content).await
}

/// 公开函数，执行 load_sql_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub fn load_sql_tool_content() -> String {
    config_access().load_sql_tool_content()
}

/// 公开函数，执行 save_sql_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub fn save_sql_tool_content(content: &str) {
    config_access().save_sql_tool_content(content)
}

/// 公开函数，执行 save_sql_tool_content_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn save_sql_tool_content_async(content: &str) -> Result<(), String> {
    config_access().save_sql_tool_content_async(content).await
}

/// 公开函数，执行 load_html_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub fn load_html_tool_content() -> String {
    config_access().load_html_tool_content()
}

/// 公开函数，执行 save_html_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub fn save_html_tool_content(content: &str) {
    config_access().save_html_tool_content(content)
}

/// 公开函数，执行 save_html_tool_content_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn save_html_tool_content_async(content: &str) -> Result<(), String> {
    config_access().save_html_tool_content_async(content).await
}

#[cfg(test)]
#[path = "config_desktop_tests.rs"]
pub(in crate::app) mod config_desktop_tests;

fn config_access() -> desktop_config::ConfigAccess {
    desktop_config::ConfigAccess::new(gateway_client_endpoint(), None)
}

#[cfg(test)]
pub(crate) use desktop_config::config_desktop::session_reasoning_efforts_from_config;
