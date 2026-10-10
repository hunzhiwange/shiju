//! 桌面应用智能体相关配置的读取、归一化与保存。
//!
//! 本模块集中处理配置读取、保存、归一化和跨平台回退边界。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护配置持久化流程。

use crate::app::Message;
use iced::Task;
use std::collections::{BTreeMap, HashMap};
use vw_config_types::{
    agent::DelegateAgentConfig,
    automation::{CronConfig, HeartbeatConfig, SchedulerConfig},
    channels::ChannelsConfig,
    config::{AcpAgentConfig, Config},
    gateway::{GatewayConfig, TunnelConfig},
    hooks::HooksConfig,
    memory::{MemoryConfig, StorageConfig},
    observability::ObservabilityConfig,
    proxy::ProxyConfig,
    reliability::ReliabilityConfig,
    routing::{EmbeddingRouteConfig, ModelRouteConfig, QueryClassificationConfig},
    runtime::RuntimeConfig,
    security::{AutonomyConfig, IdentityConfig, SecurityConfig},
    skills::SkillsConfig,
    tools::{BrowserConfig, HttpRequestConfig, MultimodalConfig, WebSearchConfig},
};

#[cfg(not(target_arch = "wasm32"))]
use super::gateway::load_config_value_at_path;
use super::gateway::{
    apply_main_agent_overrides, gateway_client, run_gateway_call, set_config_value_at_path,
    spawn_gateway_task,
};

#[derive(Debug, Clone)]
pub struct AcpSettingsSnapshot {
    pub catalog: BTreeMap<String, AcpAgentConfig>,
    pub enabled: BTreeMap<String, AcpAgentConfig>,
}

const DEFAULT_ENABLED_ACP_AGENTS: &[&str] =
    &["claude", "gemini", "opencode", "codex", "openclaw", "copilot"];

fn default_enabled_acp_config(
    catalog: &HashMap<String, AcpAgentConfig>,
) -> HashMap<String, AcpAgentConfig> {
    DEFAULT_ENABLED_ACP_AGENTS
        .iter()
        .filter_map(|name| catalog.get(*name).cloned().map(|config| ((*name).to_string(), config)))
        .collect()
}

fn resolve_enabled_acp_config(
    catalog: &HashMap<String, AcpAgentConfig>,
    enabled: HashMap<String, AcpAgentConfig>,
) -> HashMap<String, AcpAgentConfig> {
    if enabled.is_empty() { default_enabled_acp_config(catalog) } else { enabled }
}

fn build_acp_agent_enabled_patch(
    agent_name: &str,
    enabled: bool,
    spec: Option<AcpAgentConfig>,
    catalog: &HashMap<String, AcpAgentConfig>,
    configured: HashMap<String, AcpAgentConfig>,
) -> Result<serde_json::Value, String> {
    let agent_name = agent_name.trim().to_string();
    if agent_name.is_empty() {
        return Err("acp agent name must not be empty".to_string());
    }

    let mut effective = resolve_enabled_acp_config(catalog, configured.clone());
    let spec = match spec {
        Some(spec) => spec,
        None => catalog
            .get(&agent_name)
            .cloned()
            .ok_or_else(|| format!("unknown acp agent: {agent_name}"))?,
    };

    let mut acp_patch = serde_json::Map::new();
    if enabled {
        effective.insert(agent_name.clone(), spec);
        if configured.is_empty() {
            for (name, config) in effective {
                acp_patch
                    .insert(name, serde_json::to_value(config).map_err(|err| err.to_string())?);
            }
        } else {
            let config = effective
                .remove(&agent_name)
                .ok_or_else(|| format!("unknown acp agent: {agent_name}"))?;
            acp_patch
                .insert(agent_name, serde_json::to_value(config).map_err(|err| err.to_string())?);
        }
    } else {
        effective.remove(&agent_name);
        if configured.is_empty() {
            for (name, config) in effective {
                acp_patch
                    .insert(name, serde_json::to_value(config).map_err(|err| err.to_string())?);
            }
        }
        acp_patch.insert(agent_name, serde_json::Value::Null);
    }

    let mut root_patch = serde_json::Map::new();
    root_patch.insert("acp".to_string(), serde_json::Value::Object(acp_patch));
    Ok(serde_json::Value::Object(root_patch))
}

fn build_acp_agent_create_patch(
    agent_name: &str,
    mut spec: AcpAgentConfig,
    catalog: &HashMap<String, AcpAgentConfig>,
    configured: HashMap<String, AcpAgentConfig>,
) -> Result<serde_json::Value, String> {
    let agent_name = agent_name.trim();
    if agent_name.is_empty() {
        return Err("acp agent name must not be empty".to_string());
    }
    if catalog.contains_key(agent_name) {
        return Err(format!("acp agent already exists: {agent_name}"));
    }

    spec.command = spec.command.trim().to_string();
    if spec.command.is_empty() {
        return Err("acp agent command must not be empty".to_string());
    }
    spec.args = spec
        .args
        .into_iter()
        .map(|arg| arg.trim().to_string())
        .filter(|arg| !arg.is_empty())
        .collect();

    build_acp_agent_enabled_patch(agent_name, true, Some(spec), catalog, configured)
}

