//! 知识库网关路由，处理数据集、文档、索引、嵌入与检索请求。
//!
//! Knowledge base gateway routes.

use axum::{
    Json, Router,
    extract::{Extension, Path, Query},
    routing::{get, patch, post},
};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Instant;
use vw_api_types::TenantScope;
use vw_api_types::knowledge::{
    KnowledgeChunkDto, KnowledgeDatasetCreateRequest, KnowledgeDatasetDto,
    KnowledgeDocumentCreateRequest, KnowledgeDocumentDto, KnowledgeDocumentImportQueuedResponse,
    KnowledgeDocumentImportRequest, KnowledgeDocumentImportResponse, KnowledgeDocumentListQuery,
    KnowledgeDocumentPatchRequest, KnowledgeDocumentReindexRequest,
    KnowledgeDocumentsBatchActionRequest, KnowledgeEmbeddingPresetDto, KnowledgeEmbeddingStatusDto,
    KnowledgeEmbeddingTestRequest, KnowledgeEmbeddingTestResponse, KnowledgeIndexJobDto,
    KnowledgeIndexJobStatus, KnowledgeRetrievalMode, KnowledgeRetrieveRequest,
    KnowledgeRetrieveResponse, KnowledgeRuntimeStatus,
};

use crate::app::agent::gateway::ApiError;
use crate::app::agent::gateway::state::AppState;
use crate::global_database::GlobalDatabase;
use crate::knowledge::SqliteKnowledgeStore;
use crate::knowledge::import::parse_document_import;

pub(crate) fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/knowledge/status", get(knowledge_status))
        .route("/knowledge/embedding/status", get(knowledge_embedding_status))
        .route("/knowledge/embedding/presets", get(knowledge_embedding_presets))
        .route("/knowledge/embedding/test", post(knowledge_embedding_test))
        .route("/knowledge/datasets", get(knowledge_datasets_list).post(knowledge_dataset_create))
        .route(
            "/knowledge/datasets/{dataset_id}",
            get(knowledge_dataset_get).delete(knowledge_dataset_delete),
        )
        .route(
            "/knowledge/datasets/{dataset_id}/documents",
            get(knowledge_documents_list).post(knowledge_document_create),
        )
        .route("/knowledge/datasets/{dataset_id}/documents/import", post(knowledge_document_import))
        .route(
            "/knowledge/datasets/{dataset_id}/documents/import-async",
            post(knowledge_document_import_async),
        )
        .route("/knowledge/index-jobs/{job_id}", get(knowledge_index_job_get))
        .route("/knowledge/documents/batch", post(knowledge_documents_batch_update))
        .route(
            "/knowledge/documents/{document_id}",
            patch(knowledge_document_update).delete(knowledge_document_delete),
        )
        .route("/knowledge/documents/{document_id}/chunks", get(knowledge_document_chunks_list))
        .route("/knowledge/documents/{document_id}/reindex", post(knowledge_document_reindex))
        .route("/knowledge/retrieve", post(knowledge_retrieve))
        .route("/knowledge/retrieval-test", post(knowledge_retrieve))
}

async fn knowledge_status(
    Extension(tenant): Extension<TenantScope>,
) -> Result<Json<KnowledgeRuntimeStatus>, ApiError> {
    Ok(Json(knowledge_store_from_global_config(tenant).await?.status()))
}

async fn knowledge_embedding_status() -> Result<Json<KnowledgeEmbeddingStatusDto>, ApiError> {
    knowledge_embedding_status_from_config(crate::config::get().await).await
}

async fn knowledge_embedding_status_from_config(
    config: crate::config::Config,
) -> Result<Json<KnowledgeEmbeddingStatusDto>, ApiError> {
    let (embedder, resolved) = crate::memory::create_embedding_provider_with_routes(
        &config.memory,
        &config.embedding_routes,
        config.api_key.as_deref(),
    );
    let vector_available = embedder.dimensions() > 0;
    let mut retrieval_modes = vec![KnowledgeRetrievalMode::FullText];
    if vector_available {
        retrieval_modes.push(KnowledgeRetrievalMode::Vector);
        retrieval_modes.push(KnowledgeRetrievalMode::Hybrid);
    }

    let mut notes = BTreeMap::new();
    notes
        .insert("embedding_cache_size".to_string(), config.memory.embedding_cache_size.to_string());
    if !vector_available {
        notes.insert(
            "vector".to_string(),
            "embedding provider is configured as none or unsupported".to_string(),
        );
    }

    Ok(Json(KnowledgeEmbeddingStatusDto {
        provider_id: resolved.provider.clone(),
        route_hint: resolved_embedding_route_hint(&config),
        model: resolved.model.clone(),
        dimensions: resolved.dimensions,
        api_key_configured: resolved.api_key_configured(),
        vector_available,
        hybrid_available: vector_available,
        retrieval_modes,
        notes,
    }))
}

