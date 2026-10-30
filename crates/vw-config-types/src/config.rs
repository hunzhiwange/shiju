//! 顶层配置聚合模块。
//!
//! 本模块定义 VibeWindow 的总配置结构 [`Config`]，负责把各子系统配置组合到一个
//! 可序列化、可反序列化的统一入口中，供 `vw-agent`、`vw-desktop`、`vw-cli`
//! 等运行时共享。
//!
//! # 主要职责
//!
//! - 聚合 provider、runtime、security、memory、tools、channels 等子配置
//! - 提供默认配置值，确保最小可用启动
//! - 提供少量配置兼容与归一化辅助函数
//! - 为 JSON Schema 导出提供统一根结构

use directories::UserDirs;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::agent::{DelegateAgentConfig, WorkspacesConfig};
use crate::automation::{CronConfig, HeartbeatConfig, SchedulerConfig};
use crate::channels::ChannelsConfig;
use crate::gateway::GatewayConfig;
use crate::gateway::TunnelConfig;
use crate::hooks::HooksConfig;
use crate::memory::{MemoryConfig, StorageConfig};
use crate::observability::ObservabilityConfig;
use crate::plugins::PluginsConfig;
use crate::provider::{ModelProviderConfig, ProviderApiMode, ProviderConfig};
use crate::proxy::ProxyConfig;
use crate::reliability::ReliabilityConfig;
use crate::routing::{EmbeddingRouteConfig, ModelRouteConfig, QueryClassificationConfig};
use crate::runtime::RuntimeConfig;
use crate::security::{AutonomyConfig, IdentityConfig, SecretsConfig, SecurityConfig};
use crate::skills::SkillsConfig;
use crate::tools::{
    BrowserConfig, HttpRequestConfig, MultimodalConfig, WebFetchConfig, WebSearchConfig,
};
use crate::ui::AppUiConfig;

