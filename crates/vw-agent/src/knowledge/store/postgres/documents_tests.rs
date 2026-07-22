use anyhow::Result;
use serde_json::{Value, json};
use vw_api_types::TenantScope;
use vw_api_types::knowledge::{
    KnowledgeChunkingMode, KnowledgeDatasetCreateRequest, KnowledgeDocumentCreateRequest,
    KnowledgeDocumentListQuery, KnowledgeDocumentStatus, KnowledgeIndexingMode,
    KnowledgeRetrievalMode,
};

use super::{PostgresKnowledgeStore, content_hash, metadata_or_object};
use crate::global_database::test_support::PostgresTestDatabase;

fn dataset_request() -> KnowledgeDatasetCreateRequest {
    KnowledgeDatasetCreateRequest {
        name: "Documents".to_string(),
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

#[test]
fn document_metadata_and_content_hash_are_normalized_deterministically() {
    assert_eq!(metadata_or_object(Value::Null), json!({}));
    assert_eq!(metadata_or_object(json!(["kept"])), json!(["kept"]));
    assert_eq!(
        content_hash(""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_ne!(content_hash("knowledge"), content_hash("Knowledge"));
}

#[test]
fn document_lifecycle_filters_updates_and_deletes_with_tenant_isolation() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let store = PostgresKnowledgeStore::new(fixture.database().clone())?;
    let tenant = TenantScope::new("document-platform", "document-primary")?;
    let other = TenantScope::new("document-platform", "document-other")?;
    let dataset = store.create_dataset(&tenant, dataset_request())?;

    let document = store.create_document(
        &tenant,
        &dataset.id,
        KnowledgeDocumentCreateRequest {
            name: "  Original  ".to_string(),
            content: "你好 knowledge".to_string(),
            metadata: Value::Null,
            enabled: true,
        },
    )?;
    assert_eq!(document.name, "Original");
    assert_eq!(document.metadata, json!({}));
    assert_eq!(document.char_count, 12);
    assert_eq!(document.status, KnowledgeDocumentStatus::Completed);
    assert_eq!(store.get_document(&other, &document.id)?, None);

    let completed = store.list_documents(
        &tenant,
        &dataset.id,
        &KnowledgeDocumentListQuery { status: Some(KnowledgeDocumentStatus::Completed) },
    )?;
    assert_eq!(completed, vec![document.clone()]);

    let updated = store
        .update_document(
            &tenant,
            &document.id,
            Some("Renamed"),
            true,
            &KnowledgeDocumentStatus::Archived,
            Some(50),
            100,
        )?
        .expect("existing document");
    assert_eq!(updated.name, "Renamed");
    assert_eq!(updated.status, KnowledgeDocumentStatus::Archived);
    assert_eq!(updated.archived_at_ms, Some(50));
    assert_eq!(
        store.update_document(
            &tenant,
            "missing",
            None,
            true,
            &KnowledgeDocumentStatus::Completed,
            None,
            100,
        )?,
        None
    );

    assert_eq!(store.delete_document(&tenant, &document.id)?, Some(updated));
    assert_eq!(store.delete_document(&tenant, &document.id)?, None);

    fixture.cleanup()
}

#[test]
fn document_create_rejects_missing_dataset_without_side_effects() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let store = PostgresKnowledgeStore::new(fixture.database().clone())?;
    let tenant = TenantScope::new("document-platform", "missing-dataset")?;

    let error = store
        .create_document(
            &tenant,
            "missing",
            KnowledgeDocumentCreateRequest {
                name: "Document".to_string(),
                content: "content".to_string(),
                metadata: json!({}),
                enabled: true,
            },
        )
        .expect_err("missing dataset must be rejected");
    assert!(error.to_string().contains("knowledge dataset not found: missing"));

    fixture.cleanup()
}
