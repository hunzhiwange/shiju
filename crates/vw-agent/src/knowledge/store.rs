//! 基于全局数据库配置的知识存储与检索实现。
//!
//! 该模块承载 SQLite 主实现，并把 PostgreSQL 路径委托给对应后端模块。

use super::chunker::{chunk_text, chunk_text_with_limits};
use super::fusion::{
    FusedCandidate, RetrievalSource, RetrievedCandidate, deduplicate_by_score, fuse_with_rrf,
};
use crate::app::agent::gateway::ApiError;
use crate::global_database::GlobalDatabase;
use crate::global_database::config::DatabaseProvider;
use crate::memory::embeddings::{EmbeddingProvider, NoopEmbedding};
use crate::memory::vector;
use crate::workflow::{
    WorkflowKnowledgeChunk, WorkflowKnowledgeProvider, WorkflowKnowledgeRequest,
};
use base64::Engine as _;
use rusqlite::{
    Connection, OptionalExtension, Transaction, TransactionBehavior, params, params_from_iter,
};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use uuid::Uuid;
use vw_api_types::TenantScope;
use vw_api_types::knowledge::{
    KnowledgeCapabilitiesDto, KnowledgeChunkDto, KnowledgeChunkType, KnowledgeChunkingMode,
    KnowledgeDatasetCreateRequest, KnowledgeDatasetDto, KnowledgeDocumentCreateRequest,
    KnowledgeDocumentDto, KnowledgeDocumentImportQueuedResponse, KnowledgeDocumentImportRequest,
    KnowledgeDocumentListQuery, KnowledgeDocumentPatchRequest, KnowledgeDocumentStatus,
    KnowledgeDocumentsBatchAction, KnowledgeDocumentsBatchActionRequest, KnowledgeIndexJobDto,
    KnowledgeIndexJobStatus, KnowledgeIndexingMode, KnowledgeMatchType, KnowledgeRetrievalMode,
    KnowledgeRetrieveRequest, KnowledgeRetrieveResponse, KnowledgeRuntimeStatus,
};

mod postgres;

use postgres::PostgresKnowledgeStore;

const KNOWLEDGE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS knowledge_datasets (
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    chunking_mode TEXT NOT NULL DEFAULT 'general',
    indexing_mode TEXT NOT NULL,
    retrieval_mode TEXT NOT NULL,
    keyword_count INTEGER NOT NULL DEFAULT 10,
    top_k INTEGER NOT NULL DEFAULT 10,
    score_threshold_enabled INTEGER NOT NULL DEFAULT 0,
    score_threshold REAL NOT NULL DEFAULT 0.15,
    rerank_enabled INTEGER NOT NULL DEFAULT 0,
    embedding_model TEXT,
    rerank_model TEXT,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY (platform_id, company_id, id)
);

CREATE TABLE IF NOT EXISTS knowledge_documents (
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    id TEXT NOT NULL,
    dataset_id TEXT NOT NULL,
    name TEXT NOT NULL,
    content TEXT NOT NULL,
    metadata_json TEXT NOT NULL,
    enabled INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'completed',
    error_message TEXT NOT NULL DEFAULT '',
    content_hash TEXT NOT NULL DEFAULT '',
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY (platform_id, company_id, id),
    FOREIGN KEY(platform_id, company_id, dataset_id)
        REFERENCES knowledge_datasets(platform_id, company_id, id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS knowledge_index_jobs (
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    id TEXT NOT NULL,
    dataset_id TEXT NOT NULL,
    document_id TEXT NOT NULL,
    status TEXT NOT NULL,
    error_message TEXT NOT NULL DEFAULT '',
    payload_kind TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    started_at_ms INTEGER,
    attempt_count INTEGER NOT NULL DEFAULT 0,
    retry_after_ms INTEGER NOT NULL DEFAULT 0,
    lease_expires_at_ms INTEGER NOT NULL DEFAULT 0,
    finished_at_ms INTEGER,
    PRIMARY KEY (platform_id, company_id, id),
    FOREIGN KEY(platform_id, company_id, dataset_id)
        REFERENCES knowledge_datasets(platform_id, company_id, id) ON DELETE CASCADE,
    FOREIGN KEY(platform_id, company_id, document_id)
        REFERENCES knowledge_documents(platform_id, company_id, id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS knowledge_chunks (
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    id TEXT NOT NULL,
    dataset_id TEXT NOT NULL,
    document_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    metadata_json TEXT NOT NULL,
    chunk_type TEXT NOT NULL DEFAULT 'text',
    start_at INTEGER NOT NULL DEFAULT 0,
    end_at INTEGER NOT NULL DEFAULT 0,
    parent_chunk_id TEXT,
    prev_chunk_id TEXT,
    next_chunk_id TEXT,
    context_header TEXT NOT NULL DEFAULT '',
    index_content_hash TEXT NOT NULL DEFAULT '',
    embedding BLOB,
    enabled INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY (platform_id, company_id, id),
    FOREIGN KEY(platform_id, company_id, dataset_id)
        REFERENCES knowledge_datasets(platform_id, company_id, id) ON DELETE CASCADE,
    FOREIGN KEY(platform_id, company_id, document_id)
        REFERENCES knowledge_documents(platform_id, company_id, id) ON DELETE CASCADE
);

CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_chunks_fts USING fts5(
    platform_id UNINDEXED,
    company_id UNINDEXED,
    chunk_id UNINDEXED,
    dataset_id UNINDEXED,
    document_id UNINDEXED,
    title,
    content,
    tokenize = 'unicode61'
);

CREATE INDEX IF NOT EXISTS knowledge_documents_dataset_idx
ON knowledge_documents(platform_id, company_id, dataset_id);

CREATE INDEX IF NOT EXISTS knowledge_index_jobs_dataset_idx
ON knowledge_index_jobs(platform_id, company_id, dataset_id);

CREATE INDEX IF NOT EXISTS knowledge_index_jobs_document_idx
ON knowledge_index_jobs(platform_id, company_id, document_id);

CREATE INDEX IF NOT EXISTS knowledge_index_jobs_status_idx
ON knowledge_index_jobs(platform_id, company_id, status, created_at_ms);

CREATE INDEX IF NOT EXISTS knowledge_chunks_dataset_idx
ON knowledge_chunks(platform_id, company_id, dataset_id);

CREATE INDEX IF NOT EXISTS knowledge_chunks_document_idx
ON knowledge_chunks(platform_id, company_id, document_id);

CREATE TABLE IF NOT EXISTS knowledge_embedding_cache (
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    embedding BLOB NOT NULL,
    created_at_ms INTEGER NOT NULL,
    accessed_at_ms INTEGER NOT NULL,
    PRIMARY KEY (platform_id, company_id, content_hash)
);
"#;

const MAX_TOP_K: usize = 50;
const MAX_CANDIDATES: usize = 250;
const MIN_OVERLAP_CHARS: usize = 12;
const DEFAULT_CONTEXT_SEARCH_SPAN: usize = 400;
const KNOWLEDGE_EMBEDDING_REQUEST_INTERVAL: Duration = Duration::from_millis(1_200);
const INDEX_JOB_MAX_ATTEMPTS: i64 = 3;
const INDEX_JOB_LEASE_TIMEOUT_MS: u64 = 15 * 60 * 1_000;

/// 绑定全局数据库提供方与租户作用域的知识库服务。
///
/// 类型名为兼容既有调用者而保留；实际会依据全局配置分派到 SQLite 或 PostgreSQL。
#[derive(Clone)]
pub struct SqliteKnowledgeStore {
    database: GlobalDatabase,
    tenant: TenantScope,
    embedder: Arc<dyn EmbeddingProvider>,
    embedding_model: Option<String>,
    vector_weight: f32,
    keyword_weight: f32,
    embedding_cache_size: usize,
    embedding_request_interval: Duration,
    last_embedding_request_at: Arc<Mutex<Option<Instant>>>,
}

#[derive(Debug, Clone)]
struct PreparedKnowledgeChunk {
    ordinal: usize,
    title: String,
    content: String,
    index_content: String,
    metadata: Value,
    chunk_type: KnowledgeChunkType,
    start_at: usize,
    end_at: usize,
    parent_position: Option<usize>,
    context_header: String,
    indexed: bool,
    embedding: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
struct StoredKnowledgeDocument {
    id: String,
    dataset_id: String,
    body: KnowledgeDocumentCreateRequest,
}

#[derive(Debug, Clone)]
struct ReindexKnowledgeDocument {
    document: StoredKnowledgeDocument,
    dataset: KnowledgeDatasetDto,
}

#[derive(Debug, Clone)]
struct ClaimedIndexJob {
    id: String,
    dataset_id: String,
    document_id: String,
    payload_kind: String,
    payload_json: String,
    attempt_count: i64,
}

#[derive(Debug, Clone)]
struct KeywordRetrievalDataset {
    id: String,
    keyword_count: usize,
}

#[derive(Debug, Clone)]
struct RetrievalScope {
    keyword_datasets: Vec<KeywordRetrievalDataset>,
    vector_dataset_ids: Vec<String>,
    rerank_dataset_ids: Vec<String>,
}

impl SqliteKnowledgeStore {
    /// 使用无向量能力的默认嵌入器创建租户知识库。
    pub fn new(database: GlobalDatabase, tenant: TenantScope) -> Result<Self, ApiError> {
        Self::with_embedder(database, tenant, Arc::new(NoopEmbedding), None, 0.7, 0.3, 10_000)
    }

    /// 使用显式嵌入器、模型与融合权重创建知识库，并初始化所选数据库模式。
    pub fn with_embedder(
        database: GlobalDatabase,
        tenant: TenantScope,
        embedder: Arc<dyn EmbeddingProvider>,
        embedding_model: Option<String>,
        vector_weight: f32,
        keyword_weight: f32,
        embedding_cache_size: usize,
    ) -> Result<Self, ApiError> {
        match database.provider() {
            DatabaseProvider::Sqlite => database.initialize_schema(|connection| {
                reject_unscoped_knowledge_schema(connection)?;
                connection.execute_batch(KNOWLEDGE_SCHEMA)?;
                migrate_schema(connection, &tenant)?;
                Ok(())
            }),
            DatabaseProvider::Postgres => crate::global_database::run_sync_database(|| {
                PostgresKnowledgeStore::new(database.clone()).map(|_| ())
            }),
        }
        .map_err(|error| {
            ApiError::internal(format!("initialize knowledge database failed: {error}"))
        })?;

        Ok(Self {
            database,
            tenant,
            embedder,
            embedding_model,
            vector_weight,
            keyword_weight,
            embedding_cache_size,
            embedding_request_interval: KNOWLEDGE_EMBEDDING_REQUEST_INTERVAL,
            last_embedding_request_at: Arc::new(Mutex::new(None)),
        })
    }

    /// 返回当前全文、向量、混合检索与重排能力快照。
    pub fn status(&self) -> KnowledgeRuntimeStatus {
        let mut notes = BTreeMap::new();
        if self.embedder.dimensions() == 0 {
            notes.insert(
                "vector".to_string(),
                "embedding provider is configured as none".to_string(),
            );
        }
        notes.insert("rerank".to_string(), "local deterministic reranker is available".to_string());
        let vector_enabled = self.embedder.dimensions() > 0;
        if vector_enabled {
            notes.insert(
                "embedding_request_interval_ms".to_string(),
                self.embedding_request_interval.as_millis().to_string(),
            );
        }
        KnowledgeRuntimeStatus {
            full_text: true,
            vector: vector_enabled,
            hybrid: vector_enabled,
            rerank: true,
            notes,
        }
    }

    /// 列出当前租户可见的全部知识数据集。
    pub async fn list_datasets(&self) -> Result<Vec<KnowledgeDatasetDto>, ApiError> {
        let database = self.database.clone();
        spawn_store_task(database, self.tenant.clone(), list_datasets_blocking).await
    }

    /// 按标识读取当前租户中的数据集，不存在时返回领域错误。
    pub async fn get_dataset(&self, dataset_id: String) -> Result<KnowledgeDatasetDto, ApiError> {
        let database = self.database.clone();
        spawn_store_task(database, self.tenant.clone(), move |path, tenant| {
            get_dataset_blocking(path, &tenant, &dataset_id)
        })
        .await
    }

    /// 校验检索与嵌入配置后创建数据集。
    pub async fn create_dataset(
        &self,
        body: KnowledgeDatasetCreateRequest,
    ) -> Result<KnowledgeDatasetDto, ApiError> {
        validate_dataset_request(
            &body,
            self.embedder.dimensions() > 0,
            self.embedding_model.as_deref(),
        )?;
        let database = self.database.clone();
        spawn_store_task(database, self.tenant.clone(), move |path, tenant| {
            create_dataset_blocking(path, &tenant, body)
        })
        .await
    }

    /// 删除数据集及其受租户作用域约束的关联内容。
    pub async fn delete_dataset(
        &self,
        dataset_id: String,
    ) -> Result<KnowledgeDatasetDto, ApiError> {
        let database = self.database.clone();
        spawn_store_task(database, self.tenant.clone(), move |path, tenant| {
            delete_dataset_blocking(path, &tenant, &dataset_id)
        })
        .await
    }

    /// 按数据集列出全部文档。
    pub async fn list_documents(
        &self,
        dataset_id: String,
    ) -> Result<Vec<KnowledgeDocumentDto>, ApiError> {
        let database = self.database.clone();
        spawn_store_task(database, self.tenant.clone(), move |path, tenant| {
            list_documents_blocking(path, &tenant, &dataset_id)
        })
        .await
    }

    /// 按状态、搜索词和分页条件列出数据集文档。
    pub async fn list_documents_with_query(
        &self,
        dataset_id: String,
        query: KnowledgeDocumentListQuery,
    ) -> Result<Vec<KnowledgeDocumentDto>, ApiError> {
        let database = self.database.clone();
        spawn_store_task(database, self.tenant.clone(), move |path, tenant| {
            list_documents_with_query_blocking(path, &tenant, &dataset_id, &query)
        })
        .await
    }

    /// 列出文档的持久化分块；文档不存在时返回领域错误。
    pub async fn list_chunks(
        &self,
        document_id: String,
    ) -> Result<Vec<KnowledgeChunkDto>, ApiError> {
        let database = self.database.clone();
        spawn_store_task(database, self.tenant.clone(), move |path, tenant| {
            list_chunks_blocking(path, &tenant, &document_id)
        })
        .await
    }

    /// 校验文档、生成索引分块并以一次存储操作创建文档。
    pub async fn create_document(
        &self,
        dataset_id: String,
        body: KnowledgeDocumentCreateRequest,
    ) -> Result<KnowledgeDocumentDto, ApiError> {
        validate_document_request(&body)?;
        let dataset = self.get_dataset(dataset_id.clone()).await?;
        let prepared_chunks = self.prepare_index_chunks(&dataset, &body).await?;

        let database = self.database.clone();
        spawn_store_task(database, self.tenant.clone(), move |path, tenant| {
            create_document_blocking(path, &tenant, &dataset_id, body, prepared_chunks)
        })
        .await
    }

    /// 将编码后的文档导入请求写入异步索引队列。
    pub async fn enqueue_import(
        &self,
        dataset_id: String,
        body: KnowledgeDocumentImportRequest,
    ) -> Result<KnowledgeDocumentImportQueuedResponse, ApiError> {
        validate_import_enqueue_request(&body)?;
        let database = self.database.clone();
        spawn_store_task(database, self.tenant.clone(), move |path, tenant| {
            enqueue_import_blocking(path, &tenant, &dataset_id, body)
        })
        .await
    }

    /// 读取当前租户作用域内的索引任务状态。
    pub async fn get_index_job(&self, job_id: String) -> Result<KnowledgeIndexJobDto, ApiError> {
        let database = self.database.clone();
        spawn_store_task(database, self.tenant.clone(), move |path, tenant| {
            get_index_job_blocking(path, &tenant, &job_id)
        })
        .await
    }

    /// 领取并处理一个待执行索引任务；队列为空时返回 `None`。
    ///
    /// 处理或提交失败会进入有上限的重试流程，避免任务永久停留在运行态。
    pub async fn process_next_index_job(&self) -> Result<Option<KnowledgeIndexJobDto>, ApiError> {
        let database = self.database.clone();
        let tenant = self.tenant.clone();
        let Some(claimed) =
            spawn_store_task(database.clone(), tenant.clone(), claim_next_index_job_blocking)
                .await?
        else {
            return Ok(None);
        };

        let result = self.prepare_claimed_index_job(&claimed).await;
        match result {
            Ok((body, chunks)) => {
                match spawn_store_task(database.clone(), tenant.clone(), {
                    let claimed = claimed.clone();
                    move |path, tenant| {
                        complete_index_job_blocking(path, &tenant, claimed, body, chunks)
                    }
                })
                .await
                {
                    Ok(job) => Ok(Some(job)),
                    Err(error) => {
                        let message = error.to_string();
                        let job = spawn_store_task(database, tenant, move |path, tenant| {
                            retry_or_fail_index_job_blocking(
                                path,
                                &tenant,
                                &claimed.id,
                                &claimed.document_id,
                                claimed.attempt_count,
                                &message,
                            )
                        })
                        .await?;
                        Ok(Some(job))
                    }
                }
            }
            Err(error) => {
                let message = error.to_string();
                let job = spawn_store_task(database, tenant, move |path, tenant| {
                    retry_or_fail_index_job_blocking(
                        path,
                        &tenant,
                        &claimed.id,
                        &claimed.document_id,
                        claimed.attempt_count,
                        &message,
                    )
                })
                .await?;
                Ok(Some(job))
            }
        }
    }

    /// 重新生成文档分块；准备失败时同步把文档标记为失败。
    pub async fn reindex_document(
        &self,
        document_id: String,
    ) -> Result<KnowledgeDocumentDto, ApiError> {
        let database = self.database.clone();
        let tenant = self.tenant.clone();
        let load_document_id = document_id.clone();
        let reindex = spawn_store_task(database.clone(), tenant.clone(), move |path, tenant| {
            load_document_for_reindex_blocking(path, &tenant, &load_document_id)
        })
        .await?;
        let prepared_chunks = match self
            .prepare_index_chunks(&reindex.dataset, &reindex.document.body)
            .await
        {
            Ok(chunks) => chunks,
            Err(error) => {
                let message = error.to_string();
                let fail_document_id = reindex.document.id.clone();
                let _ = spawn_store_task(database.clone(), tenant.clone(), move |path, tenant| {
                    mark_document_failed_blocking(path, &tenant, &fail_document_id, &message)
                })
                .await;
                return Err(error);
            }
        };

        spawn_store_task(database, tenant, move |path, tenant| {
            reindex_document_blocking(path, &tenant, &reindex.document.id, prepared_chunks)
        })
        .await
    }

    /// 删除单个文档及其分块和索引记录。
    pub async fn delete_document(
        &self,
        document_id: String,
    ) -> Result<KnowledgeDocumentDto, ApiError> {
        let database = self.database.clone();
        spawn_store_task(database, self.tenant.clone(), move |path, tenant| {
            delete_document_blocking(path, &tenant, &document_id)
        })
        .await
    }

    /// 在状态机约束内更新文档元数据、启用状态或归档状态。
    pub async fn update_document(
        &self,
        document_id: String,
        body: KnowledgeDocumentPatchRequest,
    ) -> Result<KnowledgeDocumentDto, ApiError> {
        let database = self.database.clone();
        spawn_store_task(database, self.tenant.clone(), move |path, tenant| {
            update_document_blocking(path, &tenant, &document_id, body)
        })
        .await
    }

    /// 以同一批处理动作更新多个文档，并保持事务原子性。
    pub async fn batch_update_documents(
        &self,
        body: KnowledgeDocumentsBatchActionRequest,
    ) -> Result<Vec<KnowledgeDocumentDto>, ApiError> {
        let database = self.database.clone();
        spawn_store_task(database, self.tenant.clone(), move |path, tenant| {
            batch_update_documents_blocking(path, &tenant, body)
        })
        .await
    }

    /// 在请求指定的数据集作用域内执行全文、向量或混合检索。
    ///
    /// 向量或混合数据集没有可用嵌入器时明确拒绝，不静默降级为全文检索。
    pub async fn retrieve(
        &self,
        request: KnowledgeRetrieveRequest,
    ) -> Result<KnowledgeRetrieveResponse, ApiError> {
        validate_retrieve_request(&request)?;
        let scope = {
            let database = self.database.clone();
            let dataset_ids = request.dataset_ids.clone();
            spawn_store_task(database, self.tenant.clone(), move |path, tenant| {
                resolve_retrieval_scope_blocking(path, &tenant, &dataset_ids)
            })
            .await?
        };
        let needs_vector = !scope.vector_dataset_ids.is_empty();
        if needs_vector && self.embedder.dimensions() == 0 {
            return Err(ApiError::not_implemented(
                "knowledge retrieval requires an embedding provider for vector or hybrid datasets",
            ));
        }
        let query_embedding =
            if needs_vector { self.get_or_compute_embedding(&request.query).await? } else { None };
        let database = self.database.clone();
        let vector_weight = self.vector_weight;
        let keyword_weight = self.keyword_weight;
        spawn_store_task(database, self.tenant.clone(), move |path, tenant| {
            retrieve_blocking(
                path,
                &tenant,
                request,
                scope,
                query_embedding,
                vector_weight,
                keyword_weight,
            )
        })
        .await
    }

    async fn get_or_compute_embedding(&self, text: &str) -> Result<Option<Vec<f32>>, ApiError> {
        if self.embedder.dimensions() == 0 {
            return Ok(None);
        }

        let hash = content_hash(text);
        let cached = {
            let database = self.database.clone();
            let hash = hash.clone();
            spawn_store_task(database, self.tenant.clone(), move |path, tenant| {
                read_embedding_cache_blocking(path, &tenant, &hash)
            })
            .await?
        };
        if cached.is_some() {
            return Ok(cached);
        }

        self.wait_for_embedding_request_slot().await;
        let embedding = self.embedder.embed_one(text).await.map_err(|error| {
            ApiError::bad_request(format!("knowledge embedding failed: {error}"))
        })?;
        if embedding.is_empty() {
            return Ok(None);
        }

        let database = self.database.clone();
        let bytes = vector::vec_to_bytes(&embedding);
        let cache_size = self.embedding_cache_size;
        spawn_store_task(database, self.tenant.clone(), move |path, tenant| {
            write_embedding_cache_blocking(path, &tenant, &hash, bytes, cache_size)
        })
        .await?;
        Ok(Some(embedding))
    }

    async fn wait_for_embedding_request_slot(&self) {
        if self.embedding_request_interval.is_zero() {
            return;
        }

        let mut last_request_at = self.last_embedding_request_at.lock().await;
        if let Some(previous) = *last_request_at {
            let elapsed = previous.elapsed();
            if elapsed < self.embedding_request_interval {
                tokio::time::sleep(self.embedding_request_interval - elapsed).await;
            }
        }
        *last_request_at = Some(Instant::now());
    }

    async fn prepare_claimed_index_job(
        &self,
        job: &ClaimedIndexJob,
    ) -> Result<(KnowledgeDocumentCreateRequest, Vec<PreparedKnowledgeChunk>), ApiError> {
        if job.payload_kind != "import" {
            return Err(ApiError::bad_request(format!(
                "unsupported knowledge index payload kind: {}",
                job.payload_kind
            )));
        }
        let import: KnowledgeDocumentImportRequest = serde_json::from_str(&job.payload_json)
            .map_err(|error| {
                ApiError::bad_request(format!("knowledge index job payload is invalid: {error}"))
            })?;
        let parsed = super::import::parse_document_import(import)?;
        let dataset_id = job.dataset_id.clone();
        let database = self.database.clone();
        let dataset = spawn_store_task(database, self.tenant.clone(), move |path, tenant| {
            get_dataset_blocking(path, &tenant, &dataset_id)
        })
        .await?;
        let chunks = self.prepare_index_chunks(&dataset, &parsed.create).await?;
        Ok((parsed.create, chunks))
    }

    async fn prepare_index_chunks(
        &self,
        dataset: &KnowledgeDatasetDto,
        body: &KnowledgeDocumentCreateRequest,
    ) -> Result<Vec<PreparedKnowledgeChunk>, ApiError> {
        let chunks = prepare_chunks_for_dataset(dataset, body)?;
        if chunks.is_empty() {
            return Err(ApiError::bad_request(
                "document has no indexable text chunks; check that the file is not empty and PDF text is extractable",
            ));
        }
        let needs_embedding = dataset_needs_embedding(dataset);
        if needs_embedding && self.embedder.dimensions() == 0 {
            return Err(ApiError::not_implemented(
                "knowledge dataset requires an embedding provider",
            ));
        }

        let mut prepared_chunks = Vec::with_capacity(chunks.len());
        for mut chunk in chunks {
            chunk.embedding = if needs_embedding && chunk.indexed {
                self.get_or_compute_embedding(&chunk.index_content)
                    .await?
                    .map(|embedding| vector::vec_to_bytes(&embedding))
            } else {
                None
            };
            prepared_chunks.push(chunk);
        }
        Ok(prepared_chunks)
    }
}

#[async_trait::async_trait]
impl WorkflowKnowledgeProvider for SqliteKnowledgeStore {
    async fn retrieve(
        &self,
        request: WorkflowKnowledgeRequest,
    ) -> Result<Vec<WorkflowKnowledgeChunk>, String> {
        let response = SqliteKnowledgeStore::retrieve(
            self,
            KnowledgeRetrieveRequest {
                query: request.query,
                dataset_ids: request.dataset_ids,
                top_k: request.top_k,
                score_threshold: request.score_threshold,
                metadata_filter: request.metadata_filter,
                include_context: false,
            },
        )
        .await
        .map_err(|error| error.to_string())?;

        Ok(response
            .chunks
            .into_iter()
            .map(|chunk| WorkflowKnowledgeChunk {
                content: chunk.content,
                title: chunk.title,
                metadata: chunk.metadata,
                score: chunk.score,
            })
            .collect())
    }
}

async fn spawn_store_task<T: Send + 'static>(
    database: GlobalDatabase,
    tenant: TenantScope,
    task: impl FnOnce(GlobalDatabase, TenantScope) -> Result<T, ApiError> + Send + 'static,
) -> Result<T, ApiError> {
    tokio::task::spawn_blocking(move || task(database, tenant))
        .await
        .map_err(|error| ApiError::internal(format!("knowledge store task failed: {error}")))?
}

fn list_datasets_blocking(
    database: GlobalDatabase,
    tenant: TenantScope,
) -> Result<Vec<KnowledgeDatasetDto>, ApiError> {
    if database.provider() == DatabaseProvider::Postgres {
        return PostgresKnowledgeStore::new(database)
            .and_then(|store| store.list_datasets(&tenant))
            .map_err(database_error);
    }
    let conn = database.connect().map_err(database_error)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, description, chunking_mode, indexing_mode, retrieval_mode, \
             keyword_count, top_k, score_threshold_enabled, score_threshold, rerank_enabled, \
             embedding_model, rerank_model, created_at_ms, updated_at_ms FROM knowledge_datasets \
             WHERE platform_id = ?1 AND company_id = ?2 \
             ORDER BY updated_at_ms DESC, name ASC",
        )
        .map_err(sql_error)?;
    let rows = stmt
        .query_map(params![tenant.platform_id().as_ref(), tenant.company_id().as_ref()], |row| {
            dataset_from_row(&conn, &tenant, row)
        })
        .map_err(sql_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)
}

fn get_dataset_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    dataset_id: &str,
) -> Result<KnowledgeDatasetDto, ApiError> {
    validate_id(dataset_id, "dataset_id")?;
    if database.provider() == DatabaseProvider::Postgres {
        return PostgresKnowledgeStore::new(database)
            .and_then(|store| store.get_dataset(tenant, dataset_id))
            .map_err(database_error)?
            .ok_or_else(|| {
                ApiError::not_found(format!("knowledge dataset not found: {dataset_id}"))
            });
    }
    let conn = database.connect().map_err(database_error)?;
    query_dataset(&conn, tenant, dataset_id)?
        .ok_or_else(|| ApiError::not_found(format!("knowledge dataset not found: {dataset_id}")))
}

fn create_dataset_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    body: KnowledgeDatasetCreateRequest,
) -> Result<KnowledgeDatasetDto, ApiError> {
    if database.provider() == DatabaseProvider::Postgres {
        return PostgresKnowledgeStore::new(database)
            .and_then(|store| store.create_dataset(tenant, body))
            .map_err(database_error);
    }
    let conn = database.connect().map_err(database_error)?;
    let id = Uuid::new_v4().to_string();
    let now = now_ms();
    conn.execute(
        "INSERT INTO knowledge_datasets \
         (platform_id, company_id, id, name, description, chunking_mode, indexing_mode, retrieval_mode, keyword_count, \
          top_k, score_threshold_enabled, score_threshold, rerank_enabled, embedding_model, \
          rerank_model, created_at_ms, updated_at_ms) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        params![
            tenant.platform_id().as_ref(),
            tenant.company_id().as_ref(),
            id,
            body.name.trim(),
            body.description.trim(),
            chunking_mode_as_str(&body.chunking_mode),
            indexing_mode_as_str(&body.indexing_mode),
            retrieval_mode_as_str(&body.retrieval_mode),
            i64::try_from(body.keyword_count).unwrap_or(i64::MAX),
            i64::try_from(body.top_k.clamp(1, MAX_TOP_K)).unwrap_or(i64::MAX),
            bool_to_i64(body.score_threshold_enabled),
            body.score_threshold,
            bool_to_i64(body.rerank_enabled),
            body.embedding_model.as_deref(),
            body.rerank_model.as_deref(),
            now,
            now,
        ],
    )
    .map_err(sql_error)?;

    query_dataset(&conn, tenant, &id)?
        .ok_or_else(|| ApiError::internal("knowledge dataset was not saved"))
}

