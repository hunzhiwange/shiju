use anyhow::Result;
use serde_json::{Value, json};
use vw_api_types::TenantScope;
use vw_api_types::knowledge::{
    KnowledgeChunkType, KnowledgeChunkingMode, KnowledgeDatasetCreateRequest,
    KnowledgeDocumentCreateRequest, KnowledgeIndexingMode, KnowledgeRetrievalMode,
};

use super::{PostgresKnowledgeStore, PreparedKnowledgeChunk, metadata_or_object};
use crate::global_database::test_support::PostgresTestDatabase;

fn dataset_request() -> KnowledgeDatasetCreateRequest {
    KnowledgeDatasetCreateRequest {
        name: "Chunk dataset".to_string(),
        description: String::new(),
        chunking_mode: KnowledgeChunkingMode::ParentChild,
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

fn prepared_chunk(
    ordinal: usize,
    content: &str,
    chunk_type: KnowledgeChunkType,
    parent_position: Option<usize>,
    indexed: bool,
) -> PreparedKnowledgeChunk {
    PreparedKnowledgeChunk {
        ordinal,
        title: format!("Chunk {ordinal}"),
        content: content.to_string(),
        index_content: format!("index {content}"),
        metadata: json!({ "local": ordinal }),
        chunk_type,
        start_at: ordinal * 10,
        end_at: ordinal * 10 + content.chars().count(),
        parent_position,
        context_header: format!("Context {ordinal}"),
        indexed,
        embedding: None,
    }
}

#[test]
fn chunk_metadata_null_is_normalized_without_changing_non_null_values() {
    assert_eq!(metadata_or_object(Value::Null), json!({}));
    assert_eq!(metadata_or_object(json!([1, 2])), json!([1, 2]));
}

#[test]
fn document_chunks_round_trip_parent_and_sibling_links() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let store = PostgresKnowledgeStore::new(fixture.database().clone())?;
    let tenant = TenantScope::new("chunk-platform", "chunk-company")?;
    let dataset = store.create_dataset(&tenant, dataset_request())?;

    let document = store.create_document_with_chunks(
        &tenant,
        &dataset.id,
        KnowledgeDocumentCreateRequest {
            name: "  Linked chunks  ".to_string(),
            content: "parent child one child two".to_string(),
            metadata: json!({ "source": "test" }),
            enabled: true,
        },
        vec![
            prepared_chunk(0, "parent", KnowledgeChunkType::ParentText, None, false),
            prepared_chunk(1, "child one", KnowledgeChunkType::Text, Some(0), true),
            prepared_chunk(2, "child two", KnowledgeChunkType::Text, Some(0), true),
        ],
    )?;

    let chunks = store.list_chunks(&tenant, &document.id)?.expect("created document chunks");
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].chunk_type, KnowledgeChunkType::ParentText);
    assert_eq!(chunks[1].parent_chunk_id.as_deref(), Some(chunks[0].id.as_str()));
    assert_eq!(chunks[2].parent_chunk_id.as_deref(), Some(chunks[0].id.as_str()));
    assert_eq!(chunks[0].next_chunk_id, None);
    assert_eq!(chunks[1].prev_chunk_id, None);
    assert_eq!(chunks[1].next_chunk_id.as_deref(), Some(chunks[2].id.as_str()));
    assert_eq!(chunks[2].prev_chunk_id.as_deref(), Some(chunks[1].id.as_str()));
    assert_eq!(chunks[1].metadata["source"], "test");
    assert_eq!(chunks[1].metadata["local"], 1);
    assert_eq!(store.list_chunks(&tenant, "missing")?, None);

    fixture.cleanup()
}

#[test]
fn chunked_document_create_rejects_missing_dataset() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let store = PostgresKnowledgeStore::new(fixture.database().clone())?;
    let tenant = TenantScope::new("chunk-platform", "missing-dataset")?;

    let error = store
        .create_document_with_chunks(
            &tenant,
            "missing",
            KnowledgeDocumentCreateRequest {
                name: "Document".to_string(),
                content: "content".to_string(),
                metadata: json!({}),
                enabled: true,
            },
            Vec::new(),
        )
        .expect_err("missing dataset must be rejected");
    assert!(error.to_string().contains("knowledge dataset not found: missing"));

    fixture.cleanup()
}
