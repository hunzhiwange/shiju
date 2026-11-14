//! Fixtures authored for deferred validation; these tests have not been run.
use rusqlite::Connection;
use vw_agent_apps::usage::{
    cursors::{UsageCursorKey, initialize_usage_cursors},
    mcode::scan_mcode_database,
    source_settings::initialize_source_settings,
    store::{StoredUsageRecord, initialize_usage_store},
};
use vw_api_types::{id::{ProjectId, ProjectScope}, usage::{UsageCostSource, UsageReasoningSemantics}};

#[test]
fn usage17_mcode_committed_projection_uses_native_cost_once() {
    // Canonicalize to avoid macOS /var -> /private/var symlink rejection.
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    std::fs::create_dir_all(root.join("v2/sqlite")).unwrap();
    let native = Connection::open(root.join("v2/sqlite/runtime-state.sqlite")).unwrap();
    native.execute_batch("PRAGMA journal_mode=WAL;
        CREATE TABLE local_runtime_token_usage (
            id INTEGER PRIMARY KEY,session_id TEXT,model TEXT,ts INTEGER,input_tokens INTEGER,
            output_tokens INTEGER,reasoning_tokens INTEGER,cache_read_tokens INTEGER,
            cache_write_tokens INTEGER,cost_usd REAL);
        CREATE TABLE local_runtime_pi_history_rows (
            id INTEGER PRIMARY KEY,session_id TEXT,role TEXT,created_at_ms INTEGER,data_json TEXT);
        INSERT INTO local_runtime_pi_history_rows VALUES
            (1,'session','assistant',2000,'{\"model\":\"vendor/history\"}');
        INSERT INTO local_runtime_token_usage VALUES
            (1,'session','custom_provider:router/vendor/model',1000,10,20,3,4,5,0),
            (2,'session',NULL,2000,1,2,0,0,0,NULL);").unwrap();
    let mut ledger = Connection::open_in_memory().unwrap();
    initialize_usage_store(&mut ledger).unwrap();
    initialize_usage_cursors(&ledger).unwrap();
    initialize_source_settings(&ledger).unwrap();
    let scope = ProjectScope::for_default_tenant(ProjectId("project".into())).unwrap();
    let key = UsageCursorKey { target_host: "host".into(), app: "mcode".into(),
        instance_id: "instance".into(), file_identity: "database".into() };
    let result = scan_mcode_database(&mut ledger, &scope, &key, &root, |model| {
        assert_eq!(model, "vendor/history"); // Native zero never consults pricing.
        Ok(None)
    }).unwrap();
    assert_eq!(result.imported, 2);
    let records: Vec<StoredUsageRecord> = ledger.prepare("SELECT payload_json FROM usage_records ORDER BY json_extract(payload_json,'$.record.started_at_unix_seconds')").unwrap()
        .query_map([], |r| r.get::<_, String>(0)).unwrap()
        .map(|r| serde_json::from_str(&r.unwrap()).unwrap()).collect();
    let first = &records[0].record;
    assert_eq!(first.models.response_model.as_deref(), Some("vendor/model"));
    assert_eq!(first.cost.source, UsageCostSource::Native);
    assert_eq!(first.cost.amount_usd.as_deref(), Some("0"));
    let tokens = first.tokens.as_ref().unwrap();
    assert_eq!(tokens.output, Some(23));
    assert_eq!(tokens.reasoning_semantics, UsageReasoningSemantics::IncludedInOutput);
    assert_eq!(records[1].record.cost.source, UsageCostSource::Unknown);
    assert_eq!(records[1].record.models.response_model.as_deref(), Some("vendor/history"));
    native.execute_batch("BEGIN IMMEDIATE; INSERT INTO local_runtime_token_usage VALUES
        (3,'session','provider/model',3000,1,2,0,0,0,0.25);").unwrap();
    assert_eq!(scan_mcode_database(&mut ledger, &scope, &key, &root, |_| panic!("already scanned")).unwrap().imported, 0);
    native.execute_batch("COMMIT").unwrap();
    assert_eq!(scan_mcode_database(&mut ledger, &scope, &key, &root, |_| panic!("native cost")).unwrap().imported, 1);
    ledger.execute("DELETE FROM usage_records", []).unwrap();
    assert_eq!(scan_mcode_database(&mut ledger, &scope, &key, &root, |_| panic!("must not replay archived rows")).unwrap().imported, 0);
}
