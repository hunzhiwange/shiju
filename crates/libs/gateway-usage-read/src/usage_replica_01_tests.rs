//! Deferred tests: no execution is authorized during task 1.
use super::*;
use app_usage_store::{groups, retention, store::*, summary};
use rusqlite::Connection;
use vw_api_transport::id::{ProjectId, ProjectScope};

fn tokens(total: bool) -> UsageTokens {
    UsageTokens {
        input: Some(if total { 1000 } else { 700 }), output: Some(200),
        reasoning: Some(30), cache_read: Some(200), cache_creation: Some(100),
        cache_creation_1h: Some(50),
        input_semantics: if total { UsageInputSemantics::Total } else { UsageInputSemantics::Fresh },
        reasoning_semantics: UsageReasoningSemantics::IncludedInOutput,
    }
}

#[test]
fn usage_replica_01_cache_semantics_and_missing_values() {
    let a = tokens(false);
    let b = tokens(true);
    let totals = UsageTokenTotals::from_groups([&a, &b]);
    assert_eq!(totals.fresh_input, Some(1400));
    assert_eq!(totals.real_tokens, Some(2400));
    assert_eq!(totals.cache_hit_rate(), Some(0.2));
    let mut unknown = a.clone();
    unknown.input_semantics = UsageInputSemantics::Unknown;
    assert_eq!(UsageTokenTotals::from_groups([&unknown]).real_tokens, None);
    let mut zero = a;
    zero.input = Some(0);
    zero.cache_read = Some(0);
    zero.cache_creation = Some(0);
    assert_eq!(UsageTokenTotals::from_groups([&zero]).cache_hit_rate(), Some(0.0));
}

fn scope() -> ProjectScope {
    ProjectScope::for_default_tenant(ProjectId("replica-fixture".into())).unwrap()
}

fn insert(db: &mut Connection, id: &str, time: i64, first: f64) {
    let record: UsageRecord = serde_json::from_value(serde_json::json!({
        "id":id,"started_at_unix_seconds":time,"provider":{"kind":"unknown"},
        "sources":[{"source_id":id,"kind":"relay","app":"codex",
            "native_request_id":id,"native_session_id":"fixture"}],
        "models":{"request_model":"fixture","response_model":null,"pricing_model":null},
        "status":"succeeded","billable":true,"tokens":tokens(true),
        "cost":{"source":"unknown","amount_usd":null},
        "performance":{"first_token_ms":{"value":first,"kind":"measured"},
            "latency_ms":{"value":first+1000.0,"kind":"measured"},
            "output_tokens_per_second":null},"suspected_duplicate_of":null
    })).unwrap();
    save_usage_record(db, &scope(), &UsageSourceIdentity {
        target_host:"host".into(), app:"codex".into(), instance_id:"fixture".into(),
        kind:UsageSourceKind::Relay, file_identity:"fixture".into(),
        native_session_id:Some("fixture".into()), native_request_id:id.into(),
    }, &StoredUsageRecord { record, status_code:Some(200), failure:None }).unwrap();
}

#[test]
fn usage_replica_01_repository_summary_median_and_30_day_boundary() {
    let mut db = Connection::open_in_memory().unwrap();
    initialize_usage_store(&mut db).unwrap();
    let cutoff = 10 * 86400;
    for (id, time, first) in [("old",cutoff-1,100.0),("boundary",cutoff,200.0),
        ("tail",cutoff+1,9000.0)] {
        insert(&mut db,id,time,first);
    }
    let range = UsageResolvedRange {
        start_unix_seconds:None, end_unix_seconds:UsageUnixSeconds(50*86400),
        time_zone:"UTC".into(),
    };
    let filter = UsageFilter::default();
    let before = summary::get_usage_summary(&db,&scope(),"host",&range,&filter).unwrap();
    let wire: UsageSummary = serde_json::from_value(serde_json::to_value(&before).unwrap()).unwrap();
    assert_eq!(wire.counts.total_requests,3);
    assert_eq!(wire.replica_success_rate(),1.0);
    let mut mixed_status = wire.clone();
    mixed_status.counts.total_requests = 4;
    mixed_status.unknown_status_requests = 1;
    assert_eq!(mixed_status.replica_success_rate(),0.75);
    mixed_status.counts.total_requests = 0;
    assert_eq!(mixed_status.replica_success_rate(),0.0);
    assert_eq!(wire.known_cost_usd,"0");
    assert_eq!(wire.counts.unknown_cost_requests,3);
    assert_eq!(wire.token_totals().real_tokens,Some(3600));
    let stats = groups::get_model_stats(&db,&scope(),"host",&range,&filter).unwrap();
    let wire: Vec<ModelStats> = serde_json::from_value(serde_json::to_value(stats).unwrap()).unwrap();
    assert_eq!(wire[0].stats.measured.first_token_median_ms,Some(200.0));
    assert_eq!(wire[0].stats.measured.first_token_samples,3);
    assert_eq!(wire[0].stats.measured.speed_output_tokens,600);
    assert_eq!(wire[0].stats.measured.speed_duration_ms,3000.0);
    assert_eq!(retention::rollup_and_prune(&mut db,&scope(),"host",40*86400,||false).unwrap(),1);
    let after = summary::get_usage_summary(&db,&scope(),"host",&range,&filter).unwrap();
    assert_eq!(before,after);
    let stats = groups::get_model_stats(&db,&scope(),"host",&range,&filter).unwrap();
    assert_eq!(stats[0].stats.measured.first_token_samples,2);
}
