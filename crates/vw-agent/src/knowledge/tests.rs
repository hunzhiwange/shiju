use super::SqliteKnowledgeStore;
use super::chunker::chunk_text_with_limits;
use crate::global_database::GlobalDatabase;
use crate::memory::embeddings::EmbeddingProvider;
use crate::workflow::{WorkflowKnowledgeProvider, WorkflowKnowledgeRequest};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use vw_api_types::TenantScope;
use vw_api_types::knowledge::{
    KnowledgeChunkingMode, KnowledgeDatasetCreateRequest, KnowledgeDocumentCreateRequest,
    KnowledgeIndexingMode, KnowledgeRetrievalMode, KnowledgeRetrieveRequest,
};

fn store() -> SqliteKnowledgeStore {
    let dir = tempfile::tempdir().expect("tempdir");
    let database =
        GlobalDatabase::new(dir.keep().join("knowledge.sqlite")).expect("global database");
    SqliteKnowledgeStore::new(database, TenantScope::default()).expect("knowledge store")
}

fn vector_store() -> SqliteKnowledgeStore {
    let dir = tempfile::tempdir().expect("tempdir");
    SqliteKnowledgeStore::with_embedder(
        GlobalDatabase::new(dir.keep().join("knowledge.sqlite")).expect("global database"),
        TenantScope::default(),
        Arc::new(FakeEmbedding),
        Some("fake-embedding".to_string()),
        0.8,
        0.2,
        128,
    )
    .expect("knowledge store")
}

fn dataset_request(
    name: &str,
    indexing_mode: KnowledgeIndexingMode,
    retrieval_mode: KnowledgeRetrievalMode,
    embedding_model: Option<String>,
) -> KnowledgeDatasetCreateRequest {
    KnowledgeDatasetCreateRequest {
        name: name.to_string(),
        description: String::new(),
        chunking_mode: KnowledgeChunkingMode::General,
        indexing_mode,
        retrieval_mode,
        keyword_count: 10,
        top_k: 10,
        score_threshold_enabled: false,
        score_threshold: 0.15,
        rerank_enabled: false,
        embedding_model,
        rerank_model: None,
    }
}

struct FakeEmbedding;

#[async_trait]
impl EmbeddingProvider for FakeEmbedding {
    fn name(&self) -> &str {
        "fake"
    }

    fn dimensions(&self) -> usize {
        3
    }

    async fn embed(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|text| fake_vector(text)).collect())
    }
}

fn fake_vector(text: &str) -> Vec<f32> {
    let text = text.to_ascii_lowercase();
    if text.contains("refund") || text.contains("return") {
        vec![1.0, 0.0, 0.0]
    } else if text.contains("image") || text.contains("category") {
        vec![0.0, 1.0, 0.0]
    } else {
        vec![0.0, 0.0, 1.0]
    }
}

#[test]
fn chunk_text_prefers_sentence_boundary() {
    let chunks = chunk_text_with_limits("alpha beta. gamma delta. epsilon.", 18, 0);

    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].content, "alpha beta.");
}

#[tokio::test]
async fn sqlite_store_retrieves_uploaded_document() {
    let store = store();
    let dataset = store
        .create_dataset(dataset_request(
            "Support",
            KnowledgeIndexingMode::Economy,
            KnowledgeRetrievalMode::FullText,
            None,
        ))
        .await
        .expect("dataset");
    store
        .create_document(
            dataset.id.clone(),
            KnowledgeDocumentCreateRequest {
                name: "Return policy".to_string(),
                content: "Return policy allows refunds within seven days.".to_string(),
                metadata: json!({ "tag": "support" }),
                enabled: true,
            },
        )
        .await
        .expect("document");

    let response = store
        .retrieve(KnowledgeRetrieveRequest {
            query: "refunds".to_string(),
            dataset_ids: vec![dataset.id],
            top_k: 3,
            score_threshold: None,
            metadata_filter: Some(json!({ "tag": "support" })),
            include_context: true,
        })
        .await
        .expect("retrieve");

    assert_eq!(response.chunks.len(), 1);
    assert!(response.chunks[0].content.contains("refunds"));
    assert_eq!(response.chunks[0].metadata["tag"], json!("support"));
}

#[tokio::test]
async fn parent_child_dataset_returns_parent_context() {
    let store = store();
    let mut request = dataset_request(
        "Parent Child KB",
        KnowledgeIndexingMode::Economy,
        KnowledgeRetrievalMode::FullText,
        None,
    );
    request.chunking_mode = KnowledgeChunkingMode::ParentChild;
    let dataset = store.create_dataset(request).await.expect("dataset");
    let content = format!(
        "Parent context should be returned. {}\nneedlechild appears in the child chunk.",
        "filler sentence. ".repeat(40)
    );
    store
        .create_document(
            dataset.id.clone(),
            KnowledgeDocumentCreateRequest {
                name: "Parent child doc".to_string(),
                content,
                metadata: json!({ "tag": "pc" }),
                enabled: true,
            },
        )
        .await
        .expect("document");

    let response = store
        .retrieve(KnowledgeRetrieveRequest {
            query: "needlechild".to_string(),
            dataset_ids: vec![dataset.id],
            top_k: 1,
            score_threshold: None,
            metadata_filter: None,
            include_context: true,
        })
        .await
        .expect("retrieve");

    assert_eq!(response.chunks.len(), 1);
    assert_eq!(response.chunks[0].metadata["chunking_mode"], json!("parent_child"));
    assert!(
        response.chunks[0]
            .context
            .as_deref()
            .unwrap_or("")
            .contains("Parent context should be returned")
    );
    assert!(
        response.chunks[0].metadata["child_content"].as_str().unwrap_or("").contains("needlechild")
    );
}

