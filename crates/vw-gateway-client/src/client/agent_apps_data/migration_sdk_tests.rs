use super::super::{test_support, DataMigrationError};
use vw_api_types::{agent_apps::data_management::*, id::{ProjectScope, TenantScope}};
use serde_json::json;

fn target() -> DataManagementTarget {
    DataManagementTarget {
        scope: ProjectScope::new(TenantScope::default(), "isolated-project").unwrap(),
        target_host_id: "executor".into(), principal_id: "anonymous".into(),
    }
}

#[tokio::test]
async fn cc_data_37_migration_sdk_preserves_operation_status() {
    let target = target();
    let progress: DataOperationProgress = serde_json::from_value(json!({
        "target": target, "operationId": "operation", "operation": "import",
        "status": "completed", "phase": "terminal", "completedItems": 1,
        "totalItems": 1, "cancelRequested": true, "committed": true,
        "rollback": "prepared", "rollbackBackupId": "rollback", "items": [],
        "result": {"kind": "partial", "errors": [{"code": "projection_failed", "category": "providers", "entryId": null}]},
        "createdBackup": null, "exportedTransferId": null
    })).unwrap();
    let server = test_support::server(vec![(200, json!(progress)); 4]);
    let input = GetDataOperationRequest { target: target.clone(), operation_id: progress.operation_id.clone() };
    let execute = ExecuteDataImportRequest {
        target: target.clone(), preview_id: DataPreviewId("preview".into()),
        expected_revision: "revision".into(), conflicts: vec![],
        idempotency_key: "same-key".into(), confirmed: true,
    };
    let cancel = CancelDataOperationRequest { target, operation_id: progress.operation_id.clone() };
    let client = server.client();
    assert_eq!(client.data_migration_execute(&execute, Some("grant")).await.unwrap(), progress);
    assert_eq!(client.data_migration_operation(&input, Some("grant")).await.unwrap(), progress);
    assert_eq!(client.data_migration_cancel(&cancel, Some("grant")).await.unwrap(), progress);
    assert_eq!(client.data_migration_retry_projections(&input, Some("grant")).await.unwrap(), progress);
    for (route, body) in [("import/execute", json!(execute)), ("operations/get", json!(input)), ("operations/cancel", json!(cancel)), ("operations/retry-projections", json!(input))] {
        let request = server.take_request();
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, format!("/v1/agent-apps/data/{route}"));
        assert_eq!(request.body, body);
        assert_eq!(request.header("x-agent-apps-token"), Some("grant"));
    }
    server.join();
}

#[tokio::test]
async fn cc_data_37_migration_sdk_does_not_retry_write() {
    for status in [307, 403, 409, 503] {
        let error = DataManagementError { code: DataManagementErrorCode::RevisionConflict, category: None, entry_id: None };
        let server = test_support::server(vec![(status, json!(error))]);
        let input = ExecuteDataImportRequest {
            target: target(), preview_id: DataPreviewId("preview".into()), expected_revision: "revision".into(),
            conflicts: vec![], idempotency_key: "same-key".into(), confirmed: true,
        };
        assert_eq!(server.client().data_migration_execute(&input, None).await.unwrap_err(),
            DataMigrationError::Http { status, error: Some(error) });
        assert_eq!(server.take_request().body, json!(input));
        server.join();
    }
}

#[tokio::test]
async fn cc_data_37_source_preview_and_portable_download() {
    let target = target();
    let summary = json!({"transferId":"staged", "kind":"cc_switch", "applicationVersion":"unknown", "schemaVersion":24, "formatVersion":0, "format":"sqlite", "digestSha256":"digest"});
    let preview = json!({"target":target, "previewId":"preview", "revision":"revision", "source":summary, "categories":[], "conflicts":[], "exclusions":[]});
    let server = test_support::server_raw(vec![(200, summary.to_string()), (200, preview.to_string()),
        (200, json!({"transferId":"download", "expiresInSeconds":300}).to_string()), (200, "portable-bytes".into())]);
    let client = server.client();
    let identified = client.data_migration_identify(&IdentifyDataSourceRequest { target:target.clone(), transfer_id:DataTransferId("authorized-source".into()) }, None).await.unwrap();
    assert_eq!(identified.transfer_id.0, "staged");
    let selections = vec![DataSelection { category:DataCategory::Preferences, entries:DataEntrySelection::All }];
    let result = client.data_migration_preview(&PreviewDataImportRequest { target:target.clone(), transfer_id:identified.transfer_id, selections:selections.clone() }, None).await.unwrap();
    assert_eq!(result.revision, "revision");
    let export = client.data_migration_export(&ExportDataRequest { target:target.clone(), selections, format:DataArchiveFormat::NativePackage, idempotency_key:"export-key".into() }, None).await.unwrap();
    assert_eq!(export.expires_in_seconds, 300);
    assert_eq!(client.data_migration_download(&target, &export.transfer_id, None).await.unwrap(), b"portable-bytes");
    for route in ["sources/identify", "import/preview", "export", "export/download"] {
        assert_eq!(server.take_request().path, format!("/v1/agent-apps/data/{route}"));
    }
    server.join();
}
