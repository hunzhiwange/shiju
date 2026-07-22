//! Knowledge base API DTOs.
//!
//! This module only defines data crossing the gateway boundary. Storage,
//! indexing and retrieval behavior live in `vw-agent`.

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeIndexingMode {
    /// Local lexical indexing only.
    Economy,
    /// Reserved for embedding-backed indexing.
    HighQuality,
}

/// Knowledge retrieval strategy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeRetrievalMode {
    FullText,
    Vector,
    Hybrid,
}

/// Knowledge chunk structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeChunkingMode {
    General,
    ParentChild,
    Qa,
}

/// Document processing status.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeDocumentStatus {
    Pending,
    Processing,
    #[default]
    Completed,
    Failed,
    Disabled,
    Archived,
}

/// Stored knowledge chunk type.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeChunkType {
    #[default]
    Text,
    ParentText,
    Qa,
}

/// Retrieval match source.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeMatchType {
    #[default]
    FullText,
    Vector,
    Hybrid,
    Parent,
    Nearby,
}

/// Persistent indexing job status exposed to polling clients.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeIndexJobStatus {
    #[default]
    Queued,
    Processing,
    Completed,
    Failed,
}

/// Dataset capabilities exposed by the local runtime contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeCapabilitiesDto {
    #[serde(default = "default_enabled")]
    pub full_text: bool,
    #[serde(default)]
    pub vector: bool,
    #[serde(default)]
    pub hybrid: bool,
    #[serde(default)]
    pub rerank: bool,
}

impl Default for KnowledgeCapabilitiesDto {
    fn default() -> Self {
        Self { full_text: true, vector: false, hybrid: false, rerank: false }
    }
}

/// Dataset metadata returned by the gateway.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeDatasetDto {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default = "default_capabilities")]
    pub capabilities: KnowledgeCapabilitiesDto,
    #[serde(default = "default_chunking_mode")]
    pub chunking_mode: KnowledgeChunkingMode,
    #[serde(default = "default_indexing_mode")]
    pub indexing_mode: KnowledgeIndexingMode,
    #[serde(default = "default_retrieval_mode")]
    pub retrieval_mode: KnowledgeRetrievalMode,
    #[serde(default = "default_keyword_count")]
    pub keyword_count: usize,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    #[serde(default)]
    pub score_threshold_enabled: bool,
    #[serde(default = "default_score_threshold")]
    pub score_threshold: f64,
    #[serde(default)]
    pub rerank_enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rerank_model: Option<String>,
    pub document_count: u64,
    pub chunk_count: u64,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

/// Create a dataset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeDatasetCreateRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_chunking_mode")]
    pub chunking_mode: KnowledgeChunkingMode,
    #[serde(default = "default_indexing_mode")]
    pub indexing_mode: KnowledgeIndexingMode,
    #[serde(default = "default_retrieval_mode")]
    pub retrieval_mode: KnowledgeRetrievalMode,
    #[serde(default = "default_keyword_count")]
    pub keyword_count: usize,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    #[serde(default)]
    pub score_threshold_enabled: bool,
    #[serde(default = "default_score_threshold")]
    pub score_threshold: f64,
    #[serde(default)]
    pub rerank_enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rerank_model: Option<String>,
}

/// Document metadata returned by the gateway.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeDocumentDto {
    pub id: String,
    pub dataset_id: String,
    pub name: String,
    pub metadata: Value,
    pub enabled: bool,
    #[serde(default = "default_document_status")]
    pub status: KnowledgeDocumentStatus,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub error_message: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub content_hash: String,
    pub chunk_count: u64,
    #[serde(default)]
    pub char_count: u64,
    #[serde(default)]
    pub recall_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived_at_ms: Option<u64>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

/// Add one text document to a dataset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeDocumentCreateRequest {
    pub name: String,
    pub content: String,
    #[serde(default)]
    pub metadata: Value,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

/// Filter document listings returned by the gateway.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct KnowledgeDocumentListQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<KnowledgeDocumentStatus>,
}

/// Patch one existing document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct KnowledgeDocumentPatchRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived: Option<bool>,
}

/// Explicit batch operation supported by the knowledge document gateway routes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeDocumentsBatchAction {
    Enable,
    Disable,
    Archive,
}

