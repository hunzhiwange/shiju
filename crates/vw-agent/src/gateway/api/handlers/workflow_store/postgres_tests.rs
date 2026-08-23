// 验证 PostgreSQL 工作流存储的租户隔离、创建时间与增删改查契约。
use super::*;

use crate::global_database::test_support::PostgresTestDatabase;

fn body(name: &str, description: &str, workflow_yaml: &str) -> WorkflowRecordUpsertBody {
    WorkflowRecordUpsertBody {
        uuid: None,
        name: name.to_string(),
        description: description.to_string(),
        workflow_yaml: workflow_yaml.to_string(),
    }
}

#[test]
fn postgres_workflow_store_crud_preserves_tenant_scope_and_creation_time() {
    let fixture = crate::global_database::test_support::postgres_test_database_unit!();
    let store = PostgresWorkflowStore::new(fixture.database().clone()).expect("workflow store");
    let tenant_a = TenantScope::new("platform-a", "company-a").expect("tenant A");
    let tenant_b = TenantScope::new("platform-a", "company-b").expect("tenant B");
    let uuid = "11111111-1111-4111-8111-111111111111";

    assert!(store.list_records(&tenant_a).expect("empty list").is_empty());
    assert!(store.query_record(&tenant_a, uuid).expect("missing record").is_none());

    let created = store
        .upsert_record(&tenant_a, uuid, &body("  First  ", "  description  ", "steps: []"), 41)
        .expect("create record")
        .expect("created record");
    assert_eq!(created.name, "First");
    assert_eq!(created.description, "description");
    assert_eq!(created.created_at_ms, 41);
    assert_eq!(created.updated_at_ms, 41);
    assert!(store.query_record(&tenant_b, uuid).expect("other tenant query").is_none());

    let updated = store
        .upsert_record(&tenant_a, uuid, &body("Second", "changed", "steps: [run]"), 84)
        .expect("update record")
        .expect("updated record");
    assert_eq!(updated.created_at_ms, 41);
    assert_eq!(updated.updated_at_ms, 84);
    assert_eq!(updated.workflow_yaml, "steps: [run]");

    let summaries = store.list_records(&tenant_a).expect("list records");
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].uuid, uuid);
    assert_eq!(summaries[0].name, "Second");
    assert_eq!(summaries[0].created_at_ms, 41);
    assert_eq!(summaries[0].updated_at_ms, 84);

    assert!(store.delete_record(&tenant_a, uuid).expect("delete record"));
    assert!(!store.delete_record(&tenant_a, uuid).expect("delete missing record"));
    assert!(store.query_record(&tenant_a, uuid).expect("deleted query").is_none());

    fixture.cleanup().expect("clean PostgreSQL test database");
}