fn build_main_agent_overrides_patch(config: &Config) -> Result<Option<serde_json::Value>, String> {
    let Some(main) = config.agents.get("main") else {
        return Ok(None);
    };

    let provider = main.provider.trim().to_string();
    let model = main.model.trim().to_string();
    let identity =
        IdentityConfig { format: "openclaw".to_string(), aieos_path: None, aieos_inline: None };

    let mut patch = serde_json::Map::new();
    if !provider.is_empty() {
        patch.insert("default_provider".to_string(), serde_json::Value::String(provider.clone()));
    }
    if !provider.is_empty() && !model.is_empty() {
        patch.insert(
            "default_model".to_string(),
            serde_json::Value::String(format!("{provider}/{model}")),
        );
    }
    if let Some(temperature) = main.temperature
        && let Some(number) = serde_json::Number::from_f64(temperature)
    {
        patch.insert("default_temperature".to_string(), serde_json::Value::Number(number));
    }
    let identity_value = serde_json::to_value(identity).map_err(|err| err.to_string())?;
    patch.insert("identity".to_string(), identity_value);

    Ok(Some(serde_json::Value::Object(patch)))
}

async fn fetch_agent_config_via_gateway() -> Result<Config, String> {
    let client = gateway_client()?;
    let value = client.config_get(None).await?;
    let mut config = serde_json::from_value::<Config>(value).map_err(|err| err.to_string())?;
    apply_main_agent_overrides(&mut config);
    Ok(config)
}

async fn fetch_global_agent_config_via_gateway() -> Result<Config, String> {
    let client = gateway_client()?;
    let value = client.global_config_get().await?;
    let mut config = serde_json::from_value::<Config>(value).map_err(|err| err.to_string())?;
    apply_main_agent_overrides(&mut config);
    Ok(config)
}

async fn fetch_global_config_value_via_gateway() -> Result<serde_json::Value, String> {
    let client = gateway_client()?;
    client.global_config_get().await
}

#[cfg(not(target_arch = "wasm32"))]
fn load_local_agent_config() -> Option<Config> {
    let mut config = load_config_value_at_path::<Config>(&[])?;
    config.gateway.skeys.clear();
    apply_main_agent_overrides(&mut config);
    Some(config)
}

#[cfg(target_arch = "wasm32")]
fn load_local_agent_config() -> Option<Config> {
    None
}

async fn fetch_global_acp_config_via_gateway() -> Result<HashMap<String, AcpAgentConfig>, String> {
    let client = gateway_client()?;
    client.global_acp_config_get().await
}

async fn patch_agent_config_via_gateway(patch: &serde_json::Value) -> Result<(), String> {
    let client = gateway_client()?;
    client.global_config_patch(patch).await
}

fn load_agent_config_via_gateway() -> Config {
    let result = run_gateway_call(async { fetch_agent_config_via_gateway().await });

    match result {
        Ok(config) => config,
        Err(err) => {
            if vw_gateway_client::is_gateway_unavailable_error(&err) {
                tracing::debug!(target: "vw_desktop", error = %err, "failed to load agent config via gateway");
            } else {
                tracing::warn!(target: "vw_desktop", error = %err, "failed to load agent config via gateway");
            }
            if let Some(config) = load_local_agent_config() {
                return config;
            }
            Config::default()
        }
    }
}

#[allow(dead_code)]
fn patch_agent_config(path: &[&str], value: serde_json::Value) {
    let mut patch = serde_json::json!({});
    set_config_value_at_path(&mut patch, path, value);

    let outcome = run_gateway_call(async { patch_agent_config_via_gateway(&patch).await });

    if let Err(err) = outcome {
        tracing::warn!(target: "vw_desktop", error = %err, "failed to patch agent config via gateway");
    }
}

/// 读取、保存或转换 `load_full_agent_config_async` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 当底层配置、文件或运行时调用失败时，错误会通过 `Result` 返回给上层统一处理。
pub async fn load_full_agent_config_async() -> Result<Config, String> {
    fetch_agent_config_via_gateway().await
}

pub async fn load_project_agent_config_async(project_path: &str) -> Result<Config, String> {
    let client = gateway_client()?;
    let value = client.config_get(Some(project_path)).await?;
    serde_json::from_value(value).map_err(|error| error.to_string())
}

/// 读取全局配置原始 JSON，用于尚未进入 `Config` 强类型结构的字段。
pub async fn load_global_config_value_async() -> Result<serde_json::Value, String> {
    fetch_global_config_value_via_gateway().await
}

