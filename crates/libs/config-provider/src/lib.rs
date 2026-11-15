//! Provider 兼容配置模块。
//!
//! 本模块定义模型 provider 的少量通用配置类型，主要用于：
//! - 描述 OpenAI 兼容接口模式
//! - 为特定模型提供补充的 provider 档案
//! - 放置 provider 级别的通用开关

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// 运行时内部使用的兼容 API 模式。
///
/// 这是一个非 Schema 类型，通常用于把用户配置转换为更窄的内部枚举。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompatibleApiMode {
    OpenAiChatCompletions,
    OpenAiResponses,
}

/// Provider API 模式。
///
/// 用于描述模型 provider 走哪种 OpenAI 兼容线路，常见于不同网关或兼容层之间的
/// 请求格式切换。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderApiMode {
    OpenAiChatCompletions,
    OpenAiResponses,
}

impl ProviderApiMode {
    /// 转换为运行时内部使用的兼容模式。
    pub fn as_compatible_mode(self) -> CompatibleApiMode {
        match self {
            Self::OpenAiChatCompletions => CompatibleApiMode::OpenAiChatCompletions,
            Self::OpenAiResponses => CompatibleApiMode::OpenAiResponses,
        }
    }
}

/// 模型 provider 档案配置。
///
/// 用于为某个 provider 名称预定义额外的连接与协议元数据，便于在运行时根据
/// provider 名称查找补充配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct ModelProviderConfig {
    /// 档案显示名称或标准名称。
    #[serde(default)]
    pub name: Option<String>,
    /// 该 provider 的基础请求地址。
    #[serde(default)]
    pub base_url: Option<String>,
    /// 底层线路 API 标识。
    #[serde(default)]
    pub wire_api: Option<String>,
    /// 是否要求使用 OpenAI 风格认证头。
    #[serde(default)]
    pub requires_openai_auth: bool,
}

/// Provider 通用附加配置。
///
/// 当前仅承载推理等级等少量跨 provider 的统一选项。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct ProviderConfig {
    /// provider 级别的推理等级覆盖值。
    #[serde(default)]
    pub reasoning_level: Option<String>,
}
#[cfg(test)]
#[path = "provider_tests.rs"]
mod provider_tests;
