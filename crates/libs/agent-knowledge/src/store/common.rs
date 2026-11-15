//! SQLite 与 PostgreSQL 共用的知识库存储值及纯辅助逻辑。
use gateway_error::ApiError;
use base64::Engine as _;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use vw_api_knowledge::*;
pub const MIN_OVERLAP_CHARS: usize = 12;
pub const DEFAULT_CONTEXT_SEARCH_SPAN: usize = 400;

pub const MAX_TOP_K: usize = 50;

pub const MAX_CANDIDATES: usize = 250;

pub const INDEX_JOB_MAX_ATTEMPTS: i64 = 3;

pub const INDEX_JOB_LEASE_TIMEOUT_MS: u64 = 15 * 60 * 1_000;

#[derive(Debug, Clone)]
pub struct PreparedKnowledgeChunk {
    pub ordinal: usize,
    pub title: String,
    pub content: String,
    pub index_content: String,
    pub metadata: Value,
    pub chunk_type: KnowledgeChunkType,
    pub start_at: usize,
    pub end_at: usize,
    pub parent_position: Option<usize>,
    pub context_header: String,
    pub indexed: bool,
    pub embedding: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct StoredKnowledgeDocument {
    pub id: String,
    pub dataset_id: String,
    pub body: KnowledgeDocumentCreateRequest,
}

#[derive(Debug, Clone)]
pub struct ReindexKnowledgeDocument {
    pub document: StoredKnowledgeDocument,
    pub dataset: KnowledgeDatasetDto,
}

#[derive(Debug, Clone)]
pub struct ClaimedIndexJob {
    pub id: String,
    pub dataset_id: String,
    pub document_id: String,
    pub payload_kind: String,
    pub payload_json: String,
    pub attempt_count: i64,
}

#[derive(Debug, Clone)]
pub struct KeywordRetrievalDataset {
    pub id: String,
    pub keyword_count: usize,
}

#[derive(Debug, Clone)]
pub struct RetrievalScope {
    pub keyword_datasets: Vec<KeywordRetrievalDataset>,
    pub vector_dataset_ids: Vec<String>,
    pub rerank_dataset_ids: Vec<String>,
}

pub fn index_job_retry_delay_ms(attempt_count: i64) -> u64 {
    match attempt_count {
        i64::MIN..=1 => 2_000,
        2 => 5_000,
        _ => 10_000,
    }
}

pub fn index_job_timed_out_message() -> String {
    format!("knowledge index job timed out after {INDEX_JOB_MAX_ATTEMPTS} attempts")
}

pub fn previous_link<'a>(
    chunk_ids: &'a [String],
    positions: &[usize],
    index: usize,
) -> Option<&'a str> {
    positions
        .iter()
        .position(|position| *position == index)
        .and_then(|position| position.checked_sub(1))
        .and_then(|previous| positions.get(previous))
        .and_then(|chunk_index| chunk_ids.get(*chunk_index))
        .map(String::as_str)
}

pub fn next_link<'a>(chunk_ids: &'a [String], positions: &[usize], index: usize) -> Option<&'a str> {
    positions
        .iter()
        .position(|position| *position == index)
        .and_then(|position| positions.get(position + 1))
        .and_then(|chunk_index| chunk_ids.get(*chunk_index))
        .map(String::as_str)
}

pub fn validate_id(value: &str, label: &str) -> Result<(), ApiError> {
    Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| ApiError::bad_request(format!("knowledge {label} is invalid")))
}

pub fn metadata_or_object(value: Value) -> Value {
    match value {
        Value::Null => Value::Object(Map::new()),
        other => other,
    }
}

pub fn clean_import_filename(filename: &str) -> Result<String, ApiError> {
    let normalized = filename.trim().replace('\\', "/");
    let name = normalized.rsplit('/').find(|part| !part.trim().is_empty()).unwrap_or("").trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("knowledge import filename is required"));
    }
    Ok(name.to_string())
}