fn delete_dataset_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    dataset_id: &str,
) -> Result<KnowledgeDatasetDto, ApiError> {
    validate_id(dataset_id, "dataset_id")?;
    if database.provider() == DatabaseProvider::Postgres {
        return PostgresKnowledgeStore::new(database)
            .and_then(|store| store.delete_dataset(tenant, dataset_id))
            .map_err(database_error)?
            .ok_or_else(|| {
                ApiError::not_found(format!("knowledge dataset not found: {dataset_id}"))
            });
    }
    let conn = database.connect().map_err(database_error)?;
    let dataset = query_dataset(&conn, tenant, dataset_id)?
        .ok_or_else(|| ApiError::not_found(format!("knowledge dataset not found: {dataset_id}")))?;
    let scope = params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), dataset_id];
    conn.execute(
        "DELETE FROM knowledge_chunks_fts \
         WHERE platform_id = ?1 AND company_id = ?2 AND dataset_id = ?3",
        scope,
    )
    .map_err(sql_error)?;
    conn.execute(
        "DELETE FROM knowledge_chunks \
         WHERE platform_id = ?1 AND company_id = ?2 AND dataset_id = ?3",
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), dataset_id],
    )
    .map_err(sql_error)?;
    conn.execute(
        "DELETE FROM knowledge_documents \
         WHERE platform_id = ?1 AND company_id = ?2 AND dataset_id = ?3",
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), dataset_id],
    )
    .map_err(sql_error)?;
    conn.execute(
        "DELETE FROM knowledge_datasets \
         WHERE platform_id = ?1 AND company_id = ?2 AND id = ?3",
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), dataset_id],
    )
    .map_err(sql_error)?;
    Ok(dataset)
}

fn list_documents_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    dataset_id: &str,
) -> Result<Vec<KnowledgeDocumentDto>, ApiError> {
    list_documents_with_query_blocking(
        database,
        tenant,
        dataset_id,
        &KnowledgeDocumentListQuery::default(),
    )
}

fn list_documents_with_query_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    dataset_id: &str,
    query: &KnowledgeDocumentListQuery,
) -> Result<Vec<KnowledgeDocumentDto>, ApiError> {
    validate_id(dataset_id, "dataset_id")?;
    if database.provider() == DatabaseProvider::Postgres {
        return PostgresKnowledgeStore::new(database)
            .and_then(|store| store.list_documents(tenant, dataset_id, query))
            .map_err(database_error);
    }
    let conn = database.connect().map_err(database_error)?;
    ensure_dataset_exists(&conn, tenant, dataset_id)?;
    if let Some(status) = query.status.as_ref() {
        let status_value = document_status_as_str(status);
        let mut stmt = conn
            .prepare(
                "SELECT id, dataset_id, name, metadata_json, enabled, created_at_ms, updated_at_ms, \
                 status, error_message, content_hash, char_count, recall_count, archived_at_ms \
                 FROM knowledge_documents WHERE platform_id = ?1 AND company_id = ?2 \
                 AND dataset_id = ?3 AND status = ?4 \
                 ORDER BY updated_at_ms DESC, name ASC",
            )
            .map_err(sql_error)?;
        let rows = stmt
            .query_map(
                params![
                    tenant.platform_id().as_ref(),
                    tenant.company_id().as_ref(),
                    dataset_id,
                    status_value,
                ],
                |row| document_from_row(&conn, tenant, row),
            )
            .map_err(sql_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)
    } else {
        let mut stmt = conn
            .prepare(
                "SELECT id, dataset_id, name, metadata_json, enabled, created_at_ms, updated_at_ms, \
                 status, error_message, content_hash, char_count, recall_count, archived_at_ms \
                 FROM knowledge_documents WHERE platform_id = ?1 AND company_id = ?2 \
                 AND dataset_id = ?3 ORDER BY updated_at_ms DESC, name ASC",
            )
            .map_err(sql_error)?;
        let rows = stmt
            .query_map(
                params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), dataset_id],
                |row| document_from_row(&conn, tenant, row),
            )
            .map_err(sql_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)
    }
}

