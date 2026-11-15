use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// 将任务提示词路由到指定的 provider 与 model。
///
/// ```toml
/// [[model_routes]]
/// hint = "reasoning"
/// provider = "openrouter"
/// model = "anthropic/claude-opus-4-20250514"
///
/// [[model_routes]]
/// hint = "fast"
/// provider = "groq"
/// model = "llama-3.3-70b-versatile"
/// ```
///
/// 用法：将 `hint:reasoning` 作为模型参数传入，即可按提示词路由请求。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModelRouteConfig {
    /// 任务提示词名称，例如 `reasoning`、`fast`、`code`、`summarize`。
    pub hint: String,
    /// 要路由到的 provider，必须与已知 provider 名称匹配。
    pub provider: String,
    /// 该 provider 使用的模型名称。
    pub model: String,
    /// 该路由的可选 `max_tokens` 覆盖值。
    /// 设置后，provider 请求会将输出 token 上限限制为该值。
    #[serde(default)]
    pub max_tokens: Option<u32>,
    /// 该路由 provider 的可选 API Key 覆盖值。
    #[serde(default)]
    pub api_key: Option<String>,
}

/// 将 embedding 提示词路由到指定的 provider 与 model。
///
/// ```toml
/// [[embedding_routes]]
/// hint = "semantic"
/// provider = "alibaba-cn"
/// model = "text-embedding-v4"
/// dimensions = 1024
///
/// [memory]
/// embedding_model = "hint:semantic"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EmbeddingRouteConfig {
    /// 路由提示词名称，例如 `semantic`、`archive`、`faq`。
    pub hint: String,
    /// Embedding provider，支持 `none`、`openai`、`alibaba`、`alibaba-cn` 或 `custom:<url>`。
    pub provider: String,
    /// 该 provider 使用的 embedding 模型名称。
    pub model: String,
    /// 该路由的可选 embedding 维度覆盖值。
    #[serde(default)]
    pub dimensions: Option<usize>,
    /// 该路由 provider 的可选 API Key 覆盖值。
    #[serde(default)]
    pub api_key: Option<String>,
}

/// 自动查询分类配置。
/// 根据关键词或模式识别用户消息，并路由到对应的模型提示词。默认关闭。
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub struct QueryClassificationConfig {
    /// 是否启用自动查询分类。默认值为 `false`。
    #[serde(default)]
    pub enabled: bool,
    /// 按优先级顺序评估的分类规则列表。
    #[serde(default)]
    pub rules: Vec<ClassificationRule>,
}

/// 将消息模式映射到模型提示词的单条分类规则。
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub struct ClassificationRule {
    /// 必须匹配某个 `[[model_routes]]` 中的 `hint` 值。
    pub hint: String,
    /// 不区分大小写的子串匹配列表。
    #[serde(default)]
    pub keywords: Vec<String>,
    /// 区分大小写的字面量匹配列表，例如 `"```"`、`"fn "`。
    #[serde(default)]
    pub patterns: Vec<String>,
    /// 仅当消息长度大于等于该值时才匹配。
    #[serde(default)]
    pub min_length: Option<usize>,
    /// 仅当消息长度小于等于该值时才匹配。
    #[serde(default)]
    pub max_length: Option<usize>,
    /// 优先级更高的规则会先被检查。
    #[serde(default)]
    pub priority: i32,
}
#[cfg(test)]
#[path = "routing_tests.rs"]
mod routing_tests;
