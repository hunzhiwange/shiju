use super::{
    mcp::normalize,
    source::{SourceCategory, SourceRecord},
};
use serde_json::json;
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectId, ProjectScope, TenantScope},
    mcp_management::McpCatalogScope,
};
fn scope() -> McpCatalogScope {
    McpCatalogScope::Project {
        scope: ProjectScope::new(
            TenantScope::new("fixture-platform", "fixture-company").unwrap(),
            ProjectId::from("fixture-project"),
        )
        .unwrap(),
    }
}
fn row() -> SourceRecord {
    SourceRecord {
        category: SourceCategory::Mcp,
        origin: "mcp_servers".into(),
        fields: json!({"id":"fixture", "name":"Fixture", "server_config":"{\"command\":\"fixture-command\"}", "tags":"[]", "enabled_codex":1, "enabled_claude":0}),
    }
}
#[test]
fn cc_data_10_mcp_preserves_scope_and_enabled_state() {
    let row = row();
    let records = normalize(&[&row], &scope()).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].record.scope, scope());
    assert!(records[0].applications.contains(&(AppId::Codex, true)));
    assert!(records[0].applications.contains(&(AppId::Claude, false)));
    assert_eq!(records[0].record.record.id.0, "fixture");
}
#[test]
fn cc_data_10_mcp_import_never_executes_command() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join("marker");
    let mut row = row();
    row.fields["server_config"] =
        json!({"command":"sh", "args":["-c", format!("touch {}", marker.display())]})
            .to_string()
            .into();
    let _ = normalize(&[&row], &scope());
    assert!(!marker.exists());
    let mut invalid = SourceRecord {
        category: SourceCategory::Mcp,
        origin: "mcp_servers".into(),
        fields: json!({"id":"invalid", "name":"Invalid"}),
    };
    invalid.fields["server_config"] = "malformed".into();
    assert!(normalize(&[&row, &invalid], &scope()).is_err());
}