fn list_chunks_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    document_id: &str,
) -> Result<Vec<KnowledgeChunkDto>, ApiError> {
    validate_id(document_id, "document_id")?;
    if database.provider() == DatabaseProvider::Postgres {
        return PostgresKnowledgeStore::new(database)
            .and_then(|store| store.list_chunks(tenant, document_id))
            .map_err(database_error)?
            .ok_or_else(|| {
                ApiError::not_found(format!("knowledge document not found: {document_id}"))
            });
    }
    let conn = database.connect().map_err(database_error)?;
    if query_document(&conn, tenant, document_id)?.is_none() {
        return Err(ApiError::not_found(format!("knowledge document not found: {document_id}")));
    }
    let mut stmt = conn
        .prepare(
            "SELECT id, dataset_id, document_id, title, content, metadata_json, ordinal, chunk_type, \
             start_at, end_at, parent_chunk_id, prev_chunk_id, next_chunk_id, context_header \
             FROM knowledge_chunks WHERE platform_id = ?1 AND company_id = ?2 \
             AND document_id = ?3 ORDER BY ordinal ASC, id ASC",
        )
        .map_err(sql_error)?;
    let rows = stmt
        .query_map(
            params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), document_id],
            |row| chunk_from_row(row, None, KnowledgeMatchType::FullText),
        )
        .map_err(sql_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)
}

fn prepare_chunks_for_dataset(
    dataset: &KnowledgeDatasetDto,
    body: &KnowledgeDocumentCreateRequest,
) -> Result<Vec<PreparedKnowledgeChunk>, ApiError> {
    match dataset.chunking_mode {
        KnowledgeChunkingMode::General => Ok(chunk_text(&body.content)
            .into_iter()
            .map(|chunk| PreparedKnowledgeChunk {
                ordinal: chunk.ordinal,
                title: body.name.trim().to_string(),
                index_content: chunk.embedding_content(),
                content: chunk.content,
                metadata: json!({ "chunking_mode": "general" }),
                chunk_type: KnowledgeChunkType::Text,
                start_at: chunk.start_at,
                end_at: chunk.end_at,
                parent_position: None,
                context_header: chunk.context_header,
                indexed: true,
                embedding: None,
            })
            .collect()),
        KnowledgeChunkingMode::ParentChild => Ok(prepare_parent_child_chunks(body)),
        KnowledgeChunkingMode::Qa => prepare_qa_chunks(body),
    }
}

fn prepare_parent_child_chunks(
    body: &KnowledgeDocumentCreateRequest,
) -> Vec<PreparedKnowledgeChunk> {
    let parents = chunk_text_with_limits(&body.content, 1600, 120);
    let mut chunks = Vec::new();
    for parent in parents {
        let parent_position = chunks.len();
        chunks.push(PreparedKnowledgeChunk {
            ordinal: parent_position,
            title: format!("{} · {}", body.name.trim(), parent.ordinal + 1),
            content: parent.content.clone(),
            index_content: parent.embedding_content(),
            metadata: json!({
                "chunking_mode": "parent_child",
                "parent_index": parent.ordinal,
            }),
            chunk_type: KnowledgeChunkType::ParentText,
            start_at: parent.start_at,
            end_at: parent.end_at,
            parent_position: None,
            context_header: parent.context_header.clone(),
            indexed: false,
            embedding: None,
        });
        let children = chunk_text_with_limits(&parent.content, 360, 40);
        for child in children {
            let context_header = if child.context_header.is_empty() {
                parent.context_header.clone()
            } else {
                child.context_header.clone()
            };
            let index_content = if context_header.is_empty() {
                child.content.trim().to_string()
            } else {
                format!("{}\n\n{}", context_header, child.content.trim())
            };
            chunks.push(PreparedKnowledgeChunk {
                ordinal: chunks.len(),
                title: format!(
                    "{} · {}.{}",
                    body.name.trim(),
                    parent.ordinal + 1,
                    child.ordinal + 1
                ),
                content: child.content.clone(),
                index_content,
                metadata: json!({
                    "chunking_mode": "parent_child",
                    "parent_index": parent.ordinal,
                    "child_index": child.ordinal,
                    "child_content": child.content,
                }),
                chunk_type: KnowledgeChunkType::Text,
                start_at: parent.start_at + child.start_at,
                end_at: parent.start_at + child.end_at,
                parent_position: Some(parent_position),
                context_header,
                indexed: true,
                embedding: None,
            });
        }
    }
    chunks
}

fn prepare_qa_chunks(
    body: &KnowledgeDocumentCreateRequest,
) -> Result<Vec<PreparedKnowledgeChunk>, ApiError> {
    let pairs = parse_qa_pairs(&body.content);
    if pairs.is_empty() {
        return Err(ApiError::bad_request(
            "Q&A knowledge document must contain question and answer pairs",
        ));
    }
    Ok(pairs
        .into_iter()
        .enumerate()
        .map(|(index, (question, answer))| PreparedKnowledgeChunk {
            ordinal: index,
            title: question.clone(),
            index_content: question.clone(),
            content: answer.clone(),
            metadata: json!({
                "chunking_mode": "qa",
                "question": question,
                "answer": answer,
            }),
            chunk_type: KnowledgeChunkType::Qa,
            start_at: 0,
            end_at: 0,
            parent_position: None,
            context_header: String::new(),
            indexed: true,
            embedding: None,
        })
        .collect())
}

fn parse_qa_pairs(content: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let mut question = String::new();
    let mut answer = String::new();
    let mut answer_started = false;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(value) = strip_question_prefix(line) {
            push_qa_pair(&mut pairs, &mut question, &mut answer);
            question = value.to_string();
            answer_started = false;
            continue;
        }
        if let Some(value) = strip_answer_prefix(line) {
            if !question.is_empty() {
                push_joined_line(&mut answer, value);
                answer_started = true;
            }
            continue;
        }
        if !question.is_empty() {
            push_joined_line(&mut answer, line);
            answer_started = true;
        }
    }
    if answer_started {
        push_qa_pair(&mut pairs, &mut question, &mut answer);
    }

    if pairs.is_empty() { parse_paragraph_qa_pairs(content) } else { pairs }
}

fn parse_paragraph_qa_pairs(content: &str) -> Vec<(String, String)> {
    content
        .split("\n\n")
        .filter_map(|block| {
            let lines =
                block.lines().map(str::trim).filter(|line| !line.is_empty()).collect::<Vec<_>>();
            if lines.len() < 2 {
                return None;
            }
            let question = lines[0].to_string();
            let answer = lines[1..].join("\n");
            (!question.is_empty() && !answer.trim().is_empty()).then_some((question, answer))
        })
        .collect()
}

fn strip_question_prefix(line: &str) -> Option<&str> {
    strip_prefixed_line(
        line,
        &[
            "Q:",
            "Q：",
            "q:",
            "q：",
            "Question:",
            "Question：",
            "question:",
            "question：",
            "问题:",
            "问题：",
            "问:",
            "问：",
        ],
    )
}

fn strip_answer_prefix(line: &str) -> Option<&str> {
    strip_prefixed_line(
        line,
        &[
            "A:",
            "A：",
            "a:",
            "a：",
            "Answer:",
            "Answer：",
            "answer:",
            "answer：",
            "答案:",
            "答案：",
            "答:",
            "答：",
        ],
    )
}

fn strip_prefixed_line<'a>(line: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    for prefix in prefixes {
        if let Some(value) = line.strip_prefix(prefix) {
            return Some(value.trim());
        }
    }
    None
}

fn push_qa_pair(pairs: &mut Vec<(String, String)>, question: &mut String, answer: &mut String) {
    let question_value = question.trim();
    let answer_value = answer.trim();
    if !question_value.is_empty() && !answer_value.is_empty() {
        pairs.push((question_value.to_string(), answer_value.to_string()));
    }
    question.clear();
    answer.clear();
}

fn push_joined_line(target: &mut String, line: &str) {
    if !target.is_empty() {
        target.push('\n');
    }
    target.push_str(line);
}

fn create_document_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    dataset_id: &str,
    body: KnowledgeDocumentCreateRequest,
    chunks: Vec<PreparedKnowledgeChunk>,
) -> Result<KnowledgeDocumentDto, ApiError> {
    validate_id(dataset_id, "dataset_id")?;
    if database.provider() == DatabaseProvider::Postgres {
        return PostgresKnowledgeStore::new(database)
            .and_then(|store| store.create_document_with_chunks(tenant, dataset_id, body, chunks))
            .map_err(database_error);
    }
    let mut conn = database.connect().map_err(database_error)?;
    ensure_dataset_exists(&conn, tenant, dataset_id)?;

    let document_id = Uuid::new_v4().to_string();
    let now = now_ms();
    let document_hash = content_hash(&body.content);
    let char_count = unicode_char_count(&body.content);
    let metadata_json = serde_json::to_string(&metadata_or_object(body.metadata.clone()))
        .map_err(|error| ApiError::bad_request(format!("document metadata is invalid: {error}")))?;
    let tx = conn.transaction().map_err(sql_error)?;
    tx.execute(
        "INSERT INTO knowledge_documents \
         (platform_id, company_id, id, dataset_id, name, content, metadata_json, enabled, status, error_message, \
          content_hash, char_count, recall_count, archived_at_ms, created_at_ms, updated_at_ms) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, '', ?10, ?11, 0, NULL, ?12, ?13)",
        params![
            tenant.platform_id().as_ref(),
            tenant.company_id().as_ref(),
            document_id,
            dataset_id,
            body.name.trim(),
            body.content,
            metadata_json,
            bool_to_i64(body.enabled),
            document_status_as_str(&KnowledgeDocumentStatus::Processing),
            document_hash,
            i64::try_from(char_count).unwrap_or(i64::MAX),
            now,
            now,
        ],
    )
    .map_err(sql_error)?;
    replace_document_chunks_tx(
        &tx,
        tenant,
        dataset_id,
        &document_id,
        body.name.trim(),
        &body.metadata,
        body.enabled,
        chunks,
        now,
    )?;
    tx.execute(
        "UPDATE knowledge_documents \
         SET status = ?1, error_message = '', archived_at_ms = NULL, updated_at_ms = ?2 \
         WHERE platform_id = ?3 AND company_id = ?4 AND id = ?5",
        params![
            final_document_status(body.enabled),
            now,
            tenant.platform_id().as_ref(),
            tenant.company_id().as_ref(),
            document_id,
        ],
    )
    .map_err(sql_error)?;
    tx.commit().map_err(sql_error)?;

    query_document(&conn, tenant, &document_id)?
        .ok_or_else(|| ApiError::internal("knowledge document was not saved"))
}

fn enqueue_import_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    dataset_id: &str,
    body: KnowledgeDocumentImportRequest,
) -> Result<KnowledgeDocumentImportQueuedResponse, ApiError> {
    validate_id(dataset_id, "dataset_id")?;
    if database.provider() == DatabaseProvider::Postgres {
        return PostgresKnowledgeStore::new(database)
            .and_then(|store| store.enqueue_import(tenant, dataset_id, body))
            .map_err(database_error);
    }
    let mut conn = database.connect().map_err(database_error)?;
    ensure_dataset_exists(&conn, tenant, dataset_id)?;

    let filename = clean_import_filename(&body.filename)?;
    let mime_type = normalize_import_mime(body.mime_type.as_deref())
        .or_else(|| mime_guess::from_path(&filename).first_raw().map(str::to_string));
    let source_bytes = import_source_bytes(&body.content_base64)?;
    let metadata_json = serde_json::to_string(&metadata_or_object(body.metadata.clone()))
        .map_err(|error| ApiError::bad_request(format!("document metadata is invalid: {error}")))?;
    let payload_json = serde_json::to_string(&body).map_err(|error| {
        ApiError::bad_request(format!("knowledge import payload is invalid: {error}"))
    })?;
    let document_id = Uuid::new_v4().to_string();
    let job_id = Uuid::new_v4().to_string();
    let now = now_ms();

    let tx = conn.transaction().map_err(sql_error)?;
    tx.execute(
        "INSERT INTO knowledge_documents \
         (platform_id, company_id, id, dataset_id, name, content, metadata_json, enabled, status, error_message, \
          content_hash, char_count, recall_count, archived_at_ms, created_at_ms, updated_at_ms) \
         VALUES (?1, ?2, ?3, ?4, ?5, '', ?6, ?7, ?8, '', '', 0, 0, NULL, ?9, ?10)",
        params![
            tenant.platform_id().as_ref(),
            tenant.company_id().as_ref(),
            document_id,
            dataset_id,
            filename,
            metadata_json,
            bool_to_i64(body.enabled),
            document_status_as_str(&KnowledgeDocumentStatus::Pending),
            now,
            now,
        ],
    )
    .map_err(sql_error)?;
    tx.execute(
        "INSERT INTO knowledge_index_jobs \
         (platform_id, company_id, id, dataset_id, document_id, status, error_message, payload_kind, payload_json, \
          created_at_ms, updated_at_ms, started_at_ms, finished_at_ms) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, '', 'import', ?7, ?8, ?9, NULL, NULL)",
        params![
            tenant.platform_id().as_ref(),
            tenant.company_id().as_ref(),
            job_id,
            dataset_id,
            document_id,
            index_job_status_as_str(&KnowledgeIndexJobStatus::Queued),
            payload_json,
            now,
            now,
        ],
    )
    .map_err(sql_error)?;
    tx.commit().map_err(sql_error)?;

    let document = query_document(&conn, tenant, &document_id)?
        .ok_or_else(|| ApiError::internal("queued knowledge document was not saved"))?;
    let job = query_index_job(&conn, tenant, &job_id)?
        .ok_or_else(|| ApiError::internal("knowledge index job was not saved"))?;
    Ok(KnowledgeDocumentImportQueuedResponse {
        document,
        job,
        filename,
        mime_type,
        source_bytes,
        warnings: Vec::new(),
    })
}

fn get_index_job_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    job_id: &str,
) -> Result<KnowledgeIndexJobDto, ApiError> {
    validate_id(job_id, "job_id")?;
    if database.provider() == DatabaseProvider::Postgres {
        return PostgresKnowledgeStore::new(database)
            .and_then(|store| store.get_index_job(tenant, job_id))
            .map_err(database_error)?
            .ok_or_else(|| {
                ApiError::not_found(format!("knowledge index job not found: {job_id}"))
            });
    }
    let conn = database.connect().map_err(database_error)?;
    query_index_job(&conn, tenant, job_id)?
        .ok_or_else(|| ApiError::not_found(format!("knowledge index job not found: {job_id}")))
}

