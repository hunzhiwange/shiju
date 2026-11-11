use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// 超过 200k 上下文窗口时使用的分档价格。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelCostOver200k {
    #[serde(default)]
    pub input: f64,
    #[serde(default)]
    pub output: f64,
    #[serde(default)]
    pub cache_read: Option<f64>,
    #[serde(default)]
    pub cache_write: Option<f64>,
}

/// 模型输入、输出与缓存相关的成本信息。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelCost {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<f64>,
    #[serde(default)]
    pub cache_read: Option<f64>,
    #[serde(default)]
    pub cache_write: Option<f64>,
    #[serde(default)]
    pub context_over_200k: Option<ModelCostOver200k>,
}

/// 模型的上下文与输出限制信息。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelLimit {
    #[serde(default)]
    pub context: u64,
    #[serde(default)]
    pub input: Option<u64>,
    #[serde(default)]
    pub output: u64,
}

/// 模型支持的输入与输出模态。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelModalities {
    #[serde(default)]
    pub input: Vec<String>,
    #[serde(default)]
    pub output: Vec<String>,
}

/// 模型是否支持交错式输入输出的配置表示。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ModelInterleaved {
    Bool(bool),
    Field { field: String },
}

/// 模型级别的 provider 适配补充信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProviderInfo {
    /// 原始 SDK 身份；不等同于已支持的运行时协议。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub npm: Option<String>,
    /// 模型部署端点，保留模板及非 HTTP 传输声明供后续投影验证。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<String>,
    #[serde(default)]
    pub adapter: String,
}

/// 单个模型的静态元数据定义。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    #[serde(default)]
    pub id: String,
    /// 原厂目录条目的指向，保留原样。部署身份仍由所属供应商与模型 ID 决定。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_model_id: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub family: Option<String>,
    #[serde(default)]
    pub release_date: String,
    #[serde(default)]
    pub attachment: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_options: Option<Vec<super::reasoning::ReasoningOption>>,
    #[serde(default)]
    pub temperature: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call: Option<bool>,
    #[serde(default)]
    pub interleaved: Option<ModelInterleaved>,
    #[serde(default)]
    pub cost: Option<ModelCost>,
    #[serde(default)]
    pub limit: ModelLimit,
    #[serde(default)]
    pub modalities: Option<ModelModalities>,
    #[serde(default)]
    pub experimental: Option<Value>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub options: HashMap<String, Value>,
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub provider: Option<ModelProviderInfo>,
    #[serde(default)]
    pub variants: Option<HashMap<String, HashMap<String, Value>>>,
}

/// 提供商及其模型集合定义。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    /// 来源目录 SDK 声明，独立保留显式 adapter。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub npm: Option<String>,
    #[serde(default)]
    pub api: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub adapter: Option<String>,
    #[serde(default)]
    pub models: HashMap<String, Model>,
}

#[cfg(test)]
#[path = "models_tests.rs"]
mod models_tests;