/// ACP 代理进程配置。
///
/// 用于描述如何启动一个 ACP 子代理，包括启动命令、参数以及附加环境变量。
/// 该结构通常挂载在顶层配置的 `acp` 字段下，按代理名称进行索引。
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct AcpAgentConfig {
    /// 启动 ACP 代理所使用的命令。
    #[serde(default)]
    pub command: String,
    /// 启动命令的参数列表。
    #[serde(default)]
    pub args: Vec<String>,
    /// 启动进程时额外注入的环境变量。
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// VibeWindow 顶层配置对象。
///
/// 这是整个配置系统的根结构，包含运行代理所需的几乎全部静态配置。
/// 在大多数场景下，外部调用方只需要加载并传递这个结构，而不需要单独处理
/// 各子模块配置。
///
/// # 配置来源
///
/// - 共享系统配置数据库
/// - 项目数据库配置覆盖
/// - 运行时默认值
/// - 少量兼容字段别名
///
/// # 设计说明
///
/// `workspace_dir` 与 `config_path` 使用 `#[serde(skip)]`，因为它们属于运行时解析结果，
/// 而不是用户手写配置内容本身。
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Config {
    /// 工作区根目录。
    ///
    /// 该字段由运行时注入，不从配置文件中反序列化。
    #[serde(skip)]
    pub workspace_dir: PathBuf,

    /// 当前配置的运行时兼容路径。
    ///
    /// 该字段由运行时注入，不从配置文件中反序列化。
    #[serde(skip)]
    pub config_path: PathBuf,

    /// 显式启用的 provider 名称列表。
    #[serde(default)]
    pub enabled_providers: Vec<String>,

    /// UI 或引导流程中优先展示的热门 provider 列表。
    #[serde(default)]
    pub popular_providers: Vec<String>,

    /// 兼容旧格式的 provider 原始配置映射。
    #[serde(default)]
    pub providers: HashMap<String, serde_json::Value>,

    /// 小模型名称，用于轻量任务或回退场景。
    #[serde(default)]
    pub small_model: Option<String>,

    /// 默认 API Key。
    pub api_key: Option<String>,

    /// 默认 API URL。
    pub api_url: Option<String>,

    /// 默认 provider 名称。
    ///
    /// 兼容字段别名 `model_provider`。
    #[serde(alias = "model_provider")]
    pub default_provider: Option<String>,

    /// 默认 provider 所使用的 API 线路模式。
    #[serde(default)]
    pub provider_api: Option<ProviderApiMode>,

    /// 默认模型名称。
    ///
    /// 兼容字段别名 `model`。
    #[serde(alias = "model")]
    pub default_model: Option<String>,

    /// 预定义模型 provider 档案。
    #[serde(default)]
    pub model_providers: HashMap<String, ModelProviderConfig>,

    /// Provider 级别的附加配置。
    #[serde(default)]
    pub provider: ProviderConfig,

    /// 默认温度参数。
    pub default_temperature: f64,

    /// 可观测性配置。
    #[serde(default)]
    pub observability: ObservabilityConfig,

    #[serde(default)]
    pub autonomy: AutonomyConfig,

    #[serde(default)]
    pub security: SecurityConfig,

    #[serde(default)]
    pub runtime: RuntimeConfig,

    #[serde(default)]
    pub reliability: ReliabilityConfig,

    #[serde(default)]
    pub scheduler: SchedulerConfig,

    #[serde(default)]
    pub workspaces: WorkspacesConfig,

    #[serde(default)]
    pub skills: SkillsConfig,

    #[serde(default)]
    pub plugins: PluginsConfig,

    #[serde(default)]
    pub model_routes: Vec<ModelRouteConfig>,

    #[serde(default)]
    pub embedding_routes: Vec<EmbeddingRouteConfig>,

    #[serde(default)]
    pub query_classification: QueryClassificationConfig,

    #[serde(default)]
    pub heartbeat: HeartbeatConfig,

    #[serde(default)]
    pub cron: CronConfig,

    #[serde(default)]
    pub channels_config: ChannelsConfig,

    #[serde(default)]
    pub memory: MemoryConfig,

    #[serde(default)]
    pub storage: StorageConfig,

    #[serde(default)]
    pub tunnel: TunnelConfig,

    #[serde(default)]
    pub gateway: GatewayConfig,

    #[serde(default)]
    pub secrets: SecretsConfig,

    #[serde(default)]
    pub browser: BrowserConfig,

    #[serde(default)]
    pub http_request: HttpRequestConfig,

    #[serde(default)]
    pub multimodal: MultimodalConfig,

    #[serde(default)]
    pub web_fetch: WebFetchConfig,

    #[serde(default)]
    pub web_search: WebSearchConfig,

    #[serde(default)]
    pub proxy: ProxyConfig,

    #[serde(default)]
    pub identity: IdentityConfig,

    #[serde(default)]
    pub agents: HashMap<String, DelegateAgentConfig>,

    #[serde(default)]
    pub hooks: HooksConfig,

    #[serde(default)]
    pub model_support_vision: Option<bool>,

    #[serde(default)]
    pub app_ui: AppUiConfig,

    #[serde(default)]
    pub acp: HashMap<String, AcpAgentConfig>,

    #[serde(default, rename = "mcpServers", skip_serializing_if = "Vec::is_empty")]
    pub mcp_servers: Vec<serde_json::Value>,

    #[serde(default, rename = "mcpServerOverrides", skip_serializing_if = "Vec::is_empty")]
    pub mcp_server_overrides: Vec<serde_json::Value>,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let model_provider_ids: Vec<&str> =
            self.model_providers.keys().map(String::as_str).collect();

        let delegate_agent_ids: Vec<&str> = self.agents.keys().map(String::as_str).collect();

        let enabled_channel_count = [
            self.channels_config.telegram.is_some(),
            self.channels_config.discord.is_some(),
            self.channels_config.slack.is_some(),
            self.channels_config.mattermost.is_some(),
            self.channels_config.webhook.is_some(),
            self.channels_config.imessage.is_some(),
            self.channels_config.signal.is_some(),
            self.channels_config.whatsapp.is_some(),
            self.channels_config.linq.is_some(),
            self.channels_config.wati.is_some(),
            self.channels_config.nextcloud_talk.is_some(),
            #[cfg(not(target_arch = "wasm32"))]
            self.channels_config.email.is_some(),
            #[cfg(target_arch = "wasm32")]
            false,
            self.channels_config.irc.is_some(),
            self.channels_config.lark.is_some(),
            self.channels_config.feishu.is_some(),
            self.channels_config.dingtalk.is_some(),
            self.channels_config.qq.is_some(),
            self.channels_config.wecom.is_some(),
            self.channels_config.wechat.is_some(),
            self.channels_config.nostr.is_some(),
            self.channels_config.clawdtalk.is_some(),
        ]
        .into_iter()
        .filter(|enabled| *enabled)
        .count();

        f.debug_struct("Config")
            .field("workspace_dir", &self.workspace_dir)
            .field("config_path", &self.config_path)
            .field("enabled_providers", &self.enabled_providers)
            .field("popular_providers", &self.popular_providers)
            .field("providers_count", &self.providers.len())
            .field("small_model", &self.small_model)
            .field("api_key_configured", &self.api_key.is_some())
            .field("api_url_configured", &self.api_url.is_some())
            .field("default_provider", &self.default_provider)
            .field("provider_api", &self.provider_api)
            .field("default_model", &self.default_model)
            .field("model_providers", &model_provider_ids)
            .field("default_temperature", &self.default_temperature)
            .field("model_routes_count", &self.model_routes.len())
            .field("embedding_routes_count", &self.embedding_routes.len())
            .field("delegate_agents", &delegate_agent_ids)
            .field("cli_channel_enabled", &self.channels_config.cli)
            .field("enabled_channels_count", &enabled_channel_count)
            .field("acp_count", &self.acp.len())
            .field("mcp_servers_count", &self.mcp_servers.len())
            .field("sensitive_sections", &"***REDACTED***")
            .finish_non_exhaustive()
    }
}

