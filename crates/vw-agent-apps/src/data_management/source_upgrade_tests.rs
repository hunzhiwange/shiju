use rusqlite::{Connection, hooks::{AuthAction, AuthContext, Authorization}};
use super::{source::SourceError, source_upgrade::upgrade_staged_copy};

const FIXTURE: &str = include_str!("fixtures/schema_18.sql");

#[test]
fn cc_data_05_upgrade_migrates_supported_copy() {
    let connection = Connection::open_in_memory().unwrap();
    connection.execute_batch(FIXTURE).unwrap();
    connection.execute_batch("INSERT INTO session_log_sync VALUES ('fixture',1,2,3,4,5);").unwrap();
    upgrade_staged_copy(&connection).unwrap();
    upgrade_staged_copy(&connection).unwrap();
    assert_eq!(connection.pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0)).unwrap(), 24);
    let cursor = connection.query_row(
        "SELECT last_line_offset,last_byte_offset,parser_state FROM session_log_sync",
        [], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, Option<String>>(2)?)),
    ).unwrap();
    assert_eq!(cursor, (2, 4, None));
    connection.prepare("SELECT enabled_mcode,enabled_pi,enabled_agy,enabled_kimi,enabled_omp,enabled_dsh FROM mcp_servers").unwrap();
    connection.prepare("SELECT enabled_mcode,enabled_agy,enabled_kimi,enabled_dsh FROM skills").unwrap();
}

#[test]
fn cc_data_05_upgrade_failure_preserves_source() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("source.db");
    let source = Connection::open(&path).unwrap();
    source.execute_batch(FIXTURE).unwrap();
    drop(source);
    let before = std::fs::read(&path).unwrap();
    let copy = Connection::open_in_memory().unwrap();
    copy.execute_batch(FIXTURE).unwrap();
    // Inject failure after ALTER, when advancing the version, to check rollback.
    copy.authorizer(Some(|context: AuthContext<'_>| match context.action {
        AuthAction::Pragma { pragma_name, pragma_value: Some(_) }
            if pragma_name == "user_version" => Authorization::Deny,
        _ => Authorization::Allow,
    }));
    assert_eq!(upgrade_staged_copy(&copy), Err(SourceError::InvalidData));
    copy.authorizer(None::<fn(AuthContext<'_>) -> Authorization>);
    assert_eq!(copy.pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0)).unwrap(), 18);
    assert!(copy.prepare("SELECT enabled_mcode FROM mcp_servers").is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    for version in [17, 25] {
        copy.pragma_update(None, "user_version", version).unwrap();
        assert!(upgrade_staged_copy(&copy).is_err());
    }
    copy.pragma_update(None, "user_version", 18).unwrap();
    copy.execute_batch("DROP TABLE skills").unwrap();
    assert_eq!(upgrade_staged_copy(&copy), Err(SourceError::InvalidData));
    assert!(copy.prepare("SELECT enabled_mcode FROM mcp_servers").is_err());
}
