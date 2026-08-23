//! Knowledge base API DTOs.
//!
//! This module only defines data crossing the gateway boundary. Storage,
//! indexing and retrieval behavior live in `vw-agent`.
//!
//! 本模块只描述跨越网关边界的知识库数据契约，不负责存储、索引或检索行为。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;

fn default_indexing_mode() -> KnowledgeIndexingMode {
    KnowledgeIndexingMode::Economy
}

fn default_retrieval_mode() -> KnowledgeRetrievalMode {
    KnowledgeRetrievalMode::FullText
}

fn default_chunking_mode() -> KnowledgeChunkingMode {
    KnowledgeChunkingMode::General
}

fn default_top_k() -> usize {
    10
}

fn default_keyword_count() -> usize {
    10
}

fn default_score_threshold() -> f64 {
    0.15
}

fn default_enabled() -> bool {
    true
}

fn default_capabilities() -> KnowledgeCapabilitiesDto {
    KnowledgeCapabilitiesDto::default()
}

fn default_document_status() -> KnowledgeDocumentStatus {
    KnowledgeDocumentStatus::default()
}

fn default_chunk_type() -> KnowledgeChunkType {
    KnowledgeChunkType::default()
}

fn default_match_type() -> KnowledgeMatchType {
    KnowledgeMatchType::default()
}

fn is_false(value: &bool) -> bool {
    !*value
}

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

/// Document processing status.
/// 文档从接收、处理到停用或归档的生命周期状态。
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeDocumentStatus {
    /// 文档已接收，等待处理。
    Pending,
    /// 文档正在提取内容或建立索引。
    Processing,
    #[default]
    /// 文档处理完成并可参与检索。
    Completed,
    /// 文档处理失败，错误原因见响应字段。
    Failed,
    /// 文档保留但不参与检索。
    Disabled,
    /// 文档已归档，不属于默认活跃集合。
    Archived,
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

/// Persistent indexing job status exposed to polling clients.
/// 提供给轮询客户端的持久化索引任务状态。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeIndexJobStatus {
    #[default]
    /// 任务已持久化并等待执行。
    Queued,
    /// 后台执行器正在建立索引。
    Processing,
    /// 索引已成功写入。
    Completed,
    /// 索引失败，错误原因由任务元数据返回。
    Failed,
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

/// Document metadata returned by the gateway.
/// 网关返回的文档元数据、处理状态和统计信息。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeDocumentDto {
    /// 文档稳定标识。
    pub id: String,
    /// 所属数据集标识。
    pub dataset_id: String,
    /// 文档展示名称。
    pub name: String,
    /// 调用方附加的结构化元数据。
    pub metadata: Value,
    /// 文档是否参与默认检索。
    pub enabled: bool,
    /// 当前处理或归档状态。
    #[serde(default = "default_document_status")]
    pub status: KnowledgeDocumentStatus,
    /// 处理失败时的诊断信息；无错误时省略。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub error_message: String,
    /// 用于变更检测的内容摘要；旧载荷可缺失。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub content_hash: String,
    /// 当前文档生成的知识块数量。
    pub chunk_count: u64,
    /// 提取后文本的字符数量。
    #[serde(default)]
    pub char_count: u64,
    /// 文档被检索命中的累计次数。
    #[serde(default)]
    pub recall_count: u64,
    /// 归档时间，单位为 Unix 毫秒；未归档时省略。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived_at_ms: Option<u64>,
    /// 创建时间，单位为 Unix 毫秒。
    pub created_at_ms: u64,
    /// 最近更新时间，单位为 Unix 毫秒。
    pub updated_at_ms: u64,
}

/// Add one text document to a dataset.
/// 向指定数据集添加一份纯文本文档的请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeDocumentCreateRequest {
    /// 新文档名称。
    pub name: String,
    /// 需要切分并建立索引的纯文本内容。
    pub content: String,
    /// 调用方附加的结构化元数据。
    #[serde(default)]
    pub metadata: Value,
    /// 是否在创建后参与检索；旧载荷缺失时默认启用。
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

/// Filter document listings returned by the gateway.
/// 文档列表查询条件；未指定状态时不限制生命周期状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct KnowledgeDocumentListQuery {
    /// 可选生命周期状态过滤条件。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<KnowledgeDocumentStatus>,
}

/// Patch one existing document.
/// 稀疏更新已有文档；未出现的字段保持原值。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct KnowledgeDocumentPatchRequest {
    /// 新名称；缺失时不修改名称。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 新启用状态；缺失时不修改检索可见性。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// 是否归档；显式 `false` 表示撤销归档。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived: Option<bool>,
}

/// Explicit batch operation supported by the knowledge document gateway routes.
/// 知识文档网关明确支持的批量操作。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeDocumentsBatchAction {
    /// 让指定文档参与检索。
    Enable,
    /// 暂停指定文档参与检索。
    Disable,
    /// 将指定文档移入归档状态。
    Archive,
}