/// 读取 MCP server 连接状态与工具预览。
pub async fn load_mcp_inspect_async()
-> Result<vw_gateway_client::vw_api_types::mcp::McpInspectResponse, String> {
    let client = gateway_client()?;
    client.mcp_inspect().await
}

/// 读取、保存或转换 `load_browser_config_async` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 当底层配置、文件或运行时调用失败时，错误会通过 `Result` 返回给上层统一处理。
pub async fn load_browser_config_async() -> Result<BrowserConfig, String> {
    Ok(load_full_agent_config_async().await?.browser)
}

/// 读取、保存或转换 `load_gateway_config_result` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 当底层配置、文件或运行时调用失败时，错误会通过 `Result` 返回给上层统一处理。
pub fn load_gateway_config_result() -> Result<GatewayConfig, String> {
    match run_gateway_call(async { fetch_global_agent_config_via_gateway().await }) {
        Ok(config) => Ok(config.gateway),
        Err(err) => load_local_agent_config().map(|config| config.gateway).ok_or(err),
    }
}

/// 读取、保存或转换 `load_global_acp_config_result` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 当底层配置、文件或运行时调用失败时，错误会通过 `Result` 返回给上层统一处理。
pub fn load_global_acp_config_result() -> Result<HashMap<String, AcpAgentConfig>, String> {
    run_gateway_call(async { fetch_global_acp_config_via_gateway().await })
}

/// 读取、保存或转换 `load_global_acp_config_async` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 当底层配置、文件或运行时调用失败时，错误会通过 `Result` 返回给上层统一处理。
pub async fn load_global_acp_config_async() -> Result<HashMap<String, AcpAgentConfig>, String> {
    fetch_global_acp_config_via_gateway().await
}

pub fn load_enabled_acp_config_result() -> Result<HashMap<String, AcpAgentConfig>, String> {
    run_gateway_call(async {
        let catalog = fetch_global_acp_config_via_gateway().await?;
        let enabled = fetch_global_agent_config_via_gateway().await?.acp;
        Ok(resolve_enabled_acp_config(&catalog, enabled))
    })
}

pub async fn load_enabled_acp_config_async() -> Result<HashMap<String, AcpAgentConfig>, String> {
    let catalog = fetch_global_acp_config_via_gateway().await?;
    let enabled = fetch_global_agent_config_via_gateway().await?.acp;
    Ok(resolve_enabled_acp_config(&catalog, enabled))
}

pub async fn load_acp_settings_snapshot_async() -> Result<AcpSettingsSnapshot, String> {
    let catalog = fetch_global_acp_config_via_gateway().await?;
    let enabled = fetch_global_agent_config_via_gateway().await?.acp;
    let enabled = resolve_enabled_acp_config(&catalog, enabled);

    Ok(AcpSettingsSnapshot {
        catalog: catalog.into_iter().collect(),
        enabled: enabled.into_iter().collect(),
    })
}

pub async fn set_global_acp_agent_enabled_async(
    agent_name: String,
    enabled: bool,
    spec: Option<AcpAgentConfig>,
) -> Result<AcpSettingsSnapshot, String> {
    let catalog = fetch_global_acp_config_via_gateway().await?;
    let configured = fetch_global_agent_config_via_gateway().await?.acp;
    let patch = build_acp_agent_enabled_patch(&agent_name, enabled, spec, &catalog, configured)?;

    patch_agent_config_via_gateway(&patch).await?;

    load_acp_settings_snapshot_async().await
}

pub async fn add_global_acp_agent_async(
    agent_name: String,
    spec: AcpAgentConfig,
) -> Result<AcpSettingsSnapshot, String> {
    let catalog = fetch_global_acp_config_via_gateway().await?;
    let configured = fetch_global_agent_config_via_gateway().await?.acp;
    let patch = build_acp_agent_create_patch(&agent_name, spec, &catalog, configured)?;

    patch_agent_config_via_gateway(&patch).await?;

    load_acp_settings_snapshot_async().await
}

/// 读取、保存或转换 `patch_full_agent_config_async` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 当底层配置、文件或运行时调用失败时，错误会通过 `Result` 返回给上层统一处理。
pub async fn patch_full_agent_config_async(patch: serde_json::Value) -> Result<(), String> {
    patch_agent_config_via_gateway(&patch).await
}

pub async fn patch_project_agent_config_async(
    project_path: &str,
    patch: serde_json::Value,
) -> Result<(), String> {
    let client = gateway_client()?;
    client.config_patch(Some(project_path), &patch).await.map(|_| ())
}

/// 读取、保存或转换 `patch_agent_config_result` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 当底层配置、文件或运行时调用失败时，错误会通过 `Result` 返回给上层统一处理。
pub fn patch_agent_config_result(path: &[&str], value: serde_json::Value) -> Result<(), String> {
    let mut patch = serde_json::json!({});
    set_config_value_at_path(&mut patch, path, value);

    run_gateway_call(async { patch_agent_config_via_gateway(&patch).await })
}