/// Batch update request for multiple documents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeDocumentsBatchActionRequest {
    #[serde(default)]
    pub document_ids: Vec<String>,
    pub action: KnowledgeDocumentsBatchAction,
}

/// Import one local file into a dataset through the gateway.
///
/// The payload is base64-encoded JSON so desktop clients can reuse the JSON
/// gateway transport. Size and extracted-text limits are reported by
/// [`KnowledgeDocumentImportLimitsDto`] and enforced by the gateway handler.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeDocumentImportRequest {
    pub filename: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    pub content_base64: String,
    #[serde(default)]
    pub metadata: Value,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

/// Result of a gateway-owned document import.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeDocumentImportResponse {
    pub document: KnowledgeDocumentDto,
    pub filename: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    pub source_bytes: u64,
    pub extracted_text_bytes: u64,
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// Indexing job metadata returned by asynchronous knowledge import routes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeIndexJobDto {
    pub id: String,
    pub dataset_id: String,
    pub document_id: String,
    pub status: KnowledgeIndexJobStatus,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub error_message: String,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at_ms: Option<u64>,
}

/// Result of queueing a gateway-owned document import for background indexing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeDocumentImportQueuedResponse {
    pub document: KnowledgeDocumentDto,
    pub job: KnowledgeIndexJobDto,
    pub filename: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    pub source_bytes: u64,
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// File import limits advertised by the gateway before enforcement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeDocumentImportLimitsDto {
    pub max_file_bytes: u64,
    pub max_extracted_text_bytes: u64,
}

/// Supported or explicitly unsupported file type description.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeDocumentImportTypeDto {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub mime_types: Vec<String>,
    pub supported: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unsupported_reason: Option<String>,
}

/// File import support summary returned by knowledge gateway routes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeImportSupportDto {
    pub limits: KnowledgeDocumentImportLimitsDto,
    #[serde(default)]
    pub types: Vec<KnowledgeDocumentImportTypeDto>,
}

/// Reindex one existing document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeDocumentReindexRequest {
    #[serde(default)]
    pub force: bool,
}

/// A retrieved knowledge chunk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeChunkDto {
    pub id: String,
    pub dataset_id: String,
    pub document_id: String,
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub metadata: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    #[serde(default)]
    pub ordinal: usize,
    #[serde(default = "default_chunk_type")]
    pub chunk_type: KnowledgeChunkType,
    #[serde(default)]
    pub start_at: usize,
    #[serde(default)]
    pub end_at: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_chunk_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev_chunk_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_chunk_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_content: Option<String>,
    #[serde(default = "default_match_type")]
    pub match_type: KnowledgeMatchType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

/// Retrieval request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeRetrieveRequest {
    pub query: String,
    #[serde(default)]
    pub dataset_ids: Vec<String>,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_threshold: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_filter: Option<Value>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub include_context: bool,
}

/// Retrieval response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeRetrieveResponse {
    #[serde(default)]
    pub chunks: Vec<KnowledgeChunkDto>,
}

/// Capability status for the current local knowledge runtime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeRuntimeStatus {
    pub full_text: bool,
    pub vector: bool,
    pub hybrid: bool,
    pub rerank: bool,
    #[serde(default)]
    pub notes: BTreeMap<String, String>,
}

/// Resolved embedding configuration visible to knowledge UI clients.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeEmbeddingStatusDto {
    pub provider_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_hint: Option<String>,
    pub model: String,
    pub dimensions: usize,
    pub api_key_configured: bool,
    pub vector_available: bool,
    pub hybrid_available: bool,
    #[serde(default)]
    pub retrieval_modes: Vec<KnowledgeRetrievalMode>,
    #[serde(default)]
    pub notes: BTreeMap<String, String>,
}

/// Gateway-owned preset for configuring an embedding provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeEmbeddingPresetDto {
    pub id: String,
    pub label: String,
    pub provider_id: String,
    pub model: String,
    pub dimensions: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint_url: Option<String>,
    #[serde(default)]
    pub api_key_env_vars: Vec<String>,
    pub description: String,
}

/// Live embedding test request. Inline API keys are accepted only for the test call.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeEmbeddingTestRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_override: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_text: Option<String>,
}

impl fmt::Debug for KnowledgeEmbeddingTestRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeEmbeddingTestResponse {
    pub ok: bool,
    pub provider_id: String,
    pub model: String,
    pub configured_dimensions: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_dimensions: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    pub message: String,
}
