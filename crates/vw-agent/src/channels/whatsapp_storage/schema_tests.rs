#[cfg(feature = "whatsapp-web")]
use super::RusqliteStore;

#[test]
#[cfg(not(feature = "whatsapp-web"))]
fn schema_is_disabled_without_whatsapp_web_feature() {
    assert!(!cfg!(feature = "whatsapp-web"));
}

#[test]
#[cfg(feature = "whatsapp-web")]
fn init_schema_creates_expected_tables_and_is_idempotent() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let store = RusqliteStore::new(tmp.path()).unwrap();

    store.init_schema().unwrap();

    let conn = store.conn.lock();
    let mut stmt = conn
        .prepare(
            "SELECT name FROM sqlite_master
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
             ORDER BY name",
        )
        .unwrap();
    let tables = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(
        tables,
        vec![
            "app_state_keys",
            "app_state_mutation_macs",
            "app_state_versions",
            "base_keys",
            "device",
            "device_registry",
            "identities",
            "lid_pn_mapping",
            "prekeys",
            "sender_key_status",
            "sender_keys",
            "sessions",
            "signed_prekeys",
            "skdm_recipients",
            "tc_tokens",
        ]
    );
}

#[test]
#[cfg(feature = "whatsapp-web")]
fn schema_declares_device_scoped_primary_keys() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let store = RusqliteStore::new(tmp.path()).unwrap();
    let conn = store.conn.lock();

    let mut stmt = conn.prepare("PRAGMA table_info(app_state_keys)").unwrap();
    let columns = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, i64>(5)?))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(
        columns,
        vec![
            ("key_id".to_string(), "BLOB".to_string(), 1),
            ("key_data".to_string(), "BLOB".to_string(), 0),
            ("device_id".to_string(), "INTEGER".to_string(), 2),
        ]
    );
}
// 本文件验证 WhatsApp 存储结构的建表幂等性与设备级主键约束。
