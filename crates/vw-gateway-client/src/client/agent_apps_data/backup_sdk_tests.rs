use super::super::test_support;
use super::*;
use serde_json::json;
use vw_api_types::{
    agent_apps::data_management::*,
    id::{ProjectScope, TenantScope},
};
fn target() -> DataManagementTarget {
    DataManagementTarget {
        scope: ProjectScope::new(TenantScope::default(), "isolated-project").unwrap(),
        target_host_id: "remote-executor".into(),
        principal_id: "anonymous".into(),
    }
}
#[tokio::test]
async fn cc_data_38_backup_sdk_decodes_protected_conflict() {
    // The production API returns bare 409 for both protection and revision conflict.
    let server = test_support::server_raw(vec![(409, String::new())]);
    let input = DeleteDataBackupRequest {
        target: target(),
        backup_id: DataBackupId("opaque/备份?#%".into()),
        expected_revision: "revision".into(),
        confirmed: true,
    };
    assert_eq!(
        server.client().data_backup_delete(&input, Some("grant")).await.unwrap_err(),
        DataMigrationError::Http { status: 409, error: None }
    );
    let request = server.take_request();
    assert_eq!(request.path, "/v1/agent-apps/data/backups/delete");
    assert_eq!(request.body, json!(input));
    assert_eq!(request.header("x-agent-apps-token"), Some("grant"));
    server.join();
}
#[tokio::test]
async fn cc_data_38_backup_sdk_preserves_remote_host_identity() {
    let target = target();
    let policy = json!({"intervalHours":0,"retainCount":4});
    let progress = json!({"target":target,"operationId":"operation","status":"completed",
        "rollbackBackupId":"rollback","result":{"committed":true,"projection":"refresh_required","retentionFailed":true}});
    let server = test_support::server(vec![
        (200, progress),
        (200, json!({"freedBytes":12,"undeletedCount":2})),
        (200, policy.clone()),
        (200, policy),
    ]);
    let client = server.client();
    let input = RestoreDataBackupRequest {
        target: target.clone(),
        backup_id: DataBackupId("source".into()),
        expected_revision: "revision".into(),
        categories: vec![DataCategory::Preferences],
        idempotency_key: "same-key".into(),
        confirmed: true,
    };
    let result = client.data_backup_restore(&input, None).await.unwrap();
    assert_eq!(result.target, target);
    assert_eq!(result.status, DataRestoreStatus::Completed);
    assert_eq!(result.result.unwrap().retention_failed, Some(true));
    let cleanup = client
        .data_backup_delete_location(
            &DeleteDataBackupLocationRequest {
                target: target.clone(),
                category_id: "logs".into(),
                confirmed: true,
            },
            None,
        )
        .await
        .unwrap();
    assert_eq!(cleanup.undeleted_count, 2);
    let policy = client.data_backup_policy(&target, None).await.unwrap();
    assert_eq!(policy.interval_hours, 0);
    assert_eq!(client.data_backup_set_policy(&target, &policy, None).await.unwrap(), policy);
    for route in ["backups/restore", "locations/delete", "policy", "policy"] {
        let request = server.take_request();
        assert_eq!(request.path, format!("/v1/agent-apps/data/{route}"));
        assert_eq!(request.method, "POST");
        assert_eq!(request.body["target"], json!(target));
    }
    server.join();
}
