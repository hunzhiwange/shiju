use super::*;
use axum::Json;
use axum::body::{Body, to_bytes};
use axum::extract::{Extension, Path, Query};
use axum::http::{Method, Request, StatusCode};
use rusqlite::params;
use serde_json::json;
use std::sync::Once;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tower::ServiceExt;
use vw_api_types::knowledge::KnowledgeEmbeddingTestRequest;
use vw_api_types::knowledge::{
    KnowledgeChunkingMode, KnowledgeDatasetCreateRequest, KnowledgeDocumentCreateRequest,
    KnowledgeDocumentImportRequest, KnowledgeDocumentListQuery, KnowledgeDocumentPatchRequest,
    KnowledgeDocumentReindexRequest, KnowledgeDocumentStatus, KnowledgeDocumentsBatchAction,
    KnowledgeDocumentsBatchActionRequest, KnowledgeIndexJobStatus, KnowledgeIndexingMode,
    KnowledgeMatchType, KnowledgeRetrievalMode, KnowledgeRetrieveRequest,
};

static TEST_HOME: Once = Once::new();
static KNOWLEDGE_TEST_LOCK: Mutex<()> = Mutex::const_new(());

fn test_tenant() -> TenantScope {
    TenantScope::new("platform-knowledge", "company-knowledge").expect("knowledge tenant")
}

fn tenant_extension() -> Extension<TenantScope> {
    Extension(test_tenant())
}

