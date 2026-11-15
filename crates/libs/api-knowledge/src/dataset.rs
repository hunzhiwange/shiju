//! 数据集及默认索引配置。

use serde::{Deserialize, Serialize};
use crate::defaults::{
    default_capabilities, default_chunking_mode, default_indexing_mode, default_keyword_count,
    default_retrieval_mode, default_score_threshold, default_top_k,
};
use crate::{KnowledgeCapabilitiesDto, KnowledgeRetrievalMode};

/// Knowledge indexing strategy.
/// 知识库建立索引时采用的策略。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeIndexingMode {
    /// Local lexical indexing only.
    /// 只建立本地词法索引，不依赖嵌入服务。
    Economy,
    /// Reserved for embedding-backed indexing.
    /// 为基于嵌入向量的高质量索引预留。
    HighQuality,
}

/// Knowledge chunk structure.
/// 文档切分为知识块时采用的结构。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeChunkingMode {
    /// 按通用文本边界切分。
    General,
    /// 生成父块与子块的层次结构。
    ParentChild,
    /// 按问答对组织知识块。
    Qa,
}

/// Dataset metadata returned by the gateway.
/// 网关返回的数据集元数据及其默认检索配置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeDatasetDto {
    /// 数据集稳定标识。
    pub id: String,
    /// 面向用户展示的数据集名称。
    pub name: String,
    /// 数据集用途说明，可为空字符串。
    pub description: String,
    /// 当前运行时对该数据集可提供的能力。
    #[serde(default = "default_capabilities")]
    pub capabilities: KnowledgeCapabilitiesDto,
    /// 文档切分模式；旧载荷默认使用通用切分。
    #[serde(default = "default_chunking_mode")]
    pub chunking_mode: KnowledgeChunkingMode,
    /// 索引策略；旧载荷默认使用经济模式。
    #[serde(default = "default_indexing_mode")]
    pub indexing_mode: KnowledgeIndexingMode,
    /// 默认召回策略；旧载荷默认使用全文检索。
    #[serde(default = "default_retrieval_mode")]
    pub retrieval_mode: KnowledgeRetrievalMode,
    /// 查询扩展时最多生成的关键词数量。
    #[serde(default = "default_keyword_count")]
    pub keyword_count: usize,
    /// 单次检索默认返回的最大知识块数量。
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    /// 是否按分数阈值过滤召回结果。
    #[serde(default)]
    pub score_threshold_enabled: bool,
    /// 启用阈值过滤时采用的最低分数。
    #[serde(default = "default_score_threshold")]
    pub score_threshold: f64,
    /// 是否对召回结果执行重排。
    #[serde(default)]
    pub rerank_enabled: bool,
    /// 显式选择的嵌入模型；缺失时由运行时解析。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
    /// 显式选择的重排模型；缺失时由运行时解析。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rerank_model: Option<String>,
    /// 数据集中的文档总数。
    pub document_count: u64,
    /// 数据集中的知识块总数。
    pub chunk_count: u64,
    /// 创建时间，单位为 Unix 毫秒。
    pub created_at_ms: u64,
    /// 最近更新时间，单位为 Unix 毫秒。
    pub updated_at_ms: u64,
}

/// Create a dataset.
/// 创建数据集的请求；省略策略字段时使用本地全文检索的兼容默认值。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeDatasetCreateRequest {
    /// 新数据集名称。
    pub name: String,
    /// 可选用途说明；缺失时为空字符串。
    #[serde(default)]
    pub description: String,
    /// 文档切分模式。
    #[serde(default = "default_chunking_mode")]
    pub chunking_mode: KnowledgeChunkingMode,
    /// 建立索引时使用的策略。
    #[serde(default = "default_indexing_mode")]
    pub indexing_mode: KnowledgeIndexingMode,
    /// 查询时默认使用的召回策略。
    #[serde(default = "default_retrieval_mode")]
    pub retrieval_mode: KnowledgeRetrievalMode,
    /// 查询扩展时最多生成的关键词数量。
    #[serde(default = "default_keyword_count")]
    pub keyword_count: usize,
    /// 单次检索默认返回的最大知识块数量。
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    /// 是否启用最低分过滤。
    #[serde(default)]
    pub score_threshold_enabled: bool,
    /// 最低召回分数。
    #[serde(default = "default_score_threshold")]
    pub score_threshold: f64,
    /// 是否启用结果重排。
    #[serde(default)]
    pub rerank_enabled: bool,
    /// 可选嵌入模型覆盖值。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
    /// 可选重排模型覆盖值。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rerank_model: Option<String>,
}
