use anyhow::Result;
use vw_api_types::TenantScope;
use vw_api_types::knowledge::{
    KnowledgeChunkingMode, KnowledgeDatasetCreateRequest, KnowledgeIndexingMode,
    KnowledgeRetrievalMode,
};

use super::{PostgresKnowledgeStore, i64_from_u64, u64_from_i64, usize_from_i64};
use crate::global_database::test_support::PostgresTestDatabase;

fn dataset_request(name: &str, top_k: usize) -> KnowledgeDatasetCreateRequest {
    KnowledgeDatasetCreateRequest {
        name: format!("  {name}  "),
        description: "  description  ".to_string(),
        chunking_mode: KnowledgeChunkingMode::General,
        indexing_mode: KnowledgeIndexingMode::Economy,
        retrieval_mode: KnowledgeRetrievalMode::FullText,
        keyword_count: 7,
        top_k,
        score_threshold_enabled: true,
        score_threshold: 0.25,
        rerank_enabled: false,
        embedding_model: None,
        rerank_model: None,
    }
}

#[test]
fn dataset_integer_conversions_saturate_or_default_at_boundaries() {
    assert_eq!(i64_from_u64(42), 42);
    assert_eq!(i64_from_u64(u64::MAX), i64::MAX);
    assert_eq!(u64_from_i64(42), 42);
    assert_eq!(u64_from_i64(-1), 0);
    assert_eq!(usize_from_i64(42), 42);
    assert_eq!(usize_from_i64(-1), 0);
}

#[test]
fn dataset_lifecycle_trims_fields_clamps_top_k_and_isolates_tenants() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let store = PostgresKnowledgeStore::new(fixture.database().clone())?;
    let tenant = TenantScope::new("dataset-platform", "dataset-primary")?;
    let other = TenantScope::new("dataset-platform", "dataset-other")?;

    let dataset = store.create_dataset(&tenant, dataset_request("Primary", 0))?;
    assert_eq!(dataset.name, "Primary");
    assert_eq!(dataset.description, "description");
    assert_eq!(dataset.top_k, 1);
    assert_eq!(store.list_datasets(&tenant)?, vec![dataset.clone()]);
    assert_eq!(store.get_dataset(&other, &dataset.id)?, None);

    assert_eq!(store.delete_dataset(&tenant, &dataset.id)?, Some(dataset));
    assert!(store.list_datasets(&tenant)?.is_empty());
    assert_eq!(store.delete_dataset(&tenant, "missing")?, None);

    fixture.cleanup()
}