fn ensure_test_home() {
    TEST_HOME.call_once(|| {
        let dir =
            std::env::temp_dir().join(format!("vw-knowledge-handler-tests-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("test home should be created");
        unsafe {
            std::env::set_var("VIBEWINDOW_TEST_HOME", dir);
        }
    });
}

fn dataset_request(name: &str) -> KnowledgeDatasetCreateRequest {
    KnowledgeDatasetCreateRequest {
        name: name.to_string(),
        description: "handler test dataset".to_string(),
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

fn document_request(name: &str, content: &str) -> KnowledgeDocumentCreateRequest {
    KnowledgeDocumentCreateRequest {
        name: name.to_string(),
        content: content.to_string(),
        metadata: serde_json::json!({ "source": "handler-test" }),
        enabled: true,
    }
}

fn import_request(
    filename: &str,
    mime_type: Option<&str>,
    content: &[u8],
) -> KnowledgeDocumentImportRequest {
    KnowledgeDocumentImportRequest {
        filename: filename.to_string(),
        mime_type: mime_type.map(str::to_string),
        content_base64: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, content),
        metadata: serde_json::json!({ "source": "handler-import-test" }),
        enabled: true,
    }
}

fn json_request(method: Method, uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request should build")
}

async fn response_json(response: axum::response::Response) -> serde_json::Value {
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("body should be readable");
    serde_json::from_slice(&body).expect("response should be json")
}

fn set_document_status_for_test(document_id: &str, status: &str) {
    let conn = GlobalDatabase::for_home(&crate::global::paths().home)
        .expect("global knowledge database")
        .connect()
        .expect("open knowledge db");
    conn.execute(
        "UPDATE knowledge_documents SET status = ?1, updated_at_ms = updated_at_ms + 1 WHERE id = ?2",
        params![status, document_id],
    )
    .expect("update document status");
}

#[test]
fn router_builds_with_app_state() {
    let _: axum::Router<()> = router();
}

#[test]
fn knowledge_db_path_uses_global_database_path() {
    ensure_test_home();

    let expected = GlobalDatabase::for_home(&crate::global::paths().home)
        .expect("global database")
        .path()
        .to_path_buf();

    assert_eq!(knowledge_db_path(), expected);
}

#[tokio::test]
async fn knowledge_gateway_writes_shared_database_without_touching_legacy_file() {
    let _guard = KNOWLEDGE_TEST_LOCK.lock().await;
    ensure_test_home();
    let suffix = uuid::Uuid::new_v4().to_string();
    let paths = crate::global::paths();
    let database = GlobalDatabase::for_home(&paths.home).expect("global database");
    let legacy_path = vw_config_types::paths::home_config_dir(&paths.home)
        .join("knowledge")
        .join("knowledge.sqlite");
    let legacy_contents = format!("legacy-knowledge-{suffix}").into_bytes();
    std::fs::create_dir_all(legacy_path.parent().expect("legacy parent"))
        .expect("legacy directory");
    std::fs::write(&legacy_path, &legacy_contents).expect("legacy knowledge file");

    let connection = database.connect().expect("shared database connection");
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS gateway_existing_domain (\
                 id TEXT PRIMARY KEY,\
                 value TEXT NOT NULL\
             );",
        )
        .expect("existing domain schema");
    connection
        .execute(
            "INSERT INTO gateway_existing_domain (id, value) VALUES (?1, 'preserved')",
            [&suffix],
        )
        .expect("existing domain row");
    drop(connection);

    let Json(dataset) = knowledge_dataset_create(
        tenant_extension(),
        Json(dataset_request(&format!("shared-database-{suffix}"))),
    )
    .await
    .expect("dataset create should succeed");

    let connection = database.connect().expect("shared database connection");
    let stored_name: String = connection
        .query_row("SELECT name FROM knowledge_datasets WHERE id = ?1", [&dataset.id], |row| {
            row.get(0)
        })
        .expect("dataset in shared database");
    let preserved: String = connection
        .query_row("SELECT value FROM gateway_existing_domain WHERE id = ?1", [&suffix], |row| {
            row.get(0)
        })
        .expect("existing domain row preserved");

    assert_eq!(database.path().file_name(), Some(std::ffi::OsStr::new("vibe-window.sqlite3")));
    assert_eq!(stored_name, dataset.name);
    assert_eq!(preserved, "preserved");
    assert_eq!(std::fs::read(&legacy_path).expect("legacy knowledge file"), legacy_contents);
}

#[tokio::test]
async fn knowledge_status_reports_local_capabilities() {
    ensure_test_home();

    let Json(status) = knowledge_status(tenant_extension()).await.expect("status should succeed");

    assert!(status.full_text);
    assert!(status.rerank);
    assert_eq!(status.vector, status.hybrid);
    assert!(status.notes.contains_key("rerank"));
}

#[tokio::test]
async fn knowledge_embedding_status_reports_resolved_config() {
    let config = embedding_config("http://127.0.0.1:9999", Some("configured-key"));

    let Json(status) = knowledge_embedding_status_from_config(config)
        .await
        .expect("embedding status should succeed");

    assert_eq!(status.provider_id, "custom:http://127.0.0.1:9999");
    assert_eq!(status.route_hint.as_deref(), Some("knowledge"));
    assert_eq!(status.model, "mock-embedding");
    assert_eq!(status.dimensions, 3);
    assert!(status.api_key_configured);
    assert!(status.vector_available);
    assert!(status.hybrid_available);
    assert!(status.retrieval_modes.contains(&KnowledgeRetrievalMode::FullText));
    assert!(status.retrieval_modes.contains(&KnowledgeRetrievalMode::Vector));
    assert!(status.retrieval_modes.contains(&KnowledgeRetrievalMode::Hybrid));
}

#[tokio::test]
async fn knowledge_embedding_status_reports_unavailable_vector_provider() {
    let Json(status) =
        knowledge_embedding_status_from_config(crate::app::agent::config::Config::default())
            .await
            .expect("embedding status should succeed");

    assert!(!status.vector_available);
    assert!(!status.hybrid_available);
    assert_eq!(status.retrieval_modes, vec![KnowledgeRetrievalMode::FullText]);
    assert_eq!(
        status.notes.get("vector").map(String::as_str),
        Some("embedding provider is configured as none or unsupported")
    );
}

#[tokio::test]
async fn knowledge_embedding_presets_include_supported_routes() {
    let Json(presets) =
        knowledge_embedding_presets().await.expect("embedding presets should succeed");

    assert!(presets.iter().any(|preset| preset.provider_id == "alibaba"));
    assert!(presets.iter().any(|preset| preset.provider_id == "alibaba-cn"));
    assert!(
        presets.iter().any(|preset| {
            preset.id == "doubao-volcengine"
                && preset.provider_id.starts_with("custom:")
                && preset.endpoint_url.as_deref().is_some_and(|url| url.contains("volces.com"))
        }),
        "Doubao/Volcengine must be routed through a custom OpenAI-compatible endpoint"
    );
}

#[tokio::test]
async fn knowledge_embedding_test_uses_gateway_provider() {
    let (base_url, request) =
        spawn_embedding_server("200 OK", r#"{"data":[{"embedding":[0.1,0.2,0.3]}]}"#).await;
    let config = embedding_config(&base_url, Some("gateway-secret"));

    let Json(response) = knowledge_embedding_test_with_config(
        config,
        Json(KnowledgeEmbeddingTestRequest {
            provider_id: None,
            model: None,
            dimensions: None,
            api_key_override: None,
            sample_text: Some("gateway probe".to_string()),
        }),
    )
    .await
    .expect("embedding test should return a response");
    let request = request.await.expect("mock provider request");

    assert!(response.ok);
    assert_eq!(response.provider_id, format!("custom:{base_url}"));
    assert_eq!(response.model, "mock-embedding");
    assert_eq!(response.configured_dimensions, 3);
    assert_eq!(response.observed_dimensions, Some(3));
    assert_eq!(response.message, "embedding test succeeded");
    assert!(request.starts_with("POST /v1/embeddings HTTP/1.1"));
    assert!(request.to_ascii_lowercase().contains("authorization: bearer gateway-secret"));
    assert!(request.contains(r#""model":"mock-embedding""#));
    assert!(request.contains(r#""input":["gateway probe"]"#));
}

#[tokio::test]
async fn knowledge_embedding_test_missing_api_key_returns_clear_failure() {
    let config = embedding_config("http://127.0.0.1:9999", None);

    let Json(response) = knowledge_embedding_test_with_config(
        config,
        Json(KnowledgeEmbeddingTestRequest {
            provider_id: None,
            model: None,
            dimensions: None,
            api_key_override: None,
            sample_text: Some("gateway probe".to_string()),
        }),
    )
    .await
    .expect("missing key should be a typed test result");

    assert!(!response.ok);
    assert_eq!(response.provider_id, "custom:http://127.0.0.1:9999");
    assert!(response.message.contains("API key"));
    assert!(!response.message.contains("gateway-secret"));
}

#[tokio::test]
async fn knowledge_embedding_test_sanitizes_provider_error() {
    let (base_url, request) =
        spawn_embedding_server("500 Internal Server Error", "gateway-secret rejected").await;
    let config = embedding_config(&base_url, Some("gateway-secret"));

    let Json(response) = knowledge_embedding_test_with_config(
        config,
        Json(KnowledgeEmbeddingTestRequest {
            provider_id: None,
            model: None,
            dimensions: None,
            api_key_override: None,
            sample_text: Some("gateway probe".to_string()),
        }),
    )
    .await
    .expect("provider error should be a typed test result");
    let _request = request.await.expect("mock provider request");

    assert!(!response.ok);
    assert!(response.message.contains("[REDACTED]"));
    assert!(!response.message.contains("gateway-secret"));
}

#[tokio::test]
async fn knowledge_embedding_test_rejects_invalid_overrides() {
    let config = crate::app::agent::config::Config::default();
    let blank_provider = knowledge_embedding_test_with_config(
        config.clone(),
        Json(KnowledgeEmbeddingTestRequest {
            provider_id: Some("   ".to_string()),
            model: None,
            dimensions: None,
            api_key_override: None,
            sample_text: None,
        }),
    )
    .await
    .expect_err("blank provider override should fail");
    let blank_model = knowledge_embedding_test_with_config(
        config.clone(),
        Json(KnowledgeEmbeddingTestRequest {
            provider_id: None,
            model: Some("\t".to_string()),
            dimensions: None,
            api_key_override: None,
            sample_text: None,
        }),
    )
    .await
    .expect_err("blank model override should fail");
    let zero_dimensions = knowledge_embedding_test_with_config(
        config,
        Json(KnowledgeEmbeddingTestRequest {
            provider_id: None,
            model: None,
            dimensions: Some(0),
            api_key_override: None,
            sample_text: None,
        }),
    )
    .await
    .expect_err("zero dimensions should fail");

    for error in [blank_provider, blank_model, zero_dimensions] {
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn knowledge_embedding_test_reports_none_and_unsupported_providers() {
    let config = crate::app::agent::config::Config::default();
    let Json(none) = knowledge_embedding_test_with_config(
        config.clone(),
        Json(KnowledgeEmbeddingTestRequest {
            provider_id: Some(" NONE ".to_string()),
            model: Some("model".to_string()),
            dimensions: Some(3),
            api_key_override: None,
            sample_text: None,
        }),
    )
    .await
    .expect("none provider should return typed failure");
    let Json(unsupported) = knowledge_embedding_test_with_config(
        config,
        Json(KnowledgeEmbeddingTestRequest {
            provider_id: Some("unsupported-provider".to_string()),
            model: Some("model".to_string()),
            dimensions: Some(3),
            api_key_override: Some("secret".to_string()),
            sample_text: None,
        }),
    )
    .await
    .expect("unsupported provider should return typed failure");

    assert!(!none.ok);
    assert_eq!(none.provider_id, "none");
    assert!(none.message.contains("not configured"));
    assert!(!unsupported.ok);
    assert!(unsupported.message.contains("unsupported embedding provider"));
}

#[test]
fn embedding_helpers_normalize_routes_and_bound_sanitized_errors() {
    assert_eq!(normalize_embedding_provider_id(" OpenAI "), "openai");
    assert_eq!(normalize_embedding_provider_id(" Custom:Example "), "Custom:Example");
    assert!(embedding_provider_is_supported("custom:https://example.test"));
    assert!(!embedding_provider_is_supported("custom:   "));
    assert_eq!(optional_trimmed(Some(" value ")).as_deref(), Some("value"));
    assert_eq!(optional_trimmed(Some("  ")), None);

    let secret = "sensitive-token";
    let error = format!("{secret}{}", "x".repeat(700));
    let sanitized = sanitize_embedding_error(&error, &[Some(secret), Some(" "), None]);
    assert!(!sanitized.contains(secret));
    assert!(sanitized.starts_with("[REDACTED]"));
    assert_eq!(sanitized.chars().count(), 600);
}

#[tokio::test]
async fn knowledge_dataset_document_and_retrieve_lifecycle() {
    let _guard = KNOWLEDGE_TEST_LOCK.lock().await;
    ensure_test_home();
    let suffix = uuid::Uuid::new_v4();

    let Json(dataset) = knowledge_dataset_create(
        tenant_extension(),
        Json(dataset_request(&format!("dataset-{suffix}"))),
    )
    .await
    .expect("dataset create should succeed");

    let Json(listed) =
        knowledge_datasets_list(tenant_extension()).await.expect("dataset list should succeed");
    assert!(listed.iter().any(|item| item.id == dataset.id));

    let Json(fetched) = knowledge_dataset_get(tenant_extension(), Path(dataset.id.clone()))
        .await
        .expect("dataset get should succeed");
    assert_eq!(fetched.name, dataset.name);

    let Json(document) = knowledge_document_create(
        tenant_extension(),
        Path(dataset.id.clone()),
        Json(document_request(
            &format!("document-{suffix}"),
            "Rustacean handlers can retrieve full text knowledge chunks.",
        )),
    )
    .await
    .expect("document create should succeed");
    assert_eq!(document.dataset_id, dataset.id);
    assert_eq!(document.status, KnowledgeDocumentStatus::Completed);
    assert!(!document.content_hash.is_empty());
    assert!(document.chunk_count > 0);

    let Json(documents) = knowledge_documents_list(
        tenant_extension(),
        Path(dataset.id.clone()),
        Query(KnowledgeDocumentListQuery::default()),
    )
    .await
    .expect("document list should succeed");
    assert!(documents.iter().any(|item| item.id == document.id));

    let Json(chunks) =
        knowledge_document_chunks_list(tenant_extension(), Path(document.id.clone()))
            .await
            .expect("chunk list should succeed");
    assert_eq!(chunks.len() as u64, document.chunk_count);
    assert!(chunks.iter().all(|chunk| chunk.document_id == document.id));

    let Json(reindexed) = knowledge_document_reindex(
        tenant_extension(),
        Path(document.id.clone()),
        Json(KnowledgeDocumentReindexRequest { force: false }),
    )
    .await
    .expect("document reindex should succeed");
    assert_eq!(reindexed.id, document.id);
    assert_eq!(reindexed.status, KnowledgeDocumentStatus::Completed);
    assert_eq!(reindexed.chunk_count, document.chunk_count);

    let Json(retrieved) = knowledge_retrieve(
        tenant_extension(),
        Json(KnowledgeRetrieveRequest {
            query: "Rustacean".to_string(),
            dataset_ids: vec![dataset.id.clone()],
            top_k: 3,
            score_threshold: None,
            metadata_filter: Some(serde_json::json!({ "source": "handler-test" })),
            include_context: false,
        }),
    )
    .await
    .expect("retrieve should succeed");
    assert!(retrieved.chunks.iter().any(|chunk| chunk.document_id == document.id));
    assert!(retrieved.chunks.iter().all(|chunk| chunk.match_type == KnowledgeMatchType::FullText));

    let Json(deleted_document) =
        knowledge_document_delete(tenant_extension(), Path(document.id.clone()))
            .await
            .expect("document delete should succeed");
    assert_eq!(deleted_document.id, document.id);

    let Json(deleted_dataset) =
        knowledge_dataset_delete(tenant_extension(), Path(dataset.id.clone()))
            .await
            .expect("dataset delete should succeed");
    assert_eq!(deleted_dataset.id, dataset.id);
}

#[tokio::test]
async fn knowledge_handlers_keep_crud_and_retrieval_isolated_by_request_tenant() {
    let _guard = KNOWLEDGE_TEST_LOCK.lock().await;
    ensure_test_home();
    let suffix = uuid::Uuid::new_v4();
    let tenant_a = TenantScope::new("platform-knowledge", "company-a").expect("tenant A");
    let tenant_b = TenantScope::new("platform-knowledge", "company-b").expect("tenant B");

    let Json(dataset_a) = knowledge_dataset_create(
        Extension(tenant_a.clone()),
        Json(dataset_request(&format!("tenant-a-{suffix}"))),
    )
    .await
    .expect("tenant A dataset");
    let Json(dataset_b) = knowledge_dataset_create(
        Extension(tenant_b.clone()),
        Json(dataset_request(&format!("tenant-b-{suffix}"))),
    )
    .await
    .expect("tenant B dataset");
    let Json(document_a) = knowledge_document_create(
        Extension(tenant_a.clone()),
        Path(dataset_a.id.clone()),
        Json(document_request("shared-document", "tenant alpha marker")),
    )
    .await
    .expect("tenant A document");
    let Json(document_b) = knowledge_document_create(
        Extension(tenant_b.clone()),
        Path(dataset_b.id.clone()),
        Json(document_request("shared-document", "tenant beta marker")),
    )
    .await
    .expect("tenant B document");

    let Json(retrieved_a) = knowledge_retrieve(
        Extension(tenant_a.clone()),
        Json(KnowledgeRetrieveRequest {
            query: "alpha".to_string(),
            dataset_ids: vec![dataset_a.id.clone()],
            top_k: 3,
            score_threshold: None,
            metadata_filter: None,
            include_context: false,
        }),
    )
    .await
    .expect("retrieve tenant A");
    assert!(retrieved_a.chunks.iter().any(|chunk| chunk.document_id == document_a.id));
    assert!(retrieved_a.chunks.iter().all(|chunk| chunk.document_id != document_b.id));

    let cross_tenant = knowledge_retrieve(
        Extension(tenant_b.clone()),
        Json(KnowledgeRetrieveRequest {
            query: "alpha".to_string(),
            dataset_ids: vec![dataset_a.id.clone()],
            top_k: 3,
            score_threshold: None,
            metadata_filter: None,
            include_context: false,
        }),
    )
    .await
    .expect_err("cross-tenant retrieve should not find tenant A dataset");
    assert_eq!(cross_tenant.status, StatusCode::NOT_FOUND);

    let Json(deleted_a) = knowledge_dataset_delete(Extension(tenant_a), Path(dataset_a.id))
        .await
        .expect("delete tenant A dataset");
    assert_eq!(deleted_a.name, format!("tenant-a-{suffix}"));
    let Json(dataset_b_after_delete) =
        knowledge_dataset_get(Extension(tenant_b), Path(dataset_b.id.clone()))
            .await
            .expect("tenant B dataset survives tenant A delete");
    assert_eq!(dataset_b_after_delete.id, dataset_b.id);
}

#[tokio::test]
async fn knowledge_document_import_route_creates_document() {
    let _guard = KNOWLEDGE_TEST_LOCK.lock().await;
    ensure_test_home();
    let suffix = uuid::Uuid::new_v4();

    let Json(dataset) = knowledge_dataset_create(
        tenant_extension(),
        Json(dataset_request(&format!("import-dataset-{suffix}"))),
    )
    .await
    .expect("dataset create should succeed");

    let content = b"# Import Guide\n\nGateway imports readable Markdown.";
    let Json(imported) = knowledge_document_import(
        tenant_extension(),
        Path(dataset.id.clone()),
        Json(import_request("guide.md", Some("text/markdown"), content)),
    )
    .await
    .expect("document import should succeed");

    assert_eq!(imported.document.dataset_id, dataset.id);
    assert_eq!(imported.document.name, "guide.md");
    assert_eq!(imported.filename, "guide.md");
    assert_eq!(imported.mime_type.as_deref(), Some("text/markdown"));
    assert_eq!(imported.source_bytes, content.len() as u64);
    assert!(imported.extracted_text_bytes > 0);
    assert_eq!(imported.document.metadata["source"], "handler-import-test");
    assert_eq!(imported.document.metadata["import"]["filename"], "guide.md");

    let Json(chunks) =
        knowledge_document_chunks_list(tenant_extension(), Path(imported.document.id.clone()))
            .await
            .expect("imported document chunks should list");
    assert!(chunks.iter().any(|chunk| chunk.content.contains("Gateway imports")));
}

#[tokio::test]
async fn knowledge_document_import_async_enqueues_and_worker_completes() {
    let _guard = KNOWLEDGE_TEST_LOCK.lock().await;
    ensure_test_home();
    let suffix = uuid::Uuid::new_v4();

    let Json(dataset) = knowledge_dataset_create(
        tenant_extension(),
        Json(dataset_request(&format!("async-import-dataset-{suffix}"))),
    )
    .await
    .expect("dataset create should succeed");

    let content = b"# Async Import\n\nGateway async imports complete in the background.";
    let Json(queued) = knowledge_document_import_async(
        tenant_extension(),
        Path(dataset.id.clone()),
        Json(import_request("async-guide.md", Some("text/markdown"), content)),
    )
    .await
    .expect("async import should queue");

    assert_eq!(queued.document.dataset_id, dataset.id);
    assert_eq!(queued.document.name, "async-guide.md");
    assert_eq!(queued.document.status, KnowledgeDocumentStatus::Pending);
    assert_eq!(queued.document.chunk_count, 0);
    assert_eq!(queued.job.dataset_id, dataset.id);
    assert_eq!(queued.job.document_id, queued.document.id);
    assert!(matches!(
        queued.job.status,
        KnowledgeIndexJobStatus::Queued | KnowledgeIndexJobStatus::Processing
    ));
    assert_eq!(queued.filename, "async-guide.md");
    assert_eq!(queued.mime_type.as_deref(), Some("text/markdown"));
    assert_eq!(queued.source_bytes, content.len() as u64);

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut last_job = None;
    while tokio::time::Instant::now() < deadline {
        let Json(job) = knowledge_index_job_get(tenant_extension(), Path(queued.job.id.clone()))
            .await
            .expect("job query should succeed");
        if job.status == KnowledgeIndexJobStatus::Completed {
            last_job = Some(job);
            break;
        }
        assert_ne!(
            job.status,
            KnowledgeIndexJobStatus::Failed,
            "background worker failed queued import: {}",
            job.error_message
        );
        last_job = Some(job);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let job = last_job.expect("background worker should report queued import status");
    assert_eq!(
        job.status,
        KnowledgeIndexJobStatus::Completed,
        "background worker should complete queued import, last status was {:?}",
        job.status
    );
    assert_eq!(job.document_id, queued.document.id);
    assert!(job.started_at_ms.is_some());
    assert!(job.finished_at_ms.is_some());

    let Json(chunks) =
        knowledge_document_chunks_list(tenant_extension(), Path(queued.document.id.clone()))
            .await
            .expect("async imported document chunks should list");
    assert!(chunks.iter().any(|chunk| chunk.content.contains("Gateway async imports")));
}

#[tokio::test]
async fn knowledge_handlers_return_validation_errors() {
    ensure_test_home();

    let err = knowledge_dataset_create(tenant_extension(), Json(dataset_request("   ")))
        .await
        .expect_err("blank dataset name should fail");
    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);

    let err = knowledge_dataset_get(tenant_extension(), Path("not-a-uuid".to_string()))
        .await
        .expect_err("invalid dataset id should fail");
    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);

    let err = knowledge_document_create(
        tenant_extension(),
        Path(uuid::Uuid::new_v4().to_string()),
        Json(document_request("doc", "   ")),
    )
    .await
    .expect_err("blank document content should fail");
    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);

    let err = knowledge_retrieve(
        tenant_extension(),
        Json(KnowledgeRetrieveRequest {
            query: " ".to_string(),
            dataset_ids: Vec::new(),
            top_k: 1,
            score_threshold: None,
            metadata_filter: None,
            include_context: false,
        }),
    )
    .await
    .expect_err("blank retrieve query should fail");
    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn knowledge_documents_list_filters_by_status() {
    let _guard = KNOWLEDGE_TEST_LOCK.lock().await;
    ensure_test_home();
    let suffix = uuid::Uuid::new_v4();
    let app = router::<()>().layer(Extension(test_tenant()));

    let Json(dataset) = knowledge_dataset_create(
        tenant_extension(),
        Json(dataset_request(&format!("status-filter-dataset-{suffix}"))),
    )
    .await
    .expect("dataset create should succeed");

    let Json(completed) = knowledge_document_create(
        tenant_extension(),
        Path(dataset.id.clone()),
        Json(document_request("completed", "alpha content")),
    )
    .await
    .expect("completed document should succeed");

    let Json(archived) = knowledge_document_create(
        tenant_extension(),
        Path(dataset.id.clone()),
        Json(document_request("archived", "beta content")),
    )
    .await
    .expect("archived document should succeed");

    knowledge_store_from_global_config(test_tenant())
        .await
        .expect("knowledge store")
        .update_document(
            archived.id.clone(),
            KnowledgeDocumentPatchRequest { name: None, enabled: None, archived: Some(true) },
        )
        .await
        .expect("archive document");

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/knowledge/datasets/{}/documents?status=archived", dataset.id))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::OK);

    let documents: Vec<vw_api_types::knowledge::KnowledgeDocumentDto> =
        serde_json::from_value(response_json(response).await).expect("documents json");
    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].id, archived.id);
    assert_eq!(documents[0].status, KnowledgeDocumentStatus::Archived);
    assert_ne!(documents[0].id, completed.id);

    let store = knowledge_store_from_global_config(test_tenant()).await.expect("knowledge store");
    let filtered = store
        .list_documents_with_query(
            dataset.id.clone(),
            KnowledgeDocumentListQuery { status: Some(KnowledgeDocumentStatus::Archived) },
        )
        .await
        .expect("filtered documents");
    assert_eq!(documents, filtered);
}

#[tokio::test]
async fn knowledge_document_update_and_batch_routes_apply_store_changes() {
    let _guard = KNOWLEDGE_TEST_LOCK.lock().await;
    ensure_test_home();
    let suffix = uuid::Uuid::new_v4();
    let app = router::<()>().layer(Extension(test_tenant()));

    let Json(dataset) = knowledge_dataset_create(
        tenant_extension(),
        Json(dataset_request(&format!("patch-batch-dataset-{suffix}"))),
    )
    .await
    .expect("dataset create should succeed");

    let Json(document) = knowledge_document_create(
        tenant_extension(),
        Path(dataset.id.clone()),
        Json(document_request("rename-me", "gamma content")),
    )
    .await
    .expect("document should succeed");

    let patch_response = app
        .clone()
        .oneshot(json_request(
            Method::PATCH,
            &format!("/knowledge/documents/{}", document.id),
            json!({
                "name": "renamed",
                "enabled": false
            }),
        ))
        .await
        .expect("patch request should succeed");
    assert_eq!(patch_response.status(), StatusCode::OK);
    let patched: vw_api_types::knowledge::KnowledgeDocumentDto =
        serde_json::from_value(response_json(patch_response).await).expect("patched json");
    assert_eq!(patched.name, "renamed");
    assert_eq!(patched.status, KnowledgeDocumentStatus::Disabled);

    let batch_response = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/knowledge/documents/batch",
            json!({
                "document_ids": [document.id.clone()],
                "action": "enable"
            }),
        ))
        .await
        .expect("batch request should succeed");
    assert_eq!(batch_response.status(), StatusCode::OK);
    let batch: Vec<vw_api_types::knowledge::KnowledgeDocumentDto> =
        serde_json::from_value(response_json(batch_response).await).expect("batch json");
    assert_eq!(batch.len(), 1);
    assert_eq!(batch[0].id, document.id);
    assert_eq!(batch[0].status, KnowledgeDocumentStatus::Completed);
    assert!(batch[0].enabled);

    let empty_ids = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/knowledge/documents/batch",
            json!({
                "document_ids": [],
                "action": "archive"
            }),
        ))
        .await
        .expect("empty id request should succeed");
    assert_eq!(empty_ids.status(), StatusCode::BAD_REQUEST);
    assert!(response_json(empty_ids).await.to_string().contains("document_ids must not be empty"));

    let blank_name = app
        .clone()
        .oneshot(json_request(
            Method::PATCH,
            &format!("/knowledge/documents/{}", document.id),
            json!({
                "name": "   "
            }),
        ))
        .await
        .expect("blank name request should succeed");
    assert_eq!(blank_name.status(), StatusCode::BAD_REQUEST);
    assert!(response_json(blank_name).await.to_string().contains("name"));

    let missing = app
        .clone()
        .oneshot(json_request(
            Method::PATCH,
            &format!("/knowledge/documents/{}", uuid::Uuid::new_v4()),
            json!({
                "name": "missing"
            }),
        ))
        .await
        .expect("missing request should succeed");
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    assert!(response_json(missing).await.to_string().contains("not found"));

    let Json(processing) = knowledge_document_create(
        tenant_extension(),
        Path(dataset.id.clone()),
        Json(document_request("processing-doc", "delta content")),
    )
    .await
    .expect("processing document should succeed");
    set_document_status_for_test(&processing.id, "processing");

    let pending_patch = app
        .clone()
        .oneshot(json_request(
            Method::PATCH,
            &format!("/knowledge/documents/{}", processing.id),
            json!({
                "enabled": false
            }),
        ))
        .await
        .expect("pending patch request should succeed");
    assert_eq!(pending_patch.status(), StatusCode::BAD_REQUEST);
    let pending_patch_error = response_json(pending_patch).await.to_string();
    assert!(
        pending_patch_error.contains("pending") || pending_patch_error.contains("processing"),
        "unexpected pending patch error: {pending_patch_error}"
    );

    let pending_batch = app
        .oneshot(json_request(
            Method::POST,
            "/knowledge/documents/batch",
            json!({
                "document_ids": [processing.id],
                "action": KnowledgeDocumentsBatchAction::Disable
            }),
        ))
        .await
        .expect("pending batch request should succeed");
    assert_eq!(pending_batch.status(), StatusCode::BAD_REQUEST);
    let pending_batch_error = response_json(pending_batch).await.to_string();
    assert!(
        pending_batch_error.contains("pending") || pending_batch_error.contains("processing"),
        "unexpected pending batch error: {pending_batch_error}"
    );
}

#[tokio::test]
async fn knowledge_store_helper_uses_state_config() {
    let (event_tx, _) = tokio::sync::broadcast::channel(4);
    let state = AppState {
        config: std::sync::Arc::new(parking_lot::Mutex::new(
            crate::app::agent::config::Config::default(),
        )),
        provider: std::sync::Arc::new(StaticProvider),
        model: "test".to_string(),
        temperature: 0.0,
        mem: std::sync::Arc::new(crate::app::agent::memory::NoneMemory::new()),
        auto_save: false,
        webhook_secret_hash: None,
        pairing: std::sync::Arc::new(crate::app::agent::security::PairingGuard::new(false, &[])),
        trust_forwarded_headers: false,
        rate_limiter: std::sync::Arc::new(crate::app::agent::gateway::GatewayRateLimiter::new(
            10, 10, 10,
        )),
        idempotency_store: std::sync::Arc::new(crate::app::agent::gateway::IdempotencyStore::new(
            std::time::Duration::from_secs(60),
            10,
        )),
        whatsapp: None,
        whatsapp_app_secret: None,
        linq: None,
        linq_signing_secret: None,
        nextcloud_talk: None,
        nextcloud_talk_webhook_secret: None,
        wati: None,
        qq: None,
        qq_webhook_enabled: false,
        observer: std::sync::Arc::new(crate::app::agent::observability::NoopObserver),
        tools_registry: std::sync::Arc::new(Vec::new()),
        tools_registry_exec: std::sync::Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_iterations: 10,
        event_tx,
        session_query_engines: Default::default(),
        hook_runtime: None,
    };

    let tenant = TenantScope::new("platform-knowledge", "company-knowledge").expect("tenant");
    let store = knowledge_store(&state, tenant).expect("knowledge store");
    assert!(store.status().full_text);
    assert_eq!(
        knowledge_db_path(),
        GlobalDatabase::for_home(&crate::global::paths().home)
            .expect("global database")
            .path()
            .to_path_buf()
    );
}

struct StaticProvider;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl crate::app::agent::providers::Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok(message.to_string())
    }
}

