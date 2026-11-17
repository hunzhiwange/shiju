use super::*;
use crate::store::*;
use vw_api_transport::id::{ProjectId, ProjectScope};
fn scope() -> ProjectScope {
    ProjectScope::for_default_tenant(ProjectId("replica-fixture".into())).unwrap()
}

fn insert(db: &mut Connection, id: &str, time: i64, first: f64, app: &str) {
    let record: UsageRecord = serde_json::from_value(serde_json::json!({
        "id":id,"started_at_unix_seconds":time,"provider":{"kind":"unknown"},
        "sources":[{"source_id":id,"kind":"relay","app":app,
            "native_request_id":id,"native_session_id":"fixture"}],
        "models":{"request_model":"fixture","response_model":null,"pricing_model":null},
        "status":"succeeded","billable":true,"tokens":null,
        "cost":{"source":"unknown","amount_usd":null},
        "performance":{"first_token_ms":{"value":first,"kind":"measured"},
            "latency_ms":{"value":first+1000.0,"kind":"measured"},
            "output_tokens_per_second":null},"suspected_duplicate_of":null
    }))
    .unwrap();
    save_usage_record(
        db,
        &scope(),
        &UsageSourceIdentity {
            target_host: "host".into(),
            app: app.into(),
            instance_id: "fixture".into(),
            kind: UsageSourceKind::Relay,
            file_identity: "fixture".into(),
            native_session_id: Some("fixture".into()),
            native_request_id: id.into(),
        },
        &StoredUsageRecord { record, status_code: Some(200), failure: None },
    )
    .unwrap();
}

#[test]
fn usage_replica_05_provider_application_identity_survives_rollup() {
    let mut db = Connection::open_in_memory().unwrap();
    initialize_usage_store(&mut db).unwrap();
    insert(&mut db, "one", 1, 100.0, "codex");
    insert(&mut db, "two", 2, 200.0, "claude");
    let range = UsageResolvedRange {
        start_unix_seconds: None,
        end_unix_seconds: UsageUnixSeconds(50 * 86400),
        time_zone: "UTC".into(),
    };
    let read = |db: &Connection| {
        get_provider_stats(db, &scope(), "host", &range, &UsageFilter::default()).unwrap()
    };
    let rows = read(&db);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r.stats.counts.total_requests == 1));
    assert_ne!(rows[0].app, rows[1].app);
    crate::retention::rollup_and_prune(&mut db, &scope(), "host", 40 * 86400, || false).unwrap();
    let rows = read(&db);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r.stats.counts.total_requests == 1));
}
