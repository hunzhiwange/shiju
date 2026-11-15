//! 文档、文件导入及索引任务契约。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::defaults::{default_document_status, default_enabled};

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
