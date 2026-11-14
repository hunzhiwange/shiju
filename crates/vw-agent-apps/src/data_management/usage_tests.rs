use std::collections::BTreeMap;
use rusqlite::Connection;
use serde_json::json;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use super::{source::*, usage::*, preview::{Decision, PreviewError}};

fn fixture() -> (SourceScope, SourceSnapshot, Connection) {
    let scope = SourceScope::new(ProjectScope::new(TenantScope::new("fixture-platform","fixture-company").unwrap(),
        ProjectId::from("fixture-project")).unwrap(),"fixture-host".into(),"fixture-user".into()).unwrap();
    let source = SourceSnapshot { scope: scope.clone(), summary: SourceSummary { format: SourceFormat::Sqlite,
        version:24, main_sha256:[15;32],wal_sha256:None,counts:BTreeMap::new() },
        records:vec![SourceRecord { category:SourceCategory::Usage,origin:"proxy_request_logs".into(),fields:json!({
            "request_id":"request-anonymous","provider_id":"source-account","app_type":"claude", "model":"fixture-model",
            "input_tokens":10,"output_tokens":5,"cache_read_tokens":0,"cache_creation_tokens":0,"input_token_semantics":2,
            "total_cost_usd":"0.125","cost_multiplier":"1.0","created_at":1,"status_code":200,"latency_ms":20
        }) }] };
    let mut db = Connection::open_in_memory().unwrap();
    crate::usage::store::initialize_usage_store(&mut db).unwrap();
    (scope,source,db)
}
#[test]
fn cc_data_15_usage_reimport_does_not_duplicate_cost() {
    let (scope,mut source,mut db) = fixture();
    let p = prepare(&db,&scope,&source,"fixture-registry",|_,_|Some("owned-account".into())).unwrap();
    assert_eq!(p.execute(&mut db,&scope,&source,&[],|_,p|p=="owned-account").unwrap(),1);
    let p = prepare(&db,&scope,&source,"fixture-registry",|_,_|Some("owned-account".into())).unwrap();
    assert_eq!(p.execute(&mut db,&scope,&source,&[],|_,_|true).unwrap(),0);
    source.records[0].fields["total_cost_usd"] = json!("0.25");
    let p = prepare(&db,&scope,&source,"fixture-registry",|_,_|Some("owned-account".into())).unwrap();
    let id = p.entries()[0].id;
    assert_eq!(p.execute(&mut db,&scope,&source,&[(id,Decision::Skip)],|_,_|true).unwrap(),0);
    assert_eq!(p.execute(&mut db,&scope,&source,&[(id,Decision::Replace)],|_,_|true).unwrap(),1);
    let (count,payload):(i64,String) = db.query_row("SELECT count(*),payload_json FROM usage_records",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
    assert_eq!(count,1);
    assert_eq!(serde_json::from_str::<serde_json::Value>(&payload).unwrap()["record"]["cost"]["amount_usd"],"0.25");
}
#[test]
fn cc_data_15_usage_rejects_foreign_account_binding() {
    let (scope,source,mut db) = fixture();
    assert!(matches!(prepare(&db,&scope,&source,"fixture-registry",|_,_|None),Err(PreviewError::Forbidden)));
    let p = prepare(&db,&scope,&source,"fixture-registry",|_,_|Some("owned-account".into())).unwrap();
    assert_eq!(p.execute(&mut db,&scope,&source,&[],|_,_|false),Err(PreviewError::Forbidden));
    assert_eq!(db.query_row("SELECT count(*) FROM usage_records",[],|r|r.get::<_,i64>(0)).unwrap(),0);
}