/// 读取、保存或转换 `update_gateway_config_result` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 当底层配置、文件或运行时调用失败时，错误会通过 `Result` 返回给上层统一处理。
pub fn update_gateway_config_result(update: impl FnOnce(&mut GatewayConfig)) -> Result<(), String> {
    let mut cfg = load_gateway_config_result()?;
    update(&mut cfg);
    let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;

    run_gateway_call(async {
        patch_agent_config_via_gateway(&serde_json::json!({ "gateway": value })).await
    })
}

/// 读取、保存或转换 `remove_global_provider_via_gateway` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 当底层配置、文件或运行时调用失败时，错误会通过 `Result` 返回给上层统一处理。
pub async fn remove_global_provider_via_gateway(provider_id: &str) -> Result<(), String> {
    let provider_id = provider_id.trim();
    if provider_id.is_empty() {
        return Err("provider_id must not be empty".to_string());
    }

    let mut providers = serde_json::Map::new();
    providers.insert(provider_id.to_string(), serde_json::Value::Null);

    let mut root = serde_json::Map::new();
    root.insert("providers".to_string(), serde_json::Value::Object(providers));

    patch_agent_config_via_gateway(&serde_json::Value::Object(root)).await
}

macro_rules! define_agent_config_update_result_fns {
    ($(($result_name:ident, $name:ident, $ty:ty, $field:ident, $path:expr)),* $(,)?) => {
        $(
            #[cfg(not(target_arch = "wasm32"))]
            pub fn $result_name(update: impl FnOnce(&mut $ty)) -> Result<(), String> {
                let mut cfg = load_full_agent_config_result()?.$field;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                patch_agent_config_result($path, value)
            }

            #[cfg(not(target_arch = "wasm32"))]
            pub fn $name(update: impl FnOnce(&mut $ty)) {
                if let Err(err) = $result_name(update) {
                    tracing::warn!(target: "vw_desktop", error = %err, "failed to update agent config via gateway");
                }
            }

            #[cfg(target_arch = "wasm32")]
            pub fn $result_name(_update: impl FnOnce(&mut $ty)) -> Result<(), String> {
                Ok(())
            }

            #[cfg(target_arch = "wasm32")]
            pub fn $name(_update: impl FnOnce(&mut $ty)) {}
        )*
    };
}

macro_rules! define_agent_config_update_result_async_fns {
    ($(($async_result_name:ident, $ty:ty, $field:ident, $path:expr)),* $(,)?) => {
        $(
            pub async fn $async_result_name(update: impl FnOnce(&mut $ty)) -> Result<(), String> {
                let mut cfg = load_full_agent_config_async().await?.$field;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                patch_full_agent_config_async(serde_json::json!({ $path[0]: value })).await
            }
        )*
    };
}

macro_rules! define_agent_config_update_async_fns {
    ($(($async_name:ident, $async_result_name:ident, $ty:ty, $field:ident, $path:expr)),* $(,)?) => {
        $(
            pub fn $async_name(update: impl FnOnce(&mut $ty) + Send + 'static) -> Task<Message> {
                spawn_gateway_task($path[0], async move { $async_result_name(update).await })
            }
        )*
    };
}

/// 读取、保存或转换 `load_gateway_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_gateway_config() -> GatewayConfig {
    load_gateway_config_result().unwrap_or_else(|err| {
        if vw_gateway_client::is_gateway_unavailable_error(&err) {
            tracing::debug!(target: "vw_desktop", error = %err, "failed to load gateway config via gateway");
        } else {
            tracing::warn!(target: "vw_desktop", error = %err, "failed to load gateway config via gateway");
        }
        GatewayConfig::default()
    })
}

/// 读取、保存或转换 `load_heartbeat_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_heartbeat_config() -> HeartbeatConfig {
    load_agent_config_via_gateway().heartbeat
}

/// 读取、保存或转换 `load_cron_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_cron_config() -> CronConfig {
    load_agent_config_via_gateway().cron
}

/// 读取、保存或转换 `load_scheduler_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_scheduler_config() -> SchedulerConfig {
    load_agent_config_via_gateway().scheduler
}

/// 读取、保存或转换 `load_reliability_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_reliability_config() -> ReliabilityConfig {
    load_agent_config_via_gateway().reliability
}

/// 读取、保存或转换 `load_memory_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_memory_config() -> MemoryConfig {
    load_agent_config_via_gateway().memory
}

/// 读取、保存或转换 `load_security_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_security_config() -> SecurityConfig {
    load_agent_config_via_gateway().security
}

/// 读取、保存或转换 `load_channels_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_channels_config() -> ChannelsConfig {
    load_agent_config_via_gateway().channels_config
}

