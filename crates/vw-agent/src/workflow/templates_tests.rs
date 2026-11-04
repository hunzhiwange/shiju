//! 真实数据库合同：初始化不可覆盖编辑，应用 API 与原生入口共享记录。
use super::{store, templates::*};
use crate::{
    gateway::api::handlers::workflow::applications_router, global_database::GlobalDatabase,
};
use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use std::sync::Arc;
use tokio::sync::Barrier;
use tower::ServiceExt;
use vw_api_types::{
    TenantScope,
    workflow::{WorkflowRecord, WorkflowRecordSummary, WorkflowRecordUpsertBody},
};

fn yaml(sentinel: &str, parallel: &str) -> String {
    format!(
        r#"app:
  name: editable test workflow
workflow:
  video_edit:
    max_parallel: {parallel}
  graph:
    nodes:
      - id: start
        data: {{type: start}}
      - id: text
        data:
          type: template-transform
          template: '{sentinel}'
      - id: end
        data: {{type: end}}
    edges:
      - {{source: start, target: text}}
      - {{source: text, target: end}}
"#
    )
}

fn seed() -> VideoEditTemplateSeed {
    VideoEditTemplateSeed::new(yaml("seed text", "5")).unwrap()
}

fn body(yaml: String) -> WorkflowRecordUpsertBody {
    WorkflowRecordUpsertBody {
        uuid: None,
        name: VIDEO_EDIT_TEMPLATE_NAME.into(),
        description: "edited".into(),
        workflow_yaml: yaml,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn video_edit_template_sqlite_contract() {
    let home = tempfile::tempdir().unwrap();
    let database = GlobalDatabase::for_home(home.path()).unwrap();
    contract(database).await;
    let reopened = GlobalDatabase::for_home(home.path()).unwrap();
    assert_reopened_edit(reopened).await;
}

#[test]
fn video_edit_template_postgres_contract() {
    // 同步 PostgreSQL 夹具自带运行时，在 Tokio 外创建、重开和清理。
    let fixture = crate::global_database::test_support::postgres_test_database_unit!();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(contract(fixture.database().clone()));
    let reopened = fixture.reopen().unwrap();
    runtime.block_on(assert_reopened_edit(reopened));
    fixture.cleanup().unwrap();
}

async fn contract(database: GlobalDatabase) {
    let tenant = TenantScope::new("template-contract", "company-a").unwrap();
    let other = TenantScope::new("template-contract", "company-b").unwrap();
    let seed = seed();
    // 屏障确保首次注册同时竞争，不以顺序调用冒充并发。
    let barrier = Arc::new(Barrier::new(12));
    let mut tasks = Vec::new();
    for _ in 0..12 {
        let (seed, db, tenant, barrier) =
            (seed.clone(), database.clone(), tenant.clone(), barrier.clone());
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            seed.load(db, tenant).await.unwrap()
        }));
    }
    let mut snapshots = Vec::new();
    for task in tasks {
        snapshots.push(task.await.unwrap());
    }
    let initial = snapshots.first().unwrap().clone();
    assert!(snapshots.iter().all(|s| s.record == initial.record && s.digest == initial.digest));
    assert_eq!(initial.record.uuid, VIDEO_EDIT_TEMPLATE_UUID);
    assert_eq!(initial.record.name, VIDEO_EDIT_TEMPLATE_NAME);
    assert_eq!(initial.options.max_parallel(), 5);
    assert_eq!(store::list_records(database.clone(), tenant.clone()).await.unwrap().len(), 1);

    let other_record = seed.load(database.clone(), other.clone()).await.unwrap();
    let version = initial.record.updated_at_ms;
    let barrier = Arc::new(Barrier::new(12));
    let mut initializers = Vec::new();
    for _ in 0..10 {
        let (seed, db, tenant, barrier) =
            (seed.clone(), database.clone(), tenant.clone(), barrier.clone());
        initializers.push(tokio::spawn(async move {
            barrier.wait().await;
            seed.load(db, tenant).await.unwrap()
        }));
    }
    let mut writers = Vec::new();
    for sentinel in ["saved sentinel A", "saved sentinel B"] {
        let (db, tenant, barrier) = (database.clone(), tenant.clone(), barrier.clone());
        writers.push(tokio::spawn(async move {
            barrier.wait().await;
            save_template(db, tenant, body(yaml(sentinel, "3")), version).await
        }));
    }
    let results = [writers.remove(0).await.unwrap(), writers.remove(0).await.unwrap()];
    for task in initializers {
        task.await.unwrap();
    }
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert!(matches!(
        results.iter().find_map(|result| result.as_ref().err()).unwrap(),
        WorkflowStoreError::Conflict(_)
    ));
    let edited = seed.load(database.clone(), tenant.clone()).await.unwrap();
    assert!(edited.record.workflow_yaml.contains("saved sentinel"));
    assert!(edited.record.updated_at_ms > version);
    assert_ne!(edited.digest, initial.digest);
    assert_eq!(initial.options.max_parallel(), 5); // 运行中的旧快照保持不变。
    assert_eq!(edited.options.max_parallel(), 3);
    assert_eq!(edited.record.created_at_ms, initial.record.created_at_ms);
    assert_eq!(
        seed.load(database.clone(), other.clone()).await.unwrap().record,
        other_record.record
    );

    for invalid in ["0", "6", "-1", "1.5", "null", "true", "'3'"] {
        let error = save_template(
            database.clone(),
            tenant.clone(),
            body(yaml("invalid", invalid)),
            edited.record.updated_at_ms,
        )
        .await
        .unwrap_err();
        assert!(matches!(error, WorkflowStoreError::Invalid(_)));
    }
    assert_eq!(seed.load(database.clone(), tenant.clone()).await.unwrap().record, edited.record);
    assert!(
        store::delete_record(database.clone(), tenant.clone(), VIDEO_EDIT_TEMPLATE_UUID.into())
            .await
            .is_err()
    );
    assert!(
        store::create_record(
            database.clone(),
            tenant.clone(),
            WorkflowRecordUpsertBody {
                uuid: Some(VIDEO_EDIT_TEMPLATE_UUID.into()),
                ..body(yaml("bypass", "5"))
            }
        )
        .await
        .is_err()
    );
    assert!(
        store::update_record(
            database.clone(),
            tenant.clone(),
            VIDEO_EDIT_TEMPLATE_UUID.into(),
            body(yaml("bypass", "5"))
        )
        .await
        .is_err()
    );
    assert!(
        seed.restore(database.clone(), tenant.clone(), edited.record.updated_at_ms, false)
            .await
            .is_err()
    );
    assert_eq!(seed.load(database.clone(), tenant.clone()).await.unwrap().record, edited.record);
    let restored = seed
        .restore(database.clone(), tenant.clone(), edited.record.updated_at_ms, true)
        .await
        .unwrap();
    assert_eq!(restored.workflow_yaml, initial.record.workflow_yaml);
    assert!(matches!(
        seed.restore(database.clone(), tenant.clone(), edited.record.updated_at_ms, true)
            .await
            .unwrap_err(),
        WorkflowStoreError::Conflict(_)
    ));
    // 重启前再次保存非占位哨兵，验证重新打开数据库不会恢复种子。
    save_template(
        database.clone(),
        tenant.clone(),
        body(yaml("restart sentinel", "2")),
        restored.updated_at_ms,
    )
    .await
    .unwrap();

    // 普通记录依然允许 upsert、导出和删除。
    let normal =
        store::create_record(database.clone(), tenant.clone(), body("ordinary: yaml".into()))
            .await
            .unwrap();
    let changed = store::update_record(
        database.clone(),
        tenant.clone(),
        normal.uuid.clone(),
        body("ordinary: changed".into()),
    )
    .await
    .unwrap();
    assert_eq!(changed.workflow_yaml, "ordinary: changed");
    assert!(store::delete_record(database, tenant, normal.uuid).await.unwrap().deleted);
}

