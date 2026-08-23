// 验证知识存储抽象的后端选择、输入校验与公共契约。
use super::super::fusion::{RetrievalSource, RetrievedCandidate};
use super::*;
use crate::global_database::GlobalDatabase;
use crate::global_database::config::DatabaseProvider;
use crate::global_database::test_support::PostgresTestDatabase;
use crate::memory::embeddings::EmbeddingProvider;
use crate::memory::vector;
use crate::workflow::{WorkflowKnowledgeProvider, WorkflowKnowledgeRequest};
use async_trait::async_trait;
use axum::http::StatusCode;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use vw_api_types::TenantScope;
use vw_api_types::knowledge::{
    KnowledgeCapabilitiesDto, KnowledgeChunkDto, KnowledgeChunkType, KnowledgeChunkingMode,
    KnowledgeDatasetCreateRequest, KnowledgeDatasetDto, KnowledgeDocumentCreateRequest,
    KnowledgeDocumentImportRequest, KnowledgeDocumentPatchRequest, KnowledgeDocumentStatus,
    KnowledgeDocumentsBatchAction, KnowledgeDocumentsBatchActionRequest, KnowledgeIndexJobStatus,
    KnowledgeIndexingMode, KnowledgeMatchType, KnowledgeRetrievalMode, KnowledgeRetrieveRequest,
};

fn db_path() -> PathBuf {
    tempfile::tempdir().expect("tempdir").keep().join("knowledge.sqlite")
}

fn store_for_path(path: PathBuf) -> SqliteKnowledgeStore {
    store_for_path_and_tenant(path, TenantScope::default())
}

fn store_for_path_and_tenant(path: PathBuf, tenant: TenantScope) -> SqliteKnowledgeStore {
    SqliteKnowledgeStore::new(GlobalDatabase::new(path).expect("global database"), tenant)
        .expect("knowledge store")
}

fn store() -> SqliteKnowledgeStore {
    store_for_path(db_path())
}

#[test]
fn knowledge_postgres_provider_is_wired() {
    let _ = postgres::PostgresKnowledgeStore::new;
}

#[test]
fn knowledge_postgres_indexing_and_cache_providers_are_wired() {
    let _ = postgres::PostgresKnowledgeStore::enqueue_import;
    let _ = postgres::PostgresKnowledgeStore::claim_next_index_job;
    let _ = postgres::PostgresKnowledgeStore::complete_index_job;
    let _ = postgres::PostgresKnowledgeStore::retry_or_fail_index_job;
    let _ = postgres::PostgresKnowledgeStore::reindex_document;
    let _ = postgres::PostgresKnowledgeStore::read_embedding_cache;
    let _ = postgres::PostgresKnowledgeStore::write_embedding_cache;
}

#[test]
fn knowledge_postgres_retrieval_provider_is_wired() {
    let _ = postgres::PostgresKnowledgeStore::resolve_retrieval_scope;
    let _ = postgres::PostgresKnowledgeStore::retrieve;
}

#[tokio::test]
async fn knowledge_retrieval_modes_filters_and_safe_inputs_run_by_provider() {
    let sqlite = GlobalDatabase::new(db_path()).expect("SQLite global database");
    assert_knowledge_retrieval_modes_filters_and_safe_inputs(sqlite).await;

    let postgres = crate::global_database::test_support::postgres_test_database_unit!();
    assert_knowledge_retrieval_modes_filters_and_safe_inputs(postgres.database().clone()).await;
    postgres.cleanup().expect("clean PostgreSQL test database");
}

async fn assert_knowledge_retrieval_modes_filters_and_safe_inputs(database: GlobalDatabase) {
    let tenant = TenantScope::new("platform-retrieval", "company-primary").expect("tenant");
    let other_tenant =
        TenantScope::new("platform-retrieval", "company-other").expect("other tenant");
    let provider = Arc::new(CountingEmbedding::default());
    let mut store = vector_store_for_database(provider.clone(), database.clone(), tenant.clone());
    store.embedding_request_interval = std::time::Duration::ZERO;
    let mut other_store = vector_store_for_database(provider, database, other_tenant);
    other_store.embedding_request_interval = std::time::Duration::ZERO;

    let full_text = store
        .create_dataset(dataset_request("Full text retrieval"))
        .await
        .expect("full text dataset");
    let full_text_document = store
        .create_document(
            full_text.id.clone(),
            document_request(
                "Alpha refund policy",
                "Refund alpha details include a seven day window. 中文检索内容。",
                json!({ "kind": "support", "rank": 1 }),
            ),
        )
        .await
        .expect("full text document");
    store
        .create_document(
            full_text.id.clone(),
            document_request(
                "Secondary refund policy",
                "Refund beta details are documented separately.",
                json!({ "kind": "other", "rank": 2 }),
            ),
        )
        .await
        .expect("secondary full text document");

    let mut vector_request = dataset_request("Vector retrieval");
    vector_request.indexing_mode = KnowledgeIndexingMode::HighQuality;
    vector_request.retrieval_mode = KnowledgeRetrievalMode::Vector;
    vector_request.embedding_model = Some("fake-embedding".to_string());
    let vector_dataset = store.create_dataset(vector_request).await.expect("vector dataset");
    store
        .create_document(
            vector_dataset.id.clone(),
            document_request(
                "Vector refund",
                "Return requests map to the refund vector.",
                json!({ "kind": "support" }),
            ),
        )
        .await
        .expect("vector refund document");
    store
        .create_document(
            vector_dataset.id.clone(),
            document_request(
                "Vector image",
                "Catalog image settings map to the media vector.",
                json!({ "kind": "media" }),
            ),
        )
        .await
        .expect("vector image document");

    let mut hybrid_request = dataset_request("Hybrid retrieval");
    hybrid_request.indexing_mode = KnowledgeIndexingMode::HighQuality;
    hybrid_request.retrieval_mode = KnowledgeRetrievalMode::Hybrid;
    hybrid_request.keyword_count = 1;
    hybrid_request.embedding_model = Some("fake-embedding".to_string());
    let hybrid_dataset = store.create_dataset(hybrid_request).await.expect("hybrid dataset");
    store
        .create_document(
            hybrid_dataset.id.clone(),
            document_request(
                "Hybrid refund",
                "Refund requests use the hybrid retrieval path.",
                json!({ "kind": "support" }),
            ),
        )
        .await
        .expect("hybrid refund document");
    store
        .create_document(
            hybrid_dataset.id.clone(),
            document_request(
                "Hybrid return",
                "Return windows are matched by vector similarity.",
                json!({ "kind": "support" }),
            ),
        )
        .await
        .expect("hybrid return document");

    let keyword_request = KnowledgeRetrieveRequest {
        query: "refund".to_string(),
        dataset_ids: vec![full_text.id.clone()],
        top_k: 1,
        score_threshold: Some(0.0),
        metadata_filter: Some(json!({ "kind": "support" })),
        include_context: false,
    };
    let keyword = store.retrieve(keyword_request.clone()).await.expect("keyword retrieve");
    assert_eq!(keyword.chunks.len(), 1);
    assert_eq!(keyword.chunks[0].document_id, full_text_document.id);
    assert_eq!(keyword.chunks[0].dataset_id, full_text.id);
    assert_eq!(keyword.chunks[0].match_type, KnowledgeMatchType::FullText);
    let keyword_repeat = store.retrieve(keyword_request).await.expect("repeat keyword retrieve");
    assert_eq!(keyword_repeat.chunks[0].id, keyword.chunks[0].id);

    let quoted = store
        .retrieve(KnowledgeRetrieveRequest {
            query: "\"refund\"".to_string(),
            dataset_ids: vec![full_text.id.clone()],
            top_k: 5,
            score_threshold: Some(0.0),
            metadata_filter: None,
            include_context: false,
        })
        .await
        .expect("quoted keyword input");
    assert!(quoted.chunks.iter().all(|chunk| chunk.dataset_id == full_text.id));
    let chinese = store
        .retrieve(KnowledgeRetrieveRequest {
            query: "中文检索内容".to_string(),
            dataset_ids: vec![full_text.id.clone()],
            top_k: 5,
            score_threshold: Some(0.0),
            metadata_filter: None,
            include_context: false,
        })
        .await
        .expect("Chinese keyword input");
    assert!(!chinese.chunks.is_empty());
    assert!(chinese.chunks.iter().all(|chunk| chunk.dataset_id == full_text.id));
    let wildcard = store
        .retrieve(KnowledgeRetrieveRequest {
            query: "%_".to_string(),
            dataset_ids: vec![full_text.id.clone()],
            top_k: 50,
            score_threshold: Some(0.0),
            metadata_filter: None,
            include_context: false,
        })
        .await
        .expect("escaped wildcard input");
    assert!(wildcard.chunks.is_empty());
    let malicious = store
        .retrieve(KnowledgeRetrieveRequest {
            query: "%' OR TRUE; SELECT pg_sleep(10); -- _".to_string(),
            dataset_ids: vec![full_text.id.clone()],
            top_k: 50,
            score_threshold: Some(0.0),
            metadata_filter: None,
            include_context: false,
        })
        .await
        .expect("parameterized malicious-looking input");
    assert!(malicious.chunks.is_empty());

    let vector = store
        .retrieve(KnowledgeRetrieveRequest {
            query: "How are returns handled?".to_string(),
            dataset_ids: vec![vector_dataset.id.clone()],
            top_k: 1,
            score_threshold: Some(0.0),
            metadata_filter: Some(json!({ "kind": "support" })),
            include_context: false,
        })
        .await
        .expect("vector retrieve");
    assert_eq!(vector.chunks.len(), 1);
    assert_eq!(vector.chunks[0].title, "Vector refund");
    assert_eq!(vector.chunks[0].match_type, KnowledgeMatchType::Vector);

    let hybrid_request = KnowledgeRetrieveRequest {
        query: "refund".to_string(),
        dataset_ids: vec![hybrid_dataset.id.clone()],
        top_k: 2,
        score_threshold: Some(0.0),
        metadata_filter: None,
        include_context: false,
    };
    let hybrid = store.retrieve(hybrid_request.clone()).await.expect("hybrid retrieve");
    assert_eq!(hybrid.chunks.len(), 2);
    assert_eq!(hybrid.chunks[0].match_type, KnowledgeMatchType::Hybrid);
    assert!(hybrid.chunks.iter().all(|chunk| chunk.dataset_id == hybrid_dataset.id));
    let hybrid_repeat = store.retrieve(hybrid_request).await.expect("repeat hybrid retrieve");
    assert_eq!(
        hybrid_repeat.chunks.iter().map(|chunk| &chunk.id).collect::<Vec<_>>(),
        hybrid.chunks.iter().map(|chunk| &chunk.id).collect::<Vec<_>>()
    );
    let threshold = hybrid.chunks[0].score.unwrap_or_default() + f64::EPSILON;
    let above_threshold = store
        .retrieve(KnowledgeRetrieveRequest {
            query: "refund".to_string(),
            dataset_ids: vec![hybrid_dataset.id],
            top_k: 2,
            score_threshold: Some(threshold),
            metadata_filter: None,
            include_context: false,
        })
        .await
        .expect("threshold retrieve");
    assert!(above_threshold.chunks.is_empty());

    other_store
        .create_document(
            other_store
                .create_dataset(dataset_request("Other tenant"))
                .await
                .expect("other tenant dataset")
                .id,
            document_request("Other", "Catalog image content only.", json!({})),
        )
        .await
        .expect("other tenant document");
    let isolated = other_store
        .retrieve(KnowledgeRetrieveRequest {
            query: "refund".to_string(),
            dataset_ids: Vec::new(),
            top_k: 50,
            score_threshold: Some(0.0),
            metadata_filter: None,
            include_context: false,
        })
        .await
        .expect("other tenant retrieve");
    assert!(isolated.chunks.is_empty());

    assert_api_error(
        store
            .retrieve(KnowledgeRetrieveRequest {
                query: "  ".to_string(),
                dataset_ids: vec![full_text.id],
                top_k: 1,
                score_threshold: None,
                metadata_filter: None,
                include_context: false,
            })
            .await
            .expect_err("empty query must be rejected"),
        StatusCode::BAD_REQUEST,
        "query is required",
    );
}

#[tokio::test]
async fn knowledge_indexing_chunks_and_cache_run_by_provider() {
    let sqlite = GlobalDatabase::new(db_path()).expect("SQLite global database");
    assert_knowledge_indexing_chunks_and_cache(sqlite).await;

    let postgres = crate::global_database::test_support::postgres_test_database_unit!();
    assert_knowledge_indexing_chunks_and_cache(postgres.database().clone()).await;
    postgres.cleanup().expect("clean PostgreSQL test database");
}