/// 读取、保存或转换 `load_observability_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_observability_config() -> ObservabilityConfig {
    load_agent_config_via_gateway().observability
}

/// 读取、保存或转换 `load_storage_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_storage_config() -> StorageConfig {
    load_agent_config_via_gateway().storage
}

/// 读取、保存或转换 `load_proxy_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_proxy_config() -> ProxyConfig {
    load_agent_config_via_gateway().proxy
}

/// 读取、保存或转换 `load_browser_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_browser_config() -> BrowserConfig {
    load_agent_config_via_gateway().browser
}

/// 读取、保存或转换 `load_http_request_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_http_request_config() -> HttpRequestConfig {
    load_agent_config_via_gateway().http_request
}

/// 读取、保存或转换 `load_multimodal_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_multimodal_config() -> MultimodalConfig {
    load_agent_config_via_gateway().multimodal
}

/// 读取、保存或转换 `load_web_search_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_web_search_config() -> WebSearchConfig {
    load_agent_config_via_gateway().web_search
}

#[cfg(not(target_arch = "wasm32"))]
/// 读取、保存或转换 `load_tunnel_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_tunnel_config() -> TunnelConfig {
    load_agent_config_via_gateway().tunnel
}

#[cfg(not(target_arch = "wasm32"))]
/// 读取、保存或转换 `load_hooks_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_hooks_config() -> HooksConfig {
    load_agent_config_via_gateway().hooks
}

#[cfg(not(target_arch = "wasm32"))]
/// 读取、保存或转换 `load_skills_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_skills_config() -> SkillsConfig {
    load_agent_config_via_gateway().skills
}

#[cfg(not(target_arch = "wasm32"))]
/// 读取、保存或转换 `load_delegate_agents_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_delegate_agents_config() -> HashMap<String, DelegateAgentConfig> {
    load_agent_config_via_gateway().agents
}

#[cfg(not(target_arch = "wasm32"))]
/// 读取、保存或转换 `load_embedding_routes_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_embedding_routes_config() -> Vec<EmbeddingRouteConfig> {
    load_agent_config_via_gateway().embedding_routes
}

#[cfg(not(target_arch = "wasm32"))]
/// 读取、保存或转换 `load_autonomy_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_autonomy_config() -> AutonomyConfig {
    load_agent_config_via_gateway().autonomy
}

#[cfg(not(target_arch = "wasm32"))]
/// 读取、保存或转换 `load_identity_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_identity_config() -> IdentityConfig {
    load_agent_config_via_gateway().identity
}

#[cfg(not(target_arch = "wasm32"))]
/// 读取、保存或转换 `load_full_agent_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_full_agent_config() -> Config {
    load_agent_config_via_gateway()
}

#[cfg(not(target_arch = "wasm32"))]
/// 读取、保存或转换 `load_full_agent_config_result` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 当底层配置、文件或运行时调用失败时，错误会通过 `Result` 返回给上层统一处理。
pub fn load_full_agent_config_result() -> Result<Config, String> {
    match run_gateway_call(async { fetch_agent_config_via_gateway().await }) {
        Ok(config) => Ok(config),
        Err(err) => load_local_agent_config().ok_or(err),
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// 读取、保存或转换 `update_gateway_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn update_gateway_config(update: impl FnOnce(&mut GatewayConfig)) {
    let mut cfg = load_gateway_config();
    update(&mut cfg);
    if let Ok(value) = serde_json::to_value(cfg) {
        patch_agent_config(&["gateway"], value);
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// 读取、保存或转换 `load_runtime_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_runtime_config() -> RuntimeConfig {
    load_agent_config_via_gateway().runtime
}

#[cfg(not(target_arch = "wasm32"))]
/// 读取、保存或转换 `load_model_routes_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_model_routes_config() -> Vec<ModelRouteConfig> {
    load_agent_config_via_gateway().model_routes
}

#[cfg(not(target_arch = "wasm32"))]
/// 读取、保存或转换 `load_query_classification_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_query_classification_config() -> QueryClassificationConfig {
    load_agent_config_via_gateway().query_classification
}

#[cfg(target_arch = "wasm32")]
/// 读取、保存或转换 `load_tunnel_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_tunnel_config() -> TunnelConfig {
    TunnelConfig::default()
}
#[cfg(target_arch = "wasm32")]
/// 读取、保存或转换 `load_hooks_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_hooks_config() -> HooksConfig {
    HooksConfig::default()
}
#[cfg(target_arch = "wasm32")]
/// 读取、保存或转换 `load_skills_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_skills_config() -> SkillsConfig {
    SkillsConfig::default()
}
#[cfg(target_arch = "wasm32")]
/// 读取、保存或转换 `load_delegate_agents_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_delegate_agents_config() -> HashMap<String, DelegateAgentConfig> {
    HashMap::new()
}