async fn knowledge_embedding_presets() -> Result<Json<Vec<KnowledgeEmbeddingPresetDto>>, ApiError> {
    Ok(Json(vec![
        KnowledgeEmbeddingPresetDto {
            id: "alibaba-intl".to_string(),
            label: "Alibaba Bailian DashScope International".to_string(),
            provider_id: "alibaba".to_string(),
            model: "text-embedding-v4".to_string(),
            dimensions: 1024,
            endpoint_url: Some(
                "https://dashscope-intl.aliyuncs.com/compatible-mode/v1/embeddings".to_string(),
            ),
            api_key_env_vars: vec!["DASHSCOPE_API_KEY".to_string()],
            description:
                "DashScope international embeddings through the built-in Alibaba provider."
                    .to_string(),
        },
        KnowledgeEmbeddingPresetDto {
            id: "alibaba-cn".to_string(),
            label: "Alibaba Bailian DashScope China".to_string(),
            provider_id: "alibaba-cn".to_string(),
            model: "text-embedding-v4".to_string(),
            dimensions: 1024,
            endpoint_url: Some(
                "https://dashscope.aliyuncs.com/compatible-mode/v1/embeddings".to_string(),
            ),
            api_key_env_vars: vec!["DASHSCOPE_API_KEY".to_string()],
            description: "DashScope China embeddings through the built-in Alibaba China provider."
                .to_string(),
        },
        KnowledgeEmbeddingPresetDto {
            id: "doubao-volcengine".to_string(),
            label: "Doubao Volcengine OpenAI-compatible".to_string(),
            provider_id: "custom:https://ark.cn-beijing.volces.com/api/v3".to_string(),
            model: "doubao-embedding-text-240715".to_string(),
            dimensions: 2048,
            endpoint_url: Some("https://ark.cn-beijing.volces.com/api/v3/embeddings".to_string()),
            api_key_env_vars: vec!["ARK_API_KEY".to_string()],
            description: "Volcengine Ark embeddings through the custom OpenAI-compatible route."
                .to_string(),
        },
    ]))
}

async fn knowledge_embedding_test(
    Json(body): Json<KnowledgeEmbeddingTestRequest>,
) -> Result<Json<KnowledgeEmbeddingTestResponse>, ApiError> {
    knowledge_embedding_test_with_config(crate::config::get().await, Json(body)).await
}

