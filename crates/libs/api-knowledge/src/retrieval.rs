//! 检索、能力与嵌入配置契约。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;
use crate::defaults::{
    default_chunk_type, default_enabled, default_match_type, default_top_k, is_false,
};

/// Knowledge retrieval strategy.
/// 查询知识库时采用的召回策略。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeRetrievalMode {
    /// 仅使用本地全文索引召回。
    FullText,
    /// 仅使用向量相似度召回。
    Vector,
    /// 合并全文与向量召回结果。
    Hybrid,
}

/// Stored knowledge chunk type.
/// 持久化知识块的内容类型。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeChunkType {
    #[default]
    /// 普通独立文本块。
    Text,
    /// 为子块提供扩展上下文的父文本块。
    ParentText,
    /// 由问题与答案组成的知识块。
    Qa,
}

/// Retrieval match source.
/// 检索结果的匹配来源，用于解释分数和上下文的产生方式。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeMatchType {
    #[default]
    /// 由全文索引直接命中。
    FullText,
    /// 由向量相似度直接命中。
    Vector,
    /// 由全文和向量结果融合命中。
    Hybrid,
    /// 作为命中子块的父级上下文返回。
    Parent,
    /// 作为命中块的相邻上下文返回。
    Nearby,
}

/// Dataset capabilities exposed by the local runtime contract.
/// 本地运行时对单个数据集公开的检索能力集合。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeCapabilitiesDto {
    /// 是否支持全文检索；旧载荷缺失时默认启用。
    #[serde(default = "default_enabled")]
    pub full_text: bool,
    /// 是否支持向量检索。
    #[serde(default)]
    pub vector: bool,
    /// 是否支持全文与向量混合检索。
    #[serde(default)]
    pub hybrid: bool,
    /// 是否支持对召回结果重新排序。
    #[serde(default)]
    pub rerank: bool,
}

impl Default for KnowledgeCapabilitiesDto {
    fn default() -> Self {
        Self { full_text: true, vector: false, hybrid: false, rerank: false }
    }
}

/// A retrieved knowledge chunk.
/// 一条召回的知识块，包含位置、邻接关系、匹配来源和可选上下文。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeChunkDto {
    /// 知识块稳定标识。
    pub id: String,
    /// 所属数据集标识。
    pub dataset_id: String,
    /// 来源文档标识。
    pub document_id: String,
    /// 用于展示和引用的块标题。
    pub title: String,
    /// 可供模型或调用方消费的文本内容。
    pub content: String,
    /// 来源文档或索引流程附加的结构化信息。
    #[serde(default)]
    pub metadata: Value,
    /// 召回相关性分数；非检索场景可缺失。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    /// 知识块在来源文档中的顺序编号。
    #[serde(default)]
    pub ordinal: usize,
    /// 块的内容结构类型。
    #[serde(default = "default_chunk_type")]
    pub chunk_type: KnowledgeChunkType,
    /// 块在提取文本中的起始位置。
    #[serde(default)]
    pub start_at: usize,
    /// 块在提取文本中的结束位置。
    #[serde(default)]
    pub end_at: usize,
    /// 父块标识，仅层次切分结果可能存在。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_chunk_id: Option<String>,
    /// 同一文档中的前一知识块标识。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev_chunk_id: Option<String>,
    /// 同一文档中的后一知识块标识。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_chunk_id: Option<String>,
    /// 实际触发匹配的文本片段。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_content: Option<String>,
    /// 该结果进入响应的匹配来源。
    #[serde(default = "default_match_type")]
    pub match_type: KnowledgeMatchType,
    /// 调用方请求时附加的父块或邻接上下文。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

/// Retrieval request.
/// 知识检索请求；空数据集列表表示由服务端确定查询范围。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeRetrieveRequest {
    /// 用于召回知识块的自然语言查询。
    pub query: String,
    /// 限定查询的数据集；为空时由服务端选择范围。
    #[serde(default)]
    pub dataset_ids: Vec<String>,
    /// 最多返回的知识块数量。
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    /// 可选最低相关性分数。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_threshold: Option<f64>,
    /// 由存储实现解释的结构化元数据过滤条件。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_filter: Option<Value>,
    /// 是否为命中块补充父级或邻接上下文。
    #[serde(default, skip_serializing_if = "is_false")]
    pub include_context: bool,
}