#[cfg(target_arch = "wasm32")]
/// 读取、保存或转换 `load_embedding_routes_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_embedding_routes_config() -> Vec<EmbeddingRouteConfig> {
    Vec::new()
}
#[cfg(target_arch = "wasm32")]
/// 读取、保存或转换 `load_autonomy_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_autonomy_config() -> AutonomyConfig {
    AutonomyConfig::default()
}
#[cfg(target_arch = "wasm32")]
/// 读取、保存或转换 `load_identity_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_identity_config() -> IdentityConfig {
    IdentityConfig::default()
}
#[cfg(target_arch = "wasm32")]
/// 读取、保存或转换 `load_full_agent_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_full_agent_config() -> Config {
    Config::default()
}
#[cfg(target_arch = "wasm32")]
/// 读取、保存或转换 `load_full_agent_config_result` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 当底层配置、文件或运行时调用失败时，错误会通过 `Result` 返回给上层统一处理。
pub fn load_full_agent_config_result() -> Result<Config, String> {
    Ok(Config::default())
}
#[cfg(target_arch = "wasm32")]
/// 读取、保存或转换 `update_gateway_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn update_gateway_config(_update: impl FnOnce(&mut GatewayConfig)) {}
#[cfg(target_arch = "wasm32")]
/// 读取、保存或转换 `load_runtime_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_runtime_config() -> RuntimeConfig {
    RuntimeConfig::default()
}
#[cfg(target_arch = "wasm32")]
/// 读取、保存或转换 `load_model_routes_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_model_routes_config() -> Vec<ModelRouteConfig> {
    Vec::new()
}
#[cfg(target_arch = "wasm32")]
/// 读取、保存或转换 `load_query_classification_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn load_query_classification_config() -> QueryClassificationConfig {
    QueryClassificationConfig::default()
}

#[cfg(target_arch = "wasm32")]
/// 读取、保存或转换 `update_delegate_agents_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn update_delegate_agents_config(
    _update: impl FnOnce(&mut HashMap<String, DelegateAgentConfig>),
) {
}

#[cfg(target_arch = "wasm32")]
/// 读取、保存或转换 `update_delegate_agents_config_result` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn update_delegate_agents_config_result(
    _update: impl FnOnce(&mut HashMap<String, DelegateAgentConfig>),
) -> Result<(), String> {
    Ok(())
}

#[cfg(target_arch = "wasm32")]
/// 读取、保存或转换 `update_main_agent_overrides_from_delegate_agents` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn update_main_agent_overrides_from_delegate_agents() {}

define_agent_config_update_result_fns!(
    (update_heartbeat_config_result, update_heartbeat_config, HeartbeatConfig, heartbeat, &["heartbeat"]),
    (update_cron_config_result, update_cron_config, CronConfig, cron, &["cron"]),
    (update_scheduler_config_result, update_scheduler_config, SchedulerConfig, scheduler, &["scheduler"]),
    (update_reliability_config_result, update_reliability_config, ReliabilityConfig, reliability, &["reliability"]),
    (update_memory_config_result, update_memory_config, MemoryConfig, memory, &["memory"]),
    (update_security_config_result, update_security_config, SecurityConfig, security, &["security"]),
    (update_channels_config_result, update_channels_config, ChannelsConfig, channels_config, &["channels_config"]),
    (update_observability_config_result, update_observability_config, ObservabilityConfig, observability, &["observability"]),
    (update_storage_config_result, update_storage_config, StorageConfig, storage, &["storage"]),
    (update_proxy_config_result, update_proxy_config, ProxyConfig, proxy, &["proxy"]),
    (update_browser_config_result, update_browser_config, BrowserConfig, browser, &["browser"]),
    (update_http_request_config_result, update_http_request_config, HttpRequestConfig, http_request, &["http_request"]),
    (update_multimodal_config_result, update_multimodal_config, MultimodalConfig, multimodal, &["multimodal"]),
    (update_web_search_config_result, update_web_search_config, WebSearchConfig, web_search, &["web_search"]),
    (update_tunnel_config_result, update_tunnel_config, TunnelConfig, tunnel, &["tunnel"]),
    (update_hooks_config_result, update_hooks_config, HooksConfig, hooks, &["hooks"]),
    (update_skills_config_result, update_skills_config, SkillsConfig, skills, &["skills"]),
    (update_runtime_config_result, update_runtime_config, RuntimeConfig, runtime, &["runtime"]),
    (update_model_routes_config_result, update_model_routes_config, Vec<ModelRouteConfig>, model_routes, &["model_routes"]),
    (update_embedding_routes_config_result, update_embedding_routes_config, Vec<EmbeddingRouteConfig>, embedding_routes, &["embedding_routes"]),
    (update_query_classification_config_result, update_query_classification_config, QueryClassificationConfig, query_classification, &["query_classification"]),
    (update_autonomy_config_result, update_autonomy_config, AutonomyConfig, autonomy, &["autonomy"]),
);

