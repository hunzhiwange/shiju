//! Deferred verification: these tests have not been executed by task 37.
use rusqlite::Connection;
use app_usage::logs::*;
use app_usage_store::store::*;
use {
    vw_api_transport::id::ProjectId,
    vw_api_transport::id::ProjectScope,
    vw_api_usage::*,
};

fn scope(project: &str) -> ProjectScope {
    ProjectScope::for_default_tenant(ProjectId(project.into())).unwrap()
}
fn insert(db: &mut Connection, project: &str, host: &str, id: &str, time: i64, code: u16) {
    let record: UsageRecord = serde_json::from_value(serde_json::json!({
        "id": id, "started_at_unix_seconds": time, "provider": {"kind":"unknown"},
        "sources": [{"source_id":"native-source", "kind":"native", "app":"codex",
            "native_request_id":id, "native_session_id":"session"}],
        "models":{"request_model":"request", "response_model":"response", "pricing_model":"price",
            "pricing_multiplier":null, "multiplier_basis":null},
        "status":"failed", "billable":false, "tokens":null,
        "cost":{"source":"unknown", "amount_usd":null},
        "performance":{"first_token_ms":null, "latency_ms":null,
            "output_tokens_per_second":{"value":12.0,"kind":"estimated"}},
        "suspected_duplicate_of":null
    })).unwrap();
    save_usage_record(db, &scope(project), &UsageSourceIdentity {
        target_host: host.into(), app:"codex".into(), instance_id:"instance".into(),
        kind:UsageSourceKind::Native, file_identity:"private-file".into(),
        native_session_id:Some("session".into()), native_request_id:id.into(),
    }, &StoredUsageRecord { record, status_code:Some(code), failure:Some(UsageFailure::Upstream) }).unwrap();
}
fn query() -> UsageQuery {
    UsageQuery { range:UsageRange::Absolute {
        start_unix_seconds:UsageUnixSeconds(10), end_unix_seconds:UsageUnixSeconds(20),
    }, filter:UsageFilter {
        app:Some("codex".into()), source_kind:Some(UsageSourceKind::Native),
        provider:Some(UsageProvider::Unknown), request_model:Some("request".into()),
        response_model:Some("response".into()), pricing_model:Some("price".into()),
        native_session_id:Some("session".into()), status:Some(UsageRequestStatus::Failed),
        status_code:None, billable:Some(false),
    }, cursor:None, limit:Some(1) }
}
#[test]
fn usage17_logs_combined_filters_keep_total_and_detail_consistent() {
    let mut db = Connection::open_in_memory().unwrap();
    initialize_usage_store(&mut db).unwrap();
    for (id,time,code) in [("a",10,429),("b",10,500),("before",9,408),("end",20,504)] {
        insert(&mut db,"p","host",id,time,code);
    }
    insert(&mut db,"other","host","foreign-project",10,401);
    insert(&mut db,"p","other","foreign-host",10,403);
    let mut q = query();
    let first = query_usage_logs(&db,&scope("p"),"host",&q,"UTC",UsageUnixSeconds(30)).unwrap();
    assert_eq!(first.counts.total_requests,2);
    assert_eq!(first.counts.billable_requests,0);
    assert_eq!(first.counts.unknown_cost_requests,2);
    assert_eq!(first.sources.len(),1);
    assert_eq!(first.sources[0].counts.total_requests,2);
    assert_eq!(first.page.items[0].record.id,"b");
    assert_eq!(first.status_codes, vec![429, 500]);
    q.cursor = first.page.next_cursor;
    let second = query_usage_logs(&db,&scope("p"),"host",&q,"UTC",UsageUnixSeconds(40)).unwrap();
    assert_eq!(second.page.items[0].record.id,"a");
    assert!(second.page.next_cursor.is_none());
    let detail = get_usage_log_detail(&db,&scope("p"),"host","a").unwrap().unwrap();
    assert_eq!(detail.record,second.page.items[0].record);
    assert_eq!(detail.record.provider,UsageProvider::Unknown);
    assert_eq!(detail.record.performance.output_tokens_per_second.unwrap().kind,UsageMeasurementKind::Estimated);
    assert_eq!(detail.error_message.as_deref(),Some("Upstream request failed"));
    assert!(get_usage_log_detail(&db,&scope("other"),"host","a").unwrap().is_none());
    assert!(!serde_json::to_string(&second).unwrap().contains("private-file"));
    q.cursor = None;
    q.filter.status_code = Some(429);
    let filtered = query_usage_logs(&db,&scope("p"),"host",&q,"UTC",UsageUnixSeconds(30)).unwrap();
    assert_eq!(filtered.counts.total_requests, 1);
    assert_eq!(filtered.page.items[0].record.id, "a");
    assert_eq!(filtered.status_codes, vec![429, 500]);
    q.filter.provider = Some(UsageProvider::Known { provider_id:"x".into() });
    assert_eq!(query_usage_logs(&db,&scope("p"),"host",&q,"UTC",UsageUnixSeconds(30)).unwrap_err(),UsageLogsError::InvalidInput);
}
#[test]
fn usage17_logs_rejects_invalid_limits_and_changed_snapshots() {
    let mut db = Connection::open_in_memory().unwrap();
    initialize_usage_store(&mut db).unwrap();
    insert(&mut db,"p","host","a",10,500);
    insert(&mut db,"p","host","b",11,500);
    let mut q = query();
    q.range = UsageRange::All;
    let first = query_usage_logs(&db,&scope("p"),"host",&q,"UTC",UsageUnixSeconds(30)).unwrap();
    q.cursor = first.page.next_cursor;
    assert!(query_usage_logs(&db,&scope("p"),"host",&q,"UTC",UsageUnixSeconds(40)).is_ok());
    insert(&mut db,"p","host","c",12,500);
    assert_eq!(query_usage_logs(&db,&scope("p"),"host",&q,"UTC",UsageUnixSeconds(40)).unwrap_err(),UsageLogsError::RevisionConflict);
    q.cursor = None;
    for limit in [0,MAX_USAGE_LOG_PAGE_SIZE+1] {
        q.limit = Some(limit);
        assert_eq!(query_usage_logs(&db,&scope("p"),"host",&q,"UTC",UsageUnixSeconds(30)).unwrap_err(),UsageLogsError::InvalidInput);
    }
}

#[test]
fn numbered_pages_use_the_filtered_order_and_clamp_to_the_last_page() {
    let mut db = Connection::open_in_memory().unwrap();
    initialize_usage_store(&mut db).unwrap();
    for (id, time, code) in [("a", 11, 500), ("b", 12, 500), ("ignored", 13, 429), ("c", 12, 500)] {
        insert(&mut db, "p", "host", id, time, code);
    }
    let mut q = query();
    q.filter.status_code = Some(500);
    let read = |query: &UsageQuery, page| {
        query_usage_log_page(&db, &scope("p"), "host", query, "UTC", UsageUnixSeconds(30), page)
    };

    let first = read(&q, 1).unwrap();
    assert_eq!(first.page.items[0].record.id, "c");
    let second = read(&q, 2).unwrap();
    assert_eq!(second.counts.total_requests, 3);
    assert_eq!(second.page.items[0].record.id, "b");
    let last = read(&q, u64::MAX).unwrap();
    assert_eq!(last.page.items[0].record.id, "a");
    assert!(last.page.next_cursor.is_none());
    assert_eq!(read(&q, 0).unwrap_err(), UsageLogsError::InvalidInput);

    q.cursor = first.page.next_cursor;
    assert_eq!(read(&q, 2).unwrap_err(), UsageLogsError::InvalidInput);
}
