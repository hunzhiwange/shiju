use super::{apply::*, source::*, usage};
use rusqlite::Connection;
use serde_json::json;
use std::collections::BTreeMap;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};

fn fixture() -> (SourceScope, SourceSnapshot, Connection) {
    let scope = SourceScope::new(
        ProjectScope::new(
            TenantScope::new("fixture-platform", "fixture-company").unwrap(),
            ProjectId::from("fixture-project"),
        )
        .unwrap(),
        "fixture-host".into(),
        "fixture-user".into(),
    )
    .unwrap();
    let source = SourceSnapshot {
        scope: scope.clone(),
        summary: SourceSummary {
            format: SourceFormat::Sqlite,
            version: 24,
            main_sha256: [16; 32],
            wal_sha256: None,
            counts: BTreeMap::new(),
        },
        records: vec![SourceRecord {
            category: SourceCategory::Usage,
            origin: "proxy_request_logs".into(),
            fields: json!({
                "request_id":"fixture-request", "provider_id":"source-account",
                "app_type":"claude", "model":"fixture-model", "input_tokens":10,
                "output_tokens":5, "cache_read_tokens":0, "cache_creation_tokens":0,
                "input_token_semantics":2, "total_cost_usd":"0.125",
                "cost_multiplier":"1.0", "created_at":1, "status_code":200
            }),
        }],
    };
    let mut db = Connection::open_in_memory().unwrap();
    crate::usage::store::initialize_usage_store(&mut db).unwrap();
    (scope, source, db)
}

#[test]
fn cc_data_16_apply_commits_only_selected_categories() {
    let (scope, source, mut db) = fixture();
    let preview = usage::prepare(&db, &scope, &source, "fixture-registry", |_, _| {
        Some("owned-account".into())
    })
    .unwrap();
    let summary = source.summary(&scope).unwrap().clone();
    let selected = [SourceCategory::Usage];
    let mut written = 0;
    let report = apply_selected(&scope, &source, &summary, &selected, &selected, |category| {
        assert_eq!(category, SourceCategory::Usage);
        written += preview.execute(&mut db, &scope, &source, &[], |_, _| true)?;
        Ok::<_, super::preview::PreviewError>(())
    })
    .unwrap();
    assert_eq!(written, 1);
    assert_eq!(report.completed, selected);
    assert!(report.failed.is_none());
    // A fresh preview observes the actual imported ledger, so reimport is a no-op.
    let repeated = usage::prepare(&db, &scope, &source, "fixture-registry", |_, _| {
        Some("owned-account".into())
    })
    .unwrap();
    assert_eq!(repeated.execute(&mut db, &scope, &source, &[], |_, _| true).unwrap(), 0);
    apply_selected::<()>(&scope, &source, &summary, &[], &[], |_| panic!("empty selection wrote"))
        .unwrap();
    assert!(matches!(
        apply_selected::<()>(
            &scope,
            &source,
            &summary,
            &[SourceCategory::Providers],
            &selected,
            |_| panic!("substituted selection wrote")
        ),
        Err(ApplyError::InvalidSelection)
    ));
}

#[test]
fn cc_data_16_apply_rejects_stale_preview() {
    let (scope, mut source, mut db) = fixture();
    let first = usage::prepare(&db, &scope, &source, "fixture-registry", |_, _| {
        Some("owned-account".into())
    })
    .unwrap();
    let stale = usage::prepare(&db, &scope, &source, "fixture-registry", |_, _| {
        Some("owned-account".into())
    })
    .unwrap();
    first.execute(&mut db, &scope, &source, &[], |_, _| true).unwrap();
    assert!(matches!(
        stale.execute(&mut db, &scope, &source, &[], |_, _| true),
        Err(super::preview::PreviewError::Stale)
    ));
    let summary = source.summary(&scope).unwrap().clone();
    source.summary.main_sha256 = [17; 32];
    let selected = [SourceCategory::Usage];
    assert!(matches!(
        apply_selected::<()>(&scope, &source, &summary, &selected, &selected, |_| panic!(
            "stale source wrote"
        )),
        Err(ApplyError::StaleSource)
    ));
}