/// Batch update request for multiple documents.
/// 对一组文档执行同一显式操作的请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeDocumentsBatchActionRequest {
    /// 需要更新的文档标识集合。
    #[serde(default)]
    pub document_ids: Vec<String>,
    /// 对全部文档执行的操作。
    pub action: KnowledgeDocumentsBatchAction,
}

/// Import one local file into a dataset through the gateway.
/// 通过网关把一个本地文件导入数据集。
///
/// The payload is base64-encoded JSON so desktop clients can reuse the JSON
/// gateway transport. Size and extracted-text limits are reported by
/// [`KnowledgeDocumentImportLimitsDto`] and enforced by the gateway handler.
/// 文件内容以 Base64 放入 JSON，便于桌面端复用现有传输；网关仍按
/// [`KnowledgeDocumentImportLimitsDto`] 强制校验源文件和提取文本大小。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeDocumentImportRequest {
    /// 原始文件名，用于格式识别和展示。
    pub filename: String,
    /// 可选 MIME 类型提示，不能替代网关校验。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// 原始文件内容的 Base64 文本。
    pub content_base64: String,
    /// 调用方附加的结构化元数据。
    #[serde(default)]
    pub metadata: Value,
    /// 导入后是否立即参与检索。
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

/// Result of a gateway-owned document import.
/// 网关同步完成文件导入后返回的文档、字节统计和警告。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeDocumentImportResponse {
    /// 已创建并完成处理的文档。
    pub document: KnowledgeDocumentDto,
    /// 经网关确认的原始文件名。
    pub filename: String,
    /// 经网关确认或推断的 MIME 类型。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// 解码后的源文件字节数。
    pub source_bytes: u64,
    /// 从文件中提取的文本字节数。
    pub extracted_text_bytes: u64,
    /// 不阻止导入完成的格式或内容提示。
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// Indexing job metadata returned by asynchronous knowledge import routes.
/// 异步导入接口返回的索引任务元数据，供客户端轮询进度。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeIndexJobDto {
    /// 索引任务稳定标识。
    pub id: String,
    /// 任务所属数据集标识。
    pub dataset_id: String,
    /// 任务处理的文档标识。
    pub document_id: String,
    /// 当前执行状态。
    pub status: KnowledgeIndexJobStatus,
    /// 失败诊断；没有错误时省略。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub error_message: String,
    /// 入队时间，单位为 Unix 毫秒。
    pub created_at_ms: u64,
    /// 最近更新时间，单位为 Unix 毫秒。
    pub updated_at_ms: u64,
    /// 实际开始执行的时间，单位为 Unix 毫秒。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at_ms: Option<u64>,
    /// 执行结束时间，单位为 Unix 毫秒。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at_ms: Option<u64>,
}

/// Result of queueing a gateway-owned document import for background indexing.
/// 文件已入队等待后台索引时返回的文档和任务快照。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeDocumentImportQueuedResponse {
    /// 已持久化但可能尚未完成索引的文档。
    pub document: KnowledgeDocumentDto,
    /// 可供客户端轮询的后台索引任务。
    pub job: KnowledgeIndexJobDto,
    /// 经网关确认的原始文件名。
    pub filename: String,
    /// 经网关确认或推断的 MIME 类型。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// 解码后的源文件字节数。
    pub source_bytes: u64,
    /// 入队阶段产生的非致命提示。
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// File import limits advertised by the gateway before enforcement.
/// 网关在执行校验前公开的文件导入大小上限。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeDocumentImportLimitsDto {
    /// 允许上传的源文件最大字节数。
    pub max_file_bytes: u64,
    /// 允许进入索引流程的提取文本最大字节数。
    pub max_extracted_text_bytes: u64,
}

/// Supported or explicitly unsupported file type description.
/// 单种文件格式的支持情况；不支持时携带明确原因。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeDocumentImportTypeDto {
    /// 稳定格式标识。
    pub id: String,
    /// 面向用户展示的格式名称。
    pub label: String,
    /// 不含点号的受识别扩展名。
    #[serde(default)]
    pub extensions: Vec<String>,
    /// 受识别的 MIME 类型。
    #[serde(default)]
    pub mime_types: Vec<String>,
    /// 当前运行时能否实际导入该格式。
    pub supported: bool,
    /// 格式不可用时面向客户端的明确原因。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unsupported_reason: Option<String>,
}

/// File import support summary returned by knowledge gateway routes.
/// 知识网关返回的文件格式能力和大小限制汇总。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeImportSupportDto {
    /// 网关强制执行的大小限制。
    pub limits: KnowledgeDocumentImportLimitsDto,
    /// 已知文件格式及其当前支持情况。
    #[serde(default)]
    pub types: Vec<KnowledgeDocumentImportTypeDto>,
}

/// Reindex one existing document.
/// 请求重新索引已有文档；`force` 决定是否绕过无需更新的判断。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeDocumentReindexRequest {
    /// 是否跳过内容未变化等短路条件并强制重建。
    #[serde(default)]
    pub force: bool,
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
