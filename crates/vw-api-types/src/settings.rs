//! 运行时、安全与代理相关设置类型。
//!
//! 本模块定义用户可配置的系统设置视图，覆盖：
//! - 安全约束与自主程度
//! - 沙箱与 Wasm 执行策略
//! - 网络代理、记忆后端、Web 搜索与浏览器能力
//! - 外部渠道开关与委派代理列表
//!
//! 同时提供完整设置快照和局部 patch 两套结构，分别用于读取与增量更新。

use serde::{Deserialize, Serialize};

/// 自主程度配置。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyLevelDto {
    /// 偏保守，尽量降低高风险自动化程度。
    Strict,
    /// 在安全和效率之间取平衡。
    Balanced,
    /// 允许更高的自主执行程度。
    Permissive,
}

/// Shell 重定向策略。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShellRedirectPolicyDto {
    /// 仅允许安全子集的重定向行为。
    SafeOnly,
    /// 允许 shell 重定向。
    Allow,
    /// 禁止 shell 重定向。
    Deny,
}

/// 网络访问级别。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkAccessDto {
    /// 完全禁止网络访问。
    Deny,
    /// 仅允许受限网络访问。
    Restricted,
    /// 允许网络访问。
    Allow,
}

/// 沙箱后端。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxBackendDto {
    /// 使用宿主机原生执行后端。
    Native,
    /// 使用 Docker 承载命令执行。
    Docker,
    /// 使用 Wasm 承载命令执行。
    Wasm,
}

/// Wasm 能力升级策略。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WasmCapabilityEscalationModeDto {
    /// 拒绝能力升级。
    Deny,
    /// 升级前询问用户。
    Ask,
    /// 自动允许升级。
    Allow,
}

/// Wasm 模块哈希校验策略。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WasmModuleHashPolicyDto {
    /// 忽略模块哈希校验。
    Ignore,
    /// 哈希不匹配时仅警告。
    Warn,
    /// 哈希不匹配时拒绝执行。
    Enforce,
}

/// 安全设置。
///
/// 控制代理在执行命令、访问网络和处理潜在风险操作时的默认边界。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecuritySettingsDto {
    /// 自主程度。
    pub autonomy_level: AutonomyLevelDto,
    /// Shell 重定向策略。
    pub shell_redirect_policy: ShellRedirectPolicyDto,
    /// 网络访问级别。
    pub network_access: NetworkAccessDto,
}

/// 运行时设置。
///
/// 描述命令执行后端及 Wasm 安全相关策略。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeSettingsDto {
    /// 沙箱后端。
    pub sandbox_backend: SandboxBackendDto,
    /// Wasm 能力升级模式。
    pub wasm_capability_escalation_mode: WasmCapabilityEscalationModeDto,
    /// Wasm 模块哈希校验策略。
    pub wasm_module_hash_policy: WasmModuleHashPolicyDto,
}

/// 代理设置。
///
/// 用于配置出站 HTTP 请求所使用的代理服务器。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProxySettingsDto {
    /// 是否启用代理。
    pub enabled: bool,
    /// HTTP 代理地址。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_proxy: Option<String>,
    /// HTTPS 代理地址。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub https_proxy: Option<String>,
}

/// 记忆系统设置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemorySettingsDto {
    pub backend: String,
    pub enabled: bool,
}

/// Web 搜索设置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebSearchSettingsDto {
    pub enabled: bool,
    pub provider: String,
}

/// 外部渠道设置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ChannelsSettingsDto {
    #[serde(default)]
    pub telegram_enabled: bool,
    #[serde(default)]
    pub discord_enabled: bool,
}

/// 浏览器工具设置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrowserSettingsDto {
    pub enabled: bool,
    pub headless: bool,
}

/// 委派代理配置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DelegateAgentDto {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// 完整设置视图。
///
/// 表示设置页一次性读取到的完整配置快照。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettingsDto {
    /// 安全设置。
    pub security: SecuritySettingsDto,
    /// 运行时设置。
    pub runtime: RuntimeSettingsDto,
    /// 代理设置。
    pub proxy: ProxySettingsDto,
    /// 记忆设置。
    pub memory: MemorySettingsDto,
    /// Web 搜索设置。
    pub web_search: WebSearchSettingsDto,
    /// 外部渠道设置。
    pub channels: ChannelsSettingsDto,
    /// 浏览器设置。
    pub browser: BrowserSettingsDto,
    /// 委派代理列表。
    #[serde(default)]
    pub delegates: Vec<DelegateAgentDto>,
}

/// 获取设置响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetSettingsResponse {
    pub settings: SettingsDto,
}

/// 安全设置补丁。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SecuritySettingsPatchDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autonomy_level: Option<AutonomyLevelDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell_redirect_policy: Option<ShellRedirectPolicyDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_access: Option<NetworkAccessDto>,
}

/// 运行时设置补丁。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RuntimeSettingsPatchDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sandbox_backend: Option<SandboxBackendDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wasm_capability_escalation_mode: Option<WasmCapabilityEscalationModeDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wasm_module_hash_policy: Option<WasmModuleHashPolicyDto>,
}

/// 代理设置补丁。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProxySettingsPatchDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_proxy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub https_proxy: Option<String>,
}

/// 记忆设置补丁。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MemorySettingsPatchDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

/// Web 搜索设置补丁。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WebSearchSettingsPatchDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

/// 渠道设置补丁。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ChannelsSettingsPatchDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub telegram_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discord_enabled: Option<bool>,
}

/// 浏览器设置补丁。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BrowserSettingsPatchDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headless: Option<bool>,
}

/// 完整设置补丁请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SettingsPatchDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security: Option<SecuritySettingsPatchDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<RuntimeSettingsPatchDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy: Option<ProxySettingsPatchDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<MemorySettingsPatchDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub web_search: Option<WebSearchSettingsPatchDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channels: Option<ChannelsSettingsPatchDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser: Option<BrowserSettingsPatchDto>,
}

/// 获取委派代理列表响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DelegateAgentsResponse {
    pub items: Vec<DelegateAgentDto>,
}

/// 批量更新委派代理请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchDelegateAgentsRequest {
    pub items: Vec<PatchDelegateAgentDto>,
}

/// 单个委派代理补丁项。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchDelegateAgentDto {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}
