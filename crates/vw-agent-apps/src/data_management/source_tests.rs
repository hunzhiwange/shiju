use std::{fs, path::Path};
use rusqlite::Connection;
use serde_json::json;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use super::source::{read_selected_source, SourceCategory, SourceError, SourceFormat, SourceScope};

fn scope_project() -> ProjectScope {
    ProjectScope::new(TenantScope::new("fixture-platform", "fixture-company").unwrap(), ProjectId::from("fixture-project")).unwrap()
}
fn scope() -> SourceScope {
    SourceScope::new(
        scope_project(),
        "fixture-host".into(), "fixture-principal".into(),
    ).unwrap()
}
fn database(path: &Path) -> Connection {
    let connection = Connection::open(path).unwrap();
    connection.execute_batch(
        "CREATE TABLE providers(id TEXT, app_type TEXT, name TEXT, settings_config TEXT);
         CREATE TABLE settings(key TEXT, value TEXT);
         PRAGMA user_version=24;"
    ).unwrap();
    connection
}
fn unchanged(path: &Path) -> (Vec<u8>, std::time::SystemTime) {
    (fs::read(path).unwrap(), fs::metadata(path).unwrap().modified().unwrap())
}

#[test]
fn cc_data_03_source_reads_wal_snapshot_without_mutation() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("selected.db");
    let connection = database(&path);
    connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;").unwrap();
    connection.execute("INSERT INTO providers VALUES (?1, 'claude', 'fixture', ?2)",
        ["wal-only", r#"{"env":{"TOKEN":"fixture-secret"}}"#]).unwrap();
    // Keep writer alive: the committed row exists in WAL, not the main DB.
    let wal = super::source_files::wal_path(&path);
    let shm = root.path().join("selected.db-shm");
    let before = [unchanged(&path), unchanged(&wal), unchanged(&shm)];
    let scope = scope();
    let snapshot = read_selected_source(&scope, root.path(), &path).unwrap();
    let summary = snapshot.summary(&scope).unwrap();
    assert_eq!(summary.format, SourceFormat::Sqlite);
    assert_eq!(summary.version, 24);
    assert!(summary.wal_sha256.is_some());
    assert_eq!(summary.counts.get(&SourceCategory::Providers), Some(&1));
    let rows = snapshot.records(&scope).unwrap();
    assert_eq!(rows[0].origin, "providers");
    assert_eq!(rows[0].fields["id"], "wal-only");
    assert_eq!(rows[0].fields["settings_config"], r#"{"env":{"TOKEN":"fixture-secret"}}"#);
    assert!(!format!("{summary:?}").contains("fixture-secret"));
    assert_eq!(before, [unchanged(&path), unchanged(&wal), unchanged(&shm)]);
    // Staging is owned and not a lazy connection to the source.
    connection.execute("DELETE FROM providers", []).unwrap();
    assert_eq!(snapshot.records(&scope).unwrap().len(), 1);
}

#[test]
fn cc_data_03_source_rejects_future_schema() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("future.db");
    let connection = database(&path);
    connection.execute_batch("PRAGMA user_version=25;").unwrap();
    drop(connection);
    let before = unchanged(&path);
    assert!(matches!(read_selected_source(&scope(), root.path(), &path),
        Err(SourceError::FutureVersion { found: 25, supported: 24 })));
    assert_eq!(before, unchanged(&path));
    assert!(!super::source_files::wal_path(&path).exists());
    assert!(!root.path().join("future.db-shm").exists());
}

#[test]
fn cc_data_03_source_stages_json_sections_and_checks_scope() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("selected.json");
    let config = json!({
        "version": 2,
        "claude": {"providers": {"fixture": {"id": "fixture", "settingsConfig": {"key": "fixture-secret"}}}, "current": "fixture"},
        "mcp": {"servers": {}}, "prompts": {"claude": {"prompts": {}}},
        "skills": {"repos": []}, "common_config_snippets": {"claude": "fixture"}
    });
    fs::write(&path, serde_json::to_vec(&config).unwrap()).unwrap();
    let before = unchanged(&path);
    let scope = scope();
    let snapshot = read_selected_source(&scope, root.path(), &path).unwrap();
    assert_eq!(snapshot.summary(&scope).unwrap().format, SourceFormat::Json);
    assert_eq!(snapshot.summary(&scope).unwrap().version, 2);
    let provider = snapshot.records(&scope).unwrap().iter().find(|row| row.origin == "/claude").unwrap();
    assert_eq!(provider.fields, config["claude"]);
    assert_eq!(before, unchanged(&path));
    for foreign in [
        SourceScope::new(scope_project(), "other-host".into(), "fixture-principal".into()).unwrap(),
        SourceScope::new(scope_project(), "fixture-host".into(), "other-user".into()).unwrap(),
        SourceScope::new(ProjectScope::new(TenantScope::new("other", "tenant").unwrap(), ProjectId::from("fixture-project")).unwrap(), "fixture-host".into(), "fixture-principal".into()).unwrap(),
        SourceScope::new(ProjectScope::new(scope_project().tenant_scope().clone(), ProjectId::from("other-project")).unwrap(), "fixture-host".into(), "fixture-principal".into()).unwrap(),
    ] {
        assert_eq!(snapshot.summary(&foreign), Err(SourceError::PermissionDenied));
        assert!(matches!(snapshot.records(&foreign), Err(SourceError::PermissionDenied)));
    }
}

