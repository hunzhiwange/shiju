use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// 单个模型 API 入口及传输适配信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInfo {
    pub id: String,
    pub url: String,
    #[serde(default = "default_adapter")]
    pub adapter: String,
}

/// 返回 provider 默认适配器名称。
pub fn default_adapter() -> String {
    "openai-compatible".to_string()
}

/// 输入或输出侧支持的模态集合。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityIO {
    pub text: bool,
    pub audio: bool,
    pub image: bool,
    pub video: bool,
    pub pdf: bool,
}

/// 交错输入输出能力的兼容表示。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InterleavedCapability {
    Bool(bool),
    Field { field: String },
}

/// 模型能力声明。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub temperature: bool,
    pub reasoning: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_options: Option<Vec<super::reasoning::ReasoningOption>>,
    pub attachment: bool,
    pub toolcall: bool,
    pub input: CapabilityIO,
    pub output: CapabilityIO,
    pub interleaved: InterleavedCapability,
}

/// 缓存读写成本结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCostCache {
    pub read: f64,
    pub write: f64,
}

/// 超大上下文窗口下的成本结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCostOver200k {
    pub input: f64,
    pub output: f64,
    pub cache: ModelCostCache,
}

/// 模型成本结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCost {
    pub input: f64,
    pub output: f64,
    pub cache: ModelCostCache,
    #[serde(default)]
    pub experimental_over_200k: Option<ModelCostOver200k>,
}

/// 模型上下文与输出限制。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelLimit {
    pub context: u64,
    #[serde(default)]
    pub input: Option<u64>,
    pub output: u64,
}

/// 对外暴露的模型协议对象。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    #[serde(rename = "providerID")]
    pub provider_id: String,
    pub api: ApiInfo,
    pub name: String,
    #[serde(default)]
    pub family: Option<String>,
    pub capabilities: Capabilities,
    pub cost: ModelCost,
    pub limit: ModelLimit,
    pub status: String,
    #[serde(default)]
    pub options: HashMap<String, Value>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub release_date: String,
    #[serde(default)]
    pub variants: HashMap<String, HashMap<String, Value>>,
}

/// provider 来源类型。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderSource {
    Env,
    Config,
    Custom,
    Api,
}

/// 对外暴露的 provider 定义。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    pub id: String,
    pub name: String,
    pub source: ProviderSource,
    pub env: Vec<String>,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub options: HashMap<String, Value>,
    pub models: HashMap<String, Model>,
}

/// 解析后的 provider/model 组合引用。
#[derive(Debug, Clone)]
pub struct ParsedModelRef {
    pub provider_id: String,
    pub model_id: String,
}

/// 模型未找到时返回的错误信息。
#[derive(Debug)]
pub struct ModelNotFoundError {
    pub provider_id: String,
    pub model_id: String,
    pub suggestions: Vec<String>,
}

impl std::fmt::Display for ModelNotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.suggestions.is_empty() {
            write!(f, "未找到模型：{}/{}", self.provider_id, self.model_id)
        } else {
            write!(
                f,
                "未找到模型：{}/{}，你是不是想找：{}",
                self.provider_id,
                self.model_id,
                self.suggestions.join(", ")
            )
        }
    }
}

impl std::error::Error for ModelNotFoundError {}

/// 将 `provider/model` 字符串拆分为结构化引用。
pub fn parse_model(s: &str) -> ParsedModelRef {
    let mut iter = s.split('/');
    let provider_id = iter.next().unwrap_or_default().to_string();
    let rest = iter.collect::<Vec<_>>().join("/");
    ParsedModelRef { provider_id, model_id: rest }
}

/// 对模型列表进行稳定排序，优先展示常用与 latest 型号。
pub fn sort(mut models: Vec<Model>) -> Vec<Model> {
    let priority = ["gpt-5", "claude-sonnet-4", "big-pickle", "gemini-3-pro"];

    models.sort_by(|a, b| {
        let ai = priority.iter().position(|p| a.id.contains(p)).unwrap_or(usize::MAX);
        let bi = priority.iter().position(|p| b.id.contains(p)).unwrap_or(usize::MAX);

        ai.cmp(&bi)
            .then_with(|| {
                let al = if a.id.contains("latest") { 0 } else { 1 };
                let bl = if b.id.contains("latest") { 0 } else { 1 };
                al.cmp(&bl)
            })
            .then_with(|| b.id.cmp(&a.id))
    });

    models
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod types_tests;
