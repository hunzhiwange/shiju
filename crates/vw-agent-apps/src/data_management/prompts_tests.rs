use super::{prompts::normalize, source::{SourceCategory, SourceRecord}};
use serde_json::json;
use vw_api_types::agent_apps::AppId;

#[test]
fn cc_data_12_prompts_preserve_app_and_active_state() {
    let row = SourceRecord { category: SourceCategory::Rules, origin: "prompts".into(),
        fields: json!({"id":"fixture","app_type":"claude","name":"规则","content":"原文\r\n", "enabled":1,"created_at":12}) };
    let rows = normalize(&[&row]).unwrap();
    assert_eq!(rows[0].app, AppId::Claude);
    assert!(rows[0].enabled);
    assert_eq!(rows[0].entry.content, "原文\r\n");
    assert_eq!(rows[0].entry.created_at, Some(12));
}

#[test]
fn cc_data_12_prompts_import_does_not_write_live() {
    let dir = tempfile::tempdir().unwrap();
    let live = dir.path().join("CLAUDE.md");
    std::fs::write(&live, b"untouched").unwrap();
    let valid = SourceRecord { category: SourceCategory::Rules, origin: "prompts".into(),
        fields: json!({"id":"fixture","app_type":"claude","name":"Rule","content":"body","enabled":true}) };
    let invalid = SourceRecord { category: SourceCategory::Rules, origin: "prompts".into(),
        fields: json!({"id":"other","app_type":"claude","name":"Rule","content":"other","enabled":true}) };
    assert!(normalize(&[&valid, &invalid]).is_err());
    assert_eq!(std::fs::read(live).unwrap(), b"untouched");
}
