//! HTTP fixtures only; never use ambient home or user configuration.
use super::{backup_crud_api, backup_restore_api};
use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::Extension;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt};
use vw_api_types::id::{ProjectScope, TenantScope};

struct Fixture {
    _root: tempfile::TempDir,
    server: tokio::task::JoinHandle<()>,
    url: String,
    target: Value,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}
impl Fixture {
    async fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let directory = root.path().canonicalize().unwrap();
        let backups = directory.join("backups");
        fs::create_dir(&backups).unwrap();
        fs::set_permissions(&backups, fs::Permissions::from_mode(0o700)).unwrap();
        let policy = directory.join("policy.json");
        let token = "anonymous-backup-fixture-token-00001";
        fs::write(&policy, json!({"principals":[{"id":"reader", "token_sha256":hex::encode(Sha256::digest(token)),
            "grants":[{"platform_id":"p","company_id":"c","host_id":"h","data_backups":["project"]}]}],
            "hosts":[{"id":"h","online":true,"binding":"local","home":directory,
            "applications":directory,"search_paths":[],"data_backup_root":backups}]}).to_string()).unwrap();
        fs::set_permissions(&policy, fs::Permissions::from_mode(0o600)).unwrap();
        let tenant = TenantScope::new("p", "c").unwrap();
        let project = ProjectScope::new(tenant.clone(), "project".into()).unwrap();
        let db = GlobalDatabase::new(directory.join("fixture.sqlite")).unwrap();
        GatewaySettingsStore::new(db.clone())
            .unwrap()
            .upsert(&tenant, "agent_apps_user", "reader", r#"{"appOrder":[],"visibleApps":[]}"#)
            .unwrap();
        let app = backup_crud_api::router_with_policy(Some(policy.clone()))
            .merge(backup_restore_api::router_with_policy(Some(policy)))
            .layer(Extension(tenant))
            .layer(Extension(db));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Self {
            _root: root,
            server,
            url,
            target: json!({"scope":project,"targetHostId":"h","principalId":"reader"}),
        }
    }
    async fn call(&self, operation: &str, request: Value) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("{}/agent-apps/data/backups/{operation}", self.url))
            .header("x-agent-apps-token", "anonymous-backup-fixture-token-00001")
            .json(&request)
            .send()
            .await
            .unwrap()
    }
    async fn create(&self, key: &str) -> Value {
        let response = self
            .call(
                "create",
                json!({"target":self.target,"categories":["preferences"],
            "kind":"local_rollback","name":"fixture","idempotencyKey":key}),
            )
            .await;
        assert_eq!(response.status(), 200);
        response.json().await.unwrap()
    }
}


