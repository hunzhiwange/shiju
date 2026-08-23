// 验证 PostgreSQL 知识检索的过滤、排序与结果数量边界。

// 检索模式与数据集作用域先解析，非法 ID 不应触发任何数据库访问。
use super::*;
use crate::global_database::GlobalDatabase;
use crate::global_database::test_support::PostgresTestDatabase;

fn sqlite_backed_store() -> PostgresKnowledgeStore {
    let path = tempfile::tempdir().expect("temporary directory").keep().join("knowledge.sqlite");
    PostgresKnowledgeStore { database: GlobalDatabase::new(path).expect("SQLite global database") }
}

#[test]
fn resolve_retrieval_scope_rejects_invalid_dataset_id_before_database_access() {
    let store = sqlite_backed_store();

    let error = store
        .resolve_retrieval_scope(&TenantScope::default(), &["not-a-uuid".to_string()])
        .expect_err("invalid dataset id must be rejected");

    assert_eq!(error.to_string(), "knowledge dataset_id is invalid");
}

#[test]
fn resolve_retrieval_scope_classifies_modes_and_rejects_missing_dataset() {
    let fixture = crate::global_database::test_support::postgres_test_database_unit!();
    let database = fixture.database().clone();
    let store = PostgresKnowledgeStore::new(database.clone()).expect("PostgreSQL knowledge store");
    let tenant = TenantScope::new("platform-retrieval", "company-retrieval").expect("tenant");
    let full_text_id = "00000000-0000-4000-8000-000000000001";
    let vector_id = "00000000-0000-4000-8000-000000000002";
    let hybrid_id = "00000000-0000-4000-8000-000000000003";
    let mut connection = database.connect_postgres().expect("PostgreSQL knowledge connection");
    for (id, mode, keyword_count, rerank_enabled) in [
        (full_text_id, "full_text", 7_i64, false),
        (vector_id, "vector", 11_i64, true),
        (hybrid_id, "hybrid", 13_i64, false),
    ] {
        connection
            .execute(
                "INSERT INTO knowledge_datasets
                 (platform_id, company_id, id, name, description, indexing_mode, retrieval_mode,
                  keyword_count, rerank_enabled, created_at_ms, updated_at_ms)
                 VALUES ($1, $2, $3, 'Dataset', '', 'economy', $4, $5, $6, 1, 1)",
                &[
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &id,
                    &mode,
                    &keyword_count,
                    &rerank_enabled,
                ],
            )
            .expect("insert retrieval dataset");
    }
    drop(connection);

    let scope = store.resolve_retrieval_scope(&tenant, &[]).expect("resolve all datasets");

    assert_eq!(
        scope
            .keyword_datasets
            .iter()
            .map(|dataset| (dataset.id.as_str(), dataset.keyword_count))
            .collect::<Vec<_>>(),
        vec![(full_text_id, 7), (hybrid_id, 13)]
    );
    assert_eq!(scope.vector_dataset_ids, vec![vector_id, hybrid_id]);
    assert_eq!(scope.rerank_dataset_ids, vec![vector_id]);

    let missing_id = "00000000-0000-4000-8000-000000000099".to_string();
    let error = store
        .resolve_retrieval_scope(&tenant, std::slice::from_ref(&missing_id))
        .expect_err("missing dataset must be rejected");
    assert_eq!(error.to_string(), format!("knowledge dataset not found: {missing_id}"));

    fixture.cleanup().expect("clean PostgreSQL test database");
}