async fn assert_reopened_edit(database: GlobalDatabase) {
    let tenant = TenantScope::new("template-contract", "company-a").unwrap();
    let snapshot = seed().load(database.clone(), tenant.clone()).await.unwrap();
    assert!(snapshot.record.workflow_yaml.contains("restart sentinel"));
    assert_eq!(snapshot.options.max_parallel(), 2);
    assert_eq!(
        store::get_record(database, tenant, VIDEO_EDIT_TEMPLATE_UUID.into()).await.unwrap(),
        snapshot.record
    );
}

#[test]
fn video_edit_template_seed_rejects_invalid_graph_or_options() {
    assert!(VideoEditTemplateSeed::new("workflow: {}".into()).is_err());
    assert!(VideoEditTemplateSeed::new(yaml("seed", "6")).is_err());
    assert!(
        VideoEditTemplateSeed::new(yaml("seed", "5").replace("max_parallel: 5", "unknown: 1"))
            .is_err()
    );
    let no_options = yaml("seed", "5").replace("  video_edit:\n    max_parallel: 5\n", "");
    assert_eq!(validate_template_yaml(&no_options).unwrap().max_parallel(), 5);
}

async fn request(
    router: &Router,
    method: &str,
    path: &str,
    payload: Option<serde_json::Value>,
    version: Option<u64>,
) -> (StatusCode, serde_json::Value) {
    let mut request =
        Request::builder().method(method).uri(path).header("content-type", "application/json");
    if let Some(version) = version {
        request = request.header("x-workflow-expected-updated-at-ms", version.to_string());
    }
    let response = router
        .clone()
        .oneshot(
            request.body(Body::from(payload.map(|p| p.to_string()).unwrap_or_default())).unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or_default())
}