// macOS-only module registration ensures this exercises the native file locks.
#[tokio::test]
async fn cc_data_46_macos_backup_restore_retention_round_trip() {
    use vw_agent_apps::data_management::{
        backup_create::{LocalBackupReceipt, open_local_backup},
        retention::{RetentionPolicy, cleanup_local_backups},
        safety_snapshot::SafetySnapshotScope,
    };
    let f = Fixture::new().await;
    let created = f.create("source").await;
    let response = f.call("rename", json!({"target":f.target,
        "backupId":created["backupId"], "expectedRevision":created["revision"],
        "name":"匿名恢复验收"})).await;
    assert_eq!(response.status(), 200);
    let source: Value = response.json().await.unwrap();
    let connection = rusqlite::Connection::open(f._root.path().join("fixture.sqlite")).unwrap();
    connection.execute("UPDATE gateway_settings SET payload_json=?1 WHERE namespace='agent_apps_user'",
        [r#"{"appOrder":["codex"],"visibleApps":[]}"#]).unwrap();
    // Unselected owner and foreign tenant must survive the selected restore.
    connection.execute("INSERT INTO gateway_settings(platform_id,company_id,namespace,config_key,payload_json,updated_at_ms) VALUES ('p','c','acceptance_sentinel','reader','{}',0),('p','foreign','agent_apps_user','reader','{}',0)", []).unwrap();
    let response = f.call("restore", json!({"target":f.target,
        "backupId":source["backupId"],"expectedRevision":source["revision"],
        "categories":["preferences"],"idempotencyKey":"round-trip","confirmed":true})).await;
    assert_eq!(response.status(), 200);
    let queued: Value = response.json().await.unwrap();
    let mut terminal = None;
    for _ in 0..100 {
        let response = f.call("restore/get", json!({"target":f.target,
            "operationId":queued["operationId"]})).await;
        assert_eq!(response.status(), 200);
        let progress: Value = response.json().await.unwrap();
        if progress["status"] == "completed" || progress["status"] == "failed" {
            terminal = Some(progress);
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let terminal = terminal.expect("restore must reach a terminal state within the bound");
    assert_eq!(terminal["status"], "completed");
    assert_eq!(terminal["result"]["committed"], true);
    assert!(terminal["rollbackBackupId"].as_str().is_some_and(|s| !s.is_empty()));
    let restored: String = connection.query_row(
        "SELECT payload_json FROM gateway_settings WHERE platform_id='p' AND company_id='c' AND namespace='agent_apps_user'",
        [], |r| r.get(0)).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&restored).unwrap(), json!({"appOrder":[],"visibleApps":[]}));
    let sentinels: i64 = connection.query_row(
        "SELECT COUNT(*) FROM gateway_settings WHERE payload_json='{}' AND (namespace='acceptance_sentinel' OR company_id='foreign')",
        [], |r| r.get(0)).unwrap();
    assert_eq!(sentinels, 2);

    let root = f._root.path().canonicalize().unwrap().join("backups");
    let id = source["backupId"].as_str().unwrap();
    let path = root.join(format!("{id}.local-backup"));
    let bytes = fs::read(&path).unwrap();
    let receipt: LocalBackupReceipt = serde_json::from_value(json!({
        "id":id,"digest":<[u8;32]>::from(Sha256::digest(&bytes))})).unwrap();
    let scope = SafetySnapshotScope {
        project: serde_json::from_value(f.target["scope"].clone()).unwrap(),
        principal_id: "reader".into(), host_id: "h".into(),
    };
    let guard = open_local_backup(&root, &receipt, &scope).unwrap();
    let disposable = f.create("disposable").await;
    let report = cleanup_local_backups(&root, &scope, RetentionPolicy::new(1).unwrap()).unwrap();
    assert_eq!(
        vw_agent_apps::data_management::backup_delete::delete_local_backup(&root, &scope, id),
        Err(vw_agent_apps::data_management::backup_delete::BackupDeleteError::Protected),
    );
    assert!(report.deleted.contains(&disposable["backupId"].as_str().unwrap().to_owned()));
    assert_eq!(fs::read(&path).unwrap(), bytes);
    drop(guard);
    let response = f.call("delete", json!({"target":f.target,"backupId":id,
        "expectedRevision":source["revision"],"confirmed":true})).await;
    assert_eq!(response.status(), 200);
    let response = f.call("list", json!({"target":f.target})).await;
    assert_eq!(response.status(), 200);
    let listed: Vec<Value> = response.json().await.unwrap();
    assert!(listed.iter().all(|entry| entry["backupId"] != source["backupId"]));
}

#[tokio::test]
async fn cc_data_46_macos_corrupt_backup_preserves_target() {
    let f = Fixture::new().await;
    let source = f.create("invalid-source").await;
    let root = f._root.path().canonicalize().unwrap().join("backups");
    let path = root.join(format!("{}.local-backup", source["backupId"].as_str().unwrap()));
    let original = fs::read(&path).unwrap();
    let mut future: Value = serde_json::from_slice(&original).unwrap();
    future["format_version"] = json!(u32::MAX);
    for invalid in [b"corrupt anonymous archive".to_vec(), serde_json::to_vec(&future).unwrap()] {
        fs::write(&path, &invalid).unwrap();
        let before = fs::read(f._root.path().join("fixture.sqlite")).unwrap();
        let mut names_before: Vec<_> = fs::read_dir(&root).unwrap().map(|e| e.unwrap().file_name()).collect();
        names_before.sort();
        for operation in ["restore/preview", "restore"] {
            let request = if operation == "restore" {
                json!({"target":f.target,"backupId":source["backupId"],
                    "expectedRevision":source["revision"],"categories":["preferences"],
                    "idempotencyKey":"invalid-restore","confirmed":true})
            } else {
                json!({"target":f.target,"backupId":source["backupId"]})
            };
            let response = f.call(operation, request).await;
            assert!(!response.status().is_success(), "invalid archive must fail before queuing");
        }
        assert_eq!(fs::read(f._root.path().join("fixture.sqlite")).unwrap(), before);
        assert_eq!(fs::read(&path).unwrap(), invalid);
        let mut names_after: Vec<_> = fs::read_dir(&root).unwrap().map(|e| e.unwrap().file_name()).collect();
        names_after.sort();
        assert_eq!(names_before, names_after, "rejection must not publish safety snapshots");
    }
}