async fn knowledge_embedding_test_with_config(
    config: crate::config::Config,
    Json(body): Json<KnowledgeEmbeddingTestRequest>,
) -> Result<Json<KnowledgeEmbeddingTestResponse>, ApiError> {
    let mut resolved = crate::memory::resolve_embedding_config(
        &config.memory,
        &config.embedding_routes,
        config.api_key.as_deref(),
    );

    if let Some(provider_id) =
        required_trimmed_override("provider_id", body.provider_id.as_deref())?
    {
        resolved.provider = normalize_embedding_provider_id(&provider_id);
    }
    if let Some(model) = required_trimmed_override("model", body.model.as_deref())? {
        resolved.model = model;
    }
    if let Some(dimensions) = body.dimensions {
        if dimensions == 0 {
            return Err(ApiError::bad_request("embedding test dimensions must be greater than 0"));
        }
        resolved.dimensions = dimensions;
    }
    let api_key_override = optional_trimmed(body.api_key_override.as_deref());
    if let Some(api_key) = api_key_override.as_ref() {
        resolved.set_api_key(Some(api_key.clone()));
    }

    let response_template = EmbeddingTestResponseTemplate {
        provider_id: resolved.provider.clone(),
        model: resolved.model.clone(),
        configured_dimensions: resolved.dimensions,
    };

    if resolved.dimensions == 0 || provider_is_none(&resolved.provider) {
        return Ok(Json(response_template.failure(
            None,
            "embedding provider is not configured for vector embeddings".to_string(),
        )));
    }

    if !embedding_provider_is_supported(&resolved.provider) {
        return Ok(Json(
            response_template
                .failure(None, format!("unsupported embedding provider '{}'", resolved.provider)),
        ));
    }

    if embedding_provider_requires_api_key(&resolved.provider) && !resolved.api_key_configured() {
        return Ok(Json(response_template.failure(
            None,
            format!("embedding test requires an API key for provider '{}'", resolved.provider),
        )));
    }

    let sample_text = optional_trimmed(body.sample_text.as_deref())
        .unwrap_or_else(|| "VibeWindow knowledge embedding connectivity probe.".to_string());
    let embedder = crate::memory::embeddings::create_embedding_provider(
        &resolved.provider,
        resolved.api_key(),
        &resolved.model,
        resolved.dimensions,
    );

    let started_at = Instant::now();
    match embedder.embed_one(&sample_text).await {
        Ok(vector) => Ok(Json(KnowledgeEmbeddingTestResponse {
            ok: true,
            provider_id: response_template.provider_id,
            model: response_template.model,
            configured_dimensions: response_template.configured_dimensions,
            observed_dimensions: Some(vector.len()),
            latency_ms: Some(started_at.elapsed().as_millis() as u64),
            message: "embedding test succeeded".to_string(),
        })),
        Err(error) => {
            let message = sanitize_embedding_error(
                &error.to_string(),
                &[resolved.api_key(), api_key_override.as_deref()],
            );
            Ok(Json(
                response_template.failure(Some(started_at.elapsed().as_millis() as u64), message),
            ))
        }
    }
}

async fn knowledge_datasets_list(
    Extension(tenant): Extension<TenantScope>,
) -> Result<Json<Vec<KnowledgeDatasetDto>>, ApiError> {
    Ok(Json(knowledge_store_from_global_config(tenant).await?.list_datasets().await?))
}

async fn knowledge_dataset_get(
    Extension(tenant): Extension<TenantScope>,
    Path(dataset_id): Path<String>,
) -> Result<Json<KnowledgeDatasetDto>, ApiError> {
    Ok(Json(knowledge_store_from_global_config(tenant).await?.get_dataset(dataset_id).await?))
}

async fn knowledge_dataset_create(
    Extension(tenant): Extension<TenantScope>,
    Json(body): Json<KnowledgeDatasetCreateRequest>,
) -> Result<Json<KnowledgeDatasetDto>, ApiError> {
    Ok(Json(knowledge_store_from_global_config(tenant).await?.create_dataset(body).await?))
}

async fn knowledge_dataset_delete(
    Extension(tenant): Extension<TenantScope>,
    Path(dataset_id): Path<String>,
) -> Result<Json<KnowledgeDatasetDto>, ApiError> {
    Ok(Json(knowledge_store_from_global_config(tenant).await?.delete_dataset(dataset_id).await?))
}

async fn knowledge_documents_list(
    Extension(tenant): Extension<TenantScope>,
    Path(dataset_id): Path<String>,
    Query(query): Query<KnowledgeDocumentListQuery>,
) -> Result<Json<Vec<KnowledgeDocumentDto>>, ApiError> {
    Ok(Json(
        knowledge_store_from_global_config(tenant)
            .await?
            .list_documents_with_query(dataset_id, query)
            .await?,
    ))
}

async fn knowledge_document_create(
    Extension(tenant): Extension<TenantScope>,
    Path(dataset_id): Path<String>,
    Json(body): Json<KnowledgeDocumentCreateRequest>,
) -> Result<Json<KnowledgeDocumentDto>, ApiError> {
    Ok(Json(
        knowledge_store_from_global_config(tenant).await?.create_document(dataset_id, body).await?,
    ))
}