async fn assert_knowledge_indexing_chunks_and_cache(database: GlobalDatabase) {
    let tenant = TenantScope::new("platform-index", "company-index").expect("index tenant");
    let other = TenantScope::new("platform-index", "company-other").expect("other tenant");
    let provider = Arc::new(CountingEmbedding::default());
    let store = vector_store_for_database(provider.clone(), database.clone(), tenant.clone());
    let _other_store = vector_store_for_database(provider, database.clone(), other.clone());
    let mut request = dataset_request("Provider indexing");
    request.indexing_mode = KnowledgeIndexingMode::HighQuality;
    request.retrieval_mode = KnowledgeRetrievalMode::Hybrid;
    request.embedding_model = Some("fake-embedding".to_string());
    let dataset = store.create_dataset(request).await.expect("dataset");
    let queued = store
        .enqueue_import(
            dataset.id,
            KnowledgeDocumentImportRequest {
                filename: "provider.md".to_string(),
                mime_type: Some("text/markdown".to_string()),
                content_base64:
                    "IyBQcm92aWRlcgoKUmVmdW5kIHBvbGljeSByZXR1cm4gd2luZG93IGlzIHNldmVuIGRheXMu"
                        .to_string(),
                metadata: json!({ "source": "provider-matrix" }),
                enabled: true,
            },
        )
        .await
        .expect("queued import");

    let completed =
        store.process_next_index_job().await.expect("process job").expect("completed job");
    assert_eq!(completed.status, KnowledgeIndexJobStatus::Completed);
    let chunks = store.list_chunks(queued.document.id.clone()).await.expect("list chunks");
    assert!(!chunks.is_empty());
    assert_eq!(
        count_document_rows(&database, "knowledge_chunks", &tenant, &queued.document.id),
        chunks.len() as i64
    );
    assert_eq!(
        count_document_rows(&database, "knowledge_chunks_fts", &tenant, &queued.document.id,),
        chunks.len() as i64
    );
    let reindexed =
        store.reindex_document(queued.document.id.clone()).await.expect("reindex document");
    assert_eq!(reindexed.chunk_count, chunks.len() as u64);
    assert_eq!(
        count_document_rows(&database, "knowledge_chunks", &tenant, &queued.document.id),
        chunks.len() as i64
    );
    assert_eq!(
        count_document_rows(&database, "knowledge_chunks_fts", &tenant, &queued.document.id,),
        chunks.len() as i64
    );

    let retry = store
        .enqueue_import(
            reindexed.dataset_id.clone(),
            KnowledgeDocumentImportRequest {
                filename: "retry.txt".to_string(),
                mime_type: Some("text/plain".to_string()),
                content_base64: "dW5pcXVlIHJldHJ5IGVtYmVkZGluZyBwYXlsb2Fk".to_string(),
                metadata: json!({}),
                enabled: true,
            },
        )
        .await
        .expect("queued retry import");
    let failing_store = SqliteKnowledgeStore::with_embedder(
        database.clone(),
        tenant.clone(),
        Arc::new(FailingEmbedding),
        Some("fake-embedding".to_string()),
        0.75,
        0.25,
        64,
    )
    .expect("failing provider store");
    for attempt in 1..INDEX_JOB_MAX_ATTEMPTS {
        let retried = failing_store
            .process_next_index_job()
            .await
            .expect("retry failed embedding")
            .expect("retried job");
        assert_eq!(retried.status, KnowledgeIndexJobStatus::Queued);
        assert!(retried.error_message.contains("retrying"));
        assert_eq!(
            index_job_attempt_count_for_database(&database, &tenant, &retry.job.id),
            attempt
        );
        make_index_job_retry_ready_for_database(&database, &tenant, &retry.job.id);
    }
    let failed = failing_store
        .process_next_index_job()
        .await
        .expect("fail exhausted embedding job")
        .expect("failed job");
    assert_eq!(failed.status, KnowledgeIndexJobStatus::Failed);
    assert!(failed.finished_at_ms.is_some());
    assert!(failed.error_message.contains("embedding failed"));
    assert_eq!(count_document_rows(&database, "knowledge_chunks", &tenant, &retry.document.id), 0);
    assert_eq!(
        count_document_rows(&database, "knowledge_chunks_fts", &tenant, &retry.document.id,),
        0
    );

    write_embedding_cache_blocking(
        database.clone(),
        &tenant,
        "old",
        vector::vec_to_bytes(&[1.0, 0.0]),
        2,
    )
    .expect("write old tenant cache");
    std::thread::sleep(std::time::Duration::from_millis(2));
    write_embedding_cache_blocking(
        database.clone(),
        &other,
        "old",
        vector::vec_to_bytes(&[0.5, 0.5]),
        1,
    )
    .expect("write other tenant cache");
    write_embedding_cache_blocking(
        database.clone(),
        &tenant,
        "new",
        vector::vec_to_bytes(&[0.0, 1.0]),
        1,
    )
    .expect("write new tenant cache");
    assert!(
        read_embedding_cache_blocking(database.clone(), &tenant, "old")
            .expect("read pruned tenant cache")
            .is_none()
    );
    assert_eq!(
        read_embedding_cache_blocking(database.clone(), &tenant, "new")
            .expect("read current tenant cache")
            .expect("current tenant cache value"),
        vec![0.0, 1.0]
    );
    assert_eq!(
        read_embedding_cache_blocking(database, &other, "old")
            .expect("read other tenant cache")
            .expect("other tenant cache value"),
        vec![0.5, 0.5]
    );
}

#[tokio::test]
async fn knowledge_index_job_claim_is_exclusive_by_provider() {
    let sqlite = GlobalDatabase::new(db_path()).expect("SQLite global database");
    assert_knowledge_index_job_claim_is_exclusive(sqlite).await;

    let postgres = crate::global_database::test_support::postgres_test_database_unit!();
    assert_knowledge_index_job_claim_is_exclusive(postgres.database().clone()).await;
    postgres.cleanup().expect("clean PostgreSQL test database");
}

#[tokio::test]
async fn knowledge_chunk_replace_rolls_back_by_provider() {
    let sqlite = GlobalDatabase::new(db_path()).expect("SQLite global database");
    assert_knowledge_chunk_replace_rolls_back(sqlite).await;

    let postgres = crate::global_database::test_support::postgres_test_database_unit!();
    assert_knowledge_chunk_replace_rolls_back(postgres.database().clone()).await;
    postgres.cleanup().expect("clean PostgreSQL test database");
}

async fn assert_knowledge_chunk_replace_rolls_back(database: GlobalDatabase) {
    let tenant =
        TenantScope::new("platform-rollback", "company-rollback").expect("rollback tenant");
    let store =
        SqliteKnowledgeStore::new(database.clone(), tenant.clone()).expect("rollback store");
    let dataset =
        store.create_dataset(dataset_request("Rollback replacement")).await.expect("dataset");
    let document = store
        .create_document(
            dataset.id,
            document_request(
                "Rollback",
                &"preserve existing chunk rows during replacement failure. ".repeat(40),
                json!({}),
            ),
        )
        .await
        .expect("document");
    let chunks_before = count_document_rows(&database, "knowledge_chunks", &tenant, &document.id);
    let fts_before = count_document_rows(&database, "knowledge_chunks_fts", &tenant, &document.id);
    install_failing_knowledge_chunk_trigger(&database);

    let error =
        store.reindex_document(document.id.clone()).await.expect_err("forced chunk insert failure");

    assert!(error.to_string().contains("forced knowledge chunk insert failure"));
    assert_eq!(
        count_document_rows(&database, "knowledge_chunks", &tenant, &document.id),
        chunks_before
    );
    assert_eq!(
        count_document_rows(&database, "knowledge_chunks_fts", &tenant, &document.id),
        fts_before
    );
    let failed = store.list_documents(document.dataset_id).await.expect("failed document");
    assert_eq!(failed[0].status, KnowledgeDocumentStatus::Failed);
}

async fn assert_knowledge_index_job_claim_is_exclusive(database: GlobalDatabase) {
    let tenant = TenantScope::new("platform-claim", "company-claim").expect("claim tenant");
    let store = SqliteKnowledgeStore::new(database.clone(), tenant.clone()).expect("claim store");
    let dataset = store.create_dataset(dataset_request("Exclusive claim")).await.expect("dataset");
    store
        .enqueue_import(
            dataset.id,
            KnowledgeDocumentImportRequest {
                filename: "claim.txt".to_string(),
                mime_type: Some("text/plain".to_string()),
                content_base64: "ZXhjbHVzaXZlIGNsYWlt".to_string(),
                metadata: json!({}),
                enabled: true,
            },
        )
        .await
        .expect("queued import");

    let barrier = Arc::new(std::sync::Barrier::new(2));
    let first_database = database.clone();
    let first_tenant = tenant.clone();
    let first_barrier = barrier.clone();
    let second_database = database;
    let second_tenant = tenant;
    let (first, second) = std::thread::scope(|scope| {
        let first = scope.spawn(move || {
            first_barrier.wait();
            claim_next_index_job_blocking(first_database, first_tenant)
                .expect("first claim")
                .is_some()
        });
        let second = scope.spawn(move || {
            barrier.wait();
            claim_next_index_job_blocking(second_database, second_tenant)
                .expect("second claim")
                .is_some()
        });
        (first.join().expect("first worker"), second.join().expect("second worker"))
    });
    assert_eq!(usize::from(first) + usize::from(second), 1);
}

#[tokio::test]
async fn knowledge_base_store_round_trips_and_isolates_tenants_by_provider() {
    let sqlite = GlobalDatabase::new(db_path()).expect("SQLite global database");
    assert_knowledge_base_store_round_trip(sqlite).await;

    let postgres = crate::global_database::test_support::postgres_test_database_unit!();
    assert_knowledge_base_store_round_trip(postgres.database().clone()).await;
    postgres.cleanup().expect("clean PostgreSQL test database");
}

async fn assert_knowledge_base_store_round_trip(database: GlobalDatabase) {
    let tenant_a = TenantScope::new("platform-a", "company-a").expect("tenant A");
    let tenant_b = TenantScope::new("platform-a", "company-b").expect("tenant B");
    let store_a = SqliteKnowledgeStore::new(database.clone(), tenant_a).expect("tenant A store");
    let store_b = SqliteKnowledgeStore::new(database.clone(), tenant_b).expect("tenant B store");

    let dataset = store_a
        .create_dataset(dataset_request("Tenant A dataset"))
        .await
        .expect("create tenant A dataset");
    assert!(store_b.list_datasets().await.expect("list tenant B datasets").is_empty());
    assert!(store_b.get_dataset(dataset.id.clone()).await.is_err());

    let document = store_a
        .create_document(
            dataset.id.clone(),
            document_request("Round trip", "tenant-scoped content", json!({ "kind": "test" })),
        )
        .await
        .expect("create tenant A document");
    let completed = store_a
        .list_documents_with_query(
            dataset.id.clone(),
            KnowledgeDocumentListQuery { status: Some(KnowledgeDocumentStatus::Completed) },
        )
        .await
        .expect("filter completed documents");
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].id, document.id);
    assert_eq!(completed[0].metadata, json!({ "kind": "test" }));
    assert!(store_b.list_documents(dataset.id.clone()).await.is_err());

    let updated = store_a
        .update_document(
            document.id.clone(),
            KnowledgeDocumentPatchRequest {
                name: Some("Updated round trip".to_string()),
                enabled: None,
                archived: None,
            },
        )
        .await
        .expect("update tenant A document");
    assert_eq!(updated.name, "Updated round trip");

    let deleted = store_a.delete_document(document.id).await.expect("delete tenant A document");
    assert_eq!(deleted.name, "Updated round trip");
    store_a
        .create_document(
            dataset.id.clone(),
            document_request("Cascade", "cascade content", Value::Null),
        )
        .await
        .expect("create cascading document");

    store_a.delete_dataset(dataset.id.clone()).await.expect("delete tenant A dataset");
    assert_eq!(count_dataset_documents(&database, &dataset.id), 0);
}

fn count_dataset_documents(database: &GlobalDatabase, dataset_id: &str) -> i64 {
    match database.provider() {
        DatabaseProvider::Sqlite => database
            .connect_sqlite()
            .expect("SQLite knowledge connection")
            .query_row(
                "SELECT COUNT(*) FROM knowledge_documents WHERE dataset_id = ?1",
                [dataset_id],
                |row| row.get(0),
            )
            .expect("count SQLite knowledge documents"),
        DatabaseProvider::Postgres => database
            .connect_postgres()
            .expect("PostgreSQL knowledge connection")
            .query_one(
                "SELECT COUNT(*) FROM knowledge_documents WHERE dataset_id = $1",
                &[&dataset_id],
            )
            .expect("count PostgreSQL knowledge documents")
            .get(0),
    }
}

fn count_document_rows(
    database: &GlobalDatabase,
    table: &str,
    tenant: &TenantScope,
    document_id: &str,
) -> i64 {
    match database.provider() {
        DatabaseProvider::Sqlite => database
            .connect_sqlite()
            .expect("SQLite knowledge connection")
            .query_row(
                &format!(
                    "SELECT COUNT(*) FROM {table} \
                     WHERE platform_id = ?1 AND company_id = ?2 AND document_id = ?3"
                ),
                params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), document_id,],
                |row| row.get(0),
            )
            .expect("count SQLite knowledge rows"),
        DatabaseProvider::Postgres => {
            let mut connection =
                database.connect_postgres().expect("PostgreSQL knowledge connection");
            connection
                .query_one(
                    &format!(
                        "SELECT COUNT(*) FROM {table} \
                         WHERE platform_id = $1 AND company_id = $2 AND document_id = $3"
                    ),
                    &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &document_id],
                )
                .expect("count PostgreSQL knowledge rows")
                .get(0)
        }
    }
}

fn index_job_attempt_count_for_database(
    database: &GlobalDatabase,
    tenant: &TenantScope,
    job_id: &str,
) -> i64 {
    match database.provider() {
        DatabaseProvider::Sqlite => database
            .connect_sqlite()
            .expect("SQLite knowledge connection")
            .query_row(
                "SELECT attempt_count FROM knowledge_index_jobs
                 WHERE platform_id = ?1 AND company_id = ?2 AND id = ?3",
                params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), job_id,],
                |row| row.get(0),
            )
            .expect("read SQLite knowledge job attempts"),
        DatabaseProvider::Postgres => database
            .connect_postgres()
            .expect("PostgreSQL knowledge connection")
            .query_one(
                "SELECT attempt_count FROM knowledge_index_jobs
                 WHERE platform_id = $1 AND company_id = $2 AND id = $3",
                &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &job_id],
            )
            .expect("read PostgreSQL knowledge job attempts")
            .get(0),
    }
}

fn make_index_job_retry_ready_for_database(
    database: &GlobalDatabase,
    tenant: &TenantScope,
    job_id: &str,
) {
    match database.provider() {
        DatabaseProvider::Sqlite => {
            database
                .connect_sqlite()
                .expect("SQLite knowledge connection")
                .execute(
                    "UPDATE knowledge_index_jobs SET retry_after_ms = 0
                     WHERE platform_id = ?1 AND company_id = ?2 AND id = ?3",
                    params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), job_id,],
                )
                .expect("make SQLite knowledge job retry ready");
        }
        DatabaseProvider::Postgres => {
            database
                .connect_postgres()
                .expect("PostgreSQL knowledge connection")
                .execute(
                    "UPDATE knowledge_index_jobs SET retry_after_ms = 0
                     WHERE platform_id = $1 AND company_id = $2 AND id = $3",
                    &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &job_id],
                )
                .expect("make PostgreSQL knowledge job retry ready");
        }
    }
}

fn install_failing_knowledge_chunk_trigger(database: &GlobalDatabase) {
    match database.provider() {
        DatabaseProvider::Sqlite => database
            .connect_sqlite()
            .expect("SQLite knowledge connection")
            .execute_batch(
                "CREATE TRIGGER fail_knowledge_chunk_insert
                 BEFORE INSERT ON knowledge_chunks
                 BEGIN
                     SELECT RAISE(ABORT, 'forced knowledge chunk insert failure');
                 END;",
            )
            .expect("install SQLite failing knowledge chunk trigger"),
        DatabaseProvider::Postgres => database
            .connect_postgres()
            .expect("PostgreSQL knowledge connection")
            .batch_execute(
                "CREATE FUNCTION fail_knowledge_chunk_insert() RETURNS trigger
                 LANGUAGE plpgsql AS $$
                 BEGIN
                     RAISE EXCEPTION 'forced knowledge chunk insert failure';
                 END;
                 $$;
                 CREATE TRIGGER fail_knowledge_chunk_insert
                 BEFORE INSERT ON knowledge_chunks
                 FOR EACH ROW EXECUTE FUNCTION fail_knowledge_chunk_insert();",
            )
            .expect("install PostgreSQL failing knowledge chunk trigger"),
    }
}