#[tokio::test]
async fn video_edit_template_applications_api_shares_snapshot_and_checks_versions() {
    let tenant = TenantScope::new("video-template-api", uuid::Uuid::new_v4().to_string()).unwrap();
    let database = GlobalDatabase::for_home(&crate::global::paths().home).unwrap();
    let seed = seed();
    let router: Router =
        applications_router().layer(Extension(tenant.clone())).layer(Extension(seed.clone()));
    let path = format!("/workflow/applications/{VIDEO_EDIT_TEMPLATE_UUID}");
    let (status, json) = request(&router, "GET", "/workflow/applications", None, None).await;
    assert_eq!(status, StatusCode::OK);
    let records: Vec<WorkflowRecordSummary> = serde_json::from_value(json).unwrap();
    assert!(records.iter().any(|record| record.uuid == VIDEO_EDIT_TEMPLATE_UUID));
    let (status, json) = request(&router, "GET", &path, None, None).await;
    assert_eq!(status, StatusCode::OK);
    let record: WorkflowRecord = serde_json::from_value(json).unwrap();
    assert_eq!(record, seed.load(database.clone(), tenant.clone()).await.unwrap().record);
    let edited = body(yaml("API edit sentinel", "1"));
    let payload = serde_json::to_value(&edited).unwrap();
    assert_eq!(
        request(&router, "PUT", &path, Some(payload.clone()), None).await.0,
        StatusCode::BAD_REQUEST
    );
    let (status, json) =
        request(&router, "PUT", &path, Some(payload.clone()), Some(record.updated_at_ms)).await;
    assert_eq!(status, StatusCode::OK);
    let saved: WorkflowRecord = serde_json::from_value(json).unwrap();
    assert_eq!(
        request(&router, "PUT", &path, Some(payload), Some(record.updated_at_ms)).await.0,
        StatusCode::CONFLICT
    );
    let (status, json) = request(&router, "GET", &path, None, None).await;
    assert_eq!(status, StatusCode::OK);
    let exported: WorkflowRecord = serde_json::from_value(json).unwrap();
    let snapshot = seed.load(database.clone(), tenant.clone()).await.unwrap();
    assert_eq!(exported, snapshot.record);
    assert!(snapshot.record.workflow_yaml.contains("API edit sentinel"));
    assert_eq!(
        snapshot.digest,
        hex::encode(<sha2::Sha256 as sha2::Digest>::digest(exported.workflow_yaml.as_bytes()))
    );
    assert_eq!(request(&router, "DELETE", &path, None, None).await.0, StatusCode::BAD_REQUEST);
    let restore_path = format!("{path}/restore-default");
    assert_eq!(
        request(
            &router,
            "POST",
            &restore_path,
            Some(serde_json::json!({"confirmed": false})),
            Some(saved.updated_at_ms)
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        request(
            &router,
            "POST",
            &restore_path,
            Some(serde_json::json!({"confirmed": true})),
            Some(saved.updated_at_ms)
        )
        .await
        .0,
        StatusCode::OK
    );
    // 清理本测试创建的随机租户，不经由禁止删除系统模板的公共 API。
    database
        .connect()
        .unwrap()
        .execute(
            "DELETE FROM workflow_workflows WHERE platform_id = ?1 AND company_id = ?2",
            rusqlite::params![tenant.platform_id().as_ref(), tenant.company_id().as_ref()],
        )
        .unwrap();
}