fn claim_next_index_job_blocking(
    database: GlobalDatabase,
    tenant: TenantScope,
) -> Result<Option<ClaimedIndexJob>, ApiError> {
    if database.provider() == DatabaseProvider::Postgres {
        return PostgresKnowledgeStore::new(database)
            .and_then(|store| store.claim_next_index_job(&tenant))
            .map_err(database_error);
    }
    let mut conn = database.connect().map_err(database_error)?;
    let now = now_ms();
    let stale_started_before = now.saturating_sub(INDEX_JOB_LEASE_TIMEOUT_MS);
    let lease_expires_at_ms = now.saturating_add(INDEX_JOB_LEASE_TIMEOUT_MS);
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(sql_error)?;
    fail_expired_index_jobs_tx(&tx, &tenant, now, stale_started_before)?;
    let claimed = tx
        .query_row(
            "SELECT id, dataset_id, document_id, payload_kind, payload_json, attempt_count + 1 \
             FROM knowledge_index_jobs \
             WHERE platform_id = ?1 AND company_id = ?2 AND ( \
                (status = ?3 AND retry_after_ms <= ?4) \
                OR (status = ?5 \
                    AND ((lease_expires_at_ms > 0 AND lease_expires_at_ms <= ?4) \
                         OR (lease_expires_at_ms = 0 AND started_at_ms IS NOT NULL \
                             AND started_at_ms <= ?6)) \
                    AND attempt_count < ?7)) \
             ORDER BY CASE WHEN status = ?5 THEN 0 ELSE 1 END, retry_after_ms ASC, \
                      created_at_ms ASC, id ASC LIMIT 1",
            params![
                tenant.platform_id().as_ref(),
                tenant.company_id().as_ref(),
                index_job_status_as_str(&KnowledgeIndexJobStatus::Queued),
                now,
                index_job_status_as_str(&KnowledgeIndexJobStatus::Processing),
                stale_started_before,
                INDEX_JOB_MAX_ATTEMPTS,
            ],
            |row| {
                Ok(ClaimedIndexJob {
                    id: row.get(0)?,
                    dataset_id: row.get(1)?,
                    document_id: row.get(2)?,
                    payload_kind: row.get(3)?,
                    payload_json: row.get(4)?,
                    attempt_count: row.get(5)?,
                })
            },
        )
        .optional()
        .map_err(sql_error)?;
    let Some(claimed) = claimed else {
        tx.commit().map_err(sql_error)?;
        return Ok(None);
    };
    let updated = tx
        .execute(
            "UPDATE knowledge_index_jobs \
             SET status = ?1, error_message = '', updated_at_ms = ?2, started_at_ms = ?3, \
                 finished_at_ms = NULL, attempt_count = attempt_count + 1, retry_after_ms = 0, \
                 lease_expires_at_ms = ?4 \
             WHERE platform_id = ?5 AND company_id = ?6 AND id = ?7 AND status IN (?8, ?9)",
            params![
                index_job_status_as_str(&KnowledgeIndexJobStatus::Processing),
                now,
                now,
                lease_expires_at_ms,
                tenant.platform_id().as_ref(),
                tenant.company_id().as_ref(),
                claimed.id,
                index_job_status_as_str(&KnowledgeIndexJobStatus::Queued),
                index_job_status_as_str(&KnowledgeIndexJobStatus::Processing),
            ],
        )
        .map_err(sql_error)?;
    if updated == 0 {
        tx.commit().map_err(sql_error)?;
        return Ok(None);
    }
    tx.execute(
        "UPDATE knowledge_documents SET status = ?1, error_message = '', updated_at_ms = ?2 \
         WHERE platform_id = ?3 AND company_id = ?4 AND id = ?5",
        params![
            document_status_as_str(&KnowledgeDocumentStatus::Processing),
            now,
            tenant.platform_id().as_ref(),
            tenant.company_id().as_ref(),
            claimed.document_id,
        ],
    )
    .map_err(sql_error)?;
    tx.commit().map_err(sql_error)?;
    Ok(Some(claimed))
}

fn fail_expired_index_jobs_tx(
    tx: &Transaction<'_>,
    tenant: &TenantScope,
    now: u64,
    stale_started_before: u64,
) -> Result<(), ApiError> {
    let expired = {
        let mut stmt = tx
            .prepare(
                "SELECT id, document_id FROM knowledge_index_jobs \
                 WHERE platform_id = ?1 AND company_id = ?2 AND status = ?3 \
                   AND ((lease_expires_at_ms > 0 AND lease_expires_at_ms <= ?4) \
                        OR (lease_expires_at_ms = 0 AND started_at_ms IS NOT NULL \
                            AND started_at_ms <= ?5)) \
                   AND attempt_count >= ?6",
            )
            .map_err(sql_error)?;
        let rows = stmt
            .query_map(
                params![
                    tenant.platform_id().as_ref(),
                    tenant.company_id().as_ref(),
                    index_job_status_as_str(&KnowledgeIndexJobStatus::Processing),
                    now,
                    stale_started_before,
                    INDEX_JOB_MAX_ATTEMPTS,
                ],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .map_err(sql_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)?
    };

    for (job_id, document_id) in expired {
        mark_index_job_failed_tx(
            tx,
            tenant,
            &job_id,
            &document_id,
            &index_job_timed_out_message(),
            now,
        )?;
    }
    Ok(())
}

fn complete_index_job_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    job: ClaimedIndexJob,
    body: KnowledgeDocumentCreateRequest,
    chunks: Vec<PreparedKnowledgeChunk>,
) -> Result<KnowledgeIndexJobDto, ApiError> {
    validate_id(&job.id, "job_id")?;
    validate_id(&job.document_id, "document_id")?;
    validate_id(&job.dataset_id, "dataset_id")?;
    if database.provider() == DatabaseProvider::Postgres {
        return PostgresKnowledgeStore::new(database)
            .and_then(|store| store.complete_index_job(tenant, job, body, chunks))
            .map_err(database_error);
    }
    let mut conn = database.connect().map_err(database_error)?;
    let now = now_ms();
    let metadata_json = serde_json::to_string(&metadata_or_object(body.metadata.clone()))
        .map_err(|error| ApiError::bad_request(format!("document metadata is invalid: {error}")))?;
    let document_hash = content_hash(&body.content);
    let char_count = unicode_char_count(&body.content);
    let result = (|| {
        let tx = conn.transaction().map_err(sql_error)?;
        replace_document_chunks_tx(
            &tx,
            tenant,
            &job.dataset_id,
            &job.document_id,
            body.name.trim(),
            &body.metadata,
            body.enabled,
            chunks,
            now,
        )?;
        tx.execute(
            "UPDATE knowledge_documents \
             SET name = ?1, content = ?2, metadata_json = ?3, enabled = ?4, status = ?5, \
                 error_message = '', content_hash = ?6, char_count = ?7, archived_at_ms = NULL, \
                 updated_at_ms = ?8 WHERE platform_id = ?9 AND company_id = ?10 AND id = ?11",
            params![
                body.name.trim(),
                body.content,
                metadata_json,
                bool_to_i64(body.enabled),
                final_document_status(body.enabled),
                document_hash,
                i64::try_from(char_count).unwrap_or(i64::MAX),
                now,
                tenant.platform_id().as_ref(),
                tenant.company_id().as_ref(),
                job.document_id,
            ],
        )
        .map_err(sql_error)?;
        tx.execute(
            "UPDATE knowledge_index_jobs \
                 SET status = ?1, error_message = '', payload_json = '', updated_at_ms = ?2, \
                 retry_after_ms = 0, lease_expires_at_ms = 0, finished_at_ms = ?3 \
                 WHERE platform_id = ?4 AND company_id = ?5 AND id = ?6",
            params![
                index_job_status_as_str(&KnowledgeIndexJobStatus::Completed),
                now,
                now,
                tenant.platform_id().as_ref(),
                tenant.company_id().as_ref(),
                job.id,
            ],
        )
        .map_err(sql_error)?;
        tx.commit().map_err(sql_error)
    })();
    if let Err(error) = result {
        return Err(error);
    }

    query_index_job(&conn, tenant, &job.id)?
        .ok_or_else(|| ApiError::not_found(format!("knowledge index job not found: {}", job.id)))
}

fn fail_index_job_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    job_id: &str,
    document_id: &str,
    error_message: &str,
) -> Result<KnowledgeIndexJobDto, ApiError> {
    validate_id(job_id, "job_id")?;
    validate_id(document_id, "document_id")?;
    if database.provider() == DatabaseProvider::Postgres {
        return PostgresKnowledgeStore::new(database)
            .and_then(|store| {
                store.retry_or_fail_index_job(
                    tenant,
                    job_id,
                    document_id,
                    INDEX_JOB_MAX_ATTEMPTS,
                    error_message,
                )
            })
            .map_err(database_error);
    }
    let mut conn = database.connect().map_err(database_error)?;
    let now = now_ms();
    let tx = conn.transaction().map_err(sql_error)?;
    mark_index_job_failed_tx(&tx, tenant, job_id, document_id, error_message, now)?;
    tx.commit().map_err(sql_error)?;

    query_index_job(&conn, tenant, job_id)?
        .ok_or_else(|| ApiError::not_found(format!("knowledge index job not found: {job_id}")))
}

fn retry_or_fail_index_job_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    job_id: &str,
    document_id: &str,
    attempt_count: i64,
    error_message: &str,
) -> Result<KnowledgeIndexJobDto, ApiError> {
    if attempt_count >= INDEX_JOB_MAX_ATTEMPTS {
        return fail_index_job_blocking(database, tenant, job_id, document_id, error_message);
    }

    validate_id(job_id, "job_id")?;
    validate_id(document_id, "document_id")?;
    if database.provider() == DatabaseProvider::Postgres {
        return PostgresKnowledgeStore::new(database)
            .and_then(|store| {
                store.retry_or_fail_index_job(
                    tenant,
                    job_id,
                    document_id,
                    attempt_count,
                    error_message,
                )
            })
            .map_err(database_error);
    }
    let mut conn = database.connect().map_err(database_error)?;
    let now = now_ms();
    let retry_after_ms = now.saturating_add(index_job_retry_delay_ms(attempt_count));
    let retry_message = format!(
        "attempt {attempt_count}/{INDEX_JOB_MAX_ATTEMPTS} failed: {error_message}; retrying"
    );
    let tx = conn.transaction().map_err(sql_error)?;
    tx.execute(
        "UPDATE knowledge_documents SET status = ?1, error_message = ?2, updated_at_ms = ?3 \
         WHERE platform_id = ?4 AND company_id = ?5 AND id = ?6",
        params![
            document_status_as_str(&KnowledgeDocumentStatus::Pending),
            retry_message,
            now,
            tenant.platform_id().as_ref(),
            tenant.company_id().as_ref(),
            document_id,
        ],
    )
    .map_err(sql_error)?;
    tx.execute(
        "UPDATE knowledge_index_jobs \
             SET status = ?1, error_message = ?2, updated_at_ms = ?3, retry_after_ms = ?4, \
             lease_expires_at_ms = 0, finished_at_ms = NULL \
             WHERE platform_id = ?5 AND company_id = ?6 AND id = ?7",
        params![
            index_job_status_as_str(&KnowledgeIndexJobStatus::Queued),
            retry_message,
            now,
            retry_after_ms,
            tenant.platform_id().as_ref(),
            tenant.company_id().as_ref(),
            job_id,
        ],
    )
    .map_err(sql_error)?;
    tx.commit().map_err(sql_error)?;

    query_index_job(&conn, tenant, job_id)?
        .ok_or_else(|| ApiError::not_found(format!("knowledge index job not found: {job_id}")))
}

fn mark_index_job_failed_tx(
    tx: &Transaction<'_>,
    tenant: &TenantScope,
    job_id: &str,
    document_id: &str,
    error_message: &str,
    now: u64,
) -> Result<(), ApiError> {
    tx.execute(
        "DELETE FROM knowledge_chunks_fts \
         WHERE platform_id = ?1 AND company_id = ?2 AND document_id = ?3",
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), document_id],
    )
    .map_err(sql_error)?;
    tx.execute(
        "DELETE FROM knowledge_chunks \
         WHERE platform_id = ?1 AND company_id = ?2 AND document_id = ?3",
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), document_id],
    )
    .map_err(sql_error)?;
    tx.execute(
        "UPDATE knowledge_documents SET status = ?1, error_message = ?2, updated_at_ms = ?3 \
         WHERE platform_id = ?4 AND company_id = ?5 AND id = ?6",
        params![
            document_status_as_str(&KnowledgeDocumentStatus::Failed),
            error_message,
            now,
            tenant.platform_id().as_ref(),
            tenant.company_id().as_ref(),
            document_id,
        ],
    )
    .map_err(sql_error)?;
    tx.execute(
        "UPDATE knowledge_index_jobs \
             SET status = ?1, error_message = ?2, payload_json = '', updated_at_ms = ?3, \
             retry_after_ms = 0, lease_expires_at_ms = 0, finished_at_ms = ?4 \
             WHERE platform_id = ?5 AND company_id = ?6 AND id = ?7",
        params![
            index_job_status_as_str(&KnowledgeIndexJobStatus::Failed),
            error_message,
            now,
            now,
            tenant.platform_id().as_ref(),
            tenant.company_id().as_ref(),
            job_id,
        ],
    )
    .map_err(sql_error)?;
    Ok(())
}

fn index_job_retry_delay_ms(attempt_count: i64) -> u64 {
    match attempt_count {
        i64::MIN..=1 => 2_000,
        2 => 5_000,
        _ => 10_000,
    }
}

fn index_job_timed_out_message() -> String {
    format!("knowledge index job timed out after {INDEX_JOB_MAX_ATTEMPTS} attempts")
}

fn load_document_for_reindex_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    document_id: &str,
) -> Result<ReindexKnowledgeDocument, ApiError> {
    validate_id(document_id, "document_id")?;
    if database.provider() == DatabaseProvider::Postgres {
        return PostgresKnowledgeStore::new(database)
            .and_then(|store| store.load_document_for_reindex(tenant, document_id))
            .map_err(database_error)?
            .ok_or_else(|| {
                ApiError::not_found(format!("knowledge document not found: {document_id}"))
            });
    }
    let conn = database.connect().map_err(database_error)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, dataset_id, name, content, metadata_json, enabled \
             FROM knowledge_documents \
             WHERE platform_id = ?1 AND company_id = ?2 AND id = ?3",
        )
        .map_err(sql_error)?;
    let document = stmt
        .query_row(
            params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), document_id],
            |row| {
                let metadata_json: String = row.get(4)?;
                let metadata = serde_json::from_str(&metadata_json).unwrap_or(Value::Null);
                Ok(StoredKnowledgeDocument {
                    id: row.get(0)?,
                    dataset_id: row.get(1)?,
                    body: KnowledgeDocumentCreateRequest {
                        name: row.get(2)?,
                        content: row.get(3)?,
                        metadata,
                        enabled: row.get::<_, i64>(5)? != 0,
                    },
                })
            },
        )
        .optional()
        .map_err(sql_error)?
        .ok_or_else(|| {
            ApiError::not_found(format!("knowledge document not found: {document_id}"))
        })?;
    drop(stmt);
    let dataset = query_dataset(&conn, tenant, &document.dataset_id)?.ok_or_else(|| {
        ApiError::not_found(format!("knowledge dataset not found: {}", document.dataset_id))
    })?;
    conn.execute(
        "UPDATE knowledge_documents SET status = ?1, error_message = '', updated_at_ms = ?2 \
         WHERE platform_id = ?3 AND company_id = ?4 AND id = ?5",
        params![
            document_status_as_str(&KnowledgeDocumentStatus::Processing),
            now_ms(),
            tenant.platform_id().as_ref(),
            tenant.company_id().as_ref(),
            document_id,
        ],
    )
    .map_err(sql_error)?;

    Ok(ReindexKnowledgeDocument { document, dataset })
}

fn reindex_document_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    document_id: &str,
    chunks: Vec<PreparedKnowledgeChunk>,
) -> Result<KnowledgeDocumentDto, ApiError> {
    validate_id(document_id, "document_id")?;
    if database.provider() == DatabaseProvider::Postgres {
        let store = PostgresKnowledgeStore::new(database.clone()).map_err(database_error)?;
        return match store.reindex_document(tenant, document_id, chunks) {
            Ok(Some(document)) => Ok(document),
            Ok(None) => {
                Err(ApiError::not_found(format!("knowledge document not found: {document_id}")))
            }
            Err(error) => {
                let message = error.to_string();
                let _ = store.mark_document_failed(tenant, document_id, &message);
                Err(database_error(error))
            }
        };
    }
    let mut conn = database.connect().map_err(database_error)?;
    let document = query_document_for_index(&conn, tenant, document_id)?.ok_or_else(|| {
        ApiError::not_found(format!("knowledge document not found: {document_id}"))
    })?;
    let now = now_ms();
    let result = (|| {
        let tx = conn.transaction().map_err(sql_error)?;
        replace_document_chunks_tx(
            &tx,
            tenant,
            &document.dataset_id,
            document_id,
            document.body.name.trim(),
            &document.body.metadata,
            document.body.enabled,
            chunks,
            now,
        )?;
        tx.execute(
            "UPDATE knowledge_documents \
                 SET status = ?1, error_message = '', char_count = ?2, archived_at_ms = NULL, \
                 updated_at_ms = ?3 WHERE platform_id = ?4 AND company_id = ?5 AND id = ?6",
            params![
                final_document_status(document.body.enabled),
                i64::try_from(unicode_char_count(&document.body.content)).unwrap_or(i64::MAX),
                now,
                tenant.platform_id().as_ref(),
                tenant.company_id().as_ref(),
                document_id
            ],
        )
        .map_err(sql_error)?;
        tx.commit().map_err(sql_error)
    })();
    if let Err(error) = result {
        mark_document_failed_conn(&conn, tenant, document_id, &error.to_string())?;
        return Err(error);
    }

    query_document(&conn, tenant, document_id)?
        .ok_or_else(|| ApiError::internal("knowledge document was not saved"))
}