define_agent_config_update_result_async_fns!(
    (update_heartbeat_config_result_async, HeartbeatConfig, heartbeat, &["heartbeat"]),
    (update_cron_config_result_async, CronConfig, cron, &["cron"]),
    (update_scheduler_config_result_async, SchedulerConfig, scheduler, &["scheduler"]),
    (update_reliability_config_result_async, ReliabilityConfig, reliability, &["reliability"]),
    (update_memory_config_result_async, MemoryConfig, memory, &["memory"]),
    (update_security_config_result_async, SecurityConfig, security, &["security"]),
    (update_channels_config_result_async, ChannelsConfig, channels_config, &["channels_config"]),
    (update_observability_config_result_async, ObservabilityConfig, observability, &["observability"]),
    (update_storage_config_result_async, StorageConfig, storage, &["storage"]),
    (update_proxy_config_result_async, ProxyConfig, proxy, &["proxy"]),
    (update_browser_config_result_async, BrowserConfig, browser, &["browser"]),
    (update_http_request_config_result_async, HttpRequestConfig, http_request, &["http_request"]),
    (update_multimodal_config_result_async, MultimodalConfig, multimodal, &["multimodal"]),
    (update_web_search_config_result_async, WebSearchConfig, web_search, &["web_search"]),
    (update_tunnel_config_result_async, TunnelConfig, tunnel, &["tunnel"]),
    (update_hooks_config_result_async, HooksConfig, hooks, &["hooks"]),
    (update_skills_config_result_async, SkillsConfig, skills, &["skills"]),
    (update_runtime_config_result_async, RuntimeConfig, runtime, &["runtime"]),
    (update_model_routes_config_result_async, Vec<ModelRouteConfig>, model_routes, &["model_routes"]),
    (update_embedding_routes_config_result_async, Vec<EmbeddingRouteConfig>, embedding_routes, &["embedding_routes"]),
    (update_query_classification_config_result_async, QueryClassificationConfig, query_classification, &["query_classification"]),
    (update_autonomy_config_result_async, AutonomyConfig, autonomy, &["autonomy"]),
);

define_agent_config_update_async_fns!(
    (update_heartbeat_config_async, update_heartbeat_config_result_async, HeartbeatConfig, heartbeat, &["heartbeat"]),
    (update_cron_config_async, update_cron_config_result_async, CronConfig, cron, &["cron"]),
    (update_scheduler_config_async, update_scheduler_config_result_async, SchedulerConfig, scheduler, &["scheduler"]),
    (update_reliability_config_async, update_reliability_config_result_async, ReliabilityConfig, reliability, &["reliability"]),
    (update_memory_config_async, update_memory_config_result_async, MemoryConfig, memory, &["memory"]),
    (update_security_config_async, update_security_config_result_async, SecurityConfig, security, &["security"]),
    (update_channels_config_async, update_channels_config_result_async, ChannelsConfig, channels_config, &["channels_config"]),
    (update_observability_config_async, update_observability_config_result_async, ObservabilityConfig, observability, &["observability"]),
    (update_storage_config_async, update_storage_config_result_async, StorageConfig, storage, &["storage"]),
    (update_proxy_config_async, update_proxy_config_result_async, ProxyConfig, proxy, &["proxy"]),
    (update_browser_config_async, update_browser_config_result_async, BrowserConfig, browser, &["browser"]),
    (update_http_request_config_async, update_http_request_config_result_async, HttpRequestConfig, http_request, &["http_request"]),
    (update_multimodal_config_async, update_multimodal_config_result_async, MultimodalConfig, multimodal, &["multimodal"]),
    (update_web_search_config_async, update_web_search_config_result_async, WebSearchConfig, web_search, &["web_search"]),
    (update_tunnel_config_async, update_tunnel_config_result_async, TunnelConfig, tunnel, &["tunnel"]),
    (update_hooks_config_async, update_hooks_config_result_async, HooksConfig, hooks, &["hooks"]),
    (update_skills_config_async, update_skills_config_result_async, SkillsConfig, skills, &["skills"]),
    (update_runtime_config_async, update_runtime_config_result_async, RuntimeConfig, runtime, &["runtime"]),
    (update_model_routes_config_async, update_model_routes_config_result_async, Vec<ModelRouteConfig>, model_routes, &["model_routes"]),
    (update_embedding_routes_config_async, update_embedding_routes_config_result_async, Vec<EmbeddingRouteConfig>, embedding_routes, &["embedding_routes"]),
    (update_query_classification_config_async, update_query_classification_config_result_async, QueryClassificationConfig, query_classification, &["query_classification"]),
    (update_autonomy_config_async, update_autonomy_config_result_async, AutonomyConfig, autonomy, &["autonomy"]),
);

