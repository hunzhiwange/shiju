//! 通过网关读写桌面偏好配置。
//! 本模块把历史本地配置调用收敛到显式的桌面偏好接口，并在网关失败时提供安全空配置回退。

use super::gateway::{gateway_client, run_gateway_call, spawn_gateway_task};
use crate::app::Message;
use iced::Task;

/// 独立解码每个会话，避免未知值影响其它会话的偏好恢复。
pub(crate) fn session_reasoning_efforts_from_config(
    cfg: &serde_json::Value,
) -> std::collections::HashMap<String, Option<vw_gateway_client::vw_api_types::ReasoningEffortDto>>
{
    cfg.get("session_reasoning_efforts")
        .and_then(serde_json::Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(id, value)| {
            serde_json::from_value(value.clone()).ok().map(|effort| (id.clone(), effort))
        })
        .collect()
}

/// 公开函数，执行 load_app_config_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn load_app_config_async() -> Result<serde_json::Value, String> {
    let client = gateway_client()?;
    let value = client.desktop_preferences_get().await?;
    Ok(if value.is_object() { value } else { serde_json::json!({}) })
}

/// 公开函数，执行 save_app_config_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn save_app_config_async(v: serde_json::Value) -> Result<(), String> {
    let patch = if v.is_object() { v } else { serde_json::json!({}) };
    let client = gateway_client()?;
    client.desktop_preferences_patch(&patch).await.map(|_| ())
}

/// 公开函数，执行 update_agents_compat_registry_result_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn update_agents_compat_registry_result_async(
    update: impl FnOnce(&mut serde_json::Map<String, serde_json::Value>),
) -> Result<(), String> {
    let mut cfg = load_app_config_async().await?;
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
    save_app_config_async(cfg).await
}

/// 公开函数，执行 update_agents_compat_registry_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub fn update_agents_compat_registry_async(
    update: impl FnOnce(&mut serde_json::Map<String, serde_json::Value>) + 'static,
) -> Task<Message> {
    spawn_gateway_task("agents_compat", async move {
        update_agents_compat_registry_result_async(update).await
    })
}

/// 公开函数，执行 update_agents_compat_registry_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub fn update_agents_compat_registry_async(
    update: impl FnOnce(&mut serde_json::Map<String, serde_json::Value>) + Send + 'static,
) -> Task<Message> {
    spawn_gateway_task("agents_compat", async move {
        update_agents_compat_registry_result_async(update).await
    })
}

/// 公开函数，执行 load_app_config 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn load_app_config() -> serde_json::Value {
    let result = run_gateway_call(load_app_config_async());
    match result {
        Ok(value) if value.is_object() => value,
        Ok(_) => serde_json::json!({}),
        Err(err) => {
            if vw_gateway_client::is_gateway_unavailable_error(&err) {
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
pub fn save_app_config(v: &serde_json::Value) {
    if let Err(err) = run_gateway_call(save_app_config_async(v.clone())) {
        if vw_gateway_client::is_gateway_unavailable_error(&err) {
            tracing::debug!(target: "vw_desktop", error = %err, "failed to save desktop preferences via gateway");
        } else {
            tracing::warn!(target: "vw_desktop", error = %err, "failed to save desktop preferences via gateway");
        }
    }
}

/// 公开函数，执行 set_config_field 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn set_config_field(key: &str, value: serde_json::Value) {
    let mut cfg = load_app_config();
    if let Some(obj) = cfg.as_object_mut() {
        obj.insert(key.to_string(), value);
    } else {
        cfg = serde_json::json!({ key: value });
    }
    save_app_config(&cfg);
}

/// 公开函数，执行 load_project_chat_preferences 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub fn load_project_chat_preferences(
    _project_path: &str,
) -> Option<(String, bool, Option<String>)> {
    None
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
}

/// 公开函数，执行 load_project_chat_preferences 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub fn load_project_chat_preferences(project_path: &str) -> Option<(String, bool, Option<String>)> {
    let result = run_gateway_call(async {
        let client = gateway_client()?;
        client.desktop_project_preferences_get(project_path).await
    });
    match result {
        Ok(value) => value,
        Err(err) => {
            if vw_gateway_client::is_gateway_unavailable_error(&err) {
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
pub async fn load_project_chat_preferences_async(
    project_path: &str,
) -> Result<Option<(String, bool, Option<String>)>, String> {
    let client = gateway_client()?;
    client.desktop_project_preferences_get(project_path).await
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
    if let Err(err) = run_gateway_call(async {
        let client = gateway_client()?;
        client.desktop_project_preferences_put(project_path, model, auto_model, acp_agent).await
    }) {
        tracing::warn!(target: "vw_desktop", error = %err, project_path, "failed to save project chat preferences via gateway");
    }
}

/// 公开函数，执行 save_project_chat_preferences_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn save_project_chat_preferences_async(
    project_path: &str,
    model: &str,
    auto_model: bool,
    acp_agent: Option<&str>,
) -> Result<(), String> {
    let client = gateway_client()?;
    client.desktop_project_preferences_put(project_path, model, auto_model, acp_agent).await
}

/// 公开函数，执行 load_json_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub fn load_json_tool_content() -> String {
    String::new()
}

/// 公开函数，执行 save_json_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub fn save_json_tool_content(_content: &str) {}

/// 公开函数，执行 load_sql_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub fn load_sql_tool_content() -> String {
    String::new()
}

/// 公开函数，执行 save_sql_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub fn save_sql_tool_content(_content: &str) {}

/// 公开函数，执行 load_html_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub fn load_html_tool_content() -> String {
    String::new()
}