fn mark_document_failed_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    document_id: &str,
    error_message: &str,
) -> Result<KnowledgeDocumentDto, ApiError> {
    validate_id(document_id, "document_id")?;
    if database.provider() == DatabaseProvider::Postgres {
        return PostgresKnowledgeStore::new(database)
            .and_then(|store| store.mark_document_failed(tenant, document_id, error_message))
            .map_err(database_error)?
            .ok_or_else(|| {
                ApiError::not_found(format!("knowledge document not found: {document_id}"))
            });
    }
    let conn = database.connect().map_err(database_error)?;
    mark_document_failed_conn(&conn, tenant, document_id, error_message)?;
    query_document(&conn, tenant, document_id)?
        .ok_or_else(|| ApiError::not_found(format!("knowledge document not found: {document_id}")))
}

fn mark_document_failed_conn(
    conn: &Connection,
    tenant: &TenantScope,
    document_id: &str,
    error_message: &str,
) -> Result<(), ApiError> {
    conn.execute(
        "UPDATE knowledge_documents SET status = ?1, error_message = ?2, updated_at_ms = ?3 \
         WHERE platform_id = ?4 AND company_id = ?5 AND id = ?6",
        params![
            document_status_as_str(&KnowledgeDocumentStatus::Failed),
            error_message,
            now_ms(),
            tenant.platform_id().as_ref(),
            tenant.company_id().as_ref(),
            document_id,
        ],
    )
    .map_err(sql_error)?;
    Ok(())
}

fn query_document_for_index(
    conn: &Connection,
    tenant: &TenantScope,
    document_id: &str,
) -> Result<Option<StoredKnowledgeDocument>, ApiError> {
    conn.query_row(
        "SELECT id, dataset_id, name, content, metadata_json, enabled \
         FROM knowledge_documents \
         WHERE platform_id = ?1 AND company_id = ?2 AND id = ?3",
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), document_id],
        |row| {
            let metadata_json: String = row.get(4)?;
            let metadata = serde_json::from_str(&metadata_json).unwrap_or(Value::Null);
            Ok(StoredKnowledgeDocument {
                id: row.get(0)?,
                dataset_id: row.get(1)?,
                body: KnowledgeDocumentCreateRequest {
                    name: row.get(2)?,
                    content: row.get(3)?,
                    metadata,
                    enabled: row.get::<_, i64>(5)? != 0,
                },
            })
        },
    )
    .optional()
    .map_err(sql_error)
}

fn query_index_job(
    conn: &Connection,
    tenant: &TenantScope,
    job_id: &str,
) -> Result<Option<KnowledgeIndexJobDto>, ApiError> {
    conn.query_row(
        "SELECT id, dataset_id, document_id, status, error_message, created_at_ms, updated_at_ms, \
         started_at_ms, finished_at_ms FROM knowledge_index_jobs \
         WHERE platform_id = ?1 AND company_id = ?2 AND id = ?3",
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), job_id],
        index_job_from_row,
    )
    .optional()
    .map_err(sql_error)
}

fn replace_document_chunks_tx(
    tx: &Transaction<'_>,
    tenant: &TenantScope,
    dataset_id: &str,
    document_id: &str,
    document_name: &str,
    document_metadata: &Value,
    document_enabled: bool,
    chunks: Vec<PreparedKnowledgeChunk>,
    now: u64,
) -> Result<(), ApiError> {
    tx.execute(
        "DELETE FROM knowledge_chunks_fts \
         WHERE platform_id = ?1 AND company_id = ?2 AND document_id = ?3",
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), document_id],
    )
    .map_err(sql_error)?;
    tx.execute(
        "DELETE FROM knowledge_chunks \
         WHERE platform_id = ?1 AND company_id = ?2 AND document_id = ?3",
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), document_id],
    )
    .map_err(sql_error)?;

    let chunk_ids = (0..chunks.len()).map(|_| Uuid::new_v4().to_string()).collect::<Vec<String>>();
    let indexed_positions = chunks
        .iter()
        .enumerate()
        .filter_map(|(index, chunk)| chunk.indexed.then_some(index))
        .collect::<Vec<_>>();
    let parent_positions = chunks
        .iter()
        .enumerate()
        .filter_map(|(index, chunk)| {
            (chunk.chunk_type == KnowledgeChunkType::ParentText).then_some(index)
        })
        .collect::<Vec<_>>();

    for (index, chunk) in chunks.into_iter().enumerate() {
        let chunk_id = &chunk_ids[index];
        let parent_chunk_id = chunk
            .parent_position
            .and_then(|parent_position| chunk_ids.get(parent_position))
            .map(String::as_str);
        let prev_chunk_id = previous_link(&chunk_ids, &indexed_positions, index)
            .or_else(|| previous_link(&chunk_ids, &parent_positions, index));
        let next_chunk_id = next_link(&chunk_ids, &indexed_positions, index)
            .or_else(|| next_link(&chunk_ids, &parent_positions, index));
        let chunk_metadata =
            chunk_metadata(document_metadata, document_name, chunk.ordinal, &chunk.metadata);
        let chunk_metadata_json = serde_json::to_string(&chunk_metadata).map_err(|error| {
            ApiError::bad_request(format!("document chunk metadata is invalid: {error}"))
        })?;
        let index_content_hash = content_hash(&chunk.index_content);
        tx.execute(
            "INSERT INTO knowledge_chunks \
             (platform_id, company_id, id, dataset_id, document_id, ordinal, title, content, metadata_json, chunk_type, \
              start_at, end_at, parent_chunk_id, prev_chunk_id, next_chunk_id, context_header, \
              index_content_hash, embedding, enabled, created_at_ms, updated_at_ms) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                tenant.platform_id().as_ref(),
                tenant.company_id().as_ref(),
                chunk_id,
                dataset_id,
                document_id,
                i64::try_from(chunk.ordinal).unwrap_or(i64::MAX),
                chunk.title.as_str(),
                chunk.content.as_str(),
                chunk_metadata_json,
                chunk_type_as_str(&chunk.chunk_type),
                i64::try_from(chunk.start_at).unwrap_or(i64::MAX),
                i64::try_from(chunk.end_at).unwrap_or(i64::MAX),
                parent_chunk_id,
                prev_chunk_id,
                next_chunk_id,
                chunk.context_header,
                index_content_hash,
                chunk.embedding,
                bool_to_i64(document_enabled),
                now,
                now,
            ],
        )
        .map_err(sql_error)?;
        if chunk.indexed {
            tx.execute(
                "INSERT INTO knowledge_chunks_fts \
                 (platform_id, company_id, chunk_id, dataset_id, document_id, title, content) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    tenant.platform_id().as_ref(),
                    tenant.company_id().as_ref(),
                    chunk_id,
                    dataset_id,
                    document_id,
                    chunk.title.as_str(),
                    chunk.index_content.as_str(),
                ],
            )
            .map_err(sql_error)?;
        }
    }
    Ok(())
}

fn previous_link<'a>(
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

fn next_link<'a>(chunk_ids: &'a [String], positions: &[usize], index: usize) -> Option<&'a str> {
    positions
        .iter()
        .position(|position| *position == index)
        .and_then(|position| positions.get(position + 1))
        .and_then(|chunk_index| chunk_ids.get(*chunk_index))
        .map(String::as_str)
}

fn delete_document_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    document_id: &str,
) -> Result<KnowledgeDocumentDto, ApiError> {
    validate_id(document_id, "document_id")?;
    let conn = database.connect().map_err(database_error)?;
    let document = query_document(&conn, tenant, document_id)?.ok_or_else(|| {
        ApiError::not_found(format!("knowledge document not found: {document_id}"))
    })?;
    conn.execute(
        "DELETE FROM knowledge_chunks_fts \
         WHERE platform_id = ?1 AND company_id = ?2 AND document_id = ?3",
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), document_id],
    )
    .map_err(sql_error)?;
    conn.execute(
        "DELETE FROM knowledge_chunks \
         WHERE platform_id = ?1 AND company_id = ?2 AND document_id = ?3",
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), document_id],
    )
    .map_err(sql_error)?;
    conn.execute(
        "DELETE FROM knowledge_documents \
         WHERE platform_id = ?1 AND company_id = ?2 AND id = ?3",
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), document_id],
    )
    .map_err(sql_error)?;
    Ok(document)
}

fn update_document_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    document_id: &str,
    body: KnowledgeDocumentPatchRequest,
) -> Result<KnowledgeDocumentDto, ApiError> {
    validate_id(document_id, "document_id")?;
    validate_document_patch_request(&body)?;
    if database.provider() == DatabaseProvider::Postgres {
        let store = PostgresKnowledgeStore::new(database).map_err(database_error)?;
        let current =
            store.get_document(tenant, document_id).map_err(database_error)?.ok_or_else(|| {
                ApiError::not_found(format!("knowledge document not found: {document_id}"))
            })?;
        ensure_document_state_mutable(&current, &body)?;
        let name = body.name.as_deref().map(str::trim).filter(|value| !value.is_empty());
        let now = now_ms();
        let (enabled, status, archived_at_ms, _) =
            resolve_document_state_patch(&current, body.enabled, body.archived, now)?;
        return store
            .update_document(tenant, document_id, name, enabled, &status, archived_at_ms, now)
            .map_err(database_error)?
            .ok_or_else(|| ApiError::internal("knowledge document was not saved"));
    }
    let mut conn = database.connect().map_err(database_error)?;
    let now = now_ms();
    let tx = conn.transaction().map_err(sql_error)?;
    let current = query_document_tx(&tx, tenant, document_id)?.ok_or_else(|| {
        ApiError::not_found(format!("knowledge document not found: {document_id}"))
    })?;
    ensure_document_state_mutable(&current, &body)?;
    let name = body.name.as_deref().map(str::trim).filter(|value| !value.is_empty());
    let (enabled, status, archived_at_ms, chunk_enabled) =
        resolve_document_state_patch(&current, body.enabled, body.archived, now)?;
    tx.execute(
        "UPDATE knowledge_documents \
             SET name = COALESCE(?1, name), enabled = ?2, status = ?3, archived_at_ms = ?4, \
             error_message = '', updated_at_ms = ?5 \
             WHERE platform_id = ?6 AND company_id = ?7 AND id = ?8",
        params![
            name,
            bool_to_i64(enabled),
            document_status_as_str(&status),
            archived_at_ms.map(|value| i64::try_from(value).unwrap_or(i64::MAX)),
            now,
            tenant.platform_id().as_ref(),
            tenant.company_id().as_ref(),
            document_id,
        ],
    )
    .map_err(sql_error)?;
    set_document_chunks_enabled_tx(&tx, tenant, document_id, chunk_enabled, now)?;
    tx.commit().map_err(sql_error)?;
    query_document(&conn, tenant, document_id)?
        .ok_or_else(|| ApiError::internal("knowledge document was not saved"))
}

fn batch_update_documents_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    body: KnowledgeDocumentsBatchActionRequest,
) -> Result<Vec<KnowledgeDocumentDto>, ApiError> {
    if body.document_ids.is_empty() {
        return Err(ApiError::bad_request("knowledge document_ids must not be empty"));
    }
    let mut conn = database.connect().map_err(database_error)?;
    let now = now_ms();
    let mut document_ids = Vec::new();
    for document_id in body.document_ids {
        validate_id(&document_id, "document_id")?;
        if !document_ids.iter().any(|existing| existing == &document_id) {
            document_ids.push(document_id);
        }
    }
    let tx = conn.transaction().map_err(sql_error)?;
    let mut current_documents = Vec::with_capacity(document_ids.len());
    for document_id in &document_ids {
        let current = query_document_tx(&tx, tenant, document_id)?.ok_or_else(|| {
            ApiError::not_found(format!("knowledge document not found: {document_id}"))
        })?;
        ensure_document_status_actionable(&current)?;
        current_documents.push(current);
    }

    for current in &current_documents {
        let (enabled, status, archived_at_ms, chunk_enabled) =
            resolve_document_state_batch(body.action.clone(), now);
        tx.execute(
            "UPDATE knowledge_documents \
                 SET enabled = ?1, status = ?2, archived_at_ms = ?3, error_message = '', \
                 updated_at_ms = ?4 \
                 WHERE platform_id = ?5 AND company_id = ?6 AND id = ?7",
            params![
                bool_to_i64(enabled),
                document_status_as_str(&status),
                archived_at_ms.map(|value| i64::try_from(value).unwrap_or(i64::MAX)),
                now,
                tenant.platform_id().as_ref(),
                tenant.company_id().as_ref(),
                current.id,
            ],
        )
        .map_err(sql_error)?;
        set_document_chunks_enabled_tx(&tx, tenant, &current.id, chunk_enabled, now)?;
    }

    tx.commit().map_err(sql_error)?;
    let mut documents = Vec::with_capacity(document_ids.len());
    for document_id in document_ids {
        documents.push(
            query_document(&conn, tenant, &document_id)?
                .ok_or_else(|| ApiError::internal("knowledge document was not saved"))?,
        );
    }
    Ok(documents)
}

fn retrieve_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    request: KnowledgeRetrieveRequest,
    scope: RetrievalScope,
    query_embedding: Option<Vec<f32>>,
    vector_weight: f32,
    keyword_weight: f32,
) -> Result<KnowledgeRetrieveResponse, ApiError> {
    if database.provider() == DatabaseProvider::Postgres {
        return PostgresKnowledgeStore::new(database).map_err(database_error)?.retrieve(
            tenant,
            request,
            scope,
            query_embedding,
            vector_weight,
            keyword_weight,
        );
    }
    let conn = database.connect().map_err(database_error)?;
    let top_k = request.top_k.clamp(1, MAX_TOP_K);
    let candidate_limit = (top_k * 5).clamp(top_k, MAX_CANDIDATES);
    let mut keyword_results =
        retrieve_fts_candidates(&conn, tenant, &request, &scope.keyword_datasets)?;
    if keyword_results.is_empty() {
        keyword_results =
            retrieve_like_candidates(&conn, tenant, &request, &scope.keyword_datasets)?;
    }
    let vector_results = query_embedding
        .as_ref()
        .map(|embedding| {
            retrieve_vector_candidates(
                &conn,
                tenant,
                &scope.vector_dataset_ids,
                embedding,
                candidate_limit,
            )
        })
        .transpose()?
        .unwrap_or_default();

    let merged = merge_retrieval_scores(
        &vector_results,
        &keyword_results,
        vector_weight,
        keyword_weight,
        candidate_limit,
    );
    let mut chunks = Vec::new();
    for scored in merged {
        let Some(chunk) = query_chunk(
            &conn,
            tenant,
            &scored.id,
            Some(f64::from(scored.score)),
            scored.match_type,
        )?
        else {
            continue;
        };
        if !metadata_matches(&chunk.metadata, &request.metadata_filter) {
            continue;
        }
        if request
            .score_threshold
            .is_some_and(|threshold| chunk.score.unwrap_or_default() < threshold)
        {
            continue;
        }
        chunks.push(chunk);
        if chunks.len() >= candidate_limit {
            break;
        }
    }
    if !scope.rerank_dataset_ids.is_empty() {
        rerank_chunks(&request.query, &scope.rerank_dataset_ids, &mut chunks);
    }
    chunks.truncate(top_k);
    if request.include_context {
        enrich_chunk_contexts(&conn, tenant, &mut chunks)?;
    }
    increment_recall_counts(&conn, tenant, &chunks)?;

    Ok(KnowledgeRetrieveResponse { chunks })
}