pub fn normalize_import_mime(mime_type: Option<&str>) -> Option<String> {
    mime_type
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
}

pub fn import_source_bytes(content_base64: &str) -> Result<u64, ApiError> {
    let encoded = content_base64.trim();
    let max_encoded = ((crate::import::MAX_IMPORT_FILE_BYTES + 2) / 3) * 4;
    if encoded.len() > max_encoded {
        return Err(ApiError::bad_request(format!(
            "knowledge import file is too large: max {} bytes",
            crate::import::MAX_IMPORT_FILE_BYTES
        )));
    }
    let bytes = base64::engine::general_purpose::STANDARD.decode(encoded).map_err(|error| {
        ApiError::bad_request(format!("knowledge import content_base64 is invalid: {error}"))
    })?;
    if bytes.len() > crate::import::MAX_IMPORT_FILE_BYTES {
        return Err(ApiError::bad_request(format!(
            "knowledge import file is too large: max {} bytes",
            crate::import::MAX_IMPORT_FILE_BYTES
        )));
    }
    Ok(u64::try_from(bytes.len()).unwrap_or(u64::MAX))
}

pub fn chunk_metadata(
    metadata: &Value,
    document_name: &str,
    ordinal: usize,
    chunk_metadata: &Value,
) -> Value {
    let mut object = match metadata {
        Value::Object(map) => map.clone(),
        _ => Map::new(),
    };
    if let Value::Object(map) = chunk_metadata {
        object.extend(map.clone());
    }
    object.insert("document_name".to_string(), Value::String(document_name.to_string()));
    object.insert("chunk_index".to_string(), json!(ordinal));
    Value::Object(object)
}

pub fn metadata_matches(metadata: &Value, filter: &Option<Value>) -> bool {
    let Some(filter) = filter.as_ref().filter(|value| !value.is_null()) else {
        return true;
    };
    let (Value::Object(metadata), Value::Object(filter)) = (metadata, filter) else {
        return metadata == filter;
    };
    filter.iter().all(|(key, value)| metadata.get(key) == Some(value))
}

pub fn like_pattern(query: &str) -> String {
    let mut escaped = String::from("%");
    for ch in query.chars() {
        match ch {
            '%' | '_' | '\\' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            other => escaped.push(other),
        }
    }
    escaped.push('%');
    escaped
}

pub fn like_score(chunk: &KnowledgeChunkDto, query: &str) -> f32 {
    let query = query.to_lowercase();
    let title = chunk.title.to_lowercase();
    let content = chunk.content.to_lowercase();
    if title.contains(&query) {
        0.65
    } else if content.contains(&query) {
        0.55
    } else {
        0.1
    }
}

pub fn content_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn dataset_capabilities(
    indexing_mode: &KnowledgeIndexingMode,
    retrieval_mode: &KnowledgeRetrievalMode,
    rerank_enabled: bool,
) -> KnowledgeCapabilitiesDto {
    let vector = matches!(indexing_mode, KnowledgeIndexingMode::HighQuality)
        || matches!(
            retrieval_mode,
            KnowledgeRetrievalMode::Vector | KnowledgeRetrievalMode::Hybrid
        );
    KnowledgeCapabilitiesDto { full_text: true, vector, hybrid: vector, rerank: rerank_enabled }
}

pub fn final_document_status(enabled: bool) -> &'static str {
    if enabled {
        document_status_as_str(&KnowledgeDocumentStatus::Completed)
    } else {
        document_status_as_str(&KnowledgeDocumentStatus::Disabled)
    }
}

pub fn document_status_as_str(status: &KnowledgeDocumentStatus) -> &'static str {
    match status {
        KnowledgeDocumentStatus::Pending => "pending",
        KnowledgeDocumentStatus::Processing => "processing",
        KnowledgeDocumentStatus::Completed => "completed",
        KnowledgeDocumentStatus::Failed => "failed",
        KnowledgeDocumentStatus::Disabled => "disabled",
        KnowledgeDocumentStatus::Archived => "archived",
    }
}