/// 公开函数，执行 save_html_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub fn save_html_tool_content(_content: &str) {}

/// 公开函数，执行 load_json_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub fn load_json_tool_content() -> String {
    load_tool_content("json")
}

/// 公开函数，执行 save_json_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub fn save_json_tool_content(content: &str) {
    save_tool_content("json", content);
}

/// 公开函数，执行 save_json_tool_content_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn save_json_tool_content_async(content: &str) -> Result<(), String> {
    save_tool_content_async("json", content).await
}

/// 公开函数，执行 load_sql_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub fn load_sql_tool_content() -> String {
    load_tool_content("sql")
}

/// 公开函数，执行 save_sql_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub fn save_sql_tool_content(content: &str) {
    save_tool_content("sql", content);
}

/// 公开函数，执行 save_sql_tool_content_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn save_sql_tool_content_async(content: &str) -> Result<(), String> {
    save_tool_content_async("sql", content).await
}

/// 公开函数，执行 load_html_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub fn load_html_tool_content() -> String {
    load_tool_content("html")
}

/// 公开函数，执行 save_html_tool_content 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub fn save_html_tool_content(content: &str) {
    save_tool_content("html", content);
}

/// 公开函数，执行 save_html_tool_content_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn save_html_tool_content_async(content: &str) -> Result<(), String> {
    save_tool_content_async("html", content).await
}

#[cfg(not(target_arch = "wasm32"))]
fn load_tool_content(tool_type: &str) -> String {
    let result = run_gateway_call(async {
        let client = gateway_client()?;
        client.desktop_tool_content_get(tool_type).await
    });
    match result {
        Ok(content) => content,
        Err(err) => {
            if vw_gateway_client::is_gateway_unavailable_error(&err) {
                tracing::debug!(target: "vw_desktop", error = %err, tool_type, "failed to load tool content via gateway");
            } else {
                tracing::warn!(target: "vw_desktop", error = %err, tool_type, "failed to load tool content via gateway");
            }
            String::new()
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn save_tool_content(tool_type: &str, content: &str) {
    if let Err(err) = run_gateway_call(async {
        let client = gateway_client()?;
        client.desktop_tool_content_put(tool_type, content).await
    }) {
        tracing::warn!(target: "vw_desktop", error = %err, tool_type, "failed to save tool content via gateway");
    }
}

async fn save_tool_content_async(tool_type: &str, content: &str) -> Result<(), String> {
    let client = gateway_client()?;
    client.desktop_tool_content_put(tool_type, content).await
}

#[cfg(test)]
#[path = "config_desktop_tests.rs"]
mod config_desktop_tests;
