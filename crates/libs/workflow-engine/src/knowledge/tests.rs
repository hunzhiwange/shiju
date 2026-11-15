use super::*;

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
