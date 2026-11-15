//! Desktop message and configuration-source adapters.
use crate::app::Message;
use config_observability::ObservabilityConfig;
use config_hooks::HooksConfig;
use config_automation::{CronConfig, HeartbeatConfig, SchedulerConfig};
use config_reliability::ReliabilityConfig;
use config_runtime::RuntimeConfig;
use iced::Task;
use std::collections::HashMap;
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

pub use desktop_config::{AcpSettingsSnapshot, resolve_enabled_acp_config};

fn access() -> desktop_config::ConfigAccess {
 desktop_config::ConfigAccess::new(super::gateway::gateway_client_endpoint(), local_config_path())
}
#[cfg(not(target_arch = "wasm32"))]
fn local_config_path() -> Option<std::path::PathBuf> { super::gateway::project_config_override_path() }
#[cfg(target_arch = "wasm32")]
fn local_config_path() -> Option<std::path::PathBuf> { None }
pub async fn save_global_acp_agent_async(
    agent_name: String,
    spec: AcpAgentConfig,
) -> Result<AcpSettingsSnapshot, String> { access().save_global_acp_agent_async(agent_name, spec).await }
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn load_local_agent_config() -> Option<Config> { access().load_local_agent_config() }
pub async fn load_full_agent_config_async() -> Result<Config, String> { access().load_full_agent_config_async().await }
pub async fn load_project_agent_config_async(project_path: &str) -> Result<Config, String> { access().load_project_agent_config_async(project_path).await }
pub async fn load_global_config_value_async() -> Result<serde_json::Value, String> { access().load_global_config_value_async().await }
pub async fn load_mcp_inspect_async()
-> Result<vw_api_mcp_inspect::McpInspectResponse, String> { access().load_mcp_inspect_async().await }
pub async fn load_browser_config_async() -> Result<BrowserConfig, String> { access().load_browser_config_async().await }
pub fn load_gateway_config_result() -> Result<GatewayConfig, String> { access().load_gateway_config_result() }
pub fn load_global_acp_config_result() -> Result<HashMap<String, AcpAgentConfig>, String> { access().load_global_acp_config_result() }
pub async fn load_global_acp_config_async() -> Result<HashMap<String, AcpAgentConfig>, String> { access().load_global_acp_config_async().await }
pub fn load_enabled_acp_config_result() -> Result<HashMap<String, AcpAgentConfig>, String> { access().load_enabled_acp_config_result() }
pub async fn load_enabled_acp_config_async() -> Result<HashMap<String, AcpAgentConfig>, String> { access().load_enabled_acp_config_async().await }
pub async fn load_acp_settings_snapshot_async() -> Result<AcpSettingsSnapshot, String> { access().load_acp_settings_snapshot_async().await }
pub async fn set_global_acp_agent_enabled_async(
    agent_name: String,
    enabled: bool,
    spec: Option<AcpAgentConfig>,
) -> Result<AcpSettingsSnapshot, String> { access().set_global_acp_agent_enabled_async(agent_name, enabled, spec).await }
pub async fn add_global_acp_agent_async(
    agent_name: String,
    spec: AcpAgentConfig,
) -> Result<AcpSettingsSnapshot, String> { access().add_global_acp_agent_async(agent_name, spec).await }
pub async fn patch_full_agent_config_async(patch: serde_json::Value) -> Result<(), String> { access().patch_full_agent_config_async(patch).await }
pub async fn patch_project_agent_config_async(
    project_path: &str,
    patch: serde_json::Value,
) -> Result<(), String> { access().patch_project_agent_config_async(project_path, patch).await }
pub fn patch_agent_config_result(path: &[&str], value: serde_json::Value) -> Result<(), String> { access().patch_agent_config_result(path, value) }
pub fn update_gateway_config_result(update: impl FnOnce(&mut GatewayConfig)) -> Result<(), String> { access().update_gateway_config_result(update) }
pub async fn remove_global_provider_via_gateway(provider_id: &str) -> Result<(), String> { access().remove_global_provider_via_gateway(provider_id).await }
pub fn load_gateway_config() -> GatewayConfig { access().load_gateway_config() }
pub fn load_heartbeat_config() -> HeartbeatConfig { access().load_heartbeat_config() }
pub fn load_cron_config() -> CronConfig { access().load_cron_config() }
pub fn load_scheduler_config() -> SchedulerConfig { access().load_scheduler_config() }
pub fn load_reliability_config() -> ReliabilityConfig { access().load_reliability_config() }
pub fn load_memory_config() -> MemoryConfig { access().load_memory_config() }
pub fn load_security_config() -> SecurityConfig { access().load_security_config() }
pub fn load_channels_config() -> ChannelsConfig { access().load_channels_config() }
pub fn load_observability_config() -> ObservabilityConfig { access().load_observability_config() }
pub fn load_storage_config() -> StorageConfig { access().load_storage_config() }
pub fn load_proxy_config() -> ProxyConfig { access().load_proxy_config() }
pub fn load_browser_config() -> BrowserConfig { access().load_browser_config() }
pub fn load_http_request_config() -> HttpRequestConfig { access().load_http_request_config() }
pub fn load_multimodal_config() -> MultimodalConfig { access().load_multimodal_config() }
pub fn load_web_search_config() -> WebSearchConfig { access().load_web_search_config() }
#[cfg(not(target_arch = "wasm32"))]
pub fn load_tunnel_config() -> TunnelConfig { access().load_tunnel_config() }
#[cfg(not(target_arch = "wasm32"))]
pub fn load_hooks_config() -> HooksConfig { access().load_hooks_config() }
#[cfg(not(target_arch = "wasm32"))]
pub fn load_skills_config() -> SkillsConfig { access().load_skills_config() }
#[cfg(not(target_arch = "wasm32"))]
pub fn load_delegate_agents_config() -> HashMap<String, DelegateAgentConfig> { access().load_delegate_agents_config() }
#[cfg(not(target_arch = "wasm32"))]
pub fn load_embedding_routes_config() -> Vec<EmbeddingRouteConfig> { access().load_embedding_routes_config() }
#[cfg(not(target_arch = "wasm32"))]
pub fn load_autonomy_config() -> AutonomyConfig { access().load_autonomy_config() }
#[cfg(not(target_arch = "wasm32"))]
pub fn load_identity_config() -> IdentityConfig { access().load_identity_config() }
#[cfg(not(target_arch = "wasm32"))]
pub fn load_full_agent_config() -> Config { access().load_full_agent_config() }
#[cfg(not(target_arch = "wasm32"))]
pub fn load_full_agent_config_result() -> Result<Config, String> { access().load_full_agent_config_result() }
#[cfg(not(target_arch = "wasm32"))]
pub fn update_gateway_config(update: impl FnOnce(&mut GatewayConfig)) { access().update_gateway_config(update) }
#[cfg(not(target_arch = "wasm32"))]
pub fn load_runtime_config() -> RuntimeConfig { access().load_runtime_config() }
#[cfg(not(target_arch = "wasm32"))]
pub fn load_model_routes_config() -> Vec<ModelRouteConfig> { access().load_model_routes_config() }
#[cfg(not(target_arch = "wasm32"))]
pub fn load_query_classification_config() -> QueryClassificationConfig { access().load_query_classification_config() }
#[cfg(target_arch = "wasm32")]
pub fn load_tunnel_config() -> TunnelConfig { access().load_tunnel_config() }
#[cfg(target_arch = "wasm32")]
pub fn load_hooks_config() -> HooksConfig { access().load_hooks_config() }
#[cfg(target_arch = "wasm32")]
pub fn load_skills_config() -> SkillsConfig { access().load_skills_config() }
#[cfg(target_arch = "wasm32")]
pub fn load_delegate_agents_config() -> HashMap<String, DelegateAgentConfig> { access().load_delegate_agents_config() }
#[cfg(target_arch = "wasm32")]
pub fn load_embedding_routes_config() -> Vec<EmbeddingRouteConfig> { access().load_embedding_routes_config() }
#[cfg(target_arch = "wasm32")]
pub fn load_autonomy_config() -> AutonomyConfig { access().load_autonomy_config() }
#[cfg(target_arch = "wasm32")]
pub fn load_identity_config() -> IdentityConfig { access().load_identity_config() }
#[cfg(target_arch = "wasm32")]
pub fn load_full_agent_config() -> Config { access().load_full_agent_config() }
#[cfg(target_arch = "wasm32")]
pub fn load_full_agent_config_result() -> Result<Config, String> { access().load_full_agent_config_result() }
#[cfg(target_arch = "wasm32")]
pub fn update_gateway_config(_update: impl FnOnce(&mut GatewayConfig)) { access().update_gateway_config(_update) }
#[cfg(target_arch = "wasm32")]
pub fn load_runtime_config() -> RuntimeConfig { access().load_runtime_config() }
#[cfg(target_arch = "wasm32")]
pub fn load_model_routes_config() -> Vec<ModelRouteConfig> { access().load_model_routes_config() }
#[cfg(target_arch = "wasm32")]
pub fn load_query_classification_config() -> QueryClassificationConfig { access().load_query_classification_config() }
#[cfg(target_arch = "wasm32")]
pub fn update_delegate_agents_config(
    _update: impl FnOnce(&mut HashMap<String, DelegateAgentConfig>),
) { access().update_delegate_agents_config(_update) }
#[cfg(target_arch = "wasm32")]
pub fn update_delegate_agents_config_result(
    _update: impl FnOnce(&mut HashMap<String, DelegateAgentConfig>),
) -> Result<(), String> { access().update_delegate_agents_config_result(_update) }
#[cfg(target_arch = "wasm32")]
pub fn update_main_agent_overrides_from_delegate_agents() { access().update_main_agent_overrides_from_delegate_agents() }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_heartbeat_config_result(update: impl FnOnce(&mut HeartbeatConfig)) -> Result<(), String> { access().update_heartbeat_config_result(update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_heartbeat_config(update: impl FnOnce(&mut HeartbeatConfig)) { access().update_heartbeat_config(update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_heartbeat_config_result(_update: impl FnOnce(&mut HeartbeatConfig)) -> Result<(), String> { access().update_heartbeat_config_result(_update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_heartbeat_config(_update: impl FnOnce(&mut HeartbeatConfig)) { access().update_heartbeat_config(_update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_cron_config_result(update: impl FnOnce(&mut CronConfig)) -> Result<(), String> { access().update_cron_config_result(update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_cron_config(update: impl FnOnce(&mut CronConfig)) { access().update_cron_config(update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_cron_config_result(_update: impl FnOnce(&mut CronConfig)) -> Result<(), String> { access().update_cron_config_result(_update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_cron_config(_update: impl FnOnce(&mut CronConfig)) { access().update_cron_config(_update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_scheduler_config_result(update: impl FnOnce(&mut SchedulerConfig)) -> Result<(), String> { access().update_scheduler_config_result(update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_scheduler_config(update: impl FnOnce(&mut SchedulerConfig)) { access().update_scheduler_config(update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_scheduler_config_result(_update: impl FnOnce(&mut SchedulerConfig)) -> Result<(), String> { access().update_scheduler_config_result(_update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_scheduler_config(_update: impl FnOnce(&mut SchedulerConfig)) { access().update_scheduler_config(_update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_reliability_config_result(update: impl FnOnce(&mut ReliabilityConfig)) -> Result<(), String> { access().update_reliability_config_result(update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_reliability_config(update: impl FnOnce(&mut ReliabilityConfig)) { access().update_reliability_config(update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_reliability_config_result(_update: impl FnOnce(&mut ReliabilityConfig)) -> Result<(), String> { access().update_reliability_config_result(_update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_reliability_config(_update: impl FnOnce(&mut ReliabilityConfig)) { access().update_reliability_config(_update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_memory_config_result(update: impl FnOnce(&mut MemoryConfig)) -> Result<(), String> { access().update_memory_config_result(update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_memory_config(update: impl FnOnce(&mut MemoryConfig)) { access().update_memory_config(update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_memory_config_result(_update: impl FnOnce(&mut MemoryConfig)) -> Result<(), String> { access().update_memory_config_result(_update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_memory_config(_update: impl FnOnce(&mut MemoryConfig)) { access().update_memory_config(_update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_security_config_result(update: impl FnOnce(&mut SecurityConfig)) -> Result<(), String> { access().update_security_config_result(update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_security_config(update: impl FnOnce(&mut SecurityConfig)) { access().update_security_config(update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_security_config_result(_update: impl FnOnce(&mut SecurityConfig)) -> Result<(), String> { access().update_security_config_result(_update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_security_config(_update: impl FnOnce(&mut SecurityConfig)) { access().update_security_config(_update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_channels_config_result(update: impl FnOnce(&mut ChannelsConfig)) -> Result<(), String> { access().update_channels_config_result(update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_channels_config(update: impl FnOnce(&mut ChannelsConfig)) { access().update_channels_config(update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_channels_config_result(_update: impl FnOnce(&mut ChannelsConfig)) -> Result<(), String> { access().update_channels_config_result(_update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_channels_config(_update: impl FnOnce(&mut ChannelsConfig)) { access().update_channels_config(_update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_observability_config_result(update: impl FnOnce(&mut ObservabilityConfig)) -> Result<(), String> { access().update_observability_config_result(update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_observability_config(update: impl FnOnce(&mut ObservabilityConfig)) { access().update_observability_config(update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_observability_config_result(_update: impl FnOnce(&mut ObservabilityConfig)) -> Result<(), String> { access().update_observability_config_result(_update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_observability_config(_update: impl FnOnce(&mut ObservabilityConfig)) { access().update_observability_config(_update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_storage_config_result(update: impl FnOnce(&mut StorageConfig)) -> Result<(), String> { access().update_storage_config_result(update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_storage_config(update: impl FnOnce(&mut StorageConfig)) { access().update_storage_config(update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_storage_config_result(_update: impl FnOnce(&mut StorageConfig)) -> Result<(), String> { access().update_storage_config_result(_update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_storage_config(_update: impl FnOnce(&mut StorageConfig)) { access().update_storage_config(_update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_proxy_config_result(update: impl FnOnce(&mut ProxyConfig)) -> Result<(), String> { access().update_proxy_config_result(update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_proxy_config(update: impl FnOnce(&mut ProxyConfig)) { access().update_proxy_config(update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_proxy_config_result(_update: impl FnOnce(&mut ProxyConfig)) -> Result<(), String> { access().update_proxy_config_result(_update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_proxy_config(_update: impl FnOnce(&mut ProxyConfig)) { access().update_proxy_config(_update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_browser_config_result(update: impl FnOnce(&mut BrowserConfig)) -> Result<(), String> { access().update_browser_config_result(update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_browser_config(update: impl FnOnce(&mut BrowserConfig)) { access().update_browser_config(update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_browser_config_result(_update: impl FnOnce(&mut BrowserConfig)) -> Result<(), String> { access().update_browser_config_result(_update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_browser_config(_update: impl FnOnce(&mut BrowserConfig)) { access().update_browser_config(_update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_http_request_config_result(update: impl FnOnce(&mut HttpRequestConfig)) -> Result<(), String> { access().update_http_request_config_result(update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_http_request_config(update: impl FnOnce(&mut HttpRequestConfig)) { access().update_http_request_config(update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_http_request_config_result(_update: impl FnOnce(&mut HttpRequestConfig)) -> Result<(), String> { access().update_http_request_config_result(_update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_http_request_config(_update: impl FnOnce(&mut HttpRequestConfig)) { access().update_http_request_config(_update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_multimodal_config_result(update: impl FnOnce(&mut MultimodalConfig)) -> Result<(), String> { access().update_multimodal_config_result(update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_multimodal_config(update: impl FnOnce(&mut MultimodalConfig)) { access().update_multimodal_config(update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_multimodal_config_result(_update: impl FnOnce(&mut MultimodalConfig)) -> Result<(), String> { access().update_multimodal_config_result(_update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_multimodal_config(_update: impl FnOnce(&mut MultimodalConfig)) { access().update_multimodal_config(_update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_web_search_config_result(update: impl FnOnce(&mut WebSearchConfig)) -> Result<(), String> { access().update_web_search_config_result(update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_web_search_config(update: impl FnOnce(&mut WebSearchConfig)) { access().update_web_search_config(update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_web_search_config_result(_update: impl FnOnce(&mut WebSearchConfig)) -> Result<(), String> { access().update_web_search_config_result(_update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_web_search_config(_update: impl FnOnce(&mut WebSearchConfig)) { access().update_web_search_config(_update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_tunnel_config_result(update: impl FnOnce(&mut TunnelConfig)) -> Result<(), String> { access().update_tunnel_config_result(update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_tunnel_config(update: impl FnOnce(&mut TunnelConfig)) { access().update_tunnel_config(update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_tunnel_config_result(_update: impl FnOnce(&mut TunnelConfig)) -> Result<(), String> { access().update_tunnel_config_result(_update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_tunnel_config(_update: impl FnOnce(&mut TunnelConfig)) { access().update_tunnel_config(_update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_hooks_config_result(update: impl FnOnce(&mut HooksConfig)) -> Result<(), String> { access().update_hooks_config_result(update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_hooks_config(update: impl FnOnce(&mut HooksConfig)) { access().update_hooks_config(update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_hooks_config_result(_update: impl FnOnce(&mut HooksConfig)) -> Result<(), String> { access().update_hooks_config_result(_update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_hooks_config(_update: impl FnOnce(&mut HooksConfig)) { access().update_hooks_config(_update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_skills_config_result(update: impl FnOnce(&mut SkillsConfig)) -> Result<(), String> { access().update_skills_config_result(update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_skills_config(update: impl FnOnce(&mut SkillsConfig)) { access().update_skills_config(update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_skills_config_result(_update: impl FnOnce(&mut SkillsConfig)) -> Result<(), String> { access().update_skills_config_result(_update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_skills_config(_update: impl FnOnce(&mut SkillsConfig)) { access().update_skills_config(_update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_runtime_config_result(update: impl FnOnce(&mut RuntimeConfig)) -> Result<(), String> { access().update_runtime_config_result(update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_runtime_config(update: impl FnOnce(&mut RuntimeConfig)) { access().update_runtime_config(update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_runtime_config_result(_update: impl FnOnce(&mut RuntimeConfig)) -> Result<(), String> { access().update_runtime_config_result(_update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_runtime_config(_update: impl FnOnce(&mut RuntimeConfig)) { access().update_runtime_config(_update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_model_routes_config_result(update: impl FnOnce(&mut Vec<ModelRouteConfig>)) -> Result<(), String> { access().update_model_routes_config_result(update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_model_routes_config(update: impl FnOnce(&mut Vec<ModelRouteConfig>)) { access().update_model_routes_config(update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_model_routes_config_result(_update: impl FnOnce(&mut Vec<ModelRouteConfig>)) -> Result<(), String> { access().update_model_routes_config_result(_update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_model_routes_config(_update: impl FnOnce(&mut Vec<ModelRouteConfig>)) { access().update_model_routes_config(_update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_embedding_routes_config_result(update: impl FnOnce(&mut Vec<EmbeddingRouteConfig>)) -> Result<(), String> { access().update_embedding_routes_config_result(update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_embedding_routes_config(update: impl FnOnce(&mut Vec<EmbeddingRouteConfig>)) { access().update_embedding_routes_config(update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_embedding_routes_config_result(_update: impl FnOnce(&mut Vec<EmbeddingRouteConfig>)) -> Result<(), String> { access().update_embedding_routes_config_result(_update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_embedding_routes_config(_update: impl FnOnce(&mut Vec<EmbeddingRouteConfig>)) { access().update_embedding_routes_config(_update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_query_classification_config_result(update: impl FnOnce(&mut QueryClassificationConfig)) -> Result<(), String> { access().update_query_classification_config_result(update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_query_classification_config(update: impl FnOnce(&mut QueryClassificationConfig)) { access().update_query_classification_config(update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_query_classification_config_result(_update: impl FnOnce(&mut QueryClassificationConfig)) -> Result<(), String> { access().update_query_classification_config_result(_update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_query_classification_config(_update: impl FnOnce(&mut QueryClassificationConfig)) { access().update_query_classification_config(_update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_autonomy_config_result(update: impl FnOnce(&mut AutonomyConfig)) -> Result<(), String> { access().update_autonomy_config_result(update) }
            #[cfg(not(target_arch = "wasm32"))]
            pub fn update_autonomy_config(update: impl FnOnce(&mut AutonomyConfig)) { access().update_autonomy_config(update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_autonomy_config_result(_update: impl FnOnce(&mut AutonomyConfig)) -> Result<(), String> { access().update_autonomy_config_result(_update) }
            #[cfg(target_arch = "wasm32")]
            pub fn update_autonomy_config(_update: impl FnOnce(&mut AutonomyConfig)) { access().update_autonomy_config(_update) }
pub async fn update_heartbeat_config_result_async(update: impl FnOnce(&mut HeartbeatConfig)) -> Result<(), String> { access().update_heartbeat_config_result_async(update).await }
pub async fn update_cron_config_result_async(update: impl FnOnce(&mut CronConfig)) -> Result<(), String> { access().update_cron_config_result_async(update).await }
pub async fn update_scheduler_config_result_async(update: impl FnOnce(&mut SchedulerConfig)) -> Result<(), String> { access().update_scheduler_config_result_async(update).await }
pub async fn update_reliability_config_result_async(update: impl FnOnce(&mut ReliabilityConfig)) -> Result<(), String> { access().update_reliability_config_result_async(update).await }
pub async fn update_memory_config_result_async(update: impl FnOnce(&mut MemoryConfig)) -> Result<(), String> { access().update_memory_config_result_async(update).await }
pub async fn update_security_config_result_async(update: impl FnOnce(&mut SecurityConfig)) -> Result<(), String> { access().update_security_config_result_async(update).await }
pub async fn update_channels_config_result_async(update: impl FnOnce(&mut ChannelsConfig)) -> Result<(), String> { access().update_channels_config_result_async(update).await }
pub async fn update_observability_config_result_async(update: impl FnOnce(&mut ObservabilityConfig)) -> Result<(), String> { access().update_observability_config_result_async(update).await }
pub async fn update_storage_config_result_async(update: impl FnOnce(&mut StorageConfig)) -> Result<(), String> { access().update_storage_config_result_async(update).await }
pub async fn update_proxy_config_result_async(update: impl FnOnce(&mut ProxyConfig)) -> Result<(), String> { access().update_proxy_config_result_async(update).await }
pub async fn update_browser_config_result_async(update: impl FnOnce(&mut BrowserConfig)) -> Result<(), String> { access().update_browser_config_result_async(update).await }
pub async fn update_http_request_config_result_async(update: impl FnOnce(&mut HttpRequestConfig)) -> Result<(), String> { access().update_http_request_config_result_async(update).await }
pub async fn update_multimodal_config_result_async(update: impl FnOnce(&mut MultimodalConfig)) -> Result<(), String> { access().update_multimodal_config_result_async(update).await }
pub async fn update_web_search_config_result_async(update: impl FnOnce(&mut WebSearchConfig)) -> Result<(), String> { access().update_web_search_config_result_async(update).await }
pub async fn update_tunnel_config_result_async(update: impl FnOnce(&mut TunnelConfig)) -> Result<(), String> { access().update_tunnel_config_result_async(update).await }
pub async fn update_hooks_config_result_async(update: impl FnOnce(&mut HooksConfig)) -> Result<(), String> { access().update_hooks_config_result_async(update).await }
pub async fn update_skills_config_result_async(update: impl FnOnce(&mut SkillsConfig)) -> Result<(), String> { access().update_skills_config_result_async(update).await }
pub async fn update_runtime_config_result_async(update: impl FnOnce(&mut RuntimeConfig)) -> Result<(), String> { access().update_runtime_config_result_async(update).await }
pub async fn update_model_routes_config_result_async(update: impl FnOnce(&mut Vec<ModelRouteConfig>)) -> Result<(), String> { access().update_model_routes_config_result_async(update).await }
pub async fn update_embedding_routes_config_result_async(update: impl FnOnce(&mut Vec<EmbeddingRouteConfig>)) -> Result<(), String> { access().update_embedding_routes_config_result_async(update).await }
pub async fn update_query_classification_config_result_async(update: impl FnOnce(&mut QueryClassificationConfig)) -> Result<(), String> { access().update_query_classification_config_result_async(update).await }
pub async fn update_autonomy_config_result_async(update: impl FnOnce(&mut AutonomyConfig)) -> Result<(), String> { access().update_autonomy_config_result_async(update).await }
pub fn update_heartbeat_config_async(update: impl FnOnce(&mut HeartbeatConfig) + Send + 'static) -> Task<Message> { access().update_heartbeat_config_async(update).map(|message| crate::app::Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: message.tag, result: message.result })) }
pub fn update_cron_config_async(update: impl FnOnce(&mut CronConfig) + Send + 'static) -> Task<Message> { access().update_cron_config_async(update).map(|message| crate::app::Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: message.tag, result: message.result })) }
pub fn update_scheduler_config_async(update: impl FnOnce(&mut SchedulerConfig) + Send + 'static) -> Task<Message> { access().update_scheduler_config_async(update).map(|message| crate::app::Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: message.tag, result: message.result })) }
pub fn update_reliability_config_async(update: impl FnOnce(&mut ReliabilityConfig) + Send + 'static) -> Task<Message> { access().update_reliability_config_async(update).map(|message| crate::app::Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: message.tag, result: message.result })) }
pub fn update_memory_config_async(update: impl FnOnce(&mut MemoryConfig) + Send + 'static) -> Task<Message> { access().update_memory_config_async(update).map(|message| crate::app::Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: message.tag, result: message.result })) }
pub fn update_security_config_async(update: impl FnOnce(&mut SecurityConfig) + Send + 'static) -> Task<Message> { access().update_security_config_async(update).map(|message| crate::app::Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: message.tag, result: message.result })) }
pub fn update_channels_config_async(update: impl FnOnce(&mut ChannelsConfig) + Send + 'static) -> Task<Message> { access().update_channels_config_async(update).map(|message| crate::app::Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: message.tag, result: message.result })) }
pub fn update_observability_config_async(update: impl FnOnce(&mut ObservabilityConfig) + Send + 'static) -> Task<Message> { access().update_observability_config_async(update).map(|message| crate::app::Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: message.tag, result: message.result })) }
pub fn update_storage_config_async(update: impl FnOnce(&mut StorageConfig) + Send + 'static) -> Task<Message> { access().update_storage_config_async(update).map(|message| crate::app::Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: message.tag, result: message.result })) }
pub fn update_proxy_config_async(update: impl FnOnce(&mut ProxyConfig) + Send + 'static) -> Task<Message> { access().update_proxy_config_async(update).map(|message| crate::app::Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: message.tag, result: message.result })) }
pub fn update_browser_config_async(update: impl FnOnce(&mut BrowserConfig) + Send + 'static) -> Task<Message> { access().update_browser_config_async(update).map(|message| crate::app::Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: message.tag, result: message.result })) }
pub fn update_http_request_config_async(update: impl FnOnce(&mut HttpRequestConfig) + Send + 'static) -> Task<Message> { access().update_http_request_config_async(update).map(|message| crate::app::Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: message.tag, result: message.result })) }
pub fn update_multimodal_config_async(update: impl FnOnce(&mut MultimodalConfig) + Send + 'static) -> Task<Message> { access().update_multimodal_config_async(update).map(|message| crate::app::Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: message.tag, result: message.result })) }
pub fn update_web_search_config_async(update: impl FnOnce(&mut WebSearchConfig) + Send + 'static) -> Task<Message> { access().update_web_search_config_async(update).map(|message| crate::app::Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: message.tag, result: message.result })) }
pub fn update_tunnel_config_async(update: impl FnOnce(&mut TunnelConfig) + Send + 'static) -> Task<Message> { access().update_tunnel_config_async(update).map(|message| crate::app::Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: message.tag, result: message.result })) }
pub fn update_hooks_config_async(update: impl FnOnce(&mut HooksConfig) + Send + 'static) -> Task<Message> { access().update_hooks_config_async(update).map(|message| crate::app::Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: message.tag, result: message.result })) }
pub fn update_skills_config_async(update: impl FnOnce(&mut SkillsConfig) + Send + 'static) -> Task<Message> { access().update_skills_config_async(update).map(|message| crate::app::Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: message.tag, result: message.result })) }
pub fn update_runtime_config_async(update: impl FnOnce(&mut RuntimeConfig) + Send + 'static) -> Task<Message> { access().update_runtime_config_async(update).map(|message| crate::app::Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: message.tag, result: message.result })) }
pub fn update_model_routes_config_async(update: impl FnOnce(&mut Vec<ModelRouteConfig>) + Send + 'static) -> Task<Message> { access().update_model_routes_config_async(update).map(|message| crate::app::Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: message.tag, result: message.result })) }
pub fn update_embedding_routes_config_async(update: impl FnOnce(&mut Vec<EmbeddingRouteConfig>) + Send + 'static) -> Task<Message> { access().update_embedding_routes_config_async(update).map(|message| crate::app::Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: message.tag, result: message.result })) }
pub fn update_query_classification_config_async(update: impl FnOnce(&mut QueryClassificationConfig) + Send + 'static) -> Task<Message> { access().update_query_classification_config_async(update).map(|message| crate::app::Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: message.tag, result: message.result })) }
pub fn update_autonomy_config_async(update: impl FnOnce(&mut AutonomyConfig) + Send + 'static) -> Task<Message> { access().update_autonomy_config_async(update).map(|message| crate::app::Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: message.tag, result: message.result })) }
pub fn update_gateway_config_async(
    update: impl FnOnce(&mut GatewayConfig) + Send + 'static,
) -> Task<Message> { access().update_gateway_config_async(update).map(|message| crate::app::Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: message.tag, result: message.result })) }
#[cfg(not(target_arch = "wasm32"))]
pub fn update_delegate_agents_config(
    update: impl FnOnce(&mut HashMap<String, DelegateAgentConfig>),
) { access().update_delegate_agents_config(update) }
#[cfg(not(target_arch = "wasm32"))]
pub fn update_delegate_agents_config_result(
    update: impl FnOnce(&mut HashMap<String, DelegateAgentConfig>),
) -> Result<(), String> { access().update_delegate_agents_config_result(update) }
pub async fn update_delegate_agents_config_result_async(
    update: impl FnOnce(&mut HashMap<String, DelegateAgentConfig>),
) -> Result<(), String> { access().update_delegate_agents_config_result_async(update).await }
#[cfg(target_arch = "wasm32")]
pub fn update_delegate_agents_config_async(
    update: impl FnOnce(&mut HashMap<String, DelegateAgentConfig>) + 'static,
) -> Task<Message> { access().update_delegate_agents_config_async(update).map(|message| crate::app::Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: message.tag, result: message.result })) }
#[cfg(not(target_arch = "wasm32"))]
pub fn update_delegate_agents_config_async(
    update: impl FnOnce(&mut HashMap<String, DelegateAgentConfig>) + Send + 'static,
) -> Task<Message> { access().update_delegate_agents_config_async(update).map(|message| crate::app::Message::Settings(crate::app::message::SettingsMessage::AgentConfigSaved { tag: message.tag, result: message.result })) }
#[cfg(not(target_arch = "wasm32"))]
pub fn update_main_agent_overrides_from_delegate_agents() { access().update_main_agent_overrides_from_delegate_agents() }
pub async fn update_main_agent_overrides_from_delegate_agents_async() -> Result<(), String> { access().update_main_agent_overrides_from_delegate_agents_async().await }
pub async fn update_gateway_config_result_async(
    update: impl FnOnce(&mut GatewayConfig),
) -> Result<(), String> { access().update_gateway_config_result_async(update).await }

/// 为领域更新注入桌面配置来源。
pub(crate) fn agent_config_access() -> desktop_config::ConfigAccess { access() }
