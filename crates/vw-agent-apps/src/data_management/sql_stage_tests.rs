use super::{source::{SourceCategory, SourceFormat, SourceScope}, sql_stage::stage_sql};
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};

fn scope() -> SourceScope {
    SourceScope::new(ProjectScope::new(
        TenantScope::new("fixture-platform", "fixture-company").unwrap(),
        ProjectId::from("fixture-project"),
    ).unwrap(), "fixture-host".into(), "fixture-principal".into()).unwrap()
}

// Anonymous export-shaped fixture: header, pragmas, transaction and required
// source tables follow database/backup.rs::dump_sql. No user data is read.
fn export() -> String {
    "-- CC Switch SQLite 导出\nPRAGMA foreign_keys=OFF; PRAGMA user_version=24;
     BEGIN TRANSACTION;
     CREATE TABLE providers(id TEXT, app_type TEXT, name TEXT, settings_config TEXT);
     CREATE TABLE settings(key TEXT, value TEXT);
     CREATE TABLE provider_endpoints(id TEXT);
     CREATE TABLE mcp_servers(id TEXT);
     CREATE TABLE prompts(id TEXT);
     CREATE TABLE skills(id TEXT);
     CREATE TABLE skill_repos(id TEXT);
     INSERT INTO providers VALUES ('fixture', 'claude', 'anonymous', '{}');
     COMMIT;".into()
}

#[test]
fn cc_data_04_sql_accepts_genuine_export() {
    let scope = scope();
    let snapshot = stage_sql(&scope, &export()).unwrap();
    let summary = snapshot.summary(&scope).unwrap();
    assert_eq!(summary.format, SourceFormat::Sql);
    assert_eq!(summary.version, 24);
    assert_eq!(summary.counts[&SourceCategory::Providers], 1);
    assert_eq!(snapshot.records(&scope).unwrap()[0].fields["id"], "fixture");
    let other = SourceScope::new(ProjectScope::new(
        TenantScope::new("fixture-platform", "fixture-company").unwrap(),
        ProjectId::from("other-project"),
    ).unwrap(), "fixture-host".into(), "fixture-principal".into()).unwrap();
    assert!(snapshot.records(&other).is_err());
}

#[test]
fn cc_data_04_sql_rejects_attach_without_disk_effect() {
    let root = tempfile::tempdir().unwrap();
    let escaped = root.path().join("escaped.db");
    for statement in [
        format!("/* prefix */ AtTaCh DATABASE '{}' AS outside;", escaped.display()),
        format!("VACUUM INTO '{}';", escaped.display()),
        "PRAGMA writable_schema=ON;".into(),
        format!("PRAGMA temp_store_directory='{}';", root.path().display()),
        "SELECT load_extension('fixture');".into(),
        "CREATE VIRTUAL TABLE outside USING fts5(body);".into(),
    ] {
        assert!(stage_sql(&scope(), &format!("{}\n{statement}", export())).is_err());
        assert!(!escaped.exists());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }
}

#[test]
fn cc_data_04_sql_rejects_empty_truncated_and_damaged_schema() {
    for sql in [
        "-- CC Switch SQLite 导出\n".into(),
        export().replace("COMMIT;", ""),
        export().replace("CREATE TABLE settings(key TEXT, value TEXT);", ""),
        export().replace("settings_config TEXT", "wrong TEXT"),
        export().replace("user_version=24", "user_version=25"),
        export().replace("-- CC Switch SQLite 导出", "-- unrelated"),
    ] {
        assert!(stage_sql(&scope(), &sql).is_err());
    }
}