fn retrieve_fts_candidates(
    conn: &Connection,
    tenant: &TenantScope,
    request: &KnowledgeRetrieveRequest,
    datasets: &[KeywordRetrievalDataset],
) -> Result<Vec<RetrievedCandidate>, ApiError> {
    if datasets.is_empty() {
        return Ok(Vec::new());
    }
    let fts_query = fts_query_from_text(&request.query);
    if fts_query.is_empty() {
        return Ok(Vec::new());
    }
    let mut candidates = Vec::new();
    for dataset in datasets {
        let limit = keyword_limit(dataset.keyword_count);
        let limit_i64 = i64::try_from(limit).unwrap_or(i64::MAX);
        let mut stmt = conn
            .prepare(
                "SELECT c.id, bm25(knowledge_chunks_fts) AS rank \
	         FROM knowledge_chunks_fts \
	         JOIN knowledge_chunks c \
	           ON c.platform_id = knowledge_chunks_fts.platform_id \
	          AND c.company_id = knowledge_chunks_fts.company_id \
	          AND c.id = knowledge_chunks_fts.chunk_id \
	         JOIN knowledge_documents d \
	           ON d.platform_id = c.platform_id AND d.company_id = c.company_id \
	          AND d.id = c.document_id \
	         WHERE knowledge_chunks_fts.platform_id = ?1 \
	           AND knowledge_chunks_fts.company_id = ?2 \
	           AND knowledge_chunks_fts MATCH ?3 AND c.enabled = 1 AND d.enabled = 1 \
	           AND d.status = 'completed' AND c.dataset_id = ?4 \
	         ORDER BY rank ASC, c.updated_at_ms DESC, c.id ASC LIMIT ?5",
            )
            .map_err(sql_error)?;
        match stmt.query_map(
            params![
                tenant.platform_id().as_ref(),
                tenant.company_id().as_ref(),
                fts_query,
                dataset.id,
                limit_i64,
            ],
            |row| {
                let id: String = row.get(0)?;
                let rank: f64 = row.get(1)?;
                Ok(RetrievedCandidate {
                    id,
                    score: fts_score(rank),
                    source: RetrievalSource::FullText,
                })
            },
        ) {
            Ok(rows) => candidates.extend(rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)?),
            Err(error) => {
                tracing::debug!(target: "vw_agent::knowledge", %error, "FTS query failed");
            }
        }
    }
    Ok(candidates)
}

fn retrieve_like_candidates(
    conn: &Connection,
    tenant: &TenantScope,
    request: &KnowledgeRetrieveRequest,
    datasets: &[KeywordRetrievalDataset],
) -> Result<Vec<RetrievedCandidate>, ApiError> {
    if datasets.is_empty() {
        return Ok(Vec::new());
    }
    let like_pattern = like_pattern(&request.query);
    let mut candidates = Vec::new();
    for dataset in datasets {
        let limit = keyword_limit(dataset.keyword_count);
        let limit_i64 = i64::try_from(limit).unwrap_or(i64::MAX);
        let mut stmt = conn
            .prepare(
                "SELECT c.id, c.dataset_id, c.document_id, c.title, c.content, c.metadata_json, \
         c.ordinal, c.chunk_type, c.start_at, c.end_at, c.parent_chunk_id, c.prev_chunk_id, \
         c.next_chunk_id, c.context_header \
	         FROM knowledge_chunks c \
	         JOIN knowledge_documents d \
	           ON d.platform_id = c.platform_id AND d.company_id = c.company_id \
	          AND d.id = c.document_id \
	         WHERE c.platform_id = ?1 AND c.company_id = ?2 \
	         AND c.enabled = 1 AND d.enabled = 1 AND d.status = 'completed' \
	         AND c.chunk_type != 'parent_text' AND c.dataset_id = ?3 \
	         AND (c.title LIKE ?4 ESCAPE '\\' OR c.content LIKE ?5 ESCAPE '\\') \
	         ORDER BY c.updated_at_ms DESC, c.id ASC LIMIT ?6",
            )
            .map_err(sql_error)?;
        let rows = stmt
            .query_map(
                params![
                    tenant.platform_id().as_ref(),
                    tenant.company_id().as_ref(),
                    dataset.id,
                    like_pattern,
                    like_pattern,
                    limit_i64,
                ],
                |row| {
                    let chunk = chunk_from_row(row, None, KnowledgeMatchType::FullText)?;
                    let score = like_score(&chunk, &request.query);
                    Ok(RetrievedCandidate {
                        id: chunk.id,
                        score,
                        source: RetrievalSource::FullText,
                    })
                },
            )
            .map_err(sql_error)?;
        candidates.extend(rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)?);
    }
    Ok(candidates)
}

fn retrieve_vector_candidates(
    conn: &Connection,
    tenant: &TenantScope,
    dataset_ids: &[String],
    query_embedding: &[f32],
    limit: usize,
) -> Result<Vec<RetrievedCandidate>, ApiError> {
    if dataset_ids.is_empty() {
        return Ok(Vec::new());
    }
    let dataset_filter = dataset_filter_sql("c", dataset_ids.len());
    let sql = format!(
        "SELECT c.id, c.embedding \
         FROM knowledge_chunks c \
	         JOIN knowledge_documents d \
	           ON d.platform_id = c.platform_id AND d.company_id = c.company_id \
	          AND d.id = c.document_id \
	         WHERE c.platform_id = ? AND c.company_id = ? \
	         AND c.embedding IS NOT NULL AND c.enabled = 1 AND d.enabled = 1 \
	         AND d.status = 'completed'{dataset_filter}",
    );
    let mut params_vec = Vec::<&dyn rusqlite::ToSql>::new();
    let platform_id: &str = tenant.platform_id().as_ref();
    let company_id: &str = tenant.company_id().as_ref();
    params_vec.push(&platform_id);
    params_vec.push(&company_id);
    for dataset_id in dataset_ids {
        params_vec.push(dataset_id);
    }
    let mut stmt = conn.prepare(&sql).map_err(sql_error)?;
    let rows = stmt
        .query_map(params_from_iter(params_vec), |row| {
            let id: String = row.get(0)?;
            let blob: Vec<u8> = row.get(1)?;
            Ok((id, blob))
        })
        .map_err(sql_error)?;

    let mut scored = Vec::new();
    for row in rows {
        let (id, blob) = row.map_err(sql_error)?;
        let embedding = vector::bytes_to_vec(&blob);
        let score = vector::cosine_similarity(query_embedding, &embedding);
        if score > 0.0 {
            scored.push(RetrievedCandidate { id, score, source: RetrievalSource::Vector });
        }
    }
    scored.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.id.cmp(&right.id))
    });
    scored.truncate(limit);
    Ok(scored)
}

fn merge_retrieval_scores(
    vector_results: &[RetrievedCandidate],
    keyword_results: &[RetrievedCandidate],
    vector_weight: f32,
    keyword_weight: f32,
    limit: usize,
) -> Vec<FusedCandidate> {
    if !vector_results.is_empty() && !keyword_results.is_empty() {
        return fuse_with_rrf(
            vector_results,
            keyword_results,
            60,
            vector_weight,
            keyword_weight,
            limit,
        );
    }
    let source = if vector_results.is_empty() { keyword_results } else { vector_results };
    deduplicate_by_score(source, limit)
}

fn keyword_limit(keyword_count: usize) -> usize {
    keyword_count.clamp(1, MAX_CANDIDATES)
}

fn rerank_chunks(query: &str, rerank_dataset_ids: &[String], chunks: &mut [KnowledgeChunkDto]) {
    chunks.sort_by(|left, right| {
        let left_score = rerank_score(query, rerank_dataset_ids, left);
        let right_score = rerank_score(query, rerank_dataset_ids, right);
        right_score.partial_cmp(&left_score).unwrap_or(std::cmp::Ordering::Equal)
    });
    for chunk in chunks {
        if rerank_dataset_ids.iter().any(|dataset_id| dataset_id == &chunk.dataset_id) {
            chunk.score = Some(rerank_score(query, rerank_dataset_ids, chunk));
        }
    }
}

fn rerank_score(query: &str, rerank_dataset_ids: &[String], chunk: &KnowledgeChunkDto) -> f64 {
    let base = chunk.score.unwrap_or_default();
    if !rerank_dataset_ids.iter().any(|dataset_id| dataset_id == &chunk.dataset_id) {
        return base;
    }
    let lexical = lexical_match_score(query, chunk);
    (base * 0.70 + lexical * 0.30).clamp(0.0, 1.0)
}

fn lexical_match_score(query: &str, chunk: &KnowledgeChunkDto) -> f64 {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return 0.0;
    }
    let mut haystack = format!("{}\n{}", chunk.title, chunk.content).to_lowercase();
    if let Some(child_content) = chunk.metadata.get("child_content").and_then(Value::as_str) {
        haystack.push('\n');
        haystack.push_str(&child_content.to_lowercase());
    }
    if let Some(question) = chunk.metadata.get("question").and_then(Value::as_str) {
        haystack.push('\n');
        haystack.push_str(&question.to_lowercase());
    }
    if haystack.contains(&query) {
        return 1.0;
    }
    let terms = query.split_whitespace().filter(|term| !term.is_empty()).collect::<Vec<_>>();
    if terms.is_empty() {
        return 0.0;
    }
    let matched = terms.iter().filter(|term| haystack.contains(**term)).count();
    matched as f64 / terms.len() as f64
}

fn query_dataset(
    conn: &Connection,
    tenant: &TenantScope,
    dataset_id: &str,
) -> Result<Option<KnowledgeDatasetDto>, ApiError> {
    conn.query_row(
        "SELECT id, name, description, chunking_mode, indexing_mode, retrieval_mode, \
         keyword_count, top_k, score_threshold_enabled, score_threshold, rerank_enabled, \
         embedding_model, rerank_model, created_at_ms, updated_at_ms \
	         FROM knowledge_datasets \
	         WHERE platform_id = ?1 AND company_id = ?2 AND id = ?3",
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), dataset_id],
        |row| dataset_from_row(conn, tenant, row),
    )
    .optional()
    .map_err(sql_error)
}

fn query_document(
    conn: &Connection,
    tenant: &TenantScope,
    document_id: &str,
) -> Result<Option<KnowledgeDocumentDto>, ApiError> {
    conn.query_row(
        "SELECT id, dataset_id, name, metadata_json, enabled, created_at_ms, updated_at_ms, \
         status, error_message, content_hash, char_count, recall_count, archived_at_ms \
	         FROM knowledge_documents \
	         WHERE platform_id = ?1 AND company_id = ?2 AND id = ?3",
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), document_id],
        |row| document_from_row(conn, tenant, row),
    )
    .optional()
    .map_err(sql_error)
}

fn query_document_tx(
    tx: &Transaction<'_>,
    tenant: &TenantScope,
    document_id: &str,
) -> Result<Option<KnowledgeDocumentDto>, ApiError> {
    tx.query_row(
        "SELECT id, dataset_id, name, metadata_json, enabled, created_at_ms, updated_at_ms, \
         status, error_message, content_hash, char_count, recall_count, archived_at_ms \
	         FROM knowledge_documents \
	         WHERE platform_id = ?1 AND company_id = ?2 AND id = ?3",
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), document_id],
        |row| document_from_row(tx, tenant, row),
    )
    .optional()
    .map_err(sql_error)
}

fn query_chunk(
    conn: &Connection,
    tenant: &TenantScope,
    chunk_id: &str,
    score: Option<f64>,
    match_type: KnowledgeMatchType,
) -> Result<Option<KnowledgeChunkDto>, ApiError> {
    conn.query_row(
        "SELECT id, dataset_id, document_id, title, content, metadata_json, ordinal, chunk_type, \
         start_at, end_at, parent_chunk_id, prev_chunk_id, next_chunk_id, context_header \
	         FROM knowledge_chunks WHERE platform_id = ?1 AND company_id = ?2 \
	         AND id = ?3 AND enabled = 1",
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), chunk_id],
        |row| chunk_from_row(row, score, match_type),
    )
    .optional()
    .map_err(sql_error)
}

fn enrich_chunk_contexts(
    conn: &Connection,
    tenant: &TenantScope,
    chunks: &mut [KnowledgeChunkDto],
) -> Result<(), ApiError> {
    for chunk in chunks {
        if chunk.chunk_type == KnowledgeChunkType::Qa {
            continue;
        }
        if let Some(parent_chunk_id) = chunk.parent_chunk_id.as_deref() {
            if let Some(parent) =
                query_chunk(conn, tenant, parent_chunk_id, None, KnowledgeMatchType::Parent)?
            {
                chunk.context = non_empty_context(parent.content);
                if chunk.matched_content.is_none() {
                    chunk.matched_content = Some(chunk.content.clone());
                }
            }
            continue;
        }
        if chunk.chunk_type != KnowledgeChunkType::Text {
            continue;
        }
        chunk.context = nearby_text_context(conn, tenant, chunk)?;
    }
    Ok(())
}

fn nearby_text_context(
    conn: &Connection,
    tenant: &TenantScope,
    chunk: &KnowledgeChunkDto,
) -> Result<Option<String>, ApiError> {
    let mut linked_chunks = Vec::new();
    for chunk_id in
        [chunk.prev_chunk_id.as_deref(), Some(chunk.id.as_str()), chunk.next_chunk_id.as_deref()]
            .into_iter()
            .flatten()
    {
        if let Some(candidate) =
            query_chunk(conn, tenant, chunk_id, None, KnowledgeMatchType::Nearby)?
        {
            if candidate.document_id == chunk.document_id
                && candidate.chunk_type == KnowledgeChunkType::Text
            {
                linked_chunks.push(candidate);
            }
        }
    }
    Ok(non_empty_context(merge_text_chunks(&linked_chunks, "\n")))
}

fn merge_text_chunks(chunks: &[KnowledgeChunkDto], gap_sep: &str) -> String {
    let mut sorted = chunks.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| {
        left.start_at
            .cmp(&right.start_at)
            .then_with(|| left.ordinal.cmp(&right.ordinal))
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut merged = String::new();
    let mut merged_end = None;
    for chunk in sorted {
        if chunk.content.is_empty() {
            continue;
        }
        if merged.is_empty() {
            merged.push_str(&chunk.content);
            if chunk.end_at > 0 {
                merged_end = Some(chunk.end_at);
            }
            continue;
        }

        let Some(last_end) = merged_end else {
            append_gap_and_text(&mut merged, gap_sep, &chunk.content);
            if chunk.end_at > 0 {
                merged_end = Some(chunk.end_at);
            }
            continue;
        };
        if chunk.end_at == 0 || chunk.start_at > last_end {
            append_gap_and_text(&mut merged, gap_sep, &chunk.content);
            if chunk.end_at > 0 {
                merged_end = Some(chunk.end_at);
            }
            continue;
        }
        if chunk.end_at > last_end {
            merged = append_with_overlap(
                &merged,
                &chunk.content,
                last_end.saturating_sub(chunk.start_at),
            );
            merged_end = Some(chunk.end_at);
        }
    }
    merged
}

fn append_gap_and_text(target: &mut String, gap_sep: &str, text: &str) {
    if !gap_sep.is_empty() {
        target.push_str(gap_sep);
    }
    target.push_str(text);
}

fn append_with_overlap(acc: &str, next: &str, position_overlap: usize) -> String {
    if acc.is_empty() {
        return next.to_string();
    }
    if next.is_empty() {
        return acc.to_string();
    }

    let acc_chars = acc.chars().collect::<Vec<_>>();
    let next_chars = next.chars().collect::<Vec<_>>();
    let mut max_overlap = acc_chars.len().min(next_chars.len());
    max_overlap =
        max_overlap.min((position_overlap.saturating_mul(3)).max(DEFAULT_CONTEXT_SEARCH_SPAN));
    let head_slack = (position_overlap.saturating_mul(2)).max(320);

    for overlap in (MIN_OVERLAP_CHARS..=max_overlap).rev() {
        let needle = &acc_chars[acc_chars.len() - overlap..];
        if let Some(position) = index_chars(&next_chars, needle, head_slack) {
            let suffix = next_chars[position + overlap..].iter().collect::<String>();
            return format!("{acc}{suffix}");
        }
    }

    format!("{acc}{next}")
}

fn index_chars(haystack: &[char], needle: &[char], max_start: usize) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    let limit = haystack.len().saturating_sub(needle.len()).min(max_start);
    (0..=limit).find(|position| &haystack[*position..*position + needle.len()] == needle)
}

fn non_empty_context(context: String) -> Option<String> {
    (!context.is_empty()).then_some(context)
}