async fn knowledge_document_import(
    Extension(tenant): Extension<TenantScope>,
    Path(dataset_id): Path<String>,
    Json(body): Json<KnowledgeDocumentImportRequest>,
) -> Result<Json<KnowledgeDocumentImportResponse>, ApiError> {
    let parsed = tokio::task::spawn_blocking(move || parse_document_import(body))
        .await
        .map_err(|error| ApiError::internal(format!("knowledge import task failed: {error}")))??;
    let document = knowledge_store_from_global_config(tenant)
        .await?
        .create_document(dataset_id, parsed.create)
        .await?;
    Ok(Json(KnowledgeDocumentImportResponse {
        document,
        filename: parsed.filename,
        mime_type: parsed.mime_type,
        source_bytes: parsed.source_bytes,
        extracted_text_bytes: parsed.extracted_text_bytes,
        warnings: parsed.warnings,
    }))
}

async fn knowledge_document_import_async(
    Extension(tenant): Extension<TenantScope>,
    Path(dataset_id): Path<String>,
    Json(body): Json<KnowledgeDocumentImportRequest>,
) -> Result<Json<KnowledgeDocumentImportQueuedResponse>, ApiError> {
    let store = knowledge_store_from_global_config(tenant).await?;
    let queued = store.enqueue_import(dataset_id, body).await?;
    trigger_index_job_drain(store);
    Ok(Json(queued))
}

async fn knowledge_index_job_get(
    Extension(tenant): Extension<TenantScope>,
    Path(job_id): Path<String>,
) -> Result<Json<KnowledgeIndexJobDto>, ApiError> {
    let store = knowledge_store_from_global_config(tenant).await?;
    let job = store.get_index_job(job_id).await?;
    if matches!(job.status, KnowledgeIndexJobStatus::Queued | KnowledgeIndexJobStatus::Processing) {
        trigger_index_job_drain(store);
    }
    Ok(Json(job))
}

fn trigger_index_job_drain(store: SqliteKnowledgeStore) {
    tokio::spawn(async move {
        loop {
            match store.process_next_index_job().await {
                Ok(Some(_job)) => {}
                Ok(None) => break,
                Err(error) => {
                    tracing::warn!(error = %error, "knowledge index job drain failed");
                    break;
                }
            }
        }
    });
}

async fn knowledge_document_delete(
    Extension(tenant): Extension<TenantScope>,
    Path(document_id): Path<String>,
) -> Result<Json<KnowledgeDocumentDto>, ApiError> {
    Ok(Json(knowledge_store_from_global_config(tenant).await?.delete_document(document_id).await?))
}

async fn knowledge_document_update(
    Extension(tenant): Extension<TenantScope>,
    Path(document_id): Path<String>,
    Json(body): Json<KnowledgeDocumentPatchRequest>,
) -> Result<Json<KnowledgeDocumentDto>, ApiError> {
    Ok(Json(
        knowledge_store_from_global_config(tenant)
            .await?
            .update_document(document_id, body)
            .await?,
    ))
}

async fn knowledge_documents_batch_update(
    Extension(tenant): Extension<TenantScope>,
    Json(body): Json<KnowledgeDocumentsBatchActionRequest>,
) -> Result<Json<Vec<KnowledgeDocumentDto>>, ApiError> {
    Ok(Json(knowledge_store_from_global_config(tenant).await?.batch_update_documents(body).await?))
}

async fn knowledge_document_chunks_list(
    Extension(tenant): Extension<TenantScope>,
    Path(document_id): Path<String>,
) -> Result<Json<Vec<KnowledgeChunkDto>>, ApiError> {
    Ok(Json(knowledge_store_from_global_config(tenant).await?.list_chunks(document_id).await?))
}

async fn knowledge_document_reindex(
    Extension(tenant): Extension<TenantScope>,
    Path(document_id): Path<String>,
    Json(_body): Json<KnowledgeDocumentReindexRequest>,
) -> Result<Json<KnowledgeDocumentDto>, ApiError> {
    Ok(Json(knowledge_store_from_global_config(tenant).await?.reindex_document(document_id).await?))
}

async fn knowledge_retrieve(
    Extension(tenant): Extension<TenantScope>,
    Json(body): Json<KnowledgeRetrieveRequest>,
) -> Result<Json<KnowledgeRetrieveResponse>, ApiError> {
    Ok(Json(knowledge_store_from_global_config(tenant).await?.retrieve(body).await?))
}

pub(super) fn knowledge_store(
    state: &AppState,
    tenant: TenantScope,
) -> Result<SqliteKnowledgeStore, ApiError> {
    let config = state.config.lock().clone();
    knowledge_store_from_config(config, tenant)
}