pub fn index_job_status_as_str(status: &KnowledgeIndexJobStatus) -> &'static str {
    match status {
        KnowledgeIndexJobStatus::Queued => "queued",
        KnowledgeIndexJobStatus::Processing => "processing",
        KnowledgeIndexJobStatus::Completed => "completed",
        KnowledgeIndexJobStatus::Failed => "failed",
    }
}

pub fn parse_document_status(value: &str, enabled: bool) -> KnowledgeDocumentStatus {
    if !enabled {
        return KnowledgeDocumentStatus::Disabled;
    }
    match value {
        "pending" => KnowledgeDocumentStatus::Pending,
        "processing" => KnowledgeDocumentStatus::Processing,
        "failed" => KnowledgeDocumentStatus::Failed,
        "disabled" => KnowledgeDocumentStatus::Disabled,
        "archived" => KnowledgeDocumentStatus::Archived,
        _ => KnowledgeDocumentStatus::Completed,
    }
}

pub fn parse_index_job_status(value: &str) -> KnowledgeIndexJobStatus {
    match value {
        "processing" => KnowledgeIndexJobStatus::Processing,
        "completed" => KnowledgeIndexJobStatus::Completed,
        "failed" => KnowledgeIndexJobStatus::Failed,
        _ => KnowledgeIndexJobStatus::Queued,
    }
}

pub fn chunk_type_as_str(chunk_type: &KnowledgeChunkType) -> &'static str {
    match chunk_type {
        KnowledgeChunkType::Text => "text",
        KnowledgeChunkType::ParentText => "parent_text",
        KnowledgeChunkType::Qa => "qa",
    }
}

pub fn parse_chunk_type(value: &str) -> KnowledgeChunkType {
    match value {
        "parent_text" => KnowledgeChunkType::ParentText,
        "qa" => KnowledgeChunkType::Qa,
        _ => KnowledgeChunkType::Text,
    }
}

pub fn matched_content_from_metadata(metadata: &Value) -> Option<String> {
    metadata
        .get("child_content")
        .or_else(|| metadata.get("question"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

pub fn chunking_mode_as_str(mode: &KnowledgeChunkingMode) -> &'static str {
    match mode {
        KnowledgeChunkingMode::General => "general",
        KnowledgeChunkingMode::ParentChild => "parent_child",
        KnowledgeChunkingMode::Qa => "qa",
    }
}

pub fn indexing_mode_as_str(mode: &KnowledgeIndexingMode) -> &'static str {
    match mode {
        KnowledgeIndexingMode::Economy => "economy",
        KnowledgeIndexingMode::HighQuality => "high_quality",
    }
}

pub fn retrieval_mode_as_str(mode: &KnowledgeRetrievalMode) -> &'static str {
    match mode {
        KnowledgeRetrievalMode::FullText => "full_text",
        KnowledgeRetrievalMode::Vector => "vector",
        KnowledgeRetrievalMode::Hybrid => "hybrid",
    }
}

pub fn parse_chunking_mode(value: &str) -> KnowledgeChunkingMode {
    match value {
        "parent_child" => KnowledgeChunkingMode::ParentChild,
        "qa" => KnowledgeChunkingMode::Qa,
        _ => KnowledgeChunkingMode::General,
    }
}

pub fn parse_indexing_mode(value: &str) -> KnowledgeIndexingMode {
    match value {
        "high_quality" => KnowledgeIndexingMode::HighQuality,
        _ => KnowledgeIndexingMode::Economy,
    }
}

pub fn parse_retrieval_mode(value: &str) -> KnowledgeRetrievalMode {
    match value {
        "vector" => KnowledgeRetrievalMode::Vector,
        "hybrid" => KnowledgeRetrievalMode::Hybrid,
        _ => KnowledgeRetrievalMode::FullText,
    }
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

pub fn database_error(error: anyhow::Error) -> ApiError {
    ApiError::internal(format!("knowledge database error: {error}"))
}
