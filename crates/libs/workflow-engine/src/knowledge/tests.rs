use super::*;
use serde_json::json;
use vw_api_knowledge::{
    KnowledgeChunkingMode, KnowledgeDatasetCreateRequest, KnowledgeDocumentCreateRequest,
    KnowledgeIndexingMode, KnowledgeRetrievalMode,
};

fn store() -> SqliteKnowledgeStore {
    let path = tempfile::tempdir().expect("tempdir").keep().join("knowledge.sqlite");
    SqliteKnowledgeStore::new(agent_database::GlobalDatabase::new(path).expect("global database"), vw_api_transport::TenantScope::default()).expect("knowledge store")
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