fn embedding_config(base_url: &str, api_key: Option<&str>) -> crate::app::agent::config::Config {
    let mut config = crate::app::agent::config::Config::default();
    config.api_key = api_key.map(str::to_string);
    config.memory.embedding_provider = "none".to_string();
    config.memory.embedding_model = "hint:knowledge".to_string();
    config.memory.embedding_dimensions = 1536;
    config.embedding_routes = vec![crate::app::agent::config::EmbeddingRouteConfig {
        hint: "knowledge".to_string(),
        provider: format!("custom:{base_url}"),
        model: "mock-embedding".to_string(),
        dimensions: Some(3),
        api_key: None,
    }];
    config
}

async fn spawn_embedding_server(
    status: &str,
    body: &str,
) -> (String, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind mock embedding server");
    let address = listener.local_addr().expect("mock server address");
    let status = status.to_string();
    let body = body.to_string();

    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept mock request");
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];

        loop {
            let read = socket.read(&mut buffer).await.expect("read mock request");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
            if http_request_is_complete(&request) {
                break;
            }
        }

        let response = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(response.as_bytes()).await.expect("write mock response");
        String::from_utf8_lossy(&request).to_string()
    });

    (format!("http://{address}"), handle)
}

fn http_request_is_complete(request: &[u8]) -> bool {
    let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n") else {
        return false;
    };
    let body_start = header_end + 4;
    let headers = String::from_utf8_lossy(&request[..header_end]);
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);

    request.len() >= body_start + content_length
}