#[test]
fn knowledge_postgres_store_initializes_six_scoped_tables() {
    let fixture = crate::global_database::test_support::postgres_test_database_unit!();
    let database = fixture.database().clone();
    SqliteKnowledgeStore::new(database.clone(), TenantScope::default())
        .expect("PostgreSQL knowledge store");

    let rows = database
        .connect_postgres()
        .expect("PostgreSQL knowledge connection")
        .query(
            "SELECT table_name FROM information_schema.tables
             WHERE table_schema = current_schema() AND table_name LIKE 'knowledge_%'
             ORDER BY table_name ASC",
            &[],
        )
        .expect("list PostgreSQL knowledge tables");
    let tables = rows.iter().map(|row| row.get::<_, String>(0)).collect::<Vec<_>>();
    assert_eq!(
        tables,
        vec![
            "knowledge_chunks",
            "knowledge_chunks_fts",
            "knowledge_datasets",
            "knowledge_documents",
            "knowledge_embedding_cache",
            "knowledge_index_jobs",
        ]
    );
    fixture.cleanup().expect("clean PostgreSQL test database");
}

#[test]
fn knowledge_postgres_store_rejects_unscoped_schema_without_mutation() {
    let fixture = crate::global_database::test_support::postgres_test_database_unit!();
    let database = fixture.database().clone();
    database
        .connect_postgres()
        .expect("PostgreSQL knowledge connection")
        .batch_execute(
            "CREATE TABLE knowledge_datasets (
                 id TEXT PRIMARY KEY NOT NULL,
                 name TEXT NOT NULL,
                 description TEXT NOT NULL
             );
             INSERT INTO knowledge_datasets (id, name, description)
             VALUES ('legacy-id', 'Legacy', 'preserve me');",
        )
        .expect("unscoped PostgreSQL knowledge schema");

    let error = match SqliteKnowledgeStore::new(database.clone(), TenantScope::default()) {
        Ok(_) => panic!("unscoped PostgreSQL knowledge schema must be rejected"),
        Err(error) => error,
    };
    assert!(error.to_string().contains(
        "unsupported unscoped knowledge schema: table knowledge_datasets must contain platform_id and company_id"
    ));

    let row = database
        .connect_postgres()
        .expect("unchanged PostgreSQL knowledge connection")
        .query_one("SELECT name, description FROM knowledge_datasets WHERE id = 'legacy-id'", &[])
        .expect("legacy PostgreSQL knowledge row remains");
    assert_eq!(row.get::<_, String>(0), "Legacy");
    assert_eq!(row.get::<_, String>(1), "preserve me");
    fixture.cleanup().expect("clean PostgreSQL test database");
}

fn vector_store(provider: Arc<CountingEmbedding>, db_path: PathBuf) -> SqliteKnowledgeStore {
    vector_store_for_database(
        provider,
        GlobalDatabase::new(db_path).expect("global database"),
        TenantScope::default(),
    )
}

fn vector_store_for_database(
    provider: Arc<CountingEmbedding>,
    database: GlobalDatabase,
    tenant: TenantScope,
) -> SqliteKnowledgeStore {
    SqliteKnowledgeStore::with_embedder(
        database,
        tenant,
        provider,
        Some("fake-embedding".to_string()),
        0.75,
        0.25,
        64,
    )
    .expect("knowledge store")
}

fn open_db(path: &Path) -> anyhow::Result<Connection> {
    GlobalDatabase::new(path.to_path_buf())?.connect()
}

fn dataset_request(name: &str) -> KnowledgeDatasetCreateRequest {
    KnowledgeDatasetCreateRequest {
        name: name.to_string(),
        description: String::new(),
        chunking_mode: KnowledgeChunkingMode::General,
        indexing_mode: KnowledgeIndexingMode::Economy,
        retrieval_mode: KnowledgeRetrievalMode::FullText,
        keyword_count: 10,
        top_k: 10,
        score_threshold_enabled: false,
        score_threshold: 0.15,
        rerank_enabled: false,
        embedding_model: None,
        rerank_model: None,
    }
}

fn document_request(name: &str, content: &str, metadata: Value) -> KnowledgeDocumentCreateRequest {
    KnowledgeDocumentCreateRequest {
        name: name.to_string(),
        content: content.to_string(),
        metadata,
        enabled: true,
    }
}

fn dataset_dto(chunking_mode: KnowledgeChunkingMode) -> KnowledgeDatasetDto {
    KnowledgeDatasetDto {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Dataset".to_string(),
        description: String::new(),
        capabilities: KnowledgeCapabilitiesDto::default(),
        chunking_mode,
        indexing_mode: KnowledgeIndexingMode::Economy,
        retrieval_mode: KnowledgeRetrievalMode::FullText,
        keyword_count: 10,
        top_k: 10,
        score_threshold_enabled: false,
        score_threshold: 0.15,
        rerank_enabled: false,
        embedding_model: None,
        rerank_model: None,
        document_count: 0,
        chunk_count: 0,
        created_at_ms: 1,
        updated_at_ms: 1,
    }
}

fn chunk(id: &str, dataset_id: &str, title: &str, content: &str, score: f64) -> KnowledgeChunkDto {
    KnowledgeChunkDto {
        id: id.to_string(),
        dataset_id: dataset_id.to_string(),
        document_id: uuid::Uuid::new_v4().to_string(),
        title: title.to_string(),
        content: content.to_string(),
        metadata: json!({}),
        score: Some(score),
        ordinal: 0,
        chunk_type: KnowledgeChunkType::Text,
        start_at: 0,
        end_at: 0,
        parent_chunk_id: None,
        prev_chunk_id: None,
        next_chunk_id: None,
        matched_content: None,
        match_type: KnowledgeMatchType::FullText,
        context: None,
    }
}

fn retrieved_candidate(id: &str, score: f32, source: RetrievalSource) -> RetrievedCandidate {
    RetrievedCandidate { id: id.to_string(), score, source }
}

fn assert_api_error(error: ApiError, status: StatusCode, needle: &str) {
    assert_eq!(error.status, status);
    assert!(error.to_string().contains(needle), "expected '{needle}' in '{}'", error);
}

#[derive(Default)]
struct CountingEmbedding {
    calls: AtomicUsize,
}

#[async_trait]
impl EmbeddingProvider for CountingEmbedding {
    fn name(&self) -> &str {
        "counting"
    }

    fn dimensions(&self) -> usize {
        3
    }

    async fn embed(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        self.calls.fetch_add(texts.len(), Ordering::SeqCst);
        Ok(texts.iter().map(|text| fake_vector(text)).collect())
    }
}

#[derive(Default)]
struct FailingEmbedding;

#[async_trait]
impl EmbeddingProvider for FailingEmbedding {
    fn name(&self) -> &str {
        "failing"
    }

    fn dimensions(&self) -> usize {
        3
    }

    async fn embed(&self, _texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        anyhow::bail!("synthetic embedding failure")
    }
}

fn fake_vector(text: &str) -> Vec<f32> {
    let text = text.to_ascii_lowercase();
    if text.contains("refund") || text.contains("return") {
        vec![1.0, 0.0, 0.0]
    } else if text.contains("image") || text.contains("catalog") {
        vec![0.0, 1.0, 0.0]
    } else {
        vec![0.0, 0.0, 1.0]
    }
}

fn count_rows(path: &std::path::Path, table: &str, document_id: &str) -> i64 {
    let conn = Connection::open(path).expect("open db");
    conn.query_row(
        &format!("SELECT COUNT(*) FROM {table} WHERE document_id = ?1"),
        params![document_id],
        |row| row.get(0),
    )
    .expect("count rows")
}

fn count_embedded_chunks(path: &std::path::Path, document_id: &str) -> i64 {
    let conn = Connection::open(path).expect("open db");
    conn.query_row(
        "SELECT COUNT(*) FROM knowledge_chunks WHERE document_id = ?1 AND embedding IS NOT NULL",
        params![document_id],
        |row| row.get(0),
    )
    .expect("count embedded chunks")
}

fn index_job_payload(path: &std::path::Path, job_id: &str) -> String {
    let conn = Connection::open(path).expect("open db");
    conn.query_row(
        "SELECT payload_json FROM knowledge_index_jobs WHERE id = ?1",
        params![job_id],
        |row| row.get(0),
    )
    .expect("job payload")
}

fn index_job_attempt_count(path: &std::path::Path, job_id: &str) -> i64 {
    let conn = open_db(path).expect("open db");
    conn.query_row(
        "SELECT attempt_count FROM knowledge_index_jobs WHERE id = ?1",
        params![job_id],
        |row| row.get(0),
    )
    .expect("job attempt count")
}

fn chunk_enabled_flags(path: &std::path::Path, document_id: &str) -> Vec<i64> {
    let conn = open_db(path).expect("open db");
    let mut stmt = conn
        .prepare("SELECT enabled FROM knowledge_chunks WHERE document_id = ?1 ORDER BY ordinal ASC")
        .expect("prepare chunk enabled query");
    let rows = stmt
        .query_map(params![document_id], |row| row.get::<_, i64>(0))
        .expect("query chunk enabled");
    rows.collect::<Result<Vec<_>, _>>().expect("collect chunk enabled")
}