/// Retrieval response.
/// 知识检索响应，结果顺序由服务端的召回与排序策略决定。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeRetrieveResponse {
    /// 按服务端相关性顺序返回的知识块。
    #[serde(default)]
    pub chunks: Vec<KnowledgeChunkDto>,
}

/// Capability status for the current local knowledge runtime.
/// 当前本地知识运行时实际可用的检索能力状态。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeRuntimeStatus {
    /// 全文检索当前是否可用。
    pub full_text: bool,
    /// 向量检索当前是否可用。
    pub vector: bool,
    /// 混合检索当前是否可用。
    pub hybrid: bool,
    /// 结果重排当前是否可用。
    pub rerank: bool,
    /// 解释能力限制或配置来源的诊断备注。
    #[serde(default)]
    pub notes: BTreeMap<String, String>,
}

/// Resolved embedding configuration visible to knowledge UI clients.
/// 提供给知识库界面的已解析嵌入配置与可用检索模式。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeEmbeddingStatusDto {
    /// 当前解析到的嵌入提供商标识。
    pub provider_id: String,
    /// 可选路由提示，用于说明配置选择来源。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_hint: Option<String>,
    /// 当前嵌入模型名称。
    pub model: String,
    /// 配置要求的向量维度。
    pub dimensions: usize,
    /// 是否已配置凭据；响应绝不包含凭据本身。
    pub api_key_configured: bool,
    /// 当前配置是否足以执行向量检索。
    pub vector_available: bool,
    /// 当前配置是否足以执行混合检索。
    pub hybrid_available: bool,
    /// 当前运行时允许选择的召回模式。
    #[serde(default)]
    pub retrieval_modes: Vec<KnowledgeRetrievalMode>,
    /// 面向界面的配置诊断信息。
    #[serde(default)]
    pub notes: BTreeMap<String, String>,
}

/// Gateway-owned preset for configuring an embedding provider.
/// 由网关维护的嵌入提供商预设，不包含实际凭据。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeEmbeddingPresetDto {
    /// 预设稳定标识。
    pub id: String,
    /// 面向用户展示的预设名称。
    pub label: String,
    /// 交给运行时解析的提供商标识。
    pub provider_id: String,
    /// 预设模型名称。
    pub model: String,
    /// 预设输出向量维度。
    pub dimensions: usize,
    /// 非默认提供商使用的嵌入接口地址。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint_url: Option<String>,
    /// 可提供凭据的环境变量名称，不包含变量值。
    #[serde(default)]
    pub api_key_env_vars: Vec<String>,
    /// 预设用途和区域信息说明。
    pub description: String,
}

/// Live embedding test request. Inline API keys are accepted only for the test call.
/// 实时嵌入连通性测试请求；内联密钥只允许用于本次测试调用。
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeEmbeddingTestRequest {
    /// 可选提供商覆盖值。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    /// 可选模型覆盖值。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// 可选向量维度覆盖值。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<usize>,
    /// 仅供本次连通性测试使用的临时密钥。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_override: Option<String>,
    /// 可选测试文本；缺失时由服务端提供安全样例。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_text: Option<String>,
}

impl fmt::Debug for KnowledgeEmbeddingTestRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 调试输出只暴露密钥是否存在，避免测试或日志泄漏调用方传入的原始凭据。
        let api_key_override = self.api_key_override.as_ref().map(|_| "[REDACTED]");
        formatter
            .debug_struct("KnowledgeEmbeddingTestRequest")
            .field("provider_id", &self.provider_id)
            .field("model", &self.model)
            .field("dimensions", &self.dimensions)
            .field("api_key_override", &api_key_override)
            .field("sample_text", &self.sample_text)
            .finish()
    }
}

/// Result of a live embedding provider check.
/// 实时检查嵌入提供商后返回的维度、延迟和诊断结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeEmbeddingTestResponse {
    /// 检查是否成功。
    pub ok: bool,
    /// 实际调用的提供商标识。
    pub provider_id: String,
    /// 实际调用的模型名称。
    pub model: String,
    /// 请求配置的向量维度。
    pub configured_dimensions: usize,
    /// 提供商实际返回的向量维度。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_dimensions: Option<usize>,
    /// 完成调用所用时间，单位为毫秒。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    /// 面向调用方的检查结果或失败诊断。
    pub message: String,
}