fn dataset_from_row(
    conn: &Connection,
    tenant: &TenantScope,
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<KnowledgeDatasetDto> {
    let id: String = row.get(0)?;
    Ok(KnowledgeDatasetDto {
        id: id.clone(),
        name: row.get(1)?,
        description: row.get(2)?,
        chunking_mode: parse_chunking_mode(row.get::<_, String>(3)?.as_str()),
        indexing_mode: parse_indexing_mode(row.get::<_, String>(4)?.as_str()),
        retrieval_mode: parse_retrieval_mode(row.get::<_, String>(5)?.as_str()),
        keyword_count: read_usize(row, 6)?,
        top_k: read_usize(row, 7)?,
        score_threshold_enabled: row.get::<_, i64>(8)? != 0,
        score_threshold: row.get(9)?,
        rerank_enabled: row.get::<_, i64>(10)? != 0,
        embedding_model: row.get(11)?,
        rerank_model: row.get(12)?,
        capabilities: dataset_capabilities(
            &parse_indexing_mode(row.get::<_, String>(4)?.as_str()),
            &parse_retrieval_mode(row.get::<_, String>(5)?.as_str()),
            row.get::<_, i64>(10)? != 0,
        ),
        document_count: count_by_dataset(conn, tenant, "knowledge_documents", &id)?,
        chunk_count: count_by_dataset(conn, tenant, "knowledge_chunks", &id)?,
        created_at_ms: read_u64(row, 13)?,
        updated_at_ms: read_u64(row, 14)?,
    })
}

fn document_from_row(
    conn: &Connection,
    tenant: &TenantScope,
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<KnowledgeDocumentDto> {
    let id: String = row.get(0)?;
    let metadata_json: String = row.get(3)?;
    let enabled = row.get::<_, i64>(4)? != 0;
    let status = parse_document_status(row.get::<_, String>(7)?.as_str(), enabled);
    Ok(KnowledgeDocumentDto {
        id: id.clone(),
        dataset_id: row.get(1)?,
        name: row.get(2)?,
        metadata: serde_json::from_str(&metadata_json).unwrap_or(Value::Null),
        enabled,
        status,
        error_message: row.get(8)?,
        content_hash: row.get(9)?,
        chunk_count: count_by_document(conn, tenant, &id)?,
        char_count: read_u64(row, 10)?,
        recall_count: read_u64(row, 11)?,
        archived_at_ms: read_optional_u64(row, 12)?,
        created_at_ms: read_u64(row, 5)?,
        updated_at_ms: read_u64(row, 6)?,
    })
}

fn chunk_from_row(
    row: &rusqlite::Row<'_>,
    score: Option<f64>,
    match_type: KnowledgeMatchType,
) -> rusqlite::Result<KnowledgeChunkDto> {
    let metadata_json: String = row.get(5)?;
    let metadata = serde_json::from_str(&metadata_json).unwrap_or(Value::Null);
    let matched_content = matched_content_from_metadata(&metadata);
    Ok(KnowledgeChunkDto {
        id: row.get(0)?,
        dataset_id: row.get(1)?,
        document_id: row.get(2)?,
        title: row.get(3)?,
        content: row.get(4)?,
        metadata,
        score,
        ordinal: read_usize(row, 6)?,
        chunk_type: parse_chunk_type(row.get::<_, String>(7)?.as_str()),
        start_at: read_usize(row, 8)?,
        end_at: read_usize(row, 9)?,
        parent_chunk_id: row.get(10)?,
        prev_chunk_id: row.get(11)?,
        next_chunk_id: row.get(12)?,
        matched_content,
        match_type,
        context: None,
    })
}

fn index_job_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<KnowledgeIndexJobDto> {
    Ok(KnowledgeIndexJobDto {
        id: row.get(0)?,
        dataset_id: row.get(1)?,
        document_id: row.get(2)?,
        status: parse_index_job_status(row.get::<_, String>(3)?.as_str()),
        error_message: row.get(4)?,
        created_at_ms: read_u64(row, 5)?,
        updated_at_ms: read_u64(row, 6)?,
        started_at_ms: read_optional_u64(row, 7)?,
        finished_at_ms: read_optional_u64(row, 8)?,
    })
}

fn count_by_dataset(
    conn: &Connection,
    tenant: &TenantScope,
    table: &str,
    dataset_id: &str,
) -> rusqlite::Result<u64> {
    let sql = format!(
        "SELECT COUNT(*) FROM {table} \
         WHERE platform_id = ?1 AND company_id = ?2 AND dataset_id = ?3"
    );
    let count: i64 = conn.query_row(
        &sql,
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), dataset_id],
        |row| row.get(0),
    )?;
    Ok(u64::try_from(count).unwrap_or_default())
}

fn count_by_document(
    conn: &Connection,
    tenant: &TenantScope,
    document_id: &str,
) -> rusqlite::Result<u64> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM knowledge_chunks \
         WHERE platform_id = ?1 AND company_id = ?2 AND document_id = ?3",
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), document_id],
        |row| row.get(0),
    )?;
    Ok(u64::try_from(count).unwrap_or_default())
}

fn validate_document_patch_request(body: &KnowledgeDocumentPatchRequest) -> Result<(), ApiError> {
    if body.name.as_deref().is_some_and(|value| value.trim().is_empty()) {
        return Err(ApiError::bad_request("knowledge document name must not be empty"));
    }
    if matches!(body.enabled, Some(false)) && matches!(body.archived, Some(true)) {
        return Err(ApiError::bad_request(
            "knowledge document cannot be disabled and archived at the same time",
        ));
    }
    Ok(())
}

fn ensure_document_state_mutable(
    current: &KnowledgeDocumentDto,
    body: &KnowledgeDocumentPatchRequest,
) -> Result<(), ApiError> {
    if body.enabled.is_some() || body.archived.is_some() {
        ensure_document_status_actionable(current)?;
    }
    Ok(())
}

fn ensure_document_status_actionable(current: &KnowledgeDocumentDto) -> Result<(), ApiError> {
    match current.status {
        KnowledgeDocumentStatus::Pending | KnowledgeDocumentStatus::Processing => {
            Err(ApiError::bad_request(format!(
                "knowledge document {} cannot change state while status is {}",
                current.id,
                document_status_as_str(&current.status)
            )))
        }
        _ => Ok(()),
    }
}

fn resolve_document_state_patch(
    current: &KnowledgeDocumentDto,
    enabled: Option<bool>,
    archived: Option<bool>,
    now: u64,
) -> Result<(bool, KnowledgeDocumentStatus, Option<u64>, bool), ApiError> {
    if matches!(enabled, Some(false)) && matches!(archived, Some(true)) {
        return Err(ApiError::bad_request(
            "knowledge document cannot be disabled and archived at the same time",
        ));
    }
    if archived == Some(true) {
        return Ok((true, KnowledgeDocumentStatus::Archived, Some(now), false));
    }
    if enabled == Some(false) {
        return Ok((false, KnowledgeDocumentStatus::Disabled, None, false));
    }
    if enabled == Some(true) {
        return Ok((true, KnowledgeDocumentStatus::Completed, None, true));
    }
    if archived == Some(false) && current.status == KnowledgeDocumentStatus::Archived {
        return Ok((true, KnowledgeDocumentStatus::Completed, None, true));
    }
    Ok((
        current.enabled,
        current.status.clone(),
        current.archived_at_ms,
        current.enabled && current.status == KnowledgeDocumentStatus::Completed,
    ))
}

fn resolve_document_state_batch(
    action: KnowledgeDocumentsBatchAction,
    now: u64,
) -> (bool, KnowledgeDocumentStatus, Option<u64>, bool) {
    match action {
        KnowledgeDocumentsBatchAction::Enable => {
            (true, KnowledgeDocumentStatus::Completed, None, true)
        }
        KnowledgeDocumentsBatchAction::Disable => {
            (false, KnowledgeDocumentStatus::Disabled, None, false)
        }
        KnowledgeDocumentsBatchAction::Archive => {
            (true, KnowledgeDocumentStatus::Archived, Some(now), false)
        }
    }
}

fn set_document_chunks_enabled_tx(
    tx: &Transaction<'_>,
    tenant: &TenantScope,
    document_id: &str,
    enabled: bool,
    now: u64,
) -> Result<(), ApiError> {
    tx.execute(
        "UPDATE knowledge_chunks SET enabled = ?1, updated_at_ms = ?2 \
         WHERE platform_id = ?3 AND company_id = ?4 AND document_id = ?5",
        params![
            bool_to_i64(enabled),
            now,
            tenant.platform_id().as_ref(),
            tenant.company_id().as_ref(),
            document_id,
        ],
    )
    .map_err(sql_error)?;
    Ok(())
}

fn increment_recall_counts(
    conn: &Connection,
    tenant: &TenantScope,
    chunks: &[KnowledgeChunkDto],
) -> Result<(), ApiError> {
    let mut counts = BTreeMap::<&str, u64>::new();
    for chunk in chunks {
        *counts.entry(chunk.document_id.as_str()).or_default() += 1;
    }
    for (document_id, count) in counts {
        conn.execute(
            "UPDATE knowledge_documents \
	             SET recall_count = recall_count + ?1, updated_at_ms = ?2 \
             WHERE platform_id = ?3 AND company_id = ?4 AND id = ?5",
            params![
                i64::try_from(count).unwrap_or(i64::MAX),
                now_ms(),
                tenant.platform_id().as_ref(),
                tenant.company_id().as_ref(),
                document_id,
            ],
        )
        .map_err(sql_error)?;
    }
    Ok(())
}

fn ensure_dataset_exists(
    conn: &Connection,
    tenant: &TenantScope,
    dataset_id: &str,
) -> Result<(), ApiError> {
    if query_dataset(conn, tenant, dataset_id)?.is_some() {
        Ok(())
    } else {
        Err(ApiError::not_found(format!("knowledge dataset not found: {dataset_id}")))
    }
}

fn validate_dataset_scope(
    conn: &Connection,
    tenant: &TenantScope,
    dataset_ids: &[String],
) -> Result<(), ApiError> {
    for dataset_id in dataset_ids {
        validate_id(dataset_id, "dataset_id")?;
        ensure_dataset_exists(conn, tenant, dataset_id)?;
    }
    Ok(())
}

fn resolve_retrieval_scope_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    dataset_ids: &[String],
) -> Result<RetrievalScope, ApiError> {
    if database.provider() == DatabaseProvider::Postgres {
        return PostgresKnowledgeStore::new(database)
            .map_err(database_error)?
            .resolve_retrieval_scope(tenant, dataset_ids);
    }
    let conn = database.connect().map_err(database_error)?;
    validate_dataset_scope(&conn, tenant, dataset_ids)?;
    let mut keyword_dataset_ids = Vec::new();
    let mut vector_dataset_ids = Vec::new();
    let mut rerank_dataset_ids = Vec::new();
    for (dataset_id, retrieval_mode, rerank_enabled, keyword_count) in
        query_retrieval_modes(&conn, tenant, dataset_ids)?
    {
        if rerank_enabled {
            rerank_dataset_ids.push(dataset_id.clone());
        }
        match retrieval_mode {
            KnowledgeRetrievalMode::FullText => {
                keyword_dataset_ids.push(KeywordRetrievalDataset { id: dataset_id, keyword_count });
            }
            KnowledgeRetrievalMode::Vector => vector_dataset_ids.push(dataset_id),
            KnowledgeRetrievalMode::Hybrid => {
                keyword_dataset_ids
                    .push(KeywordRetrievalDataset { id: dataset_id.clone(), keyword_count });
                vector_dataset_ids.push(dataset_id);
            }
        }
    }
    Ok(RetrievalScope {
        keyword_datasets: keyword_dataset_ids,
        vector_dataset_ids,
        rerank_dataset_ids,
    })
}

fn query_retrieval_modes(
    conn: &Connection,
    tenant: &TenantScope,
    dataset_ids: &[String],
) -> Result<Vec<(String, KnowledgeRetrievalMode, bool, usize)>, ApiError> {
    let sql = if dataset_ids.is_empty() {
        "SELECT id, retrieval_mode, rerank_enabled, keyword_count \
	         FROM knowledge_datasets WHERE platform_id = ? AND company_id = ? ORDER BY id ASC"
            .to_string()
    } else {
        let placeholders = (0..dataset_ids.len()).map(|_| "?").collect::<Vec<_>>().join(", ");
        format!(
            "SELECT id, retrieval_mode, rerank_enabled, keyword_count \
	             FROM knowledge_datasets WHERE platform_id = ? AND company_id = ? \
             AND id IN ({placeholders}) ORDER BY id ASC"
        )
    };
    let mut params_vec = Vec::<&dyn rusqlite::ToSql>::new();
    let platform_id: &str = tenant.platform_id().as_ref();
    let company_id: &str = tenant.company_id().as_ref();
    params_vec.push(&platform_id);
    params_vec.push(&company_id);
    for dataset_id in dataset_ids {
        params_vec.push(dataset_id);
    }
    let mut stmt = conn.prepare(&sql).map_err(sql_error)?;
    let rows = stmt
        .query_map(params_from_iter(params_vec), |row| {
            let id: String = row.get(0)?;
            let retrieval_mode: String = row.get(1)?;
            let rerank_enabled = row.get::<_, i64>(2)? != 0;
            Ok((id, parse_retrieval_mode(&retrieval_mode), rerank_enabled, read_usize(row, 3)?))
        })
        .map_err(sql_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)
}

fn validate_dataset_request(
    body: &KnowledgeDatasetCreateRequest,
    vector_available: bool,
    configured_embedding_model: Option<&str>,
) -> Result<(), ApiError> {
    if body.name.trim().is_empty() {
        return Err(ApiError::bad_request("knowledge dataset name is required"));
    }
    if body.keyword_count == 0 {
        return Err(ApiError::bad_request("knowledge keyword_count must be greater than 0"));
    }
    if body.top_k == 0 {
        return Err(ApiError::bad_request("knowledge top_k must be greater than 0"));
    }
    if !(0.0..=1.0).contains(&body.score_threshold) {
        return Err(ApiError::bad_request("knowledge score_threshold must be between 0 and 1"));
    }
    if body.indexing_mode == KnowledgeIndexingMode::HighQuality && !vector_available {
        return Err(ApiError::not_implemented(
            "high_quality knowledge indexing requires an embedding provider",
        ));
    }
    if body.retrieval_mode != KnowledgeRetrievalMode::FullText && !vector_available {
        return Err(ApiError::not_implemented(
            "vector and hybrid knowledge retrieval require a vector backend",
        ));
    }
    if let Some(requested_model) =
        body.embedding_model.as_deref().map(str::trim).filter(|value| !value.is_empty())
    {
        let Some(configured_model) = configured_embedding_model else {
            return Err(ApiError::bad_request(
                "knowledge embedding_model requires configured memory embedding provider",
            ));
        };
        if requested_model != configured_model {
            return Err(ApiError::bad_request(format!(
                "per-dataset embedding_model override is not supported; configured model is {configured_model}"
            )));
        }
    }
    if body.rerank_model.as_deref().is_some_and(|value| !value.trim().is_empty()) {
        let requested_model = body.rerank_model.as_deref().unwrap_or_default().trim();
        if requested_model != "local-rerank-v1" {
            return Err(ApiError::bad_request(
                "knowledge rerank_model only supports local-rerank-v1",
            ));
        }
    }
    Ok(())
}

fn validate_document_request(body: &KnowledgeDocumentCreateRequest) -> Result<(), ApiError> {
    if body.name.trim().is_empty() {
        return Err(ApiError::bad_request("knowledge document name is required"));
    }
    if body.content.trim().is_empty() {
        return Err(ApiError::bad_request("knowledge document content is required"));
    }
    if !matches!(body.metadata, Value::Null | Value::Object(_)) {
        return Err(ApiError::bad_request("knowledge document metadata must be an object"));
    }
    Ok(())
}

fn validate_import_enqueue_request(body: &KnowledgeDocumentImportRequest) -> Result<(), ApiError> {
    clean_import_filename(&body.filename)?;
    if body.content_base64.trim().is_empty() {
        return Err(ApiError::bad_request("knowledge import content_base64 is required"));
    }
    if !matches!(body.metadata, Value::Null | Value::Object(_)) {
        return Err(ApiError::bad_request("knowledge import metadata must be an object"));
    }
    Ok(())
}

fn validate_retrieve_request(request: &KnowledgeRetrieveRequest) -> Result<(), ApiError> {
    if request.query.trim().is_empty() {
        return Err(ApiError::bad_request("knowledge retrieve query is required"));
    }
    if request.top_k == 0 {
        return Err(ApiError::bad_request("knowledge retrieve top_k must be greater than 0"));
    }
    if request.score_threshold.is_some_and(|value| !(0.0..=1.0).contains(&value)) {
        return Err(ApiError::bad_request(
            "knowledge retrieve score_threshold must be between 0 and 1",
        ));
    }
    if request
        .metadata_filter
        .as_ref()
        .is_some_and(|value| !matches!(value, Value::Null | Value::Object(_)))
    {
        return Err(ApiError::bad_request("knowledge metadata_filter must be an object"));
    }
    Ok(())
}

fn validate_id(value: &str, label: &str) -> Result<(), ApiError> {
    Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| ApiError::bad_request(format!("knowledge {label} is invalid")))
}