fn stored_document_values(
    path: &std::path::Path,
    document_id: &str,
) -> (String, i64, String, i64, i64, Option<i64>) {
    let conn = open_db(path).expect("open db");
    conn.query_row(
        "SELECT name, enabled, status, char_count, recall_count, archived_at_ms \
         FROM knowledge_documents WHERE id = ?1",
        params![document_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
    )
    .expect("document values")
}

fn mark_index_job_processing(
    path: &std::path::Path,
    job_id: &str,
    document_id: &str,
    attempt_count: i64,
    started_at_ms: u64,
    lease_expires_at_ms: u64,
) {
    let conn = open_db(path).expect("open db");
    conn.execute(
        "UPDATE knowledge_index_jobs \
         SET status = ?1, attempt_count = ?2, started_at_ms = ?3, lease_expires_at_ms = ?4, \
             retry_after_ms = 0, finished_at_ms = NULL \
         WHERE id = ?5",
        params![
            index_job_status_as_str(&KnowledgeIndexJobStatus::Processing),
            attempt_count,
            started_at_ms,
            lease_expires_at_ms,
            job_id,
        ],
    )
    .expect("mark job processing");
    conn.execute(
        "UPDATE knowledge_documents SET status = ?1 WHERE id = ?2",
        params![document_status_as_str(&KnowledgeDocumentStatus::Processing), document_id,],
    )
    .expect("mark document processing");
}

fn make_index_job_retry_ready(path: &std::path::Path, job_id: &str) {
    let conn = open_db(path).expect("open db");
    conn.execute(
        "UPDATE knowledge_index_jobs SET retry_after_ms = 0 WHERE id = ?1",
        params![job_id],
    )
    .expect("make retry ready");
}

#[test]
fn status_reflects_embedding_provider_capabilities() {
    let plain = store().status();
    assert!(plain.full_text);
    assert!(!plain.vector);
    assert!(!plain.hybrid);
    assert!(plain.notes.contains_key("vector"));

    let provider = Arc::new(CountingEmbedding::default());
    let vector = vector_store(provider, db_path()).status();
    assert!(vector.vector);
    assert!(vector.hybrid);
    assert!(vector.rerank);
}

#[tokio::test]
async fn knowledge_store_initializes_inside_global_database_without_overwriting_existing_tables() {
    let database = GlobalDatabase::new(db_path()).expect("global database");
    let connection = database.connect().expect("global database connection");
    connection
        .execute_batch(
            "CREATE TABLE existing_domain (id INTEGER PRIMARY KEY, value TEXT NOT NULL);\
             INSERT INTO existing_domain (id, value) VALUES (1, 'preserved');",
        )
        .expect("existing domain schema");
    drop(connection);

    let store = SqliteKnowledgeStore::new(database.clone(), TenantScope::default())
        .expect("knowledge store");
    let dataset = store.create_dataset(dataset_request("Shared database")).await.expect("dataset");
    let document = store
        .create_document(
            dataset.id.clone(),
            document_request("Document", "shared database content", json!({})),
        )
        .await
        .expect("document");

    assert_eq!(
        store.get_dataset(dataset.id).await.expect("stored dataset").name,
        "Shared database"
    );
    assert_eq!(store.list_documents(document.dataset_id).await.expect("stored documents").len(), 1);

    let connection = database.connect().expect("global database connection");
    let preserved: String = connection
        .query_row("SELECT value FROM existing_domain WHERE id = 1", [], |row| row.get(0))
        .expect("preserved existing data");
    let journal_mode: String =
        connection.query_row("PRAGMA journal_mode", [], |row| row.get(0)).expect("journal mode");
    let busy_timeout: i64 =
        connection.query_row("PRAGMA busy_timeout", [], |row| row.get(0)).expect("busy timeout");
    let foreign_keys: i64 =
        connection.query_row("PRAGMA foreign_keys", [], |row| row.get(0)).expect("foreign keys");

    assert_eq!(preserved, "preserved");
    assert_eq!(journal_mode, "wal");
    assert_eq!(busy_timeout, 5_000);
    assert_eq!(foreign_keys, 1);
}

#[tokio::test]
async fn knowledge_store_isolates_all_tables_and_cache_by_tenant() {
    let path = db_path();
    let database = GlobalDatabase::new(path.clone()).expect("global database");
    let tenant_a = TenantScope::new("platform-a", "company-a").expect("tenant a");
    let tenant_b = TenantScope::new("platform-b", "company-b").expect("tenant b");
    let store_a = SqliteKnowledgeStore::with_embedder(
        database.clone(),
        tenant_a.clone(),
        Arc::new(CountingEmbedding::default()),
        Some("fake-embedding".to_string()),
        0.75,
        0.25,
        10,
    )
    .expect("tenant a store");
    let store_b = SqliteKnowledgeStore::with_embedder(
        database.clone(),
        tenant_b.clone(),
        Arc::new(CountingEmbedding::default()),
        Some("fake-embedding".to_string()),
        0.75,
        0.25,
        10,
    )
    .expect("tenant b store");
    let dataset_id = uuid::Uuid::new_v4().to_string();
    let document_id = uuid::Uuid::new_v4().to_string();
    let job_id = uuid::Uuid::new_v4().to_string();
    let chunk_id = uuid::Uuid::new_v4().to_string();
    let cache_hash = content_hash("shared cache input");

    let conn = database.connect().expect("shared database");
    for (tenant, label, content) in [
        (&tenant_a, "Tenant A", "refund alpha isolated content"),
        (&tenant_b, "Tenant B", "image beta isolated content"),
    ] {
        conn.execute(
            "INSERT INTO knowledge_datasets (
                platform_id, company_id, id, name, description, chunking_mode, indexing_mode,
                retrieval_mode, keyword_count, top_k, score_threshold_enabled, score_threshold,
                rerank_enabled, embedding_model, rerank_model, created_at_ms, updated_at_ms
             ) VALUES (?1, ?2, ?3, ?4, '', 'general', 'high_quality', 'hybrid', 10, 10, 0, 0.15,
                       0, 'fake-embedding', NULL, 1, 1)",
            params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), dataset_id, label],
        )
        .expect("seed dataset");
        conn.execute(
            "INSERT INTO knowledge_documents (
                platform_id, company_id, id, dataset_id, name, content, metadata_json, enabled,
                status, error_message, content_hash, char_count, recall_count, archived_at_ms,
                created_at_ms, updated_at_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, '{}', 1, 'completed', '', ?7, ?8, 0, NULL, 1, 1)",
            params![
                tenant.platform_id().as_ref(),
                tenant.company_id().as_ref(),
                document_id,
                dataset_id,
                label,
                content,
                content_hash(content),
                i64::try_from(content.chars().count()).expect("char count"),
            ],
        )
        .expect("seed document");
        conn.execute(
            "INSERT INTO knowledge_index_jobs (
                platform_id, company_id, id, dataset_id, document_id, status, error_message,
                payload_kind, payload_json, created_at_ms, updated_at_ms, started_at_ms,
                attempt_count, retry_after_ms, lease_expires_at_ms, finished_at_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, 'queued', '', 'import', '{}', 1, 1, NULL, 0, 0, 0, NULL)",
            params![
                tenant.platform_id().as_ref(),
                tenant.company_id().as_ref(),
                job_id,
                dataset_id,
                document_id,
            ],
        )
        .expect("seed job");
        conn.execute(
            "INSERT INTO knowledge_chunks (
                platform_id, company_id, id, dataset_id, document_id, ordinal, title, content,
                metadata_json, chunk_type, start_at, end_at, parent_chunk_id, prev_chunk_id,
                next_chunk_id, context_header, index_content_hash, embedding, enabled,
                created_at_ms, updated_at_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7, '{}', 'text', 0, 0, NULL, NULL,
                       NULL, '', ?8, ?9, 1, 1, 1)",
            params![
                tenant.platform_id().as_ref(),
                tenant.company_id().as_ref(),
                chunk_id,
                dataset_id,
                document_id,
                label,
                content,
                content_hash(content),
                vector::vec_to_bytes(&fake_vector(content)),
            ],
        )
        .expect("seed chunk");
        conn.execute(
            "INSERT INTO knowledge_chunks_fts (
                platform_id, company_id, chunk_id, dataset_id, document_id, title, content
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                tenant.platform_id().as_ref(),
                tenant.company_id().as_ref(),
                chunk_id,
                dataset_id,
                document_id,
                label,
                content,
            ],
        )
        .expect("seed fts");
    }
    drop(conn);

    write_embedding_cache_blocking(
        database.clone(),
        &tenant_a,
        &cache_hash,
        vector::vec_to_bytes(&[1.0, 0.0]),
        10,
    )
    .expect("tenant a cache");
    write_embedding_cache_blocking(
        database.clone(),
        &tenant_b,
        &cache_hash,
        vector::vec_to_bytes(&[0.0, 1.0]),
        10,
    )
    .expect("tenant b cache");

    assert_eq!(
        store_a.get_dataset(dataset_id.clone()).await.expect("tenant a dataset").name,
        "Tenant A"
    );
    assert_eq!(
        store_b.get_dataset(dataset_id.clone()).await.expect("tenant b dataset").name,
        "Tenant B"
    );
    assert_eq!(
        store_a.list_documents(dataset_id.clone()).await.expect("tenant a docs")[0].name,
        "Tenant A"
    );
    assert_eq!(
        store_b.list_documents(dataset_id.clone()).await.expect("tenant b docs")[0].name,
        "Tenant B"
    );
    assert_eq!(
        store_a.list_chunks(document_id.clone()).await.expect("tenant a chunks")[0].content,
        "refund alpha isolated content"
    );
    assert_eq!(
        store_b.list_chunks(document_id.clone()).await.expect("tenant b chunks")[0].content,
        "image beta isolated content"
    );

    let retrieve = |query: &str| KnowledgeRetrieveRequest {
        query: query.to_string(),
        dataset_ids: vec![dataset_id.clone()],
        top_k: 10,
        score_threshold: None,
        metadata_filter: None,
        include_context: false,
    };
    assert_eq!(
        store_a.retrieve(retrieve("refund")).await.expect("tenant a retrieve").chunks.len(),
        1
    );
    assert!(
        store_b.retrieve(retrieve("refund")).await.expect("tenant b isolation").chunks.is_empty()
    );
    assert_eq!(
        store_b.retrieve(retrieve("image")).await.expect("tenant b retrieve").chunks.len(),
        1
    );
    assert!(
        store_a.retrieve(retrieve("image")).await.expect("tenant a isolation").chunks.is_empty()
    );
    assert_eq!(
        read_embedding_cache_blocking(database.clone(), &tenant_a, &cache_hash)
            .expect("tenant a cache read")
            .expect("tenant a cached vector"),
        vec![1.0, 0.0]
    );
    assert_eq!(
        read_embedding_cache_blocking(database.clone(), &tenant_b, &cache_hash)
            .expect("tenant b cache read")
            .expect("tenant b cached vector"),
        vec![0.0, 1.0]
    );

    store_a
        .update_document(
            document_id.clone(),
            KnowledgeDocumentPatchRequest {
                name: Some("Tenant A updated".to_string()),
                enabled: None,
                archived: None,
            },
        )
        .await
        .expect("tenant a update");
    assert_eq!(
        store_b.list_documents(dataset_id.clone()).await.expect("tenant b unchanged")[0].name,
        "Tenant B"
    );

    let claimed = claim_next_index_job_blocking(database.clone(), tenant_a.clone())
        .expect("tenant a claim")
        .expect("tenant a job");
    assert_eq!(claimed.id, job_id);
    assert_eq!(
        store_a.get_index_job(job_id.clone()).await.expect("tenant a job").status,
        KnowledgeIndexJobStatus::Processing
    );
    assert_eq!(
        store_b.get_index_job(job_id.clone()).await.expect("tenant b job").status,
        KnowledgeIndexJobStatus::Queued
    );

    store_a.delete_dataset(dataset_id.clone()).await.expect("delete tenant a dataset");
    assert!(store_a.list_datasets().await.expect("tenant a datasets").is_empty());
    assert_eq!(
        store_b.get_dataset(dataset_id.clone()).await.expect("tenant b survives").name,
        "Tenant B"
    );
    assert_eq!(store_b.list_documents(dataset_id).await.expect("tenant b docs survive").len(), 1);
    assert_eq!(store_b.list_chunks(document_id).await.expect("tenant b chunks survive").len(), 1);
    assert_eq!(
        store_b.get_index_job(job_id).await.expect("tenant b job survives").status,
        KnowledgeIndexJobStatus::Queued
    );
}

#[test]
fn validation_rejects_invalid_dataset_document_retrieve_and_ids() {
    let mut body = dataset_request("Dataset");

    body.name = "  ".to_string();
    assert_api_error(
        validate_dataset_request(&body, false, None).unwrap_err(),
        StatusCode::BAD_REQUEST,
        "name is required",
    );

    body = dataset_request("Dataset");
    body.keyword_count = 0;
    assert_api_error(
        validate_dataset_request(&body, false, None).unwrap_err(),
        StatusCode::BAD_REQUEST,
        "keyword_count",
    );

    body = dataset_request("Dataset");
    body.top_k = 0;
    assert_api_error(
        validate_dataset_request(&body, false, None).unwrap_err(),
        StatusCode::BAD_REQUEST,
        "top_k",
    );

    body = dataset_request("Dataset");
    body.score_threshold = 1.5;
    assert_api_error(
        validate_dataset_request(&body, false, None).unwrap_err(),
        StatusCode::BAD_REQUEST,
        "score_threshold",
    );

    body = dataset_request("Dataset");
    body.indexing_mode = KnowledgeIndexingMode::HighQuality;
    assert_api_error(
        validate_dataset_request(&body, false, None).unwrap_err(),
        StatusCode::NOT_IMPLEMENTED,
        "embedding provider",
    );

    body = dataset_request("Dataset");
    body.retrieval_mode = KnowledgeRetrievalMode::Hybrid;
    assert_api_error(
        validate_dataset_request(&body, false, None).unwrap_err(),
        StatusCode::NOT_IMPLEMENTED,
        "vector backend",
    );

    body = dataset_request("Dataset");
    body.embedding_model = Some("other-model".to_string());
    assert_api_error(
        validate_dataset_request(&body, true, Some("fake-embedding")).unwrap_err(),
        StatusCode::BAD_REQUEST,
        "configured model",
    );

    body = dataset_request("Dataset");
    body.embedding_model = Some("fake-embedding".to_string());
    assert_api_error(
        validate_dataset_request(&body, true, None).unwrap_err(),
        StatusCode::BAD_REQUEST,
        "configured memory embedding provider",
    );

    body = dataset_request("Dataset");
    body.rerank_model = Some("remote-reranker".to_string());
    assert_api_error(
        validate_dataset_request(&body, true, Some("fake-embedding")).unwrap_err(),
        StatusCode::BAD_REQUEST,
        "local-rerank-v1",
    );

    body = dataset_request("Dataset");
    body.embedding_model = Some("fake-embedding".to_string());
    body.rerank_model = Some("local-rerank-v1".to_string());
    assert!(validate_dataset_request(&body, true, Some("fake-embedding")).is_ok());

    assert_api_error(
        validate_document_request(&document_request(" ", "content", json!({}))).unwrap_err(),
        StatusCode::BAD_REQUEST,
        "document name",
    );
    assert_api_error(
        validate_document_request(&document_request("Doc", "  ", json!({}))).unwrap_err(),
        StatusCode::BAD_REQUEST,
        "content",
    );
    assert_api_error(
        validate_document_request(&document_request("Doc", "content", json!(["not-object"])))
            .unwrap_err(),
        StatusCode::BAD_REQUEST,
        "metadata",
    );

    let import = KnowledgeDocumentImportRequest {
        filename: "doc.txt".to_string(),
        mime_type: None,
        content_base64: "  ".to_string(),
        metadata: json!({}),
        enabled: true,
    };
    assert_api_error(
        validate_import_enqueue_request(&import).unwrap_err(),
        StatusCode::BAD_REQUEST,
        "content_base64",
    );

    let import = KnowledgeDocumentImportRequest {
        filename: "doc.txt".to_string(),
        mime_type: None,
        content_base64: "aGVsbG8=".to_string(),
        metadata: json!(["not-object"]),
        enabled: true,
    };
    assert_api_error(
        validate_import_enqueue_request(&import).unwrap_err(),
        StatusCode::BAD_REQUEST,
        "metadata",
    );

    let max_encoded = ((crate::knowledge::import::MAX_IMPORT_FILE_BYTES + 2) / 3) * 4;
    assert_api_error(
        import_source_bytes(&"A".repeat(max_encoded + 1)).unwrap_err(),
        StatusCode::BAD_REQUEST,
        "too large",
    );

    assert_api_error(
        validate_retrieve_request(&KnowledgeRetrieveRequest {
            query: " ".to_string(),
            dataset_ids: vec![],
            top_k: 1,
            score_threshold: None,
            metadata_filter: None,
            include_context: false,
        })
        .unwrap_err(),
        StatusCode::BAD_REQUEST,
        "query",
    );
    assert_api_error(
        validate_retrieve_request(&KnowledgeRetrieveRequest {
            query: "refund".to_string(),
            dataset_ids: vec![],
            top_k: 0,
            score_threshold: None,
            metadata_filter: None,
            include_context: false,
        })
        .unwrap_err(),
        StatusCode::BAD_REQUEST,
        "top_k",
    );
    assert_api_error(
        validate_retrieve_request(&KnowledgeRetrieveRequest {
            query: "refund".to_string(),
            dataset_ids: vec![],
            top_k: 1,
            score_threshold: Some(-0.1),
            metadata_filter: None,
            include_context: false,
        })
        .unwrap_err(),
        StatusCode::BAD_REQUEST,
        "score_threshold",
    );
    assert_api_error(
        validate_retrieve_request(&KnowledgeRetrieveRequest {
            query: "refund".to_string(),
            dataset_ids: vec![],
            top_k: 1,
            score_threshold: None,
            metadata_filter: Some(json!(["bad"])),
            include_context: false,
        })
        .unwrap_err(),
        StatusCode::BAD_REQUEST,
        "metadata_filter",
    );
    assert_api_error(
        validate_id("not-a-uuid", "dataset_id").unwrap_err(),
        StatusCode::BAD_REQUEST,
        "dataset_id",
    );
}

#[test]
fn chunk_preparation_covers_general_parent_child_and_qa_modes() {
    let general = prepare_chunks_for_dataset(
        &dataset_dto(KnowledgeChunkingMode::General),
        &document_request("Doc", "alpha beta", json!({ "source": "test" })),
    )
    .expect("general chunks");
    assert_eq!(general.len(), 1);
    assert_eq!(general[0].title, "Doc");
    assert_eq!(general[0].metadata["chunking_mode"], json!("general"));

    let parent_child = prepare_chunks_for_dataset(
        &dataset_dto(KnowledgeChunkingMode::ParentChild),
        &document_request("Manual", &"parent context. child detail. ".repeat(80), Value::Null),
    )
    .expect("parent child chunks");
    assert!(!parent_child.is_empty());
    assert!(parent_child[0].title.starts_with("Manual · "));
    assert_eq!(parent_child[0].metadata["chunking_mode"], json!("parent_child"));
    assert_eq!(parent_child[0].chunk_type, KnowledgeChunkType::ParentText);
    let child = parent_child
        .iter()
        .find(|chunk| chunk.chunk_type == KnowledgeChunkType::Text)
        .expect("parent_child mode should produce child chunks");
    assert!(child.metadata.get("child_content").is_some());

    let qa = prepare_chunks_for_dataset(
        &dataset_dto(KnowledgeChunkingMode::Qa),
        &document_request(
            "FAQ",
            "问题：如何退款？\n答案：七天内处理。\n\nQuestion: Images?\nAnswer: Use catalog settings.",
            Value::Null,
        ),
    )
    .expect("qa chunks");
    assert_eq!(qa.len(), 2);
    assert_eq!(qa[0].title, "如何退款？");
    assert_eq!(qa[0].content, "七天内处理。");

    let paragraph_pairs =
        parse_qa_pairs("First question?\nFirst answer.\n\nSecond question?\nSecond answer.");
    assert_eq!(paragraph_pairs.len(), 2);

    assert_api_error(
        prepare_qa_chunks(&document_request("Bad FAQ", "only one line", Value::Null)).unwrap_err(),
        StatusCode::BAD_REQUEST,
        "question and answer",
    );
}