/// 读取、保存或转换 `update_gateway_config_async` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn update_gateway_config_async(
    update: impl FnOnce(&mut GatewayConfig) + Send + 'static,
) -> Task<Message> {
    spawn_gateway_task("gateway", async move { update_gateway_config_result_async(update).await })
}

#[cfg(not(target_arch = "wasm32"))]
/// 读取、保存或转换 `update_delegate_agents_config` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn update_delegate_agents_config(
    update: impl FnOnce(&mut HashMap<String, DelegateAgentConfig>),
) {
    let mut cfg = load_agent_config_via_gateway().agents;
    update(&mut cfg);
    if let Ok(value) = serde_json::to_value(cfg) {
        patch_agent_config(&["agents"], value);
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// 读取、保存或转换 `update_delegate_agents_config_result` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn update_delegate_agents_config_result(
    update: impl FnOnce(&mut HashMap<String, DelegateAgentConfig>),
) -> Result<(), String> {
    let mut cfg = load_full_agent_config_result()?.agents;
    update(&mut cfg);
    let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
    patch_agent_config_result(&["agents"], value)
}

/// 读取、保存或转换 `update_delegate_agents_config_result_async` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub async fn update_delegate_agents_config_result_async(
    update: impl FnOnce(&mut HashMap<String, DelegateAgentConfig>),
) -> Result<(), String> {
    let mut cfg = load_full_agent_config_async().await?.agents;
    update(&mut cfg);
    let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
    patch_full_agent_config_async(serde_json::json!({ "agents": value })).await
}

#[cfg(target_arch = "wasm32")]
/// 读取、保存或转换 `update_delegate_agents_config_async` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn update_delegate_agents_config_async(
    update: impl FnOnce(&mut HashMap<String, DelegateAgentConfig>) + 'static,
) -> Task<Message> {
    spawn_gateway_task("agents", async move {
        let mut cfg = load_full_agent_config_async().await?.agents;
        update(&mut cfg);
        let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
        patch_full_agent_config_async(serde_json::json!({ "agents": value })).await
    })
}

#[cfg(not(target_arch = "wasm32"))]
/// 读取、保存或转换 `update_delegate_agents_config_async` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn update_delegate_agents_config_async(
    update: impl FnOnce(&mut HashMap<String, DelegateAgentConfig>) + Send + 'static,
) -> Task<Message> {
    spawn_gateway_task("agents", async move {
        let mut cfg = load_full_agent_config_result()?.agents;
        update(&mut cfg);
        let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
        patch_agent_config_result(&["agents"], value)
    })
}

#[cfg(not(target_arch = "wasm32"))]
/// 读取、保存或转换 `update_main_agent_overrides_from_delegate_agents` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub fn update_main_agent_overrides_from_delegate_agents() {
    let cfg = load_agent_config_via_gateway();
    let Some(main) = cfg.agents.get("main") else {
        return;
    };

    let provider = main.provider.trim().to_string();
    if !provider.is_empty() {
        patch_agent_config(&["default_provider"], serde_json::Value::String(provider.clone()));
    }

    let model = main.model.trim().to_string();
    if !provider.is_empty() && !model.is_empty() {
        patch_agent_config(
            &["default_model"],
            serde_json::Value::String(format!("{provider}/{model}")),
        );
    }

    if let Some(temperature) = main.temperature
        && let Some(number) = serde_json::Number::from_f64(temperature)
    {
        patch_agent_config(&["default_temperature"], serde_json::Value::Number(number));
    }

    let identity =
        IdentityConfig { format: "openclaw".to_string(), aieos_path: None, aieos_inline: None };
    if let Ok(value) = serde_json::to_value(identity) {
        patch_agent_config(&["identity"], value);
    }
}

/// 读取、保存或转换 `update_main_agent_overrides_from_delegate_agents_async` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 当底层配置、文件或运行时调用失败时，错误会通过 `Result` 返回给上层统一处理。
pub async fn update_main_agent_overrides_from_delegate_agents_async() -> Result<(), String> {
    let cfg = load_full_agent_config_async().await?;
    let Some(patch) = build_main_agent_overrides_patch(&cfg)? else {
        return Ok(());
    };

    patch_full_agent_config_async(patch).await
}

/// 读取、保存或转换 `update_gateway_config_result_async` 对应的配置数据与运行时状态。
///
/// # 参数
///
/// 参数来自调用方持有的配置快照、用户输入或网关返回值，用于保持持久化状态与运行时状态同步。
///
/// # 返回值
///
/// 返回加载后的配置、归一化后的状态或持久化操作结果，供上层设置流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；缺省值和不可用状态会通过显式配置字段表达。
pub async fn update_gateway_config_result_async(
    update: impl FnOnce(&mut GatewayConfig),
) -> Result<(), String> {
    let mut cfg = load_full_agent_config_async().await?.gateway;
    update(&mut cfg);
    let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
    patch_full_agent_config_async(serde_json::json!({ "gateway": value })).await
}
