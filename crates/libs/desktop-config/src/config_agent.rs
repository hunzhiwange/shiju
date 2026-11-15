//! Agent configuration operations and ACP patch construction.
use config_observability::ObservabilityConfig;
use config_hooks::HooksConfig;
use config_automation::{CronConfig, HeartbeatConfig, SchedulerConfig};
use config_reliability::ReliabilityConfig;
use config_runtime::RuntimeConfig;
use iced::Task;
use std::collections::{BTreeMap, HashMap};
use config_skills::SkillsConfig;
use config_agent::DelegateAgentConfig;
use vw_config_types::{
    channels::ChannelsConfig,
    config::{AcpAgentConfig, Config},
    gateway::{GatewayConfig, TunnelConfig},
    memory::{MemoryConfig, StorageConfig},
    proxy::ProxyConfig,
    routing::{EmbeddingRouteConfig, ModelRouteConfig, QueryClassificationConfig},
    security::{AutonomyConfig, IdentityConfig, SecurityConfig},
    tools::{BrowserConfig, HttpRequestConfig, MultimodalConfig, WebSearchConfig},
};

use super::access::*;
use super::{ConfigAccess, ConfigSaved, spawn_gateway_task};
#[derive(Debug, Clone)]
pub struct AcpSettingsSnapshot {
    pub defaults: BTreeMap<String, AcpAgentConfig>,
    pub catalog: BTreeMap<String, AcpAgentConfig>,
    pub enabled: BTreeMap<String, AcpAgentConfig>,
}

const DEFAULT_ENABLED_ACP_AGENTS: &[&str] =
    &["claude", "gemini", "opencode", "codex", "openclaw", "copilot"];

#[allow(dead_code)]