impl Default for Config {
    fn default() -> Self {
        let home =
            UserDirs::new().map_or_else(|| PathBuf::from("."), |u| u.home_dir().to_path_buf());

        let vibewindow_dir = crate::paths::home_config_dir(home);
        let providers = HashMap::from([(
            "shiju".to_string(),
            serde_json::json!({
                "api": "https://router.shiju.cloud/v1",
                "env": ["SHIJU_API_KEY"],
                "adapter": "openai-compatible",
                "models": {
                    "deepseek-v4.1-flash": {
                        "id": "deepseek-v4.1-flash",
                        "status": "active"
                    }
                }
            }),
        )]);

        Self {
            workspace_dir: vibewindow_dir.join("workspace"),
            config_path: vibewindow_dir.join(crate::paths::CONFIG_JSON_FILENAME),
            enabled_providers: Vec::new(),
            popular_providers: vec!["shiju".to_string()],
            providers,
            small_model: None,
            api_key: None,
            api_url: None,
            default_provider: Some("shiju".to_string()),
            provider_api: None,
            default_model: Some("shiju/deepseek-v4.1-flash".to_string()),
            model_providers: HashMap::new(),
            provider: ProviderConfig::default(),
            default_temperature: 0.7,
            observability: ObservabilityConfig::default(),
            autonomy: AutonomyConfig::default(),
            security: SecurityConfig::default(),
            runtime: RuntimeConfig::default(),
            reliability: ReliabilityConfig::default(),
            scheduler: SchedulerConfig::default(),
            workspaces: WorkspacesConfig::default(),
            skills: SkillsConfig::default(),
            plugins: PluginsConfig::default(),
            model_routes: Vec::new(),
            embedding_routes: Vec::new(),
            heartbeat: HeartbeatConfig::default(),
            cron: CronConfig::default(),
            channels_config: ChannelsConfig::default(),
            memory: MemoryConfig::default(),
            storage: StorageConfig::default(),
            tunnel: TunnelConfig::default(),
            gateway: GatewayConfig::default(),
            secrets: SecretsConfig::default(),
            browser: BrowserConfig::default(),
            http_request: HttpRequestConfig::default(),
            multimodal: MultimodalConfig::default(),
            web_fetch: WebFetchConfig::default(),
            web_search: WebSearchConfig::default(),
            proxy: ProxyConfig::default(),
            identity: IdentityConfig::default(),
            agents: HashMap::new(),
            hooks: HooksConfig::default(),
            query_classification: QueryClassificationConfig::default(),
            model_support_vision: None,
            app_ui: AppUiConfig::default(),
            acp: HashMap::new(),
            mcp_servers: Vec::new(),
            mcp_server_overrides: Vec::new(),
        }
    }
}