#[test]
fn cc_data_03_source_rejects_missing_unknown_and_future_json() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("selected");
    assert!(matches!(read_selected_source(&scope(), root.path(), &path), Err(SourceError::NotFound)));
    for (bytes, expected) in [
        ("plain text", SourceError::UnknownFormat),
        ("{}", SourceError::UnknownFormat),
        (r#"{"version":2}"#, SourceError::UnknownFormat),
        (r#"{"version":3,"mcp":{}}"#, SourceError::FutureVersion { found: 3, supported: 2 }),
        (r#"{"version":1,"mcp":{}}"#, SourceError::UnsupportedVersion),
        (r#"{"version":2,"mcp":[]}"#, SourceError::InvalidData),
    ] {
        fs::write(&path, bytes).unwrap();
        assert!(matches!(read_selected_source(&scope(), root.path(), &path), Err(error) if error == expected));
    }
    fs::remove_file(&path).unwrap();
    let connection = Connection::open(&path).unwrap();
    connection.execute_batch("PRAGMA user_version=24; CREATE TABLE unrelated(id TEXT);").unwrap();
    drop(connection);
    assert!(matches!(read_selected_source(&scope(), root.path(), &path), Err(SourceError::UnknownFormat)));
}

#[cfg(unix)]
#[test]
fn cc_data_03_source_rejects_links_and_outside_roots() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let target = outside.path().join("config.json");
    fs::write(&target, r#"{"version":2,"mcp":{}}"#).unwrap();
    assert!(matches!(read_selected_source(&scope(), root.path(), &target), Err(SourceError::OutsideAuthorizedRoot)));
    let link = root.path().join("selected.json");
    symlink(&target, &link).unwrap();
    assert!(matches!(read_selected_source(&scope(), root.path(), &link), Err(SourceError::UnsafeFile)));
    let path = root.path().join("selected.db");
    drop(database(&path));
    symlink(&target, super::source_files::wal_path(&path)).unwrap();
    assert!(matches!(read_selected_source(&scope(), root.path(), &path), Err(SourceError::UnsafeFile)));
}

#[test]
fn cc_data_03_source_reads_wal_copy_without_source_shm() {
    let writer_root = tempfile::tempdir().unwrap();
    let writer_path = writer_root.path().join("writer.db");
    let writer = database(&writer_path);
    writer.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; INSERT INTO settings VALUES ('fixture-key', 'committed');").unwrap();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("selected.db");
    let wal = super::source_files::wal_path(&path);
    fs::copy(&writer_path, &path).unwrap();
    fs::copy(super::source_files::wal_path(&writer_path), &wal).unwrap();
    let before = [unchanged(&path), unchanged(&wal)];
    let scope = scope();
    let snapshot = read_selected_source(&scope, root.path(), &path).unwrap();
    assert_eq!(snapshot.records(&scope).unwrap().iter().find(|r| r.origin == "settings").unwrap().fields["value"], "committed");
    assert_eq!(before, [unchanged(&path), unchanged(&wal)]);
    assert!(!root.path().join("selected.db-shm").exists());
}

#[test]
fn cc_data_03_source_capture_rejects_changes_and_oversize_files() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("selected.json");
    fs::write(&path, r#"{"version":2,"mcp":{}}"#).unwrap();
    let captured = super::source_files::capture(&path).unwrap();
    fs::write(&path, r#"{"version":2,"prompts":{}}"#).unwrap();
    assert_eq!(captured.verify(&path), Err(SourceError::Changed));
    let file = fs::File::create(&path).unwrap();
    file.set_len(256 * 1024 * 1024 + 1).unwrap();
    assert!(matches!(read_selected_source(&scope(), root.path(), &path), Err(SourceError::TooLarge)));
}