async fn knowledge_store_from_global_config(
    tenant: TenantScope,
) -> Result<SqliteKnowledgeStore, ApiError> {
    knowledge_store_from_config(crate::config::get().await, tenant)
}

fn knowledge_store_from_config(
    config: crate::config::Config,
    tenant: TenantScope,
) -> Result<SqliteKnowledgeStore, ApiError> {
    let (embedder, resolved_embedding) = crate::memory::create_embedding_provider_with_routes(
        &config.memory,
        &config.embedding_routes,
        config.api_key.as_deref(),
    );
    let embedding_model = (embedder.dimensions() > 0).then_some(resolved_embedding.model);
    let database = GlobalDatabase::for_home(&crate::global::paths().home).map_err(|error| {
        ApiError::internal(format!("initialize global knowledge database failed: {error}"))
    })?;
    SqliteKnowledgeStore::with_embedder(
        database,
        tenant,
        embedder,
        embedding_model,
        config.memory.vector_weight as f32,
        config.memory.keyword_weight as f32,
        config.memory.embedding_cache_size,
    )
}

struct EmbeddingTestResponseTemplate {
    provider_id: String,
    model: String,
    configured_dimensions: usize,
}

impl EmbeddingTestResponseTemplate {
    fn failure(self, latency_ms: Option<u64>, message: String) -> KnowledgeEmbeddingTestResponse {
        KnowledgeEmbeddingTestResponse {
            ok: false,
            provider_id: self.provider_id,
            model: self.model,
            configured_dimensions: self.configured_dimensions,
            observed_dimensions: None,
            latency_ms,
            message,
        }
    }
}

fn resolved_embedding_route_hint(config: &crate::config::Config) -> Option<String> {
    let hint = config
        .memory
        .embedding_model
        .strip_prefix("hint:")
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let route = config.embedding_routes.iter().find(|route| route.hint.trim() == hint)?;
    let provider = route.provider.trim();
    let model = route.model.trim();
    let dimensions = route.dimensions.unwrap_or(config.memory.embedding_dimensions);
    if provider.is_empty() || model.is_empty() || dimensions == 0 {
        return None;
    }
    Some(hint.to_string())
}

fn normalize_embedding_provider_id(provider_id: &str) -> String {
    let trimmed = provider_id.trim();
    let normalized = trimmed.to_ascii_lowercase();
    match normalized.as_str() {
        "none" | "openai" | "openrouter" | "alibaba" | "alibaba-cn" => normalized,
        _ => trimmed.to_string(),
    }
}

fn embedding_provider_is_supported(provider_id: &str) -> bool {
    let trimmed = provider_id.trim();
    let normalized = trimmed.to_ascii_lowercase();
    matches!(normalized.as_str(), "none" | "openai" | "openrouter" | "alibaba" | "alibaba-cn")
        || trimmed.strip_prefix("custom:").map(str::trim).is_some_and(|url| !url.is_empty())
}

fn embedding_provider_requires_api_key(provider_id: &str) -> bool {
    !provider_is_none(provider_id)
}

fn provider_is_none(provider_id: &str) -> bool {
    provider_id.trim().eq_ignore_ascii_case("none")
}

fn optional_trimmed(value: Option<&str>) -> Option<String> {
    value.map(str::trim).filter(|value| !value.is_empty()).map(str::to_string)
}

fn required_trimmed_override(
    field_name: &str,
    value: Option<&str>,
) -> Result<Option<String>, ApiError> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ApiError::bad_request(format!(
            "embedding test {field_name} must not be empty"
        )));
    }
    Ok(Some(trimmed.to_string()))
}

fn sanitize_embedding_error(error: &str, secrets: &[Option<&str>]) -> String {
    let mut sanitized = error.to_string();
    for secret in secrets.iter().flatten() {
        let secret = secret.trim();
        if !secret.is_empty() {
            sanitized = sanitized.replace(secret, "[REDACTED]");
        }
    }
    sanitized.chars().take(600).collect()
}

pub(super) fn knowledge_db_path() -> PathBuf {
    GlobalDatabase::for_home(&crate::global::paths().home)
        .map(|database| database.path().to_path_buf())
        .unwrap_or_else(|_| {
            vw_config_types::paths::global_database_path(&crate::global::paths().home)
        })
}

#[cfg(test)]
#[path = "knowledge_tests.rs"]
mod knowledge_tests;