impl Config {
    /// 归一化推理等级覆盖值。
    ///
    /// 该函数会处理大小写、连字符与下划线差异，并过滤掉无效值。
    pub fn normalize_reasoning_level_override(raw: Option<&str>, source: &str) -> Option<String> {
        let value = raw?.trim();
        if value.is_empty() {
            return None;
        }
        let normalized = value.to_ascii_lowercase().replace(['-', '_'], "");
        match normalized.as_str() {
            "minimal" | "low" | "medium" | "high" | "xhigh" => Some(normalized),
            _ => {
                tracing::warn!(
                    reasoning_level = %value,
                    source,
                    "Ignoring invalid reasoning level override"
                );
                None
            }
        }
    }

    /// 解析最终生效的 provider 推理等级。
    ///
    /// 优先级如下：
    /// 1. `provider.reasoning_level`
    /// 2. `runtime.reasoning_level`（兼容旧配置，已弃用）
    pub fn effective_provider_reasoning_level(&self) -> Option<String> {
        let provider_level = Self::normalize_reasoning_level_override(
            self.provider.reasoning_level.as_deref(),
            "provider.reasoning_level",
        );
        let runtime_level = Self::normalize_reasoning_level_override(
            self.runtime.reasoning_level.as_deref(),
            "runtime.reasoning_level",
        );

        match (provider_level, runtime_level) {
            (Some(provider_level), Some(runtime_level)) => {
                if provider_level == runtime_level {
                    tracing::warn!(
                        reasoning_level = %provider_level,
                        "`runtime.reasoning_level` is deprecated; keep only `provider.reasoning_level`"
                    );
                } else {
                    tracing::warn!(
                        provider_reasoning_level = %provider_level,
                        runtime_reasoning_level = %runtime_level,
                        "`runtime.reasoning_level` is deprecated and ignored when `provider.reasoning_level` is set"
                    );
                }
                Some(provider_level)
            }
            (Some(provider_level), None) => Some(provider_level),
            (None, Some(runtime_level)) => {
                tracing::warn!(
                    reasoning_level = %runtime_level,
                    "`runtime.reasoning_level` is deprecated; using it as compatibility fallback to `provider.reasoning_level`"
                );
                Some(runtime_level)
            }
            (None, None) => None,
        }
    }

    /// 按名称查找模型 provider 档案。
    ///
    /// 查找时忽略大小写，并返回标准化后的键名与对应配置副本。
    pub fn lookup_model_provider_profile(
        &self,
        provider_name: &str,
    ) -> Option<(String, ModelProviderConfig)> {
        let needle = provider_name.trim();
        if needle.is_empty() {
            return None;
        }

        self.model_providers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(needle))
            .map(|(name, profile)| (name.clone(), profile.clone()))
    }
}
#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;
