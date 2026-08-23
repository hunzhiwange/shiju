//! 组装桌面端网关配置、主代理覆盖项与客户端连接任务。

use crate::app::{Message, message::SettingsMessage};
use iced::Task;
#[cfg(not(target_arch = "wasm32"))]
use serde::de::DeserializeOwned;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::OnceLock;
use vw_config_types::{config::Config, security::IdentityConfig};
use vw_gateway_client::vw_api_types::id::TenantScope;
use vw_gateway_client::{GatewayClient, GatewayEndpoint};

use super::system_settings::load_gateway_client_bootstrap_config;

fn normalize_identity_format(raw: Option<&str>) -> String {
    let _ = raw;
    "openclaw".to_string()
}

fn normalize_gateway_host(raw: &str) -> String {
    let value = raw.trim();
    if value.is_empty() { "127.0.0.1".to_string() } else { value.to_string() }
}

#[cfg(not(target_arch = "wasm32"))]
fn should_attempt_tools_list_request(_endpoint: &GatewayEndpoint) -> bool {
    true
}

/// 将 `main` 代理的显式配置覆盖到桌面端默认模型与工具选择。
pub(super) fn apply_main_agent_overrides(config: &mut Config) {
    let Some(main) = config.agents.get("main") else {
        return;
    };

    let provider = main.provider.trim();
    if !provider.is_empty() {
        config.default_provider = Some(provider.to_string());
    }

    let model = main.model.trim();
    if !provider.is_empty() && !model.is_empty() {
        config.default_model = Some(format!("{provider}/{model}"));
    }

    if let Some(temperature) = main.temperature {
        config.default_temperature = temperature;
    }

    if main.identity_format.is_some() {
        config.identity = IdentityConfig {
            format: normalize_identity_format(main.identity_format.as_deref()),
            aieos_path: None,
            aieos_inline: None,
        };
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn project_config_override_path() -> Option<std::path::PathBuf> {
    std::env::var_os("VIBEWINDOW_WORKSPACE")
        .filter(|value| !value.to_string_lossy().trim().is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .map(|root| root.join(".vibewindow.json"))
}

#[cfg(not(target_arch = "wasm32"))]
fn load_vibewindow_root_json() -> serde_json::Value {
    project_config_override_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

/// 在桌面配置 JSON 中创建缺失的对象层级并写入叶节点。
pub(super) fn set_config_value_at_path(
    root: &mut serde_json::Value,
    path: &[&str],
    value: serde_json::Value,
) {
    if path.is_empty() {
        *root = value;
        return;
    }

    if !root.is_object() {
        *root = serde_json::json!({});
    }

    let mut current = root;
    for key in &path[..path.len() - 1] {
        if current.get(*key).and_then(|v| v.as_object()).is_none()
            && let Some(obj) = current.as_object_mut()
        {
            obj.insert((*key).to_string(), serde_json::json!({}));
        }
        current =
            current.as_object_mut().and_then(|obj| obj.get_mut(*key)).expect("path object exists");
    }

    if let Some(obj) = current.as_object_mut() {
        obj.insert(path[path.len() - 1].to_string(), value);
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// 从桌面根配置按路径读取并反序列化值；缺失或类型错误均返回 `None`。
pub(super) fn load_config_value_at_path<T: DeserializeOwned>(path: &[&str]) -> Option<T> {
    let root = load_vibewindow_root_json();
    let mut current = &root;
    for key in path {
        current = current.get(*key)?;
    }
    serde_json::from_value::<T>(current.clone()).ok()
}

pub fn server_config_unreachable_error(err: impl AsRef<str>) -> String {
    let detail = err.as_ref().trim();
    if detail.is_empty() {
        "服务端配置不可达，请检查 Gateway 连接状态。".to_string()
    } else {
        format!("服务端配置不可达，请检查 Gateway 连接状态。{detail}")
    }
}

#[cfg(target_arch = "wasm32")]
pub fn spawn_gateway_task(
    tag: &'static str,
    future: impl std::future::Future<Output = Result<(), String>> + 'static,
) -> Task<Message> {
    Task::perform(async move { future.await }, move |result| {
        Message::Settings(SettingsMessage::AgentConfigSaved { tag, result })
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_gateway_task(
    tag: &'static str,
    future: impl std::future::Future<Output = Result<(), String>> + Send + 'static,
) -> Task<Message> {
    Task::perform(future, move |result| {
        Message::Settings(SettingsMessage::AgentConfigSaved { tag, result })
    })
}

/// 从当前活动服务配置构造带认证与租户范围的网关端点。
pub fn gateway_client_endpoint() -> GatewayEndpoint {
    #[cfg(not(target_arch = "wasm32"))]
    let cfg = load_gateway_client_bootstrap_config();
    #[cfg(target_arch = "wasm32")]
    let cfg = load_gateway_client_bootstrap_config();
    let active = cfg.active_server();
    let host = normalize_gateway_host(&active.host);
    let skey = if active.skey.trim().is_empty() {
        active.bearer_token.trim().to_string()
    } else {
        active.skey.trim().to_string()
    };
    let auth = vw_gateway_client::GatewayAuth {
        skey: Some(skey).filter(|value| !value.is_empty()),
        ..vw_gateway_client::GatewayAuth::default()
    };
    let tenant_scope = TenantScope::new(active.platform_id.trim(), active.company_id.trim())
        .expect("gateway client tenant IDs are validated before persistence");
    GatewayEndpoint::new(host, active.port.clamp(1, u16::MAX))
        .with_auth(auth)
        .with_tenant_scope(tenant_scope)
}

/// 创建当前桌面配置对应的网关客户端。
pub fn gateway_client() -> Result<GatewayClient, String> {
    GatewayClient::new(gateway_client_endpoint())
}

fn normalize_tool_ids(mut tools: Vec<String>) -> Vec<String> {
    tools.retain(|tool_id| {
        !tool_id.trim().is_empty() && !vw_config_types::tools::is_mcp_dynamic_tool_id(tool_id)
    });
    tools.sort();
    tools.dedup();
    tools
}

/// 同步加载静态工具标识；连接失败时返回空列表并过滤动态 MCP 工具。
pub fn load_tools_list_via_gateway() -> Vec<String> {
    let endpoint = gateway_client_endpoint();

    #[cfg(not(target_arch = "wasm32"))]
    if !should_attempt_tools_list_request(&endpoint) {
        return Vec::new();
    }

    let client = match GatewayClient::new(endpoint) {
        Ok(client) => client,
        Err(err) => {
            tracing::warn!(target: "vw_desktop", error = %err, "gateway client unavailable for tools list");
            return Vec::new();
        }
    };

    let result = run_gateway_call(async { client.tools_list().await });

    match result {
        Ok(tools) => normalize_tool_ids(tools),
        Err(err) => {
            if vw_gateway_client::is_gateway_unavailable_error(&err) {
                tracing::debug!(target: "vw_desktop", error = %err, "failed to load tools list via gateway");
            } else {
                tracing::warn!(target: "vw_desktop", error = %err, "failed to load tools list via gateway");
            }
            Vec::new()
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn gateway_blocking_runtime() -> Result<&'static tokio::runtime::Runtime, String> {
    static RUNTIME: OnceLock<Result<tokio::runtime::Runtime, String>> = OnceLock::new();

    match RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .max_blocking_threads(8)
            .enable_all()
            .build()
            .map_err(|err| err.to_string())
    }) {
        Ok(runtime) => Ok(runtime),
        Err(err) => Err(err.clone()),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn run_gateway_call<T: Send>(
    future: impl std::future::Future<Output = Result<T, String>> + Send,
) -> Result<T, String> {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread => {
            tokio::task::block_in_place(|| handle.block_on(future))
        }
        Ok(_) => std::thread::scope(|scope| {
            scope
                .spawn(|| gateway_blocking_runtime()?.block_on(future))
                .join()
                .unwrap_or_else(|_| Err("gateway blocking thread panicked".to_string()))
        }),
        Err(_) => gateway_blocking_runtime()?.block_on(future),
    }
}

#[cfg(target_arch = "wasm32")]
/// WASM keeps this shim only for legacy synchronous load sites and returns
/// `Default` immediately; synchronous save flows now run through
/// `spawn_gateway_task` and the async gateway APIs.
pub(super) fn run_gateway_call<T: Default>(
    _future: impl std::future::Future<Output = Result<T, String>>,
) -> Result<T, String> {
    tracing::warn!(target: "vw_desktop", "sync gateway call on wasm, returning default");
    Ok(T::default())
}

#[cfg(test)]
#[path = "config_gateway_tests.rs"]
mod config_gateway_tests;