#[tokio::test]
async fn qa_dataset_retrieves_by_question_and_returns_answer() {
    let store = store();
    let mut request = dataset_request(
        "QA KB",
        KnowledgeIndexingMode::Economy,
        KnowledgeRetrievalMode::FullText,
        None,
    );
    request.chunking_mode = KnowledgeChunkingMode::Qa;
    let dataset = store.create_dataset(request).await.expect("dataset");
    store
        .create_document(
            dataset.id.clone(),
            KnowledgeDocumentCreateRequest {
                name: "FAQ".to_string(),
                content: "Q: How do refunds work?\nA: Refunds are handled within seven days.\n\nQ: Where are images configured?\nA: Images are configured on the category page.".to_string(),
                metadata: json!({ "tag": "faq" }),
                enabled: true,
            },
        )
        .await
        .expect("document");

    let response = store
        .retrieve(KnowledgeRetrieveRequest {
            query: "refunds".to_string(),
            dataset_ids: vec![dataset.id],
            top_k: 1,
            score_threshold: None,
            metadata_filter: None,
            include_context: false,
        })
        .await
        .expect("retrieve");

    assert_eq!(response.chunks.len(), 1);
    assert_eq!(response.chunks[0].metadata["chunking_mode"], json!("qa"));
    assert!(response.chunks[0].title.contains("refunds"));
    assert!(response.chunks[0].content.contains("seven days"));
}

#[tokio::test]
async fn workflow_provider_returns_chunks() {
    let store = store();
    let dataset = store
        .create_dataset(dataset_request(
            "Workflow KB",
            KnowledgeIndexingMode::Economy,
            KnowledgeRetrievalMode::FullText,
            None,
        ))
        .await
        .expect("dataset");
    store
        .create_document(
            dataset.id.clone(),
            KnowledgeDocumentCreateRequest {
                name: "Menu setup".to_string(),
                content: "Menu images can be configured on the product category page.".to_string(),
                metadata: json!({ "scope": "menu" }),
                enabled: true,
            },
        )
        .await
        .expect("document");

    let chunks = WorkflowKnowledgeProvider::retrieve(
        &store,
        WorkflowKnowledgeRequest {
            query: "product category".to_string(),
            dataset_ids: vec![dataset.id],
            top_k: 2,
            score_threshold: None,
            metadata_filter: None,
        },
    )
    .await
    .expect("workflow retrieve");

    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].title, "Menu setup");
}

#[tokio::test]
async fn sqlite_store_uses_vector_retrieval_for_high_quality_dataset() {
    let store = vector_store();
    let dataset = store
        .create_dataset(dataset_request(
            "Vector KB",
            KnowledgeIndexingMode::HighQuality,
            KnowledgeRetrievalMode::Vector,
            Some("fake-embedding".to_string()),
        ))
        .await
        .expect("dataset");
    store
        .create_document(
            dataset.id.clone(),
            KnowledgeDocumentCreateRequest {
                name: "Refunds".to_string(),
                content: "Refund requests are handled by the support team.".to_string(),
                metadata: json!({ "kind": "policy" }),
                enabled: true,
            },
        )
        .await
        .expect("refund document");
    store
        .create_document(
            dataset.id.clone(),
            KnowledgeDocumentCreateRequest {
                name: "Images".to_string(),
                content: "Product category images are configured in catalog settings.".to_string(),
                metadata: json!({ "kind": "media" }),
                enabled: true,
            },
        )
        .await
        .expect("image document");

    let response = store
        .retrieve(KnowledgeRetrieveRequest {
            query: "How do returns work?".to_string(),
            dataset_ids: vec![dataset.id],
            top_k: 1,
            score_threshold: None,
            metadata_filter: None,
            include_context: false,
        })
        .await
        .expect("retrieve");

    assert_eq!(response.chunks.len(), 1);
    assert_eq!(response.chunks[0].title, "Refunds");
    assert!(response.chunks[0].score.unwrap_or_default() > 0.7);
}

#[tokio::test]
async fn unsupported_vector_dataset_is_explicit() {
    let error = store()
        .create_dataset(dataset_request(
            "Vector",
            KnowledgeIndexingMode::HighQuality,
            KnowledgeRetrievalMode::Hybrid,
            Some("text-embedding-v4".to_string()),
        ))
        .await
        .expect_err("unsupported");

    assert!(error.to_string().contains("embedding provider"));
}
