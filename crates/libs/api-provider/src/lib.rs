//! 模型提供商与模型列表相关类型。
//!
//! 本模块描述 LLM 提供商及其模型能力，主要包括：
//! - 提供商连接状态、鉴权方式与默认模型
//! - 模型是否启用、是否支持工具、是否支持 reasoning
//! - 提供商连接、断开、刷新与模型补丁更新请求
//!
//! 这些结构通常由设置页、模型选择器和能力探测逻辑共同消费。

use vw_api_transport::{ModelId, ProviderId, TimestampMs};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 提供商连接状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatus {
    /// 已成功连接并可提供模型服务。
    Connected,
    /// 当前未连接或尚未配置完成。
    Disconnected,
    /// 连接或刷新过程中出现异常。
    Error,
}

/// 提供商鉴权方式。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthKind {
    /// 使用 API Key 鉴权。
    ApiKey,
    /// 使用 OAuth 授权流程。
    OAuth,
    /// 不需要显式鉴权。
    None,
}

/// 提供商详情。
///
/// 表达某个 LLM 提供商在当前环境中的配置与可用能力。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderDto {
    /// 提供商 ID。
    pub id: ProviderId,
    /// 面向用户展示的名称。
    pub display_name: String,
    /// 当前连接状态。
    pub status: ProviderStatus,
    /// 鉴权方式。
    pub auth_kind: AuthKind,
    /// 是否已完成必要配置。
    pub configured: bool,
    /// 是否被启用。
    pub enabled: bool,
    /// 默认模型 ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_model: Option<ModelId>,
    /// 是否支持工具调用。
    pub supports_tools: bool,
    /// 是否支持 reasoning。
    pub supports_reasoning: bool,
    /// 最近一次错误说明。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

/// 模型详情。
///
/// 表示某个具体模型在默认路由和能力层面的配置状态。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelDto {
    /// 模型 ID。
    pub id: ModelId,
    /// 所属提供商 ID。
    pub provider_id: ProviderId,
    /// 展示名称。
    pub display_name: String,
    /// 是否启用。
    pub enabled: bool,
    /// 是否为聊天默认模型。
    pub default_for_chat: bool,
    /// 是否为委派默认模型。
    pub default_for_delegate: bool,
    /// 是否支持工具调用。
    pub supports_tools: bool,
    /// 是否支持 reasoning。
    pub supports_reasoning: bool,
    /// 上下文窗口大小。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u32>,
    /// 最大输出 token 数。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
}

/// 列出提供商响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListProvidersResponse {
    pub items: Vec<ProviderDto>,
}

/// 获取单个提供商响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetProviderResponse {
    pub provider: ProviderDto,
}

/// 列出模型响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListModelsResponse {
    pub items: Vec<ModelDto>,
}

/// 查询提供商列表请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ListProvidersRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configured: Option<bool>,
}

/// 连接提供商请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ConnectProviderRequest {
    #[serde(default)]
    pub credentials: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub set_as_default: bool,
}

/// 断开提供商请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DisconnectProviderRequest {
    #[serde(default)]
    pub remove_credentials: bool,
}

/// 更新提供商请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UpdateProviderRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_model: Option<ModelId>,
}

/// 单个模型补丁项。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchProviderModelDto {
    pub id: ModelId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_for_chat: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_for_delegate: Option<bool>,
}

/// 批量更新模型请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchProviderModelsRequest {
    pub models: Vec<PatchProviderModelDto>,
}

/// 刷新提供商请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RefreshProvidersRequest {
    #[serde(default)]
    pub provider_ids: Vec<ProviderId>,
}

/// 刷新提供商响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefreshProvidersResponse {
    pub refreshed_provider_ids: Vec<ProviderId>,
    pub updated_at_ms: TimestampMs,
}

#[cfg(test)]
mod provider_tests;