#[test]
fn metadata_query_and_score_helpers_handle_edge_cases() {
    assert_eq!(metadata_or_object(Value::Null), json!({}));
    assert_eq!(metadata_or_object(json!({ "a": 1 })), json!({ "a": 1 }));

    let metadata = chunk_metadata(
        &json!({ "tag": "support" }),
        "Doc",
        3,
        &json!({ "chunking_mode": "qa", "tag": "override" }),
    );
    assert_eq!(metadata["document_name"], json!("Doc"));
    assert_eq!(metadata["chunk_index"], json!(3));
    assert_eq!(metadata["tag"], json!("override"));

    assert!(metadata_matches(&metadata, &None));
    assert!(metadata_matches(&metadata, &Some(json!({ "tag": "override" }))));
    assert!(!metadata_matches(&metadata, &Some(json!({ "tag": "support" }))));
    assert!(metadata_matches(&json!("x"), &Some(json!("x"))));

    assert_eq!(dataset_filter_sql("c", 0), "");
    assert_eq!(dataset_filter_sql("c", 2), " AND c.dataset_id IN (?, ?)");
    assert_eq!(quote_fts_term(r#"hello"world"#), r#""hello""world""#);
    assert_eq!(fts_query_from_text("alpha  beta"), r#""alpha" OR "beta""#);
    assert_eq!(fts_query_from_text("   "), "");
    assert_eq!(like_pattern(r#"50%_done\ok"#), r#"%50\%\_done\\ok%"#);

    let scored = KnowledgeChunkDto {
        id: "id".into(),
        dataset_id: "dataset".into(),
        document_id: "doc".into(),
        title: "Refund policy".into(),
        content: "Content body".into(),
        metadata: json!({}),
        score: None,
        ordinal: 0,
        chunk_type: KnowledgeChunkType::Text,
        start_at: 0,
        end_at: 0,
        parent_chunk_id: None,
        prev_chunk_id: None,
        next_chunk_id: None,
        matched_content: None,
        match_type: KnowledgeMatchType::FullText,
        context: None,
    };
    assert_eq!(like_score(&scored, "refund"), 0.65);
    assert_eq!(like_score(&scored, "body"), 0.55);
    assert_eq!(like_score(&scored, "missing"), 0.1);
    assert!(fts_score(0.0) > fts_score(3.0));
}

#[test]
fn mode_hash_bool_and_time_helpers_are_stable() {
    assert_eq!(bool_to_i64(true), 1);
    assert_eq!(bool_to_i64(false), 0);
    assert_eq!(chunking_mode_as_str(&KnowledgeChunkingMode::General), "general");
    assert_eq!(chunking_mode_as_str(&KnowledgeChunkingMode::ParentChild), "parent_child");
    assert_eq!(chunking_mode_as_str(&KnowledgeChunkingMode::Qa), "qa");
    assert_eq!(indexing_mode_as_str(&KnowledgeIndexingMode::Economy), "economy");
    assert_eq!(indexing_mode_as_str(&KnowledgeIndexingMode::HighQuality), "high_quality");
    assert_eq!(retrieval_mode_as_str(&KnowledgeRetrievalMode::FullText), "full_text");
    assert_eq!(retrieval_mode_as_str(&KnowledgeRetrievalMode::Vector), "vector");
    assert_eq!(retrieval_mode_as_str(&KnowledgeRetrievalMode::Hybrid), "hybrid");
    assert_eq!(parse_chunking_mode("parent_child"), KnowledgeChunkingMode::ParentChild);
    assert_eq!(parse_chunking_mode("qa"), KnowledgeChunkingMode::Qa);
    assert_eq!(parse_chunking_mode("unknown"), KnowledgeChunkingMode::General);
    assert_eq!(parse_indexing_mode("high_quality"), KnowledgeIndexingMode::HighQuality);
    assert_eq!(parse_indexing_mode("unknown"), KnowledgeIndexingMode::Economy);
    assert_eq!(parse_retrieval_mode("vector"), KnowledgeRetrievalMode::Vector);
    assert_eq!(parse_retrieval_mode("hybrid"), KnowledgeRetrievalMode::Hybrid);
    assert_eq!(parse_retrieval_mode("unknown"), KnowledgeRetrievalMode::FullText);
    assert_eq!(content_hash("abc").len(), 64);
    assert!(now_ms() > 0);
}

#[test]
fn status_capability_and_metadata_helpers_cover_all_domain_variants() {
    assert_eq!(final_document_status(true), "completed");
    assert_eq!(final_document_status(false), "disabled");
    assert_eq!(document_status_as_str(&KnowledgeDocumentStatus::Pending), "pending");
    assert_eq!(document_status_as_str(&KnowledgeDocumentStatus::Processing), "processing");
    assert_eq!(document_status_as_str(&KnowledgeDocumentStatus::Failed), "failed");
    assert_eq!(document_status_as_str(&KnowledgeDocumentStatus::Archived), "archived");
    assert_eq!(parse_document_status("processing", true), KnowledgeDocumentStatus::Processing);
    assert_eq!(parse_document_status("failed", true), KnowledgeDocumentStatus::Failed);
    assert_eq!(parse_document_status("archived", true), KnowledgeDocumentStatus::Archived);
    assert_eq!(parse_document_status("completed", false), KnowledgeDocumentStatus::Disabled);

    assert_eq!(index_job_status_as_str(&KnowledgeIndexJobStatus::Queued), "queued");
    assert_eq!(index_job_status_as_str(&KnowledgeIndexJobStatus::Processing), "processing");
    assert_eq!(index_job_status_as_str(&KnowledgeIndexJobStatus::Completed), "completed");
    assert_eq!(index_job_status_as_str(&KnowledgeIndexJobStatus::Failed), "failed");
    assert_eq!(parse_index_job_status("processing"), KnowledgeIndexJobStatus::Processing);
    assert_eq!(parse_index_job_status("completed"), KnowledgeIndexJobStatus::Completed);
    assert_eq!(parse_index_job_status("failed"), KnowledgeIndexJobStatus::Failed);
    assert_eq!(parse_index_job_status("unknown"), KnowledgeIndexJobStatus::Queued);

    assert_eq!(chunk_type_as_str(&KnowledgeChunkType::Text), "text");
    assert_eq!(chunk_type_as_str(&KnowledgeChunkType::ParentText), "parent_text");
    assert_eq!(chunk_type_as_str(&KnowledgeChunkType::Qa), "qa");
    assert_eq!(parse_chunk_type("parent_text"), KnowledgeChunkType::ParentText);
    assert_eq!(parse_chunk_type("qa"), KnowledgeChunkType::Qa);
    assert_eq!(parse_chunk_type("unknown"), KnowledgeChunkType::Text);

    let economy = dataset_capabilities(
        &KnowledgeIndexingMode::Economy,
        &KnowledgeRetrievalMode::FullText,
        false,
    );
    assert!(economy.full_text);
    assert!(!economy.vector);
    assert!(!economy.hybrid);
    assert!(!economy.rerank);

    let vector = dataset_capabilities(
        &KnowledgeIndexingMode::HighQuality,
        &KnowledgeRetrievalMode::FullText,
        true,
    );
    assert!(vector.vector);
    assert!(vector.hybrid);
    assert!(vector.rerank);
    let economy_dataset = dataset_dto(KnowledgeChunkingMode::General);
    assert!(!dataset_needs_embedding(&economy_dataset));
    let mut vector_dataset = economy_dataset.clone();
    vector_dataset.retrieval_mode = KnowledgeRetrievalMode::Vector;
    assert!(dataset_needs_embedding(&vector_dataset));

    assert_eq!(
        matched_content_from_metadata(&json!({ "child_content": "child" })),
        Some("child".into())
    );
    assert_eq!(
        matched_content_from_metadata(&json!({ "question": "question" })),
        Some("question".into())
    );
    assert_eq!(matched_content_from_metadata(&json!({ "question": "" })), None);
    assert_eq!(matched_content_from_metadata(&json!({})), None);
    assert_eq!(unicode_char_count("a中🙂"), 3);
}

#[test]
fn retrieval_score_helpers_merge_and_rerank_results() {
    let vector = vec![
        retrieved_candidate("a", 0.9, RetrievalSource::Vector),
        retrieved_candidate("b", 0.1, RetrievalSource::Vector),
    ];
    let keyword = vec![
        retrieved_candidate("b", 0.8, RetrievalSource::FullText),
        retrieved_candidate("c", 0.7, RetrievalSource::FullText),
    ];

    let vector_only = merge_retrieval_scores(&vector, &[], 0.7, 0.3, 10);
    assert_eq!(vector_only[0].id, "a");
    assert_eq!(vector_only[0].score, 0.9);
    assert_eq!(vector_only[0].match_type, KnowledgeMatchType::Vector);

    let keyword_only = merge_retrieval_scores(&[], &keyword, 0.7, 0.3, 10);
    assert_eq!(keyword_only[0].id, "b");
    assert_eq!(keyword_only[0].score, 0.8);
    assert_eq!(keyword_only[0].match_type, KnowledgeMatchType::FullText);

    let hybrid = merge_retrieval_scores(&vector, &keyword, 0.7, 0.3, 2);
    assert_eq!(hybrid.len(), 2);
    assert_eq!(hybrid[0].id, "b");
    assert_eq!(hybrid[0].match_type, KnowledgeMatchType::Hybrid);

    let dataset_id = uuid::Uuid::new_v4().to_string();
    let other_dataset = uuid::Uuid::new_v4().to_string();
    let mut chunks = vec![
        chunk("low", &dataset_id, "General", "no exact words", 0.9),
        chunk("lexical", &dataset_id, "Refund policy", "refund terms", 0.4),
        chunk("other", &other_dataset, "Refund policy", "refund terms", 0.2),
    ];
    chunks[1].metadata = json!({ "question": "How do refunds work?" });
    rerank_chunks("refund", &[dataset_id.clone()], &mut chunks);
    assert_eq!(chunks[0].id, "low");
    assert!(chunks[0].score.unwrap_or_default() > 0.4);
    let other = chunks.iter().find(|chunk| chunk.id == "other").expect("other chunk");
    assert_eq!(rerank_score("refund", &[dataset_id], other), other.score.unwrap());
    assert_eq!(lexical_match_score("", &chunks[0]), 0.0);
}

#[test]
fn knowledge_store_rejects_unscoped_legacy_schema_without_mutating_rows() {
    let path = db_path();
    std::fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    {
        let conn = Connection::open(&path).expect("legacy db");
        conn.execute_batch(
            r#"
            CREATE TABLE knowledge_datasets (
                id TEXT NOT NULL,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                indexing_mode TEXT NOT NULL,
                retrieval_mode TEXT NOT NULL,
                embedding_model TEXT,
                rerank_model TEXT,
                created_at_ms INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL
            );
            INSERT INTO knowledge_datasets (
                id, name, description, indexing_mode, retrieval_mode, created_at_ms, updated_at_ms
            ) VALUES ('legacy-id', 'Legacy', 'preserve me', 'economy', 'full_text', 1, 2);
            "#,
        )
        .expect("legacy schema");
    }

    let database = GlobalDatabase::new(path.clone()).expect("global database");
    let error = match SqliteKnowledgeStore::new(database.clone(), TenantScope::default()) {
        Ok(_) => panic!("unscoped schema must be rejected"),
        Err(error) => error,
    };
    assert!(
        error.to_string().contains(
            "unsupported unscoped knowledge schema: table knowledge_datasets must contain platform_id and company_id"
        )
    );

    let conn = database.connect().expect("open unchanged db");
    let row: (String, String) = conn
        .query_row(
            "SELECT name, description FROM knowledge_datasets WHERE id = 'legacy-id'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("legacy row remains");
    assert_eq!(row, ("Legacy".to_string(), "preserve me".to_string()));
    assert!(!column_exists(&conn, "knowledge_datasets", "platform_id").unwrap());
    assert!(!column_exists(&conn, "knowledge_datasets", "chunking_mode").unwrap());
}

#[test]
fn embedding_cache_prunes_only_current_tenant() {
    let sqlite = GlobalDatabase::new(db_path()).expect("SQLite global database");
    assert_embedding_cache_prunes_only_current_tenant(sqlite);

    let postgres = crate::global_database::test_support::postgres_test_database_unit!();
    assert_embedding_cache_prunes_only_current_tenant(postgres.database().clone());
    postgres.cleanup().expect("clean PostgreSQL test database");
}

fn assert_embedding_cache_prunes_only_current_tenant(database: GlobalDatabase) {
    let local = TenantScope::default();
    let other = TenantScope::new("other-platform", "other-company").expect("other tenant");
    let _local_store =
        SqliteKnowledgeStore::new(database.clone(), local.clone()).expect("local store");
    let _other_store =
        SqliteKnowledgeStore::new(database.clone(), other.clone()).expect("other store");

    write_embedding_cache_blocking(
        database.clone(),
        &local,
        "a",
        vector::vec_to_bytes(&[1.0, 0.0]),
        2,
    )
    .expect("write a");
    std::thread::sleep(std::time::Duration::from_millis(2));
    write_embedding_cache_blocking(
        database.clone(),
        &other,
        "a",
        vector::vec_to_bytes(&[0.5, 0.5]),
        1,
    )
    .expect("write other a");
    write_embedding_cache_blocking(
        database.clone(),
        &local,
        "b",
        vector::vec_to_bytes(&[0.0, 1.0]),
        1,
    )
    .expect("write b");
    assert!(read_embedding_cache_blocking(database.clone(), &local, "a").unwrap().is_none());
    assert_eq!(
        read_embedding_cache_blocking(database.clone(), &local, "b").unwrap().unwrap(),
        vec![0.0, 1.0]
    );
    assert_eq!(
        read_embedding_cache_blocking(database, &other, "a").unwrap().unwrap(),
        vec![0.5, 0.5]
    );
}

#[test]
fn open_db_migrates_legacy_document_and_chunk_metadata_columns() {
    let path = db_path();
    std::fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    let dataset_id = uuid::Uuid::new_v4().to_string();
    let document_id = uuid::Uuid::new_v4().to_string();
    let chunk_id = uuid::Uuid::new_v4().to_string();
    let tenant = TenantScope::default();
    let platform_id = tenant.platform_id().as_ref();
    let company_id = tenant.company_id().as_ref();
    {
        let conn = Connection::open(&path).expect("legacy db");
        conn.execute_batch(
            r#"
            CREATE TABLE knowledge_datasets (
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
            CREATE TABLE knowledge_documents (
                platform_id TEXT NOT NULL,
                company_id TEXT NOT NULL,
                id TEXT NOT NULL,
                dataset_id TEXT NOT NULL,
                name TEXT NOT NULL,
                content TEXT NOT NULL,
                metadata_json TEXT NOT NULL,
                enabled INTEGER NOT NULL,
                created_at_ms INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL,
                PRIMARY KEY (platform_id, company_id, id)
            );
            CREATE TABLE knowledge_chunks (
                platform_id TEXT NOT NULL,
                company_id TEXT NOT NULL,
                id TEXT NOT NULL,
                dataset_id TEXT NOT NULL,
                document_id TEXT NOT NULL,
                ordinal INTEGER NOT NULL,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                metadata_json TEXT NOT NULL,
                embedding BLOB,
                enabled INTEGER NOT NULL,
                created_at_ms INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL,
                PRIMARY KEY (platform_id, company_id, id)
            );
            "#,
        )
        .expect("legacy schema");
        conn.execute(
            "INSERT INTO knowledge_datasets \
             (platform_id, company_id, id, name, description, indexing_mode, retrieval_mode, \
              created_at_ms, updated_at_ms) \
             VALUES (?1, ?2, ?3, 'Docs', '', 'economy', 'full_text', 1, 2)",
            params![platform_id, company_id, dataset_id],
        )
        .expect("legacy dataset");
        conn.execute(
            "INSERT INTO knowledge_documents \
             (platform_id, company_id, id, dataset_id, name, content, metadata_json, enabled, \
              created_at_ms, updated_at_ms) \
             VALUES (?1, ?2, ?3, ?4, 'Readme', 'hello', '{}', 1, 1, 2)",
            params![platform_id, company_id, document_id, dataset_id],
        )
        .expect("legacy document");
        conn.execute(
            "INSERT INTO knowledge_chunks \
             (platform_id, company_id, id, dataset_id, document_id, ordinal, title, content, \
              metadata_json, enabled, \
              created_at_ms, updated_at_ms) \
             VALUES (?1, ?2, ?3, ?4, ?5, 4, 'Readme', 'hello', '{}', 1, 1, 2)",
            params![platform_id, company_id, chunk_id, dataset_id, document_id],
        )
        .expect("legacy chunk");
    }

    let _store = store_for_path(path.clone());
    let conn = open_db(&path).expect("open migrated db");
    assert!(column_exists(&conn, "knowledge_documents", "status").unwrap());
    assert!(column_exists(&conn, "knowledge_documents", "error_message").unwrap());
    assert!(column_exists(&conn, "knowledge_documents", "content_hash").unwrap());
    assert!(column_exists(&conn, "knowledge_chunks", "chunk_type").unwrap());
    assert!(column_exists(&conn, "knowledge_chunks", "start_at").unwrap());
    assert!(column_exists(&conn, "knowledge_chunks", "end_at").unwrap());
    assert!(column_exists(&conn, "knowledge_chunks", "parent_chunk_id").unwrap());
    assert!(column_exists(&conn, "knowledge_chunks", "prev_chunk_id").unwrap());
    assert!(column_exists(&conn, "knowledge_chunks", "next_chunk_id").unwrap());
    assert!(column_exists(&conn, "knowledge_chunks", "context_header").unwrap());
    assert!(column_exists(&conn, "knowledge_chunks", "index_content_hash").unwrap());

    let document =
        query_document(&conn, &TenantScope::default(), &document_id).unwrap().expect("document");
    assert_eq!(document.status, KnowledgeDocumentStatus::Completed);
    assert_eq!(document.error_message, "");
    assert_eq!(document.content_hash, "");

    let chunk =
        query_chunk(&conn, &TenantScope::default(), &chunk_id, None, KnowledgeMatchType::FullText)
            .unwrap()
            .expect("chunk");
    assert_eq!(chunk.ordinal, 4);
    assert_eq!(chunk.chunk_type, KnowledgeChunkType::Text);
    assert_eq!(chunk.start_at, 0);
    assert_eq!(chunk.end_at, 0);
    assert!(chunk.parent_chunk_id.is_none());
    assert!(chunk.prev_chunk_id.is_none());
    assert!(chunk.next_chunk_id.is_none());
    assert_eq!(chunk.match_type, KnowledgeMatchType::FullText);
}

#[test]
fn open_db_migrates_document_stats_columns() {
    let path = db_path();
    std::fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    let dataset_id = uuid::Uuid::new_v4().to_string();
    let document_id = uuid::Uuid::new_v4().to_string();
    let tenant = TenantScope::default();
    let platform_id = tenant.platform_id().as_ref();
    let company_id = tenant.company_id().as_ref();
    {
        let conn = Connection::open(&path).expect("legacy db");
        conn.execute_batch(
            r#"
            CREATE TABLE knowledge_datasets (
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
            CREATE TABLE knowledge_documents (
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
                PRIMARY KEY (platform_id, company_id, id)
            );
            "#,
        )
        .expect("legacy schema");
        conn.execute(
            "INSERT INTO knowledge_datasets \
             (platform_id, company_id, id, name, description, indexing_mode, retrieval_mode, \
              created_at_ms, updated_at_ms) \
             VALUES (?1, ?2, ?3, 'Docs', '', 'economy', 'full_text', 1, 2)",
            params![platform_id, company_id, dataset_id],
        )
        .expect("legacy dataset");
        conn.execute(
            "INSERT INTO knowledge_documents \
             (platform_id, company_id, id, dataset_id, name, content, metadata_json, enabled, status, error_message, \
              content_hash, created_at_ms, updated_at_ms) \
             VALUES (?1, ?2, ?3, ?4, 'Readme', '退款abc', '{}', 1, 'completed', '', '', 1, 2)",
            params![platform_id, company_id, document_id, dataset_id],
        )
        .expect("legacy document");
    }

    let _store = store_for_path(path.clone());
    let conn = open_db(&path).expect("open migrated db");
    assert!(column_exists(&conn, "knowledge_documents", "char_count").unwrap());
    assert!(column_exists(&conn, "knowledge_documents", "recall_count").unwrap());
    assert!(column_exists(&conn, "knowledge_documents", "archived_at_ms").unwrap());

    let document =
        query_document(&conn, &TenantScope::default(), &document_id).unwrap().expect("document");
    assert_eq!(document.char_count, 5);
    assert_eq!(document.recall_count, 0);
    assert!(document.archived_at_ms.is_none());
}

#[tokio::test]
async fn document_lifecycle_updates_enabled_archive_name_and_stats() {
    let path = db_path();
    let store = store_for_path(path.clone());
    let dataset = store.create_dataset(dataset_request("Documents")).await.expect("dataset");
    let document = store
        .create_document(
            dataset.id.clone(),
            document_request("  Readme  ", "退款abc", json!({ "source": "manual" })),
        )
        .await
        .expect("document");
    assert_eq!(document.name, "Readme");
    assert_eq!(document.char_count, 5);
    assert_eq!(document.recall_count, 0);
    assert!(document.archived_at_ms.is_none());

    let renamed = store
        .update_document(
            document.id.clone(),
            KnowledgeDocumentPatchRequest {
                name: Some("  Updated Name  ".to_string()),
                enabled: Some(false),
                archived: None,
            },
        )
        .await
        .expect("rename and disable");
    assert_eq!(renamed.name, "Updated Name");
    assert!(!renamed.enabled);
    assert_eq!(renamed.status, KnowledgeDocumentStatus::Disabled);
    assert_eq!(chunk_enabled_flags(&path, &document.id), vec![0]);

    let archived = store
        .update_document(
            document.id.clone(),
            KnowledgeDocumentPatchRequest { name: None, enabled: None, archived: Some(true) },
        )
        .await
        .expect("archive");
    assert_eq!(archived.status, KnowledgeDocumentStatus::Archived);
    assert!(archived.archived_at_ms.is_some());
    assert_eq!(chunk_enabled_flags(&path, &document.id), vec![0]);

    let restored = store
        .update_document(
            document.id.clone(),
            KnowledgeDocumentPatchRequest {
                name: None,
                enabled: Some(true),
                archived: Some(false),
            },
        )
        .await
        .expect("restore");
    assert!(restored.enabled);
    assert_eq!(restored.status, KnowledgeDocumentStatus::Completed);
    assert!(restored.archived_at_ms.is_none());
    assert_eq!(chunk_enabled_flags(&path, &document.id), vec![1]);

    let queued = store
        .enqueue_import(
            dataset.id.clone(),
            KnowledgeDocumentImportRequest {
                filename: "imported.md".to_string(),
                mime_type: Some("text/markdown".to_string()),
                content_base64: "5b2S5qGj5YaF5a65".to_string(),
                metadata: json!({ "source": "import" }),
                enabled: true,
            },
        )
        .await
        .expect("queued import");
    let pending_batch = store
        .batch_update_documents(KnowledgeDocumentsBatchActionRequest {
            document_ids: vec![queued.document.id.clone()],
            action: KnowledgeDocumentsBatchAction::Disable,
        })
        .await;
    assert_api_error(pending_batch.unwrap_err(), StatusCode::BAD_REQUEST, "pending");

    let processed = store.process_next_index_job().await.expect("process import").expect("job");
    assert_eq!(processed.status, KnowledgeIndexJobStatus::Completed);
    let imported = query_document(
        &open_db(&path).expect("open db"),
        &TenantScope::default(),
        &queued.document.id,
    )
    .unwrap()
    .expect("imported doc");
    assert_eq!(imported.char_count, 4);

    let second = store
        .create_document(dataset.id.clone(), document_request("Second", "delta", json!({})))
        .await
        .expect("second");
    let third = store
        .create_document(dataset.id.clone(), document_request("Third", "echo", json!({})))
        .await
        .expect("third");
    let archived_batch = store
        .batch_update_documents(KnowledgeDocumentsBatchActionRequest {
            document_ids: vec![second.id.clone(), third.id.clone()],
            action: KnowledgeDocumentsBatchAction::Archive,
        })
        .await
        .expect("batch archive");
    assert_eq!(archived_batch.len(), 2);
    assert!(
        archived_batch.iter().all(|document| document.status == KnowledgeDocumentStatus::Archived)
    );

    let enabled_batch = store
        .batch_update_documents(KnowledgeDocumentsBatchActionRequest {
            document_ids: vec![second.id.clone(), third.id.clone()],
            action: KnowledgeDocumentsBatchAction::Enable,
        })
        .await
        .expect("batch enable");
    assert_eq!(enabled_batch.len(), 2);
    assert!(enabled_batch.iter().all(|document| document.enabled));
    assert!(
        enabled_batch.iter().all(|document| document.status == KnowledgeDocumentStatus::Completed)
    );

    let (name, enabled, status, char_count, recall_count, archived_at_ms) =
        stored_document_values(&path, &document.id);
    assert_eq!(name, "Updated Name");
    assert_eq!(enabled, 1);
    assert_eq!(status, "completed");
    assert_eq!(char_count, 5);
    assert_eq!(recall_count, 0);
    assert!(archived_at_ms.is_none());
}

#[tokio::test]
async fn retrieve_increments_document_recall_count_for_returned_chunks() {
    let path = db_path();
    let store = store_for_path(path.clone());
    let dataset = store.create_dataset(dataset_request("Recall")).await.expect("dataset");
    let refund = store
        .create_document(
            dataset.id.clone(),
            document_request(
                "Refund",
                &"refund policy return window applies. ".repeat(80),
                json!({ "kind": "refund" }),
            ),
        )
        .await
        .expect("refund");
    let other = store
        .create_document(
            dataset.id.clone(),
            document_request(
                "Catalog",
                "catalog images live elsewhere",
                json!({ "kind": "media" }),
            ),
        )
        .await
        .expect("other");
    assert_eq!(refund.recall_count, 0);
    assert_eq!(other.recall_count, 0);

    let response = store
        .retrieve(KnowledgeRetrieveRequest {
            query: "refund".to_string(),
            dataset_ids: vec![dataset.id.clone()],
            top_k: 3,
            score_threshold: Some(0.0),
            metadata_filter: None,
            include_context: false,
        })
        .await
        .expect("retrieve");
    assert!(!response.chunks.is_empty());

    let recall_count =
        response.chunks.iter().filter(|chunk| chunk.document_id == refund.id).count() as u64;
    assert!(recall_count > 0);
    let conn = open_db(&path).expect("open db");
    let refund_after =
        query_document(&conn, &TenantScope::default(), &refund.id).unwrap().expect("refund doc");
    let other_after =
        query_document(&conn, &TenantScope::default(), &other.id).unwrap().expect("other doc");
    assert_eq!(refund_after.recall_count, recall_count);
    assert_eq!(other_after.recall_count, 0);

    store
        .retrieve(KnowledgeRetrieveRequest {
            query: "refund".to_string(),
            dataset_ids: vec![dataset.id],
            top_k: 1,
            score_threshold: Some(0.0),
            metadata_filter: None,
            include_context: false,
        })
        .await
        .expect("retrieve again");
    let conn = open_db(&path).expect("open db");
    let refund_final = query_document(&conn, &TenantScope::default(), &refund.id)
        .unwrap()
        .expect("refund doc final");
    assert_eq!(refund_final.recall_count, recall_count + 1);
}

#[tokio::test]
async fn dataset_document_lifecycle_trims_counts_lists_and_deletes() {
    let store = store();
    let mut request = dataset_request("  Support  ");
    request.description = "  Help center  ".to_string();
    request.top_k = 500;
    request.score_threshold_enabled = true;
    request.score_threshold = 0.2;
    request.rerank_enabled = true;
    request.rerank_model = Some("local-rerank-v1".to_string());
    let dataset = store.create_dataset(request).await.expect("dataset");

    assert_eq!(dataset.name, "Support");
    assert_eq!(dataset.description, "Help center");
    assert_eq!(dataset.top_k, MAX_TOP_K);
    assert!(dataset.rerank_enabled);
    assert_eq!(dataset.document_count, 0);

    assert_eq!(store.list_datasets().await.unwrap().len(), 1);
    assert_eq!(store.get_dataset(dataset.id.clone()).await.unwrap().id, dataset.id);

    let document = store
        .create_document(
            dataset.id.clone(),
            document_request(
                "  Return policy  ",
                "Refunds are available within seven days.",
                Value::Null,
            ),
        )
        .await
        .expect("document");
    assert_eq!(document.name, "Return policy");
    assert_eq!(document.metadata, json!({}));
    assert_eq!(document.chunk_count, 1);

    let documents = store.list_documents(dataset.id.clone()).await.expect("documents");
    assert_eq!(documents.len(), 1);
    assert_eq!(store.get_dataset(dataset.id.clone()).await.unwrap().document_count, 1);

    let deleted_doc = store.delete_document(document.id.clone()).await.expect("delete doc");
    assert_eq!(deleted_doc.id, document.id);
    assert!(store.list_documents(dataset.id.clone()).await.unwrap().is_empty());

    let deleted_dataset = store.delete_dataset(dataset.id.clone()).await.expect("delete dataset");
    assert_eq!(deleted_dataset.id, dataset.id);
    assert_api_error(
        store.get_dataset(dataset.id).await.unwrap_err(),
        StatusCode::NOT_FOUND,
        "dataset not found",
    );
}

#[tokio::test]
async fn enqueue_import_creates_pending_document_and_queued_job_without_indexing() {
    let provider = Arc::new(CountingEmbedding::default());
    let path = db_path();
    let store = vector_store(provider.clone(), path.clone());
    let mut request = dataset_request("Vector Queue");
    request.indexing_mode = KnowledgeIndexingMode::HighQuality;
    request.retrieval_mode = KnowledgeRetrievalMode::Hybrid;
    request.embedding_model = Some("fake-embedding".to_string());
    let dataset = store.create_dataset(request).await.expect("dataset");

    let queued = store
        .enqueue_import(
            dataset.id.clone(),
            KnowledgeDocumentImportRequest {
                filename: " guide.md ".to_string(),
                mime_type: Some("text/markdown".to_string()),
                content_base64: "UmVmdW5kIHBvbGljeQ==".to_string(),
                metadata: json!({ "source": "desktop" }),
                enabled: true,
            },
        )
        .await
        .expect("queued import");

    assert_eq!(queued.document.dataset_id, dataset.id);
    assert_eq!(queued.document.name, "guide.md");
    assert_eq!(queued.document.metadata, json!({ "source": "desktop" }));
    assert_eq!(queued.document.status, KnowledgeDocumentStatus::Pending);
    assert_eq!(queued.document.chunk_count, 0);
    assert_eq!(queued.job.dataset_id, dataset.id);
    assert_eq!(queued.job.document_id, queued.document.id);
    assert_eq!(queued.job.status, KnowledgeIndexJobStatus::Queued);
    assert_eq!(queued.filename, "guide.md");
    assert_eq!(queued.mime_type.as_deref(), Some("text/markdown"));
    assert_eq!(queued.source_bytes, 13);
    assert!(queued.warnings.is_empty());
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);

    let job = store.get_index_job(queued.job.id.clone()).await.expect("job");
    assert_eq!(job, queued.job);
    assert_eq!(count_rows(&path, "knowledge_chunks", &queued.document.id), 0);
    assert_eq!(count_rows(&path, "knowledge_chunks_fts", &queued.document.id), 0);
}

#[tokio::test]
async fn index_worker_processes_queued_import_to_completed_document() {
    let provider = Arc::new(CountingEmbedding::default());
    let path = db_path();
    let store = vector_store(provider.clone(), path.clone());
    let mut request = dataset_request("Worker Queue");
    request.indexing_mode = KnowledgeIndexingMode::HighQuality;
    request.retrieval_mode = KnowledgeRetrievalMode::Hybrid;
    request.embedding_model = Some("fake-embedding".to_string());
    let dataset = store.create_dataset(request).await.expect("dataset");
    let queued = store
        .enqueue_import(
            dataset.id.clone(),
            KnowledgeDocumentImportRequest {
                filename: "refund.md".to_string(),
                mime_type: Some("text/markdown".to_string()),
                content_base64:
                    "IyBSZWZ1bmQKClJlZnVuZCBwb2xpY3kgcmV0dXJuIHdpbmRvdyBpcyBzZXZlbiBkYXlzLg=="
                        .to_string(),
                metadata: json!({ "source": "desktop" }),
                enabled: true,
            },
        )
        .await
        .expect("queued import");

    let processed =
        store.process_next_index_job().await.expect("process job").expect("processed job");

    assert_eq!(processed.id, queued.job.id);
    assert_eq!(processed.status, KnowledgeIndexJobStatus::Completed);
    assert_eq!(processed.error_message, "");
    assert!(processed.started_at_ms.is_some());
    assert!(processed.finished_at_ms.is_some());
    let conn = open_db(&path).expect("open db");
    let document = query_document(&conn, &TenantScope::default(), &queued.document.id)
        .unwrap()
        .expect("document");
    assert_eq!(document.status, KnowledgeDocumentStatus::Completed);
    assert_eq!(document.error_message, "");
    assert!(document.chunk_count > 0);
    assert_eq!(
        count_rows(&path, "knowledge_chunks_fts", &document.id),
        document.chunk_count as i64
    );
    assert_eq!(count_embedded_chunks(&path, &document.id), document.chunk_count as i64);
    assert!(provider.calls.load(Ordering::SeqCst) > 0);
    assert_eq!(index_job_payload(&path, &queued.job.id), "");

    let retrieved = store
        .retrieve(KnowledgeRetrieveRequest {
            query: "return window".to_string(),
            dataset_ids: vec![dataset.id],
            top_k: 5,
            score_threshold: Some(0.0),
            metadata_filter: Some(json!({ "source": "desktop" })),
            include_context: false,
        })
        .await
        .expect("retrieve");
    assert!(!retrieved.chunks.is_empty());
}

#[tokio::test]
async fn index_worker_retries_embedding_failure_before_marking_failed() {
    let path = db_path();
    let setup_store = vector_store(Arc::new(CountingEmbedding::default()), path.clone());
    let mut request = dataset_request("Worker Failure");
    request.indexing_mode = KnowledgeIndexingMode::HighQuality;
    request.retrieval_mode = KnowledgeRetrievalMode::Hybrid;
    request.embedding_model = Some("fake-embedding".to_string());
    let dataset = setup_store.create_dataset(request).await.expect("dataset");
    let queued = setup_store
        .enqueue_import(
            dataset.id,
            KnowledgeDocumentImportRequest {
                filename: "refund.txt".to_string(),
                mime_type: Some("text/plain".to_string()),
                content_base64: "UmVmdW5kIHJlcXVlc3RzIHJldHVybiBtb25leS4=".to_string(),
                metadata: json!({}),
                enabled: true,
            },
        )
        .await
        .expect("queued import");
    let failing_store = SqliteKnowledgeStore::with_embedder(
        GlobalDatabase::new(path.clone()).expect("global database"),
        TenantScope::default(),
        Arc::new(FailingEmbedding),
        Some("fake-embedding".to_string()),
        0.75,
        0.25,
        64,
    )
    .expect("knowledge store");
    let first =
        failing_store.process_next_index_job().await.expect("first process").expect("first retry");

    assert_eq!(first.id, queued.job.id);
    assert_eq!(first.status, KnowledgeIndexJobStatus::Queued);
    assert!(first.error_message.contains("retrying"));
    assert!(first.started_at_ms.is_some());
    assert!(first.finished_at_ms.is_none());
    assert_eq!(index_job_attempt_count(&path, &queued.job.id), 1);
    let conn = open_db(&path).expect("open db");
    let document = query_document(&conn, &TenantScope::default(), &queued.document.id)
        .unwrap()
        .expect("document");
    assert_eq!(document.status, KnowledgeDocumentStatus::Pending);
    assert!(document.error_message.contains("retrying"));

    make_index_job_retry_ready(&path, &queued.job.id);
    let second = failing_store
        .process_next_index_job()
        .await
        .expect("second process")
        .expect("second retry");
    assert_eq!(second.status, KnowledgeIndexJobStatus::Queued);
    assert_eq!(index_job_attempt_count(&path, &queued.job.id), 2);

    make_index_job_retry_ready(&path, &queued.job.id);
    let failed =
        failing_store.process_next_index_job().await.expect("third process").expect("failed job");
    assert_eq!(failed.id, queued.job.id);
    assert_eq!(failed.status, KnowledgeIndexJobStatus::Failed);
    assert!(failed.error_message.contains("embedding failed"));
    assert!(failed.started_at_ms.is_some());
    assert!(failed.finished_at_ms.is_some());
    let conn = open_db(&path).expect("open db");
    let document = query_document(&conn, &TenantScope::default(), &queued.document.id)
        .unwrap()
        .expect("document");
    assert_eq!(document.status, KnowledgeDocumentStatus::Failed);
    assert!(document.error_message.contains("embedding failed"));
    assert_eq!(count_rows(&path, "knowledge_chunks", &queued.document.id), 0);
    assert_eq!(count_rows(&path, "knowledge_chunks_fts", &queued.document.id), 0);
    assert_eq!(index_job_payload(&path, &queued.job.id), "");
}

#[tokio::test]
async fn index_worker_retries_stale_processing_job() {
    let provider = Arc::new(CountingEmbedding::default());
    let path = db_path();
    let store = vector_store(provider.clone(), path.clone());
    let mut request = dataset_request("Stale Processing");
    request.indexing_mode = KnowledgeIndexingMode::HighQuality;
    request.retrieval_mode = KnowledgeRetrievalMode::Hybrid;
    request.embedding_model = Some("fake-embedding".to_string());
    let dataset = store.create_dataset(request).await.expect("dataset");
    let queued = store
        .enqueue_import(
            dataset.id,
            KnowledgeDocumentImportRequest {
                filename: "stale.txt".to_string(),
                mime_type: Some("text/plain".to_string()),
                content_base64: "UmVmdW5kIHJlcXVlc3RzIHJldHVybiBtb25leS4=".to_string(),
                metadata: json!({}),
                enabled: true,
            },
        )
        .await
        .expect("queued import");
    let stale_started_at = now_ms().saturating_sub(INDEX_JOB_LEASE_TIMEOUT_MS + 1);
    mark_index_job_processing(&path, &queued.job.id, &queued.document.id, 1, stale_started_at, 0);

    let processed = store
        .process_next_index_job()
        .await
        .expect("process stale job")
        .expect("processed stale job");

    assert_eq!(processed.id, queued.job.id);
    assert_eq!(processed.status, KnowledgeIndexJobStatus::Completed);
    assert_eq!(index_job_attempt_count(&path, &queued.job.id), 2);
    let conn = open_db(&path).expect("open db");
    let document = query_document(&conn, &TenantScope::default(), &queued.document.id)
        .unwrap()
        .expect("document");
    assert_eq!(document.status, KnowledgeDocumentStatus::Completed);
    assert!(provider.calls.load(Ordering::SeqCst) > 0);
}

#[tokio::test]
async fn index_worker_fails_stale_processing_job_after_retry_limit() {
    let path = db_path();
    let store = vector_store(Arc::new(CountingEmbedding::default()), path.clone());
    let mut request = dataset_request("Stale Limit");
    request.indexing_mode = KnowledgeIndexingMode::HighQuality;
    request.retrieval_mode = KnowledgeRetrievalMode::Hybrid;
    request.embedding_model = Some("fake-embedding".to_string());
    let dataset = store.create_dataset(request).await.expect("dataset");
    let queued = store
        .enqueue_import(
            dataset.id,
            KnowledgeDocumentImportRequest {
                filename: "stale-limit.txt".to_string(),
                mime_type: Some("text/plain".to_string()),
                content_base64: "UmVmdW5kIHJlcXVlc3RzIHJldHVybiBtb25leS4=".to_string(),
                metadata: json!({}),
                enabled: true,
            },
        )
        .await
        .expect("queued import");
    let now = now_ms();
    mark_index_job_processing(
        &path,
        &queued.job.id,
        &queued.document.id,
        INDEX_JOB_MAX_ATTEMPTS,
        now,
        now.saturating_sub(1),
    );

    let processed = store.process_next_index_job().await.expect("process stale limit");

    assert!(processed.is_none());
    let job = store.get_index_job(queued.job.id.clone()).await.expect("job");
    assert_eq!(job.status, KnowledgeIndexJobStatus::Failed);
    assert!(job.error_message.contains("timed out"));
    assert!(job.finished_at_ms.is_some());
    let conn = open_db(&path).expect("open db");
    let document = query_document(&conn, &TenantScope::default(), &queued.document.id)
        .unwrap()
        .expect("document");
    assert_eq!(document.status, KnowledgeDocumentStatus::Failed);
    assert!(document.error_message.contains("timed out"));
    assert_eq!(index_job_payload(&path, &queued.job.id), "");
}

#[tokio::test]
async fn reindex_document_replaces_chunks_fts_and_embeddings_without_duplicates() {
    let provider = Arc::new(CountingEmbedding::default());
    let path = db_path();
    let store = vector_store(provider, path.clone());
    let mut request = dataset_request("Vector");
    request.indexing_mode = KnowledgeIndexingMode::HighQuality;
    request.retrieval_mode = KnowledgeRetrievalMode::Hybrid;
    request.embedding_model = Some("fake-embedding".to_string());
    let dataset = store.create_dataset(request).await.expect("dataset");
    let document = store
        .create_document(
            dataset.id.clone(),
            document_request(
                "Refunds",
                &"Refund policy section. Return window is seven days.\n\n".repeat(80),
                json!({ "kind": "support" }),
            ),
        )
        .await
        .expect("document");

    let chunk_count = count_rows(&path, "knowledge_chunks", &document.id);
    let fts_count = count_rows(&path, "knowledge_chunks_fts", &document.id);
    assert!(chunk_count > 0);
    assert_eq!(fts_count, chunk_count);

    let reindexed = store.reindex_document(document.id.clone()).await.expect("reindex");
    assert_eq!(reindexed.status, KnowledgeDocumentStatus::Completed);
    assert_eq!(reindexed.chunk_count, u64::try_from(chunk_count).unwrap());
    assert_eq!(count_rows(&path, "knowledge_chunks", &document.id), chunk_count);
    assert_eq!(count_rows(&path, "knowledge_chunks_fts", &document.id), fts_count);

    let retrieved = store
        .retrieve(KnowledgeRetrieveRequest {
            query: "seven days".to_string(),
            dataset_ids: vec![dataset.id],
            top_k: 50,
            score_threshold: Some(0.0),
            metadata_filter: Some(json!({ "kind": "support" })),
            include_context: false,
        })
        .await
        .expect("retrieve");
    let mut ids = retrieved.chunks.iter().map(|chunk| chunk.id.as_str()).collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), retrieved.chunks.len());
}

#[tokio::test]
async fn reindex_embedding_failure_marks_document_failed_without_duplicate_rows() {
    let provider = Arc::new(CountingEmbedding::default());
    let path = db_path();
    let store = vector_store(provider, path.clone());
    let mut request = dataset_request("Vector");
    request.indexing_mode = KnowledgeIndexingMode::HighQuality;
    request.retrieval_mode = KnowledgeRetrievalMode::Hybrid;
    request.embedding_model = Some("fake-embedding".to_string());
    let dataset = store.create_dataset(request).await.expect("dataset");
    let document = store
        .create_document(
            dataset.id,
            document_request("Refunds", "Refund requests return money.", json!({})),
        )
        .await
        .expect("document");
    let chunk_count = count_rows(&path, "knowledge_chunks", &document.id);
    let fts_count = count_rows(&path, "knowledge_chunks_fts", &document.id);
    open_db(&path)
        .expect("open db")
        .execute("DELETE FROM knowledge_embedding_cache", [])
        .expect("clear embedding cache");

    let failing_store = SqliteKnowledgeStore::with_embedder(
        GlobalDatabase::new(path.clone()).expect("global database"),
        TenantScope::default(),
        Arc::new(FailingEmbedding),
        Some("fake-embedding".to_string()),
        0.75,
        0.25,
        64,
    )
    .expect("knowledge store");
    open_db(&path)
        .expect("open document database")
        .execute(
            "UPDATE knowledge_documents SET content = ?1 WHERE id = ?2",
            params!["Refund requests now require revised evidence.", document.id],
        )
        .expect("change content before failed reindex");
    assert_api_error(
        failing_store.reindex_document(document.id.clone()).await.unwrap_err(),
        StatusCode::BAD_REQUEST,
        "embedding failed",
    );

    let conn = open_db(&path).expect("open db");
    let failed =
        query_document(&conn, &TenantScope::default(), &document.id).unwrap().expect("document");
    assert_eq!(failed.status, KnowledgeDocumentStatus::Failed);
    assert!(failed.error_message.contains("embedding failed"));
    assert_eq!(count_rows(&path, "knowledge_chunks", &document.id), chunk_count);
    assert_eq!(count_rows(&path, "knowledge_chunks_fts", &document.id), fts_count);
}

#[tokio::test]
async fn disabled_documents_are_not_retrieved_and_empty_scope_searches_all_datasets() {
    let store = store();
    let disabled_dataset = store.create_dataset(dataset_request("Disabled")).await.unwrap();
    let all_dataset = store.create_dataset(dataset_request("All")).await.unwrap();

    let mut disabled_doc =
        document_request("Hidden", "secret refund policy", json!({ "tag": "hidden" }));
    disabled_doc.enabled = false;
    store.create_document(disabled_dataset.id.clone(), disabled_doc).await.unwrap();
    store
        .create_document(
            all_dataset.id.clone(),
            document_request("Visible", "visible refund policy", json!({ "tag": "visible" })),
        )
        .await
        .unwrap();

    let hidden = store
        .retrieve(KnowledgeRetrieveRequest {
            query: "secret refund".to_string(),
            dataset_ids: vec![disabled_dataset.id],
            top_k: 5,
            score_threshold: None,
            metadata_filter: None,
            include_context: false,
        })
        .await
        .unwrap();
    assert!(hidden.chunks.is_empty());

    let visible = store
        .retrieve(KnowledgeRetrieveRequest {
            query: "visible refund".to_string(),
            dataset_ids: vec![],
            top_k: 5,
            score_threshold: Some(0.0),
            metadata_filter: Some(json!({ "tag": "visible" })),
            include_context: false,
        })
        .await
        .unwrap();
    assert_eq!(visible.chunks.len(), 1);
    assert_eq!(visible.chunks[0].title, "Visible");
}

#[tokio::test]
async fn list_chunks_returns_document_chunks_in_ordinal_order() {
    let store = store();
    let mut request = dataset_request("Parent Child");
    request.chunking_mode = KnowledgeChunkingMode::ParentChild;
    let dataset = store.create_dataset(request).await.expect("dataset");
    let document = store
        .create_document(
            dataset.id,
            document_request("Manual", &"parent context child detail. ".repeat(140), json!({})),
        )
        .await
        .expect("document");

    let chunks = store.list_chunks(document.id.clone()).await.expect("chunks");

    assert_eq!(chunks.len() as u64, document.chunk_count);
    assert_eq!(
        chunks.iter().map(|chunk| chunk.ordinal).collect::<Vec<_>>(),
        (0..chunks.len()).collect::<Vec<_>>()
    );
    assert_eq!(
        chunks.first().map(|chunk| &chunk.chunk_type),
        Some(&KnowledgeChunkType::ParentText)
    );
    assert!(chunks.iter().any(|chunk| chunk.parent_chunk_id.is_some()));
    assert!(chunks.iter().all(|chunk| chunk.context.is_none()));
}

#[tokio::test]
async fn retrieval_include_context_adds_parent_and_nearby_text() {
    let store = store();

    let mut parent_request = dataset_request("Parent Context");
    parent_request.chunking_mode = KnowledgeChunkingMode::ParentChild;
    let parent_dataset = store.create_dataset(parent_request).await.expect("parent dataset");
    let parent_document = store
        .create_document(
            parent_dataset.id.clone(),
            document_request(
                "Manual",
                &format!(
                    "{} UNIQUE_PARENT_CHILD_NEEDLE child explanation. {}",
                    "parent opening context. ".repeat(50),
                    "parent closing context. ".repeat(50)
                ),
                json!({}),
            ),
        )
        .await
        .expect("parent document");

    let parent_response = store
        .retrieve(KnowledgeRetrieveRequest {
            query: "UNIQUE_PARENT_CHILD_NEEDLE".to_string(),
            dataset_ids: vec![parent_dataset.id],
            top_k: 1,
            score_threshold: Some(0.0),
            metadata_filter: None,
            include_context: true,
        })
        .await
        .expect("parent retrieve");
    let parent_chunk = parent_response.chunks.first().expect("parent result");
    assert_eq!(parent_chunk.document_id, parent_document.id);
    assert!(parent_chunk.parent_chunk_id.is_some());
    assert!(
        parent_chunk
            .matched_content
            .as_deref()
            .is_some_and(|text| { text.contains("UNIQUE_PARENT_CHILD_NEEDLE") })
    );
    assert!(parent_chunk.context.as_deref().is_some_and(|text| {
        text.contains("parent opening context") && text.contains("parent closing context")
    }));

    let flat_dataset =
        store.create_dataset(dataset_request("Flat Context")).await.expect("flat dataset");
    store
        .create_document(
            flat_dataset.id.clone(),
            document_request(
                "Flat",
                &format!(
                    "{} {} {}",
                    "PREVIOUS_CONTEXT ".repeat(80),
                    "UNIQUE_FLAT_CONTEXT_NEEDLE ".repeat(4),
                    "NEXT_CONTEXT ".repeat(80)
                ),
                json!({}),
            ),
        )
        .await
        .expect("flat document");
    let flat_response = store
        .retrieve(KnowledgeRetrieveRequest {
            query: "UNIQUE_FLAT_CONTEXT_NEEDLE".to_string(),
            dataset_ids: vec![flat_dataset.id],
            top_k: 1,
            score_threshold: Some(0.0),
            metadata_filter: None,
            include_context: true,
        })
        .await
        .expect("flat retrieve");
    let flat_context = flat_response.chunks[0].context.as_deref().expect("flat context");
    assert!(flat_context.contains("PREVIOUS_CONTEXT"));
    assert!(flat_context.contains("UNIQUE_FLAT_CONTEXT_NEEDLE"));
    assert!(flat_context.contains("NEXT_CONTEXT"));
}

#[test]
fn append_with_overlap_removes_text_overlap_without_trusting_offsets() {
    let merged = append_with_overlap(
        "alpha intro shared overlap phrase",
        "shared overlap phrase omega outro",
        0,
    );

    assert_eq!(merged, "alpha intro shared overlap phrase omega outro");
}

#[tokio::test]
async fn vector_retrieval_uses_embedding_cache_metadata_and_thresholds() {
    let provider = Arc::new(CountingEmbedding::default());
    let store = vector_store(provider.clone(), db_path());
    let mut request = dataset_request("Vector");
    request.indexing_mode = KnowledgeIndexingMode::HighQuality;
    request.retrieval_mode = KnowledgeRetrievalMode::Hybrid;
    request.embedding_model = Some("fake-embedding".to_string());
    let dataset = store.create_dataset(request).await.expect("dataset");

    store
        .create_document(
            dataset.id.clone(),
            document_request(
                "Refunds",
                "Refund requests return money to customers.",
                json!({ "kind": "support" }),
            ),
        )
        .await
        .expect("refund doc");
    store
        .create_document(
            dataset.id.clone(),
            document_request(
                "Images",
                "Catalog image setup belongs to media settings.",
                json!({ "kind": "media" }),
            ),
        )
        .await
        .expect("image doc");

    let calls_before_query = provider.calls.load(Ordering::SeqCst);
    let request = KnowledgeRetrieveRequest {
        query: "refund".to_string(),
        dataset_ids: vec![dataset.id.clone()],
        top_k: 1,
        score_threshold: Some(0.0),
        metadata_filter: Some(json!({ "kind": "support" })),
        include_context: false,
    };
    let response = store.retrieve(request.clone()).await.expect("retrieve");
    assert_eq!(response.chunks.len(), 1);
    assert_eq!(response.chunks[0].title, "Refunds");
    assert!(provider.calls.load(Ordering::SeqCst) > calls_before_query);

    let calls_after_first_query = provider.calls.load(Ordering::SeqCst);
    let cached = store.retrieve(request).await.expect("cached retrieve");
    assert_eq!(cached.chunks.len(), 1);
    assert_eq!(provider.calls.load(Ordering::SeqCst), calls_after_first_query);

    let filtered = store
        .retrieve(KnowledgeRetrieveRequest {
            query: "refund".to_string(),
            dataset_ids: vec![dataset.id],
            top_k: 1,
            score_threshold: Some(1.1_f64.min(1.0)),
            metadata_filter: Some(json!({ "kind": "media" })),
            include_context: false,
        })
        .await
        .expect("filtered retrieve");
    assert!(filtered.chunks.is_empty());
}

#[tokio::test]
async fn hybrid_retrieval_uses_rrf_keyword_count_and_stable_match_types() {
    let provider = Arc::new(CountingEmbedding::default());
    let store = vector_store(provider, db_path());
    let mut request = dataset_request("Hybrid");
    request.indexing_mode = KnowledgeIndexingMode::HighQuality;
    request.retrieval_mode = KnowledgeRetrievalMode::Hybrid;
    request.keyword_count = 1;
    request.embedding_model = Some("fake-embedding".to_string());
    let dataset = store.create_dataset(request).await.expect("dataset");

    store
        .create_document(
            dataset.id.clone(),
            document_request("Refund primary", "Refund policy allows returns.", json!({})),
        )
        .await
        .expect("primary doc");
    store
        .create_document(
            dataset.id.clone(),
            document_request("Refund secondary", "Refund workflow tracks tickets.", json!({})),
        )
        .await
        .expect("secondary doc");
    store
        .create_document(
            dataset.id.clone(),
            document_request("Returns", "Return windows are seven days.", json!({})),
        )
        .await
        .expect("return doc");

    let response = store
        .retrieve(KnowledgeRetrieveRequest {
            query: "refund".to_string(),
            dataset_ids: vec![dataset.id],
            top_k: 3,
            score_threshold: Some(0.0),
            metadata_filter: None,
            include_context: false,
        })
        .await
        .expect("retrieve");

    assert_eq!(response.chunks.len(), 3);
    assert_eq!(response.chunks[0].match_type, KnowledgeMatchType::Hybrid);
    assert_eq!(
        response
            .chunks
            .iter()
            .filter(|chunk| chunk.match_type == KnowledgeMatchType::Hybrid)
            .count(),
        1
    );
    assert!(response.chunks.iter().any(|chunk| chunk.match_type == KnowledgeMatchType::Vector));
    assert!(response.chunks.iter().all(|chunk| chunk.score.is_some_and(f64::is_finite)));
}

#[tokio::test]
async fn vector_scope_without_embedder_returns_not_implemented() {
    let path = db_path();
    let provider = Arc::new(CountingEmbedding::default());
    let vector = vector_store(provider, path.clone());
    let mut request = dataset_request("Vector only");
    request.retrieval_mode = KnowledgeRetrievalMode::Vector;
    request.embedding_model = Some("fake-embedding".to_string());
    let dataset = vector.create_dataset(request).await.expect("vector dataset");

    let plain = store_for_path(path);
    assert_api_error(
        plain
            .retrieve(KnowledgeRetrieveRequest {
                query: "refund".to_string(),
                dataset_ids: vec![dataset.id],
                top_k: 1,
                score_threshold: None,
                metadata_filter: None,
                include_context: false,
            })
            .await
            .unwrap_err(),
        StatusCode::NOT_IMPLEMENTED,
        "embedding provider",
    );
}

#[tokio::test]
async fn invalid_and_missing_records_return_stable_api_errors() {
    let store = store();

    assert_api_error(
        store.list_documents("not-a-uuid".to_string()).await.unwrap_err(),
        StatusCode::BAD_REQUEST,
        "dataset_id",
    );
    assert_api_error(
        store.delete_document(uuid::Uuid::new_v4().to_string()).await.unwrap_err(),
        StatusCode::NOT_FOUND,
        "document not found",
    );
    assert_api_error(
        store.delete_dataset(uuid::Uuid::new_v4().to_string()).await.unwrap_err(),
        StatusCode::NOT_FOUND,
        "dataset not found",
    );
}

#[tokio::test]
async fn workflow_provider_maps_retrieval_errors_to_strings() {
    let store = store();
    let err = match WorkflowKnowledgeProvider::retrieve(
        &store,
        WorkflowKnowledgeRequest {
            query: " ".to_string(),
            dataset_ids: vec![],
            top_k: 1,
            score_threshold: None,
            metadata_filter: None,
        },
    )
    .await
    {
        Ok(_) => panic!("invalid workflow query should fail"),
        Err(error) => error,
    };

    assert!(err.contains("query is required"));
}