fn migrate_schema(conn: &Connection, tenant: &TenantScope) -> Result<(), ApiError> {
    add_column_if_missing(
        conn,
        "knowledge_datasets",
        "chunking_mode",
        "ALTER TABLE knowledge_datasets ADD COLUMN chunking_mode TEXT NOT NULL DEFAULT 'general'",
    )?;
    add_column_if_missing(
        conn,
        "knowledge_datasets",
        "keyword_count",
        "ALTER TABLE knowledge_datasets ADD COLUMN keyword_count INTEGER NOT NULL DEFAULT 10",
    )?;
    add_column_if_missing(
        conn,
        "knowledge_datasets",
        "top_k",
        "ALTER TABLE knowledge_datasets ADD COLUMN top_k INTEGER NOT NULL DEFAULT 10",
    )?;
    add_column_if_missing(
        conn,
        "knowledge_datasets",
        "score_threshold_enabled",
        "ALTER TABLE knowledge_datasets ADD COLUMN score_threshold_enabled INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "knowledge_datasets",
        "score_threshold",
        "ALTER TABLE knowledge_datasets ADD COLUMN score_threshold REAL NOT NULL DEFAULT 0.15",
    )?;
    add_column_if_missing(
        conn,
        "knowledge_datasets",
        "rerank_enabled",
        "ALTER TABLE knowledge_datasets ADD COLUMN rerank_enabled INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "knowledge_documents",
        "status",
        "ALTER TABLE knowledge_documents ADD COLUMN status TEXT NOT NULL DEFAULT 'completed'",
    )?;
    add_column_if_missing(
        conn,
        "knowledge_documents",
        "error_message",
        "ALTER TABLE knowledge_documents ADD COLUMN error_message TEXT NOT NULL DEFAULT ''",
    )?;
    add_column_if_missing(
        conn,
        "knowledge_documents",
        "content_hash",
        "ALTER TABLE knowledge_documents ADD COLUMN content_hash TEXT NOT NULL DEFAULT ''",
    )?;
    add_column_if_missing(
        conn,
        "knowledge_documents",
        "char_count",
        "ALTER TABLE knowledge_documents ADD COLUMN char_count INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "knowledge_documents",
        "recall_count",
        "ALTER TABLE knowledge_documents ADD COLUMN recall_count INTEGER NOT NULL DEFAULT 0",
    )?;
    if !column_exists(conn, "knowledge_documents", "archived_at_ms")? {
        conn.execute("ALTER TABLE knowledge_documents ADD COLUMN archived_at_ms INTEGER", [])
            .map_err(sql_error)?;
    }
    conn.execute(
        "UPDATE knowledge_documents SET char_count = length(content) \
	         WHERE platform_id = ?1 AND company_id = ?2 \
         AND char_count = 0 AND content != ''",
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref()],
    )
    .map_err(sql_error)?;
    add_column_if_missing(
        conn,
        "knowledge_index_jobs",
        "attempt_count",
        "ALTER TABLE knowledge_index_jobs ADD COLUMN attempt_count INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "knowledge_index_jobs",
        "retry_after_ms",
        "ALTER TABLE knowledge_index_jobs ADD COLUMN retry_after_ms INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "knowledge_index_jobs",
        "lease_expires_at_ms",
        "ALTER TABLE knowledge_index_jobs ADD COLUMN lease_expires_at_ms INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "knowledge_chunks",
        "chunk_type",
        "ALTER TABLE knowledge_chunks ADD COLUMN chunk_type TEXT NOT NULL DEFAULT 'text'",
    )?;
    add_column_if_missing(
        conn,
        "knowledge_chunks",
        "start_at",
        "ALTER TABLE knowledge_chunks ADD COLUMN start_at INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "knowledge_chunks",
        "end_at",
        "ALTER TABLE knowledge_chunks ADD COLUMN end_at INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "knowledge_chunks",
        "parent_chunk_id",
        "ALTER TABLE knowledge_chunks ADD COLUMN parent_chunk_id TEXT",
    )?;
    add_column_if_missing(
        conn,
        "knowledge_chunks",
        "prev_chunk_id",
        "ALTER TABLE knowledge_chunks ADD COLUMN prev_chunk_id TEXT",
    )?;
    add_column_if_missing(
        conn,
        "knowledge_chunks",
        "next_chunk_id",
        "ALTER TABLE knowledge_chunks ADD COLUMN next_chunk_id TEXT",
    )?;
    add_column_if_missing(
        conn,
        "knowledge_chunks",
        "context_header",
        "ALTER TABLE knowledge_chunks ADD COLUMN context_header TEXT NOT NULL DEFAULT ''",
    )?;
    add_column_if_missing(
        conn,
        "knowledge_chunks",
        "index_content_hash",
        "ALTER TABLE knowledge_chunks ADD COLUMN index_content_hash TEXT NOT NULL DEFAULT ''",
    )?;
    if !column_exists(conn, "knowledge_chunks", "embedding")? {
        conn.execute("ALTER TABLE knowledge_chunks ADD COLUMN embedding BLOB", [])
            .map_err(sql_error)?;
    }
    Ok(())
}

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    sql: &str,
) -> Result<(), ApiError> {
    if !column_exists(conn, table, column)? {
        conn.execute(sql, []).map_err(sql_error)?;
    }
    Ok(())
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool, ApiError> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})")).map_err(sql_error)?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1)).map_err(sql_error)?;
    for row in rows {
        if row.map_err(sql_error)? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn reject_unscoped_knowledge_schema(conn: &Connection) -> anyhow::Result<()> {
    for table in [
        "knowledge_datasets",
        "knowledge_documents",
        "knowledge_index_jobs",
        "knowledge_chunks",
        "knowledge_chunks_fts",
        "knowledge_embedding_cache",
    ] {
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
            [table],
            |row| row.get(0),
        )?;
        if exists
            && (!column_exists(conn, table, "platform_id")?
                || !column_exists(conn, table, "company_id")?)
        {
            anyhow::bail!(
                "unsupported unscoped knowledge schema: table {table} must contain platform_id and company_id"
            );
        }
    }
    Ok(())
}

fn metadata_or_object(value: Value) -> Value {
    match value {
        Value::Null => Value::Object(Map::new()),
        other => other,
    }
}

fn clean_import_filename(filename: &str) -> Result<String, ApiError> {
    let normalized = filename.trim().replace('\\', "/");
    let name = normalized.rsplit('/').find(|part| !part.trim().is_empty()).unwrap_or("").trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("knowledge import filename is required"));
    }
    Ok(name.to_string())
}

fn normalize_import_mime(mime_type: Option<&str>) -> Option<String> {
    mime_type
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
}

fn import_source_bytes(content_base64: &str) -> Result<u64, ApiError> {
    let encoded = content_base64.trim();
    let max_encoded = ((super::import::MAX_IMPORT_FILE_BYTES + 2) / 3) * 4;
    if encoded.len() > max_encoded {
        return Err(ApiError::bad_request(format!(
            "knowledge import file is too large: max {} bytes",
            super::import::MAX_IMPORT_FILE_BYTES
        )));
    }
    let bytes = base64::engine::general_purpose::STANDARD.decode(encoded).map_err(|error| {
        ApiError::bad_request(format!("knowledge import content_base64 is invalid: {error}"))
    })?;
    if bytes.len() > super::import::MAX_IMPORT_FILE_BYTES {
        return Err(ApiError::bad_request(format!(
            "knowledge import file is too large: max {} bytes",
            super::import::MAX_IMPORT_FILE_BYTES
        )));
    }
    Ok(u64::try_from(bytes.len()).unwrap_or(u64::MAX))
}

fn chunk_metadata(
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

fn metadata_matches(metadata: &Value, filter: &Option<Value>) -> bool {
    let Some(filter) = filter.as_ref().filter(|value| !value.is_null()) else {
        return true;
    };
    let (Value::Object(metadata), Value::Object(filter)) = (metadata, filter) else {
        return metadata == filter;
    };
    filter.iter().all(|(key, value)| metadata.get(key) == Some(value))
}

fn dataset_filter_sql(alias: &str, dataset_count: usize) -> String {
    if dataset_count == 0 {
        return String::new();
    }
    let placeholders = (0..dataset_count).map(|_| "?").collect::<Vec<_>>().join(", ");
    format!(" AND {alias}.dataset_id IN ({placeholders})")
}

fn fts_query_from_text(query: &str) -> String {
    query
        .split_whitespace()
        .map(quote_fts_term)
        .filter(|term| !term.is_empty())
        .collect::<Vec<_>>()
        .join(" OR ")
}

fn quote_fts_term(term: &str) -> String {
    let escaped = term.trim().replace('"', "\"\"");
    if escaped.is_empty() { String::new() } else { format!("\"{escaped}\"") }
}

fn like_pattern(query: &str) -> String {
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

fn like_score(chunk: &KnowledgeChunkDto, query: &str) -> f32 {
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

fn fts_score(rank: f64) -> f32 {
    #[allow(clippy::cast_possible_truncation)]
    let score = 1.0 / (1.0 + rank.abs());
    score as f32
}

fn read_embedding_cache_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    hash: &str,
) -> Result<Option<Vec<f32>>, ApiError> {
    if database.provider() == DatabaseProvider::Postgres {
        return PostgresKnowledgeStore::new(database)
            .and_then(|store| store.read_embedding_cache(tenant, hash))
            .map(|embedding| embedding.map(|bytes| vector::bytes_to_vec(&bytes)))
            .map_err(database_error);
    }
    let conn = database.connect().map_err(database_error)?;
    let blob = conn
        .query_row(
            "SELECT embedding FROM knowledge_embedding_cache \
             WHERE platform_id = ?1 AND company_id = ?2 AND content_hash = ?3",
            params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), hash],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
        .map_err(sql_error)?;
    if let Some(blob) = blob {
        conn.execute(
            "UPDATE knowledge_embedding_cache SET accessed_at_ms = ?1 \
             WHERE platform_id = ?2 AND company_id = ?3 AND content_hash = ?4",
            params![now_ms(), tenant.platform_id().as_ref(), tenant.company_id().as_ref(), hash,],
        )
        .map_err(sql_error)?;
        return Ok(Some(vector::bytes_to_vec(&blob)));
    }
    Ok(None)
}

fn write_embedding_cache_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    hash: &str,
    embedding: Vec<u8>,
    cache_size: usize,
) -> Result<(), ApiError> {
    if database.provider() == DatabaseProvider::Postgres {
        return PostgresKnowledgeStore::new(database)
            .and_then(|store| store.write_embedding_cache(tenant, hash, embedding, cache_size))
            .map_err(database_error);
    }
    let conn = database.connect().map_err(database_error)?;
    let now = now_ms();
    conn.execute(
        "INSERT OR REPLACE INTO knowledge_embedding_cache \
	         (platform_id, company_id, content_hash, embedding, created_at_ms, accessed_at_ms) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            tenant.platform_id().as_ref(),
            tenant.company_id().as_ref(),
            hash,
            embedding,
            now,
            now,
        ],
    )
    .map_err(sql_error)?;
    let cache_size = i64::try_from(cache_size).unwrap_or(i64::MAX);
    conn.execute(
        "DELETE FROM knowledge_embedding_cache \
         WHERE platform_id = ?1 AND company_id = ?2 AND content_hash IN (
	            SELECT content_hash FROM knowledge_embedding_cache
	            WHERE platform_id = ?1 AND company_id = ?2
	            ORDER BY accessed_at_ms ASC
	            LIMIT MAX(0, (
                SELECT COUNT(*) FROM knowledge_embedding_cache
                WHERE platform_id = ?1 AND company_id = ?2
            ) - ?3)
	        )",
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), cache_size],
    )
    .map_err(sql_error)?;
    Ok(())
}

fn content_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hex::encode(hasher.finalize())
}

fn unicode_char_count(text: &str) -> u64 {
    u64::try_from(text.chars().count()).unwrap_or(u64::MAX)
}

fn bool_to_i64(value: bool) -> i64 {
    if value { 1 } else { 0 }
}

fn dataset_capabilities(
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

fn dataset_needs_embedding(dataset: &KnowledgeDatasetDto) -> bool {
    dataset.indexing_mode == KnowledgeIndexingMode::HighQuality
        || dataset.retrieval_mode != KnowledgeRetrievalMode::FullText
}

fn final_document_status(enabled: bool) -> &'static str {
    if enabled {
        document_status_as_str(&KnowledgeDocumentStatus::Completed)
    } else {
        document_status_as_str(&KnowledgeDocumentStatus::Disabled)
    }
}

fn document_status_as_str(status: &KnowledgeDocumentStatus) -> &'static str {
    match status {
        KnowledgeDocumentStatus::Pending => "pending",
        KnowledgeDocumentStatus::Processing => "processing",
        KnowledgeDocumentStatus::Completed => "completed",
        KnowledgeDocumentStatus::Failed => "failed",
        KnowledgeDocumentStatus::Disabled => "disabled",
        KnowledgeDocumentStatus::Archived => "archived",
    }
}

fn index_job_status_as_str(status: &KnowledgeIndexJobStatus) -> &'static str {
    match status {
        KnowledgeIndexJobStatus::Queued => "queued",
        KnowledgeIndexJobStatus::Processing => "processing",
        KnowledgeIndexJobStatus::Completed => "completed",
        KnowledgeIndexJobStatus::Failed => "failed",
    }
}

fn parse_document_status(value: &str, enabled: bool) -> KnowledgeDocumentStatus {
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

fn parse_index_job_status(value: &str) -> KnowledgeIndexJobStatus {
    match value {
        "processing" => KnowledgeIndexJobStatus::Processing,
        "completed" => KnowledgeIndexJobStatus::Completed,
        "failed" => KnowledgeIndexJobStatus::Failed,
        _ => KnowledgeIndexJobStatus::Queued,
    }
}

fn chunk_type_as_str(chunk_type: &KnowledgeChunkType) -> &'static str {
    match chunk_type {
        KnowledgeChunkType::Text => "text",
        KnowledgeChunkType::ParentText => "parent_text",
        KnowledgeChunkType::Qa => "qa",
    }
}

fn parse_chunk_type(value: &str) -> KnowledgeChunkType {
    match value {
        "parent_text" => KnowledgeChunkType::ParentText,
        "qa" => KnowledgeChunkType::Qa,
        _ => KnowledgeChunkType::Text,
    }
}

fn matched_content_from_metadata(metadata: &Value) -> Option<String> {
    metadata
        .get("child_content")
        .or_else(|| metadata.get("question"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn chunking_mode_as_str(mode: &KnowledgeChunkingMode) -> &'static str {
    match mode {
        KnowledgeChunkingMode::General => "general",
        KnowledgeChunkingMode::ParentChild => "parent_child",
        KnowledgeChunkingMode::Qa => "qa",
    }
}

fn indexing_mode_as_str(mode: &KnowledgeIndexingMode) -> &'static str {
    match mode {
        KnowledgeIndexingMode::Economy => "economy",
        KnowledgeIndexingMode::HighQuality => "high_quality",
    }
}

fn retrieval_mode_as_str(mode: &KnowledgeRetrievalMode) -> &'static str {
    match mode {
        KnowledgeRetrievalMode::FullText => "full_text",
        KnowledgeRetrievalMode::Vector => "vector",
        KnowledgeRetrievalMode::Hybrid => "hybrid",
    }
}

fn parse_chunking_mode(value: &str) -> KnowledgeChunkingMode {
    match value {
        "parent_child" => KnowledgeChunkingMode::ParentChild,
        "qa" => KnowledgeChunkingMode::Qa,
        _ => KnowledgeChunkingMode::General,
    }
}

fn parse_indexing_mode(value: &str) -> KnowledgeIndexingMode {
    match value {
        "high_quality" => KnowledgeIndexingMode::HighQuality,
        _ => KnowledgeIndexingMode::Economy,
    }
}

fn parse_retrieval_mode(value: &str) -> KnowledgeRetrievalMode {
    match value {
        "vector" => KnowledgeRetrievalMode::Vector,
        "hybrid" => KnowledgeRetrievalMode::Hybrid,
        _ => KnowledgeRetrievalMode::FullText,
    }
}

fn read_u64(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<u64> {
    let value: i64 = row.get(index)?;
    Ok(u64::try_from(value).unwrap_or_default())
}

fn read_usize(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<usize> {
    let value: i64 = row.get(index)?;
    Ok(usize::try_from(value).unwrap_or_default())
}

fn read_optional_u64(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<Option<u64>> {
    let value: Option<i64> = row.get(index)?;
    Ok(value.map(|value| u64::try_from(value).unwrap_or_default()))
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn sql_error(error: rusqlite::Error) -> ApiError {
    ApiError::internal(format!("knowledge sqlite error: {error}"))
}

fn database_error(error: anyhow::Error) -> ApiError {
    ApiError::internal(format!("knowledge database error: {error}"))
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod store_tests;
