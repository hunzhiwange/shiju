// 验证知识导入入队前的校验及待处理文档与队列任务持久化。
use super::*;
use crate::global_database::GlobalDatabase;
use crate::global_database::test_support::PostgresTestDatabase;
use serde_json::json;

fn sqlite_backed_store() -> PostgresKnowledgeStore {
    let path = tempfile::tempdir().expect("temporary directory").keep().join("knowledge.sqlite");
    PostgresKnowledgeStore { database: GlobalDatabase::new(path).expect("SQLite global database") }
}

fn import_request(filename: &str, content_base64: &str) -> KnowledgeDocumentImportRequest {
    KnowledgeDocumentImportRequest {
        filename: filename.to_string(),
        mime_type: Some("Text/Plain; charset=utf-8".to_string()),
        content_base64: content_base64.to_string(),
        metadata: json!({ "source": "batch-1" }),
        enabled: true,
    }
}

#[test]
fn enqueue_import_rejects_missing_filename_before_database_access() {
    let store = sqlite_backed_store();

    let error = store
        .enqueue_import(&TenantScope::default(), "unused", import_request(" / ", "aGVsbG8="))
        .expect_err("missing filename must be rejected");

    assert_eq!(error.to_string(), "knowledge import filename is required");
}

#[test]
fn enqueue_import_rejects_invalid_base64_before_database_access() {
    let store = sqlite_backed_store();

    let error = store
        .enqueue_import(&TenantScope::default(), "unused", import_request("notes.txt", "%%%"))
        .expect_err("invalid base64 must be rejected");

    assert!(error.to_string().contains("knowledge import content_base64 is invalid"));
}

#[test]
fn enqueue_import_persists_pending_document_and_queued_job() {
    let fixture = crate::global_database::test_support::postgres_test_database_unit!();
    let database = fixture.database().clone();
    let store = PostgresKnowledgeStore::new(database.clone()).expect("PostgreSQL knowledge store");
    let tenant = TenantScope::new("platform-indexing", "company-indexing").expect("tenant");
    let dataset_id = "00000000-0000-4000-8000-000000000051";
    database
        .connect_postgres()
        .expect("PostgreSQL knowledge connection")
        .execute(
            "INSERT INTO knowledge_datasets
             (platform_id, company_id, id, name, description, indexing_mode, retrieval_mode,
              created_at_ms, updated_at_ms)
             VALUES ($1, $2, $3, 'Dataset', '', 'economy', 'full_text', 1, 1)",
            &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &dataset_id],
        )
        .expect("insert dataset");

    let queued = store
        .enqueue_import(&tenant, dataset_id, import_request(" nested/report.txt ", "aGVsbG8="))
        .expect("queue import");

    assert_eq!(queued.filename, "report.txt");
    assert_eq!(queued.mime_type.as_deref(), Some("text/plain"));
    assert_eq!(queued.source_bytes, 5);
    assert_eq!(queued.document.dataset_id, dataset_id);
    assert_eq!(queued.document.status, KnowledgeDocumentStatus::Pending);
    assert_eq!(queued.document.metadata, json!({ "source": "batch-1" }));
    assert_eq!(queued.job.dataset_id, dataset_id);
    assert_eq!(queued.job.document_id, queued.document.id);
    assert_eq!(queued.job.status, KnowledgeIndexJobStatus::Queued);
    assert!(queued.job.started_at_ms.is_none());
    assert!(queued.job.finished_at_ms.is_none());

    fixture.cleanup().expect("clean PostgreSQL test database");
}