fn default_enabled_acp_config(
    catalog: &HashMap<String, AcpAgentConfig>,
) -> HashMap<String, AcpAgentConfig> {
    DEFAULT_ENABLED_ACP_AGENTS
        .iter()
        .filter_map(|name| catalog.get(*name).cloned().map(|config| ((*name).to_string(), config)))
        .collect()
}
pub fn resolve_enabled_acp_config(
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
    build_acp_agent_enabled_patch(agent_name, true, Some(spec), catalog, configured)
}
fn build_acp_agent_edit_patch(
    agent_name: &str,
    spec: AcpAgentConfig,
    catalog: &HashMap<String, AcpAgentConfig>,
    configured: HashMap<String, AcpAgentConfig>,
) -> Result<serde_json::Value, String> {
    let name = agent_name.trim();
    let old = catalog.get(name).ok_or_else(|| format!("unknown acp agent: {name}"))?;
    if spec.command.trim().is_empty() {
        return Err("acp agent command must not be empty".into());
    }
    let mut value = serde_json::to_value(&spec).map_err(|err| err.to_string())?;
    for key in old.env.keys().chain(configured.get(name).into_iter().flat_map(|v| v.env.keys())) {
        if !spec.env.contains_key(key) {
            value["env"][key] = serde_json::Value::Null;
        }
    }
    let mut patch = serde_json::json!({"acp_saved": {name: value.clone()}});
    if resolve_enabled_acp_config(catalog, configured.clone()).contains_key(name) {
        let enabled_patch =
            build_acp_agent_enabled_patch(name, true, Some(spec), catalog, configured)?;
        patch["acp"] = enabled_patch["acp"].clone();
        patch["acp"][name] = value;
    }
    Ok(patch)
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
impl ConfigAccess {
pub async fn save_global_acp_agent_async(&self,
    agent_name: String,
    spec: AcpAgentConfig,
) -> Result<AcpSettingsSnapshot, String> {
    let snapshot = self.load_acp_settings_snapshot_async().await?;
    let catalog = snapshot.catalog.into_iter().collect();
    let configured = self.fetch_global_agent_config_via_gateway().await?.acp;
    let patch = build_acp_agent_edit_patch(&agent_name, spec, &catalog, configured)?;
    self.patch_agent_config_via_gateway(&patch).await?;
    self.load_acp_settings_snapshot_async().await
}
async fn fetch_agent_config_via_gateway(&self) -> Result<Config, String> {
    let client = self.gateway_client()?;
    let value = gateway_config::GatewayConfigClient::new(client.transport().clone()).config_get(None).await?;
    let mut config = serde_json::from_value::<Config>(value).map_err(|err| err.to_string())?;
    apply_main_agent_overrides(&mut config);
    Ok(config)
}
async fn fetch_global_agent_config_via_gateway(&self) -> Result<Config, String> {
    let client = self.gateway_client()?;
    let value = gateway_config::GatewayConfigClient::new(client.transport().clone()).global_config_get().await?;
    let mut config = serde_json::from_value::<Config>(value).map_err(|err| err.to_string())?;
    apply_main_agent_overrides(&mut config);
    Ok(config)
}
async fn fetch_global_config_value_via_gateway(&self) -> Result<serde_json::Value, String> {
    let client = self.gateway_client()?;
    gateway_config::GatewayConfigClient::new(client.transport().clone()).global_config_get().await
}
#[cfg(not(target_arch = "wasm32"))]
pub fn load_local_agent_config(&self) -> Option<Config> {
    let mut config = self.local_config()?;
    config.gateway.skeys.clear();
    apply_main_agent_overrides(&mut config);
    Some(config)
}
#[cfg(target_arch = "wasm32")]
pub fn load_local_agent_config(&self) -> Option<Config> {
    None
}
async fn fetch_global_acp_config_via_gateway(&self) -> Result<HashMap<String, AcpAgentConfig>, String> {
    let client = self.gateway_client()?;
    gateway_config::GatewayConfigClient::new(client.transport().clone()).global_acp_config_get().await
}
async fn patch_agent_config_via_gateway(&self, patch: &serde_json::Value) -> Result<(), String> {
    let client = self.gateway_client()?;
    gateway_config::GatewayConfigClient::new(client.transport().clone()).global_config_patch(patch).await
}
fn load_agent_config_via_gateway(&self) -> Config {
    let result = run_gateway_call(async { self.fetch_agent_config_via_gateway().await });

    match result {
        Ok(config) => config,
        Err(err) => {
            if gateway_http::is_gateway_unavailable_error(&err) {
                tracing::debug!(target: "vw_desktop", error = %err, "failed to load agent config via gateway");
            } else {
                tracing::warn!(target: "vw_desktop", error = %err, "failed to load agent config via gateway");
            }
            if let Some(config) = self.load_local_agent_config() {
                return config;
            }
            Config::default()
        }
    }
}
fn patch_agent_config(&self, path: &[&str], value: serde_json::Value) {
    let mut patch = serde_json::json!({});
    set_config_value_at_path(&mut patch, path, value);

    let outcome = run_gateway_call(async { self.patch_agent_config_via_gateway(&patch).await });

    if let Err(err) = outcome {
        tracing::warn!(target: "vw_desktop", error = %err, "failed to patch agent config via gateway");
    }
}
pub async fn load_full_agent_config_async(&self) -> Result<Config, String> {
    self.fetch_agent_config_via_gateway().await
}
pub async fn load_project_agent_config_async(&self, project_path: &str) -> Result<Config, String> {
    let client = self.gateway_client()?;
    let value = gateway_config::GatewayConfigClient::new(client.transport().clone()).config_get(Some(project_path)).await?;
    serde_json::from_value(value).map_err(|error| error.to_string())
}
pub async fn load_global_config_value_async(&self) -> Result<serde_json::Value, String> {
    self.fetch_global_config_value_via_gateway().await
}
pub async fn load_mcp_inspect_async(&self)
-> Result<vw_gateway_client::vw_api_types::mcp::McpInspectResponse, String> {
    let client = self.gateway_client()?;
    client.mcp_inspect().await
}
pub async fn load_browser_config_async(&self) -> Result<BrowserConfig, String> {
    Ok(self.load_full_agent_config_async().await?.browser)
}
pub fn load_gateway_config_result(&self) -> Result<GatewayConfig, String> {
    match run_gateway_call(async { self.fetch_global_agent_config_via_gateway().await }) {
        Ok(config) => Ok(config.gateway),
        Err(err) => self.load_local_agent_config().map(|config| config.gateway).ok_or(err),
    }
}
pub fn load_global_acp_config_result(&self) -> Result<HashMap<String, AcpAgentConfig>, String> {
    run_gateway_call(async { self.fetch_global_acp_config_via_gateway().await })
}
pub async fn load_global_acp_config_async(&self) -> Result<HashMap<String, AcpAgentConfig>, String> {
    self.fetch_global_acp_config_via_gateway().await
}
pub fn load_enabled_acp_config_result(&self) -> Result<HashMap<String, AcpAgentConfig>, String> {
    run_gateway_call(async {
        let catalog = self.fetch_global_acp_config_via_gateway().await?;
        let enabled = self.fetch_global_agent_config_via_gateway().await?.acp;
        Ok(resolve_enabled_acp_config(&catalog, enabled))
    })
}
pub async fn load_enabled_acp_config_async(&self) -> Result<HashMap<String, AcpAgentConfig>, String> {
    let catalog = self.fetch_global_acp_config_via_gateway().await?;
    let enabled = self.fetch_global_agent_config_via_gateway().await?.acp;
    Ok(resolve_enabled_acp_config(&catalog, enabled))
}
pub async fn load_acp_settings_snapshot_async(&self) -> Result<AcpSettingsSnapshot, String> {
    let defaults = gateway_config::GatewayConfigClient::new(self.gateway_client()?.transport().clone()).global_acp_defaults_get().await?;
    let mut catalog = self.fetch_global_acp_config_via_gateway().await?;
    let config = self.fetch_global_agent_config_via_gateway().await?;
    catalog.extend(config.acp_saved);
    let enabled = resolve_enabled_acp_config(&catalog, config.acp);

    Ok(AcpSettingsSnapshot {
        defaults,
        catalog: catalog.into_iter().collect(),
        enabled: enabled.into_iter().collect(),
    })
}
pub async fn set_global_acp_agent_enabled_async(&self,
    agent_name: String,
    enabled: bool,
    spec: Option<AcpAgentConfig>,
) -> Result<AcpSettingsSnapshot, String> {
    let catalog = self.fetch_global_acp_config_via_gateway().await?;
    let configured = self.fetch_global_agent_config_via_gateway().await?.acp;
    let patch = build_acp_agent_enabled_patch(&agent_name, enabled, spec, &catalog, configured)?;

    self.patch_agent_config_via_gateway(&patch).await?;

    self.load_acp_settings_snapshot_async().await
}
pub async fn add_global_acp_agent_async(&self,
    agent_name: String,
    spec: AcpAgentConfig,
) -> Result<AcpSettingsSnapshot, String> {
    let catalog = self.fetch_global_acp_config_via_gateway().await?;
    let configured = self.fetch_global_agent_config_via_gateway().await?.acp;
    let patch = build_acp_agent_create_patch(&agent_name, spec, &catalog, configured)?;

    self.patch_agent_config_via_gateway(&patch).await?;

    self.load_acp_settings_snapshot_async().await
}
pub async fn patch_full_agent_config_async(&self, patch: serde_json::Value) -> Result<(), String> {
    self.patch_agent_config_via_gateway(&patch).await
}
pub async fn patch_project_agent_config_async(&self,
    project_path: &str,
    patch: serde_json::Value,
) -> Result<(), String> {
    let client = self.gateway_client()?;
    gateway_config::GatewayConfigClient::new(client.transport().clone()).config_patch(Some(project_path), &patch).await.map(|_| ())
}
pub fn patch_agent_config_result(&self, path: &[&str], value: serde_json::Value) -> Result<(), String> {
    let mut patch = serde_json::json!({});
    set_config_value_at_path(&mut patch, path, value);

    run_gateway_call(async { self.patch_agent_config_via_gateway(&patch).await })
}
pub fn update_gateway_config_result(&self, update: impl FnOnce(&mut GatewayConfig)) -> Result<(), String> {
    let mut cfg = self.load_gateway_config_result()?;
    update(&mut cfg);
    let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;

    run_gateway_call(async {
        self.patch_agent_config_via_gateway(&serde_json::json!({ "gateway": value })).await
    })
}
pub async fn remove_global_provider_via_gateway(&self, provider_id: &str) -> Result<(), String> {
    let provider_id = provider_id.trim();
    if provider_id.is_empty() {
        return Err("provider_id must not be empty".to_string());
    }

    let mut providers = serde_json::Map::new();
    providers.insert(provider_id.to_string(), serde_json::Value::Null);

    let mut root = serde_json::Map::new();
    root.insert("providers".to_string(), serde_json::Value::Object(providers));

    self.patch_agent_config_via_gateway(&serde_json::Value::Object(root)).await
}
pub fn load_gateway_config(&self) -> GatewayConfig {
    self.load_gateway_config_result().unwrap_or_else(|err| {
        if gateway_http::is_gateway_unavailable_error(&err) {
            tracing::debug!(target: "vw_desktop", error = %err, "failed to load gateway config via gateway");
        } else {
            tracing::warn!(target: "vw_desktop", error = %err, "failed to load gateway config via gateway");
        }
        GatewayConfig::default()
    })
}
pub fn load_heartbeat_config(&self) -> HeartbeatConfig {
    self.load_agent_config_via_gateway().heartbeat
}
pub fn load_cron_config(&self) -> CronConfig {
    self.load_agent_config_via_gateway().cron
}
pub fn load_scheduler_config(&self) -> SchedulerConfig {
    self.load_agent_config_via_gateway().scheduler
}
pub fn load_reliability_config(&self) -> ReliabilityConfig {
    self.load_agent_config_via_gateway().reliability
}
pub fn load_memory_config(&self) -> MemoryConfig {
    self.load_agent_config_via_gateway().memory
}
pub fn load_security_config(&self) -> SecurityConfig {
    self.load_agent_config_via_gateway().security
}
pub fn load_channels_config(&self) -> ChannelsConfig {
    self.load_agent_config_via_gateway().channels_config
}
pub fn load_observability_config(&self) -> ObservabilityConfig {
    self.load_agent_config_via_gateway().observability
}
pub fn load_storage_config(&self) -> StorageConfig {
    self.load_agent_config_via_gateway().storage
}
pub fn load_proxy_config(&self) -> ProxyConfig {
    self.load_agent_config_via_gateway().proxy
}
pub fn load_browser_config(&self) -> BrowserConfig {
    self.load_agent_config_via_gateway().browser
}
pub fn load_http_request_config(&self) -> HttpRequestConfig {
    self.load_agent_config_via_gateway().http_request
}
pub fn load_multimodal_config(&self) -> MultimodalConfig {
    self.load_agent_config_via_gateway().multimodal
}
pub fn load_web_search_config(&self) -> WebSearchConfig {
    self.load_agent_config_via_gateway().web_search
}
#[cfg(not(target_arch = "wasm32"))]
pub fn load_tunnel_config(&self) -> TunnelConfig {
    self.load_agent_config_via_gateway().tunnel
}
#[cfg(not(target_arch = "wasm32"))]
pub fn load_hooks_config(&self) -> HooksConfig {
    self.load_agent_config_via_gateway().hooks
}
#[cfg(not(target_arch = "wasm32"))]
pub fn load_skills_config(&self) -> SkillsConfig {
    self.load_agent_config_via_gateway().skills
}
#[cfg(not(target_arch = "wasm32"))]
pub fn load_delegate_agents_config(&self) -> HashMap<String, DelegateAgentConfig> {
    self.load_agent_config_via_gateway().agents
}
#[cfg(not(target_arch = "wasm32"))]
pub fn load_embedding_routes_config(&self) -> Vec<EmbeddingRouteConfig> {
    self.load_agent_config_via_gateway().embedding_routes
}
#[cfg(not(target_arch = "wasm32"))]
pub fn load_autonomy_config(&self) -> AutonomyConfig {
    self.load_agent_config_via_gateway().autonomy
}
#[cfg(not(target_arch = "wasm32"))]
pub fn load_identity_config(&self) -> IdentityConfig {
    self.load_agent_config_via_gateway().identity
}
#[cfg(not(target_arch = "wasm32"))]
pub fn load_full_agent_config(&self) -> Config {
    self.load_agent_config_via_gateway()
}
#[cfg(not(target_arch = "wasm32"))]
pub fn load_full_agent_config_result(&self) -> Result<Config, String> {
    match run_gateway_call(async { self.fetch_agent_config_via_gateway().await }) {
        Ok(config) => Ok(config),
        Err(err) => self.load_local_agent_config().ok_or(err),
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub fn update_gateway_config(&self, update: impl FnOnce(&mut GatewayConfig)) {
    let mut cfg = self.load_gateway_config();
    update(&mut cfg);
    if let Ok(value) = serde_json::to_value(cfg) {
        self.patch_agent_config(&["gateway"], value);
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub fn load_runtime_config(&self) -> RuntimeConfig {
    self.load_agent_config_via_gateway().runtime
}
#[cfg(not(target_arch = "wasm32"))]
pub fn load_model_routes_config(&self) -> Vec<ModelRouteConfig> {
    self.load_agent_config_via_gateway().model_routes
}
#[cfg(not(target_arch = "wasm32"))]
pub fn load_query_classification_config(&self) -> QueryClassificationConfig {
    self.load_agent_config_via_gateway().query_classification
}
#[cfg(target_arch = "wasm32")]
pub fn load_tunnel_config(&self) -> TunnelConfig {
    TunnelConfig::default()
}
#[cfg(target_arch = "wasm32")]
pub fn load_hooks_config(&self) -> HooksConfig {
    HooksConfig::default()
}
#[cfg(target_arch = "wasm32")]
pub fn load_skills_config(&self) -> SkillsConfig {
    SkillsConfig::default()
}
#[cfg(target_arch = "wasm32")]
pub fn load_delegate_agents_config(&self) -> HashMap<String, DelegateAgentConfig> {
    HashMap::new()
}
#[cfg(target_arch = "wasm32")]
pub fn load_embedding_routes_config(&self) -> Vec<EmbeddingRouteConfig> {
    Vec::new()
}
#[cfg(target_arch = "wasm32")]
pub fn load_autonomy_config(&self) -> AutonomyConfig {
    AutonomyConfig::default()
}
#[cfg(target_arch = "wasm32")]
pub fn load_identity_config(&self) -> IdentityConfig {
    IdentityConfig::default()
}
#[cfg(target_arch = "wasm32")]
pub fn load_full_agent_config(&self) -> Config {
    Config::default()
}
#[cfg(target_arch = "wasm32")]
pub fn load_full_agent_config_result(&self) -> Result<Config, String> {
    Ok(Config::default())
}
#[cfg(target_arch = "wasm32")]
pub fn update_gateway_config(&self, _update: impl FnOnce(&mut GatewayConfig)) {}
#[cfg(target_arch = "wasm32")]
pub fn load_runtime_config(&self) -> RuntimeConfig {
    RuntimeConfig::default()
}
#[cfg(target_arch = "wasm32")]
pub fn load_model_routes_config(&self) -> Vec<ModelRouteConfig> {
    Vec::new()
}
#[cfg(target_arch = "wasm32")]
pub fn load_query_classification_config(&self) -> QueryClassificationConfig {
    QueryClassificationConfig::default()
}
#[cfg(target_arch = "wasm32")]
pub fn update_delegate_agents_config(&self,
    _update: impl FnOnce(&mut HashMap<String, DelegateAgentConfig>),
) {
}
#[cfg(target_arch = "wasm32")]
pub fn update_delegate_agents_config_result(&self,
    _update: impl FnOnce(&mut HashMap<String, DelegateAgentConfig>),
) -> Result<(), String> {
    Ok(())
}
#[cfg(target_arch = "wasm32")]
pub fn update_main_agent_overrides_from_delegate_agents(&self) {}
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_heartbeat_config_result(&self, update: impl FnOnce(&mut HeartbeatConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_result()?.heartbeat;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_agent_config_result(&["heartbeat"], value)
            }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_heartbeat_config(&self, update: impl FnOnce(&mut HeartbeatConfig)) {
                if let Err(err) = self.update_heartbeat_config_result(update) {
                    tracing::warn!(target: "vw_desktop", error = %err, "failed to update agent config via gateway");
                }
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_heartbeat_config_result(&self, _update: impl FnOnce(&mut HeartbeatConfig)) -> Result<(), String> {
                Ok(())
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_heartbeat_config(&self, _update: impl FnOnce(&mut HeartbeatConfig)) {}
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_cron_config_result(&self, update: impl FnOnce(&mut CronConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_result()?.cron;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_agent_config_result(&["cron"], value)
            }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_cron_config(&self, update: impl FnOnce(&mut CronConfig)) {
                if let Err(err) = self.update_cron_config_result(update) {
                    tracing::warn!(target: "vw_desktop", error = %err, "failed to update agent config via gateway");
                }
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_cron_config_result(&self, _update: impl FnOnce(&mut CronConfig)) -> Result<(), String> {
                Ok(())
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_cron_config(&self, _update: impl FnOnce(&mut CronConfig)) {}
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_scheduler_config_result(&self, update: impl FnOnce(&mut SchedulerConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_result()?.scheduler;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_agent_config_result(&["scheduler"], value)
            }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_scheduler_config(&self, update: impl FnOnce(&mut SchedulerConfig)) {
                if let Err(err) = self.update_scheduler_config_result(update) {
                    tracing::warn!(target: "vw_desktop", error = %err, "failed to update agent config via gateway");
                }
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_scheduler_config_result(&self, _update: impl FnOnce(&mut SchedulerConfig)) -> Result<(), String> {
                Ok(())
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_scheduler_config(&self, _update: impl FnOnce(&mut SchedulerConfig)) {}
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_reliability_config_result(&self, update: impl FnOnce(&mut ReliabilityConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_result()?.reliability;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_agent_config_result(&["reliability"], value)
            }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_reliability_config(&self, update: impl FnOnce(&mut ReliabilityConfig)) {
                if let Err(err) = self.update_reliability_config_result(update) {
                    tracing::warn!(target: "vw_desktop", error = %err, "failed to update agent config via gateway");
                }
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_reliability_config_result(&self, _update: impl FnOnce(&mut ReliabilityConfig)) -> Result<(), String> {
                Ok(())
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_reliability_config(&self, _update: impl FnOnce(&mut ReliabilityConfig)) {}
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_memory_config_result(&self, update: impl FnOnce(&mut MemoryConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_result()?.memory;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_agent_config_result(&["memory"], value)
            }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_memory_config(&self, update: impl FnOnce(&mut MemoryConfig)) {
                if let Err(err) = self.update_memory_config_result(update) {
                    tracing::warn!(target: "vw_desktop", error = %err, "failed to update agent config via gateway");
                }
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_memory_config_result(&self, _update: impl FnOnce(&mut MemoryConfig)) -> Result<(), String> {
                Ok(())
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_memory_config(&self, _update: impl FnOnce(&mut MemoryConfig)) {}
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_security_config_result(&self, update: impl FnOnce(&mut SecurityConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_result()?.security;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_agent_config_result(&["security"], value)
            }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_security_config(&self, update: impl FnOnce(&mut SecurityConfig)) {
                if let Err(err) = self.update_security_config_result(update) {
                    tracing::warn!(target: "vw_desktop", error = %err, "failed to update agent config via gateway");
                }
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_security_config_result(&self, _update: impl FnOnce(&mut SecurityConfig)) -> Result<(), String> {
                Ok(())
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_security_config(&self, _update: impl FnOnce(&mut SecurityConfig)) {}
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_channels_config_result(&self, update: impl FnOnce(&mut ChannelsConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_result()?.channels_config;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_agent_config_result(&["channels_config"], value)
            }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_channels_config(&self, update: impl FnOnce(&mut ChannelsConfig)) {
                if let Err(err) = self.update_channels_config_result(update) {
                    tracing::warn!(target: "vw_desktop", error = %err, "failed to update agent config via gateway");
                }
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_channels_config_result(&self, _update: impl FnOnce(&mut ChannelsConfig)) -> Result<(), String> {
                Ok(())
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_channels_config(&self, _update: impl FnOnce(&mut ChannelsConfig)) {}
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_observability_config_result(&self, update: impl FnOnce(&mut ObservabilityConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_result()?.observability;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_agent_config_result(&["observability"], value)
            }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_observability_config(&self, update: impl FnOnce(&mut ObservabilityConfig)) {
                if let Err(err) = self.update_observability_config_result(update) {
                    tracing::warn!(target: "vw_desktop", error = %err, "failed to update agent config via gateway");
                }
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_observability_config_result(&self, _update: impl FnOnce(&mut ObservabilityConfig)) -> Result<(), String> {
                Ok(())
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_observability_config(&self, _update: impl FnOnce(&mut ObservabilityConfig)) {}
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_storage_config_result(&self, update: impl FnOnce(&mut StorageConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_result()?.storage;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_agent_config_result(&["storage"], value)
            }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_storage_config(&self, update: impl FnOnce(&mut StorageConfig)) {
                if let Err(err) = self.update_storage_config_result(update) {
                    tracing::warn!(target: "vw_desktop", error = %err, "failed to update agent config via gateway");
                }
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_storage_config_result(&self, _update: impl FnOnce(&mut StorageConfig)) -> Result<(), String> {
                Ok(())
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_storage_config(&self, _update: impl FnOnce(&mut StorageConfig)) {}
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_proxy_config_result(&self, update: impl FnOnce(&mut ProxyConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_result()?.proxy;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_agent_config_result(&["proxy"], value)
            }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_proxy_config(&self, update: impl FnOnce(&mut ProxyConfig)) {
                if let Err(err) = self.update_proxy_config_result(update) {
                    tracing::warn!(target: "vw_desktop", error = %err, "failed to update agent config via gateway");
                }
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_proxy_config_result(&self, _update: impl FnOnce(&mut ProxyConfig)) -> Result<(), String> {
                Ok(())
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_proxy_config(&self, _update: impl FnOnce(&mut ProxyConfig)) {}
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_browser_config_result(&self, update: impl FnOnce(&mut BrowserConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_result()?.browser;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_agent_config_result(&["browser"], value)
            }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_browser_config(&self, update: impl FnOnce(&mut BrowserConfig)) {
                if let Err(err) = self.update_browser_config_result(update) {
                    tracing::warn!(target: "vw_desktop", error = %err, "failed to update agent config via gateway");
                }
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_browser_config_result(&self, _update: impl FnOnce(&mut BrowserConfig)) -> Result<(), String> {
                Ok(())
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_browser_config(&self, _update: impl FnOnce(&mut BrowserConfig)) {}
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_http_request_config_result(&self, update: impl FnOnce(&mut HttpRequestConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_result()?.http_request;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_agent_config_result(&["http_request"], value)
            }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_http_request_config(&self, update: impl FnOnce(&mut HttpRequestConfig)) {
                if let Err(err) = self.update_http_request_config_result(update) {
                    tracing::warn!(target: "vw_desktop", error = %err, "failed to update agent config via gateway");
                }
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_http_request_config_result(&self, _update: impl FnOnce(&mut HttpRequestConfig)) -> Result<(), String> {
                Ok(())
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_http_request_config(&self, _update: impl FnOnce(&mut HttpRequestConfig)) {}
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_multimodal_config_result(&self, update: impl FnOnce(&mut MultimodalConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_result()?.multimodal;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_agent_config_result(&["multimodal"], value)
            }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_multimodal_config(&self, update: impl FnOnce(&mut MultimodalConfig)) {
                if let Err(err) = self.update_multimodal_config_result(update) {
                    tracing::warn!(target: "vw_desktop", error = %err, "failed to update agent config via gateway");
                }
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_multimodal_config_result(&self, _update: impl FnOnce(&mut MultimodalConfig)) -> Result<(), String> {
                Ok(())
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_multimodal_config(&self, _update: impl FnOnce(&mut MultimodalConfig)) {}
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_web_search_config_result(&self, update: impl FnOnce(&mut WebSearchConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_result()?.web_search;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_agent_config_result(&["web_search"], value)
            }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_web_search_config(&self, update: impl FnOnce(&mut WebSearchConfig)) {
                if let Err(err) = self.update_web_search_config_result(update) {
                    tracing::warn!(target: "vw_desktop", error = %err, "failed to update agent config via gateway");
                }
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_web_search_config_result(&self, _update: impl FnOnce(&mut WebSearchConfig)) -> Result<(), String> {
                Ok(())
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_web_search_config(&self, _update: impl FnOnce(&mut WebSearchConfig)) {}
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_tunnel_config_result(&self, update: impl FnOnce(&mut TunnelConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_result()?.tunnel;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_agent_config_result(&["tunnel"], value)
            }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_tunnel_config(&self, update: impl FnOnce(&mut TunnelConfig)) {
                if let Err(err) = self.update_tunnel_config_result(update) {
                    tracing::warn!(target: "vw_desktop", error = %err, "failed to update agent config via gateway");
                }
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_tunnel_config_result(&self, _update: impl FnOnce(&mut TunnelConfig)) -> Result<(), String> {
                Ok(())
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_tunnel_config(&self, _update: impl FnOnce(&mut TunnelConfig)) {}
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_hooks_config_result(&self, update: impl FnOnce(&mut HooksConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_result()?.hooks;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_agent_config_result(&["hooks"], value)
            }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_hooks_config(&self, update: impl FnOnce(&mut HooksConfig)) {
                if let Err(err) = self.update_hooks_config_result(update) {
                    tracing::warn!(target: "vw_desktop", error = %err, "failed to update agent config via gateway");
                }
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_hooks_config_result(&self, _update: impl FnOnce(&mut HooksConfig)) -> Result<(), String> {
                Ok(())
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_hooks_config(&self, _update: impl FnOnce(&mut HooksConfig)) {}
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_skills_config_result(&self, update: impl FnOnce(&mut SkillsConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_result()?.skills;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_agent_config_result(&["skills"], value)
            }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_skills_config(&self, update: impl FnOnce(&mut SkillsConfig)) {
                if let Err(err) = self.update_skills_config_result(update) {
                    tracing::warn!(target: "vw_desktop", error = %err, "failed to update agent config via gateway");
                }
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_skills_config_result(&self, _update: impl FnOnce(&mut SkillsConfig)) -> Result<(), String> {
                Ok(())
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_skills_config(&self, _update: impl FnOnce(&mut SkillsConfig)) {}
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_runtime_config_result(&self, update: impl FnOnce(&mut RuntimeConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_result()?.runtime;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_agent_config_result(&["runtime"], value)
            }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_runtime_config(&self, update: impl FnOnce(&mut RuntimeConfig)) {
                if let Err(err) = self.update_runtime_config_result(update) {
                    tracing::warn!(target: "vw_desktop", error = %err, "failed to update agent config via gateway");
                }
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_runtime_config_result(&self, _update: impl FnOnce(&mut RuntimeConfig)) -> Result<(), String> {
                Ok(())
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_runtime_config(&self, _update: impl FnOnce(&mut RuntimeConfig)) {}
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_model_routes_config_result(&self, update: impl FnOnce(&mut Vec<ModelRouteConfig>)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_result()?.model_routes;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_agent_config_result(&["model_routes"], value)
            }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_model_routes_config(&self, update: impl FnOnce(&mut Vec<ModelRouteConfig>)) {
                if let Err(err) = self.update_model_routes_config_result(update) {
                    tracing::warn!(target: "vw_desktop", error = %err, "failed to update agent config via gateway");
                }
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_model_routes_config_result(&self, _update: impl FnOnce(&mut Vec<ModelRouteConfig>)) -> Result<(), String> {
                Ok(())
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_model_routes_config(&self, _update: impl FnOnce(&mut Vec<ModelRouteConfig>)) {}
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_embedding_routes_config_result(&self, update: impl FnOnce(&mut Vec<EmbeddingRouteConfig>)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_result()?.embedding_routes;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_agent_config_result(&["embedding_routes"], value)
            }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_embedding_routes_config(&self, update: impl FnOnce(&mut Vec<EmbeddingRouteConfig>)) {
                if let Err(err) = self.update_embedding_routes_config_result(update) {
                    tracing::warn!(target: "vw_desktop", error = %err, "failed to update agent config via gateway");
                }
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_embedding_routes_config_result(&self, _update: impl FnOnce(&mut Vec<EmbeddingRouteConfig>)) -> Result<(), String> {
                Ok(())
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_embedding_routes_config(&self, _update: impl FnOnce(&mut Vec<EmbeddingRouteConfig>)) {}
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_query_classification_config_result(&self, update: impl FnOnce(&mut QueryClassificationConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_result()?.query_classification;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_agent_config_result(&["query_classification"], value)
            }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_query_classification_config(&self, update: impl FnOnce(&mut QueryClassificationConfig)) {
                if let Err(err) = self.update_query_classification_config_result(update) {
                    tracing::warn!(target: "vw_desktop", error = %err, "failed to update agent config via gateway");
                }
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_query_classification_config_result(&self, _update: impl FnOnce(&mut QueryClassificationConfig)) -> Result<(), String> {
                Ok(())
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_query_classification_config(&self, _update: impl FnOnce(&mut QueryClassificationConfig)) {}
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_autonomy_config_result(&self, update: impl FnOnce(&mut AutonomyConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_result()?.autonomy;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_agent_config_result(&["autonomy"], value)
            }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_autonomy_config(&self, update: impl FnOnce(&mut AutonomyConfig)) {
                if let Err(err) = self.update_autonomy_config_result(update) {
                    tracing::warn!(target: "vw_desktop", error = %err, "failed to update agent config via gateway");
                }
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_autonomy_config_result(&self, _update: impl FnOnce(&mut AutonomyConfig)) -> Result<(), String> {
                Ok(())
            }
            #[cfg(target_arch = "wasm32")]
            pub fn update_autonomy_config(&self, _update: impl FnOnce(&mut AutonomyConfig)) {}
pub async fn update_heartbeat_config_result_async(&self, update: impl FnOnce(&mut HeartbeatConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_async().await?.heartbeat;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_full_agent_config_async(serde_json::json!({ ["heartbeat"][0]: value })).await
            }
pub async fn update_cron_config_result_async(&self, update: impl FnOnce(&mut CronConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_async().await?.cron;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_full_agent_config_async(serde_json::json!({ ["cron"][0]: value })).await
            }
pub async fn update_scheduler_config_result_async(&self, update: impl FnOnce(&mut SchedulerConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_async().await?.scheduler;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_full_agent_config_async(serde_json::json!({ ["scheduler"][0]: value })).await
            }
pub async fn update_reliability_config_result_async(&self, update: impl FnOnce(&mut ReliabilityConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_async().await?.reliability;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_full_agent_config_async(serde_json::json!({ ["reliability"][0]: value })).await
            }
pub async fn update_memory_config_result_async(&self, update: impl FnOnce(&mut MemoryConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_async().await?.memory;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_full_agent_config_async(serde_json::json!({ ["memory"][0]: value })).await
            }
pub async fn update_security_config_result_async(&self, update: impl FnOnce(&mut SecurityConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_async().await?.security;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_full_agent_config_async(serde_json::json!({ ["security"][0]: value })).await
            }
pub async fn update_channels_config_result_async(&self, update: impl FnOnce(&mut ChannelsConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_async().await?.channels_config;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_full_agent_config_async(serde_json::json!({ ["channels_config"][0]: value })).await
            }
pub async fn update_observability_config_result_async(&self, update: impl FnOnce(&mut ObservabilityConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_async().await?.observability;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_full_agent_config_async(serde_json::json!({ ["observability"][0]: value })).await
            }
pub async fn update_storage_config_result_async(&self, update: impl FnOnce(&mut StorageConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_async().await?.storage;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_full_agent_config_async(serde_json::json!({ ["storage"][0]: value })).await
            }
pub async fn update_proxy_config_result_async(&self, update: impl FnOnce(&mut ProxyConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_async().await?.proxy;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_full_agent_config_async(serde_json::json!({ ["proxy"][0]: value })).await
            }
pub async fn update_browser_config_result_async(&self, update: impl FnOnce(&mut BrowserConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_async().await?.browser;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_full_agent_config_async(serde_json::json!({ ["browser"][0]: value })).await
            }
pub async fn update_http_request_config_result_async(&self, update: impl FnOnce(&mut HttpRequestConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_async().await?.http_request;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_full_agent_config_async(serde_json::json!({ ["http_request"][0]: value })).await
            }
pub async fn update_multimodal_config_result_async(&self, update: impl FnOnce(&mut MultimodalConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_async().await?.multimodal;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_full_agent_config_async(serde_json::json!({ ["multimodal"][0]: value })).await
            }
pub async fn update_web_search_config_result_async(&self, update: impl FnOnce(&mut WebSearchConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_async().await?.web_search;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_full_agent_config_async(serde_json::json!({ ["web_search"][0]: value })).await
            }
pub async fn update_tunnel_config_result_async(&self, update: impl FnOnce(&mut TunnelConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_async().await?.tunnel;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_full_agent_config_async(serde_json::json!({ ["tunnel"][0]: value })).await
            }
pub async fn update_hooks_config_result_async(&self, update: impl FnOnce(&mut HooksConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_async().await?.hooks;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_full_agent_config_async(serde_json::json!({ ["hooks"][0]: value })).await
            }
pub async fn update_skills_config_result_async(&self, update: impl FnOnce(&mut SkillsConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_async().await?.skills;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_full_agent_config_async(serde_json::json!({ ["skills"][0]: value })).await
            }
pub async fn update_runtime_config_result_async(&self, update: impl FnOnce(&mut RuntimeConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_async().await?.runtime;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_full_agent_config_async(serde_json::json!({ ["runtime"][0]: value })).await
            }
pub async fn update_model_routes_config_result_async(&self, update: impl FnOnce(&mut Vec<ModelRouteConfig>)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_async().await?.model_routes;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_full_agent_config_async(serde_json::json!({ ["model_routes"][0]: value })).await
            }
pub async fn update_embedding_routes_config_result_async(&self, update: impl FnOnce(&mut Vec<EmbeddingRouteConfig>)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_async().await?.embedding_routes;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_full_agent_config_async(serde_json::json!({ ["embedding_routes"][0]: value })).await
            }
pub async fn update_query_classification_config_result_async(&self, update: impl FnOnce(&mut QueryClassificationConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_async().await?.query_classification;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_full_agent_config_async(serde_json::json!({ ["query_classification"][0]: value })).await
            }
pub async fn update_autonomy_config_result_async(&self, update: impl FnOnce(&mut AutonomyConfig)) -> Result<(), String> {
                let mut cfg = self.load_full_agent_config_async().await?.autonomy;
                update(&mut cfg);
                let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
                self.patch_full_agent_config_async(serde_json::json!({ ["autonomy"][0]: value })).await
            }
pub fn update_heartbeat_config_async(&self, update: impl FnOnce(&mut HeartbeatConfig) + Send + 'static) -> Task<ConfigSaved> {
let access = self.clone();

                spawn_gateway_task(["heartbeat"][0], async move { access.update_heartbeat_config_result_async(update).await })
            }
pub fn update_cron_config_async(&self, update: impl FnOnce(&mut CronConfig) + Send + 'static) -> Task<ConfigSaved> {
let access = self.clone();

                spawn_gateway_task(["cron"][0], async move { access.update_cron_config_result_async(update).await })
            }
pub fn update_scheduler_config_async(&self, update: impl FnOnce(&mut SchedulerConfig) + Send + 'static) -> Task<ConfigSaved> {
let access = self.clone();

                spawn_gateway_task(["scheduler"][0], async move { access.update_scheduler_config_result_async(update).await })
            }
pub fn update_reliability_config_async(&self, update: impl FnOnce(&mut ReliabilityConfig) + Send + 'static) -> Task<ConfigSaved> {
let access = self.clone();

                spawn_gateway_task(["reliability"][0], async move { access.update_reliability_config_result_async(update).await })
            }
pub fn update_memory_config_async(&self, update: impl FnOnce(&mut MemoryConfig) + Send + 'static) -> Task<ConfigSaved> {
let access = self.clone();

                spawn_gateway_task(["memory"][0], async move { access.update_memory_config_result_async(update).await })
            }
pub fn update_security_config_async(&self, update: impl FnOnce(&mut SecurityConfig) + Send + 'static) -> Task<ConfigSaved> {
let access = self.clone();

                spawn_gateway_task(["security"][0], async move { access.update_security_config_result_async(update).await })
            }
pub fn update_channels_config_async(&self, update: impl FnOnce(&mut ChannelsConfig) + Send + 'static) -> Task<ConfigSaved> {
let access = self.clone();

                spawn_gateway_task(["channels_config"][0], async move { access.update_channels_config_result_async(update).await })
            }
pub fn update_observability_config_async(&self, update: impl FnOnce(&mut ObservabilityConfig) + Send + 'static) -> Task<ConfigSaved> {
let access = self.clone();

                spawn_gateway_task(["observability"][0], async move { access.update_observability_config_result_async(update).await })
            }
pub fn update_storage_config_async(&self, update: impl FnOnce(&mut StorageConfig) + Send + 'static) -> Task<ConfigSaved> {
let access = self.clone();

                spawn_gateway_task(["storage"][0], async move { access.update_storage_config_result_async(update).await })
            }
pub fn update_proxy_config_async(&self, update: impl FnOnce(&mut ProxyConfig) + Send + 'static) -> Task<ConfigSaved> {
let access = self.clone();

                spawn_gateway_task(["proxy"][0], async move { access.update_proxy_config_result_async(update).await })
            }
pub fn update_browser_config_async(&self, update: impl FnOnce(&mut BrowserConfig) + Send + 'static) -> Task<ConfigSaved> {
let access = self.clone();

                spawn_gateway_task(["browser"][0], async move { access.update_browser_config_result_async(update).await })
            }
pub fn update_http_request_config_async(&self, update: impl FnOnce(&mut HttpRequestConfig) + Send + 'static) -> Task<ConfigSaved> {
let access = self.clone();

                spawn_gateway_task(["http_request"][0], async move { access.update_http_request_config_result_async(update).await })
            }
pub fn update_multimodal_config_async(&self, update: impl FnOnce(&mut MultimodalConfig) + Send + 'static) -> Task<ConfigSaved> {
let access = self.clone();

                spawn_gateway_task(["multimodal"][0], async move { access.update_multimodal_config_result_async(update).await })
            }
pub fn update_web_search_config_async(&self, update: impl FnOnce(&mut WebSearchConfig) + Send + 'static) -> Task<ConfigSaved> {
let access = self.clone();

                spawn_gateway_task(["web_search"][0], async move { access.update_web_search_config_result_async(update).await })
            }
pub fn update_tunnel_config_async(&self, update: impl FnOnce(&mut TunnelConfig) + Send + 'static) -> Task<ConfigSaved> {
let access = self.clone();

                spawn_gateway_task(["tunnel"][0], async move { access.update_tunnel_config_result_async(update).await })
            }
pub fn update_hooks_config_async(&self, update: impl FnOnce(&mut HooksConfig) + Send + 'static) -> Task<ConfigSaved> {
let access = self.clone();

                spawn_gateway_task(["hooks"][0], async move { access.update_hooks_config_result_async(update).await })
            }
pub fn update_skills_config_async(&self, update: impl FnOnce(&mut SkillsConfig) + Send + 'static) -> Task<ConfigSaved> {
let access = self.clone();

                spawn_gateway_task(["skills"][0], async move { access.update_skills_config_result_async(update).await })
            }
pub fn update_runtime_config_async(&self, update: impl FnOnce(&mut RuntimeConfig) + Send + 'static) -> Task<ConfigSaved> {
let access = self.clone();

                spawn_gateway_task(["runtime"][0], async move { access.update_runtime_config_result_async(update).await })
            }
pub fn update_model_routes_config_async(&self, update: impl FnOnce(&mut Vec<ModelRouteConfig>) + Send + 'static) -> Task<ConfigSaved> {
let access = self.clone();

                spawn_gateway_task(["model_routes"][0], async move { access.update_model_routes_config_result_async(update).await })
            }
pub fn update_embedding_routes_config_async(&self, update: impl FnOnce(&mut Vec<EmbeddingRouteConfig>) + Send + 'static) -> Task<ConfigSaved> {
let access = self.clone();

                spawn_gateway_task(["embedding_routes"][0], async move { access.update_embedding_routes_config_result_async(update).await })
            }
pub fn update_query_classification_config_async(&self, update: impl FnOnce(&mut QueryClassificationConfig) + Send + 'static) -> Task<ConfigSaved> {
let access = self.clone();

                spawn_gateway_task(["query_classification"][0], async move { access.update_query_classification_config_result_async(update).await })
            }
pub fn update_autonomy_config_async(&self, update: impl FnOnce(&mut AutonomyConfig) + Send + 'static) -> Task<ConfigSaved> {
let access = self.clone();

                spawn_gateway_task(["autonomy"][0], async move { access.update_autonomy_config_result_async(update).await })
            }
pub fn update_gateway_config_async(&self,
    update: impl FnOnce(&mut GatewayConfig) + Send + 'static,
) -> Task<ConfigSaved> {
let access = self.clone();

    spawn_gateway_task("gateway", async move { access.update_gateway_config_result_async(update).await })
}
#[cfg(not(target_arch = "wasm32"))]
pub fn update_delegate_agents_config(&self,
    update: impl FnOnce(&mut HashMap<String, DelegateAgentConfig>),
) {
    let mut cfg = self.load_agent_config_via_gateway().agents;
    update(&mut cfg);
    if let Ok(value) = serde_json::to_value(cfg) {
        self.patch_agent_config(&["agents"], value);
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub fn update_delegate_agents_config_result(&self,
    update: impl FnOnce(&mut HashMap<String, DelegateAgentConfig>),
) -> Result<(), String> {
    let mut cfg = self.load_full_agent_config_result()?.agents;
    update(&mut cfg);
    let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
    self.patch_agent_config_result(&["agents"], value)
}
pub async fn update_delegate_agents_config_result_async(&self,
    update: impl FnOnce(&mut HashMap<String, DelegateAgentConfig>),
) -> Result<(), String> {
    let mut cfg = self.load_full_agent_config_async().await?.agents;
    update(&mut cfg);
    let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
    self.patch_full_agent_config_async(serde_json::json!({ "agents": value })).await
}
#[cfg(target_arch = "wasm32")]
pub fn update_delegate_agents_config_async(&self,
    update: impl FnOnce(&mut HashMap<String, DelegateAgentConfig>) + 'static,
) -> Task<ConfigSaved> {
let access = self.clone();

    spawn_gateway_task("agents", async move {
        let mut cfg = access.load_full_agent_config_async().await?.agents;
        update(&mut cfg);
        let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
        access.patch_full_agent_config_async(serde_json::json!({ "agents": value })).await
    })
}
#[cfg(not(target_arch = "wasm32"))]
pub fn update_delegate_agents_config_async(&self,
    update: impl FnOnce(&mut HashMap<String, DelegateAgentConfig>) + Send + 'static,
) -> Task<ConfigSaved> {
let access = self.clone();

    spawn_gateway_task("agents", async move {
        let mut cfg = access.load_full_agent_config_result()?.agents;
        update(&mut cfg);
        let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
        access.patch_agent_config_result(&["agents"], value)
    })
}
#[cfg(not(target_arch = "wasm32"))]
pub fn update_main_agent_overrides_from_delegate_agents(&self) {
    let cfg = self.load_agent_config_via_gateway();
    let Some(main) = cfg.agents.get("main") else {
        return;
    };

    let provider = main.provider.trim().to_string();
    if !provider.is_empty() {
        self.patch_agent_config(&["default_provider"], serde_json::Value::String(provider.clone()));
    }

    let model = main.model.trim().to_string();
    if !provider.is_empty() && !model.is_empty() {
        self.patch_agent_config(
            &["default_model"],
            serde_json::Value::String(format!("{provider}/{model}")),
        );
    }

    if let Some(temperature) = main.temperature
        && let Some(number) = serde_json::Number::from_f64(temperature)
    {
        self.patch_agent_config(&["default_temperature"], serde_json::Value::Number(number));
    }

    let identity =
        IdentityConfig { format: "openclaw".to_string(), aieos_path: None, aieos_inline: None };
    if let Ok(value) = serde_json::to_value(identity) {
        self.patch_agent_config(&["identity"], value);
    }
}
pub async fn update_main_agent_overrides_from_delegate_agents_async(&self) -> Result<(), String> {
    let cfg = self.load_full_agent_config_async().await?;
    let Some(patch) = build_main_agent_overrides_patch(&cfg)? else {
        return Ok(());
    };

    self.patch_full_agent_config_async(patch).await
}
pub async fn update_gateway_config_result_async(&self,
    update: impl FnOnce(&mut GatewayConfig),
) -> Result<(), String> {
    let mut cfg = self.load_full_agent_config_async().await?.gateway;
    update(&mut cfg);
    let value = serde_json::to_value(cfg).map_err(|err| err.to_string())?;
    self.patch_full_agent_config_async(serde_json::json!({ "gateway": value })).await
}
}
#[cfg(test)]
#[path = "config_agent_tests.rs"]
mod tests;
