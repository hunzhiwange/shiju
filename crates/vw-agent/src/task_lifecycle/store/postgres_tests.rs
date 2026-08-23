//! 验证 PostgreSQL 生命周期存储对后端类型与作用域的约束。
//! PostgreSQL 夹具固定租户作用域，并验证时间戳转换采用饱和策略。

use super::*;
use crate::global_database::test_support::PostgresTestDatabase;
use crate::task_lifecycle::{DEFAULT_TASK_LIFECYCLE_WORKFLOW_ID, built_in_workflow_by_id};

fn custom_workflow(id: &str, label: &str) -> TaskLifecycleWorkflowDto {
    let mut workflow = built_in_workflow_by_id(DEFAULT_TASK_LIFECYCLE_WORKFLOW_ID).unwrap();
    workflow.id = id.to_string();
    workflow.label = label.to_string();
    workflow.built_in = false;
    workflow
}

#[test]
fn postgres_store_crud_is_tenant_scoped_and_saturates_timestamps() {
    let fixture = crate::global_database::test_support::postgres_test_database_unit!();
    let store = PostgresTaskLifecycleStore::new(fixture.database().clone()).unwrap();
    let tenant_a = TenantScope::new("platform-a", "company-a").unwrap();
    let tenant_b = TenantScope::new("platform-a", "company-b").unwrap();
    let mut workflow = custom_workflow("custom", "First");

    store.save_custom_workflow(&tenant_a, &workflow, u64::MAX, u64::MAX).unwrap();
    assert!(store.query_custom_workflows(&tenant_b).unwrap().is_empty());

    let stored = store.query_custom_workflow(&tenant_a, "custom").unwrap().unwrap();
    assert_eq!(stored.workflow, workflow);
    assert_eq!(stored.created_at_ms, i64::MAX as u64);

    workflow.label = "Updated".to_string();
    store.save_custom_workflow(&tenant_a, &workflow, 1, 2).unwrap();
    let listed = store.query_custom_workflows(&tenant_a).unwrap();
    assert_eq!(listed, vec![workflow.clone()]);
    let stored = store.query_custom_workflow(&tenant_a, "custom").unwrap().unwrap();
    assert_eq!(stored.workflow.label, "Updated");
    assert_eq!(stored.created_at_ms, i64::MAX as u64);

    assert_eq!(store.delete_custom_workflow(&tenant_b, "custom").unwrap(), 0);
    assert_eq!(store.delete_custom_workflow(&tenant_a, "custom").unwrap(), 1);
    assert!(store.query_custom_workflow(&tenant_a, "custom").unwrap().is_none());
    fixture.cleanup().expect("clean PostgreSQL test database");
}
