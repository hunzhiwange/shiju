#[cfg(feature = "whatsapp-web")]
use super::RusqliteStore;

#[cfg(feature = "whatsapp-web")]
fn test_store() -> (tempfile::TempDir, RusqliteStore) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("session.db");
    let store = RusqliteStore::new(&path).unwrap();
    (dir, store)
}

#[test]
#[cfg(not(feature = "whatsapp-web"))]
fn device_store_is_disabled_without_whatsapp_web_feature() {
    assert!(!cfg!(feature = "whatsapp-web"));
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn create_exists_save_and_load_device() {
    use wa_rs_binary::jid::Jid;
    use wa_rs_core::store::Device as CoreDevice;
    use wa_rs_core::store::traits::DeviceStore as DeviceStoreTrait;

    let (_dir, store) = test_store();
    assert_eq!(DeviceStoreTrait::create(&store).await.unwrap(), 1);
    assert!(!DeviceStoreTrait::exists(&store).await.unwrap());
    assert!(DeviceStoreTrait::load(&store).await.unwrap().is_none());

    let mut device = CoreDevice::new();
    device.pn = Some(Jid::pn("15550000001"));
    device.lid = Some(Jid::lid("100000000000001"));
    device.registration_id = 12345;
    device.push_name = "VibeWindow".to_string();
    device.edge_routing_info = Some(vec![1, 2, 3]);
    device.props_hash = Some("props-hash".to_string());

    DeviceStoreTrait::save(&store, &device).await.unwrap();
    assert!(DeviceStoreTrait::exists(&store).await.unwrap());

    let loaded = DeviceStoreTrait::load(&store).await.unwrap().unwrap();
    assert_eq!(loaded.pn.map(|jid| jid.to_string()), Some(Jid::pn("15550000001").to_string()));
    assert_eq!(
        loaded.lid.map(|jid| jid.to_string()),
        Some(Jid::lid("100000000000001").to_string())
    );
    assert_eq!(loaded.registration_id, 12345);
    assert_eq!(loaded.push_name, "VibeWindow");
    assert_eq!(loaded.edge_routing_info, Some(vec![1, 2, 3]));
    assert_eq!(loaded.props_hash.as_deref(), Some("props-hash"));
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn save_replaces_existing_device_row() {
    use wa_rs_core::store::Device as CoreDevice;
    use wa_rs_core::store::traits::DeviceStore as DeviceStoreTrait;

    let (_dir, store) = test_store();
    let mut first = CoreDevice::new();
    first.push_name = "First".to_string();
    DeviceStoreTrait::save(&store, &first).await.unwrap();

    let mut second = first.clone();
    second.push_name = "Second".to_string();
    second.registration_id = 999;
    DeviceStoreTrait::save(&store, &second).await.unwrap();

    let loaded = DeviceStoreTrait::load(&store).await.unwrap().unwrap();
    assert_eq!(loaded.push_name, "Second");
    assert_eq!(loaded.registration_id, 999);
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn device_records_are_scoped_by_device_id() {
    use wa_rs_core::store::Device as CoreDevice;
    use wa_rs_core::store::traits::DeviceStore as DeviceStoreTrait;

    let (_dir, store) = test_store();
    let mut other_device = store.clone();
    other_device.device_id = 2;

    let mut first = CoreDevice::new();
    first.push_name = "First Device".to_string();
    DeviceStoreTrait::save(&store, &first).await.unwrap();

    let mut second = CoreDevice::new();
    second.push_name = "Second Device".to_string();
    DeviceStoreTrait::save(&other_device, &second).await.unwrap();

    assert!(DeviceStoreTrait::exists(&store).await.unwrap());
    assert!(DeviceStoreTrait::exists(&other_device).await.unwrap());
    assert_eq!(DeviceStoreTrait::load(&store).await.unwrap().unwrap().push_name, "First Device");
    assert_eq!(
        DeviceStoreTrait::load(&other_device).await.unwrap().unwrap().push_name,
        "Second Device"
    );
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn save_and_load_preserves_account_data() {
    use wa_rs_core::store::Device as CoreDevice;
    use wa_rs_core::store::traits::DeviceStore as DeviceStoreTrait;
    use wa_rs_proto::whatsapp::AdvSignedDeviceIdentity;

    let (_dir, store) = test_store();
    let mut device = CoreDevice::new();
    device.account = Some(AdvSignedDeviceIdentity {
        details: Some(vec![1, 2, 3]),
        account_signature_key: Some(vec![4, 5, 6]),
        account_signature: Some(vec![7, 8, 9]),
        device_signature: Some(vec![10, 11, 12]),
    });

    DeviceStoreTrait::save(&store, &device).await.unwrap();

    let loaded = DeviceStoreTrait::load(&store).await.unwrap().unwrap();
    let account = loaded.account.unwrap();
    assert_eq!(account.details, Some(vec![1, 2, 3]));
    assert_eq!(account.account_signature_key, Some(vec![4, 5, 6]));
    assert_eq!(account.account_signature, Some(vec![7, 8, 9]));
    assert_eq!(account.device_signature, Some(vec![10, 11, 12]));
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn load_returns_error_for_malformed_key_pair_blob() {
    use rusqlite::params;
    use wa_rs_core::store::traits::DeviceStore as DeviceStoreTrait;

    let (_dir, store) = test_store();
    {
        let conn = store.conn.lock();
        conn.execute(
            "INSERT INTO device (
                id, registration_id, noise_key, identity_key, signed_pre_key,
                signed_pre_key_id, signed_pre_key_signature, adv_secret_key,
                push_name, app_version_primary, app_version_secondary,
                app_version_tertiary, app_version_last_fetched_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                store.device_id,
                1u32,
                vec![0u8; 63],
                vec![0u8; 64],
                vec![0u8; 64],
                1u32,
                vec![0u8; 64],
                vec![0u8; 32],
                "",
                2u32,
                3000u32,
                1031424117u32,
                0i64,
            ],
        )
        .unwrap();
    }

    let err = match DeviceStoreTrait::load(&store).await {
        Ok(_) => panic!("expected malformed key pair to fail"),
        Err(err) => err,
    };
    assert!(err.to_string().contains("key_pair"));
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn load_ignores_invalid_jid_strings() {
    use rusqlite::params;
    use wa_rs_core::store::Device as CoreDevice;
    use wa_rs_core::store::traits::DeviceStore as DeviceStoreTrait;

    let (_dir, store) = test_store();
    DeviceStoreTrait::save(&store, &CoreDevice::new()).await.unwrap();

    {
        let conn = store.conn.lock();
        conn.execute(
            "UPDATE device SET lid = ?1, pn = ?2 WHERE id = ?3",
            params!["not-a-lid", "not-a-phone-number", store.device_id],
        )
        .unwrap();
    }

    let loaded = DeviceStoreTrait::load(&store).await.unwrap().unwrap();
    assert!(loaded.lid.is_none());
    assert!(loaded.pn.is_none());
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn load_returns_error_for_invalid_account_blob() {
    use rusqlite::params;
    use wa_rs_core::store::Device as CoreDevice;
    use wa_rs_core::store::traits::DeviceStore as DeviceStoreTrait;

    let (_dir, store) = test_store();
    DeviceStoreTrait::save(&store, &CoreDevice::new()).await.unwrap();

    {
        let conn = store.conn.lock();
        conn.execute(
            "UPDATE device SET account = ?1 WHERE id = ?2",
            params![vec![0xFF_u8], store.device_id],
        )
        .unwrap();
    }

    let err = match DeviceStoreTrait::load(&store).await {
        Ok(_) => panic!("expected invalid protobuf payload to fail"),
        Err(err) => err,
    };
    assert!(err.to_string().contains("failed to decode"));
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn save_and_exists_return_errors_when_device_table_is_missing() {
    use wa_rs_core::store::Device as CoreDevice;
    use wa_rs_core::store::traits::DeviceStore as DeviceStoreTrait;

    let (_dir, store) = test_store();
    {
        let conn = store.conn.lock();
        conn.execute("DROP TABLE device", []).unwrap();
    }

    let save_err = match DeviceStoreTrait::save(&store, &CoreDevice::new()).await {
        Ok(_) => panic!("expected save to fail without device table"),
        Err(err) => err,
    };
    assert!(save_err.to_string().contains("no such table"));

    let exists_err = match DeviceStoreTrait::exists(&store).await {
        Ok(_) => panic!("expected exists to fail without device table"),
        Err(err) => err,
    };
    assert!(exists_err.to_string().contains("no such table"));
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn snapshot_db_copies_database_and_optional_extra_content() {
    use wa_rs_core::store::Device as CoreDevice;
    use wa_rs_core::store::traits::DeviceStore as DeviceStoreTrait;

    let (dir, store) = test_store();
    let mut device = CoreDevice::new();
    device.push_name = "Snapshot".to_string();
    DeviceStoreTrait::save(&store, &device).await.unwrap();

    DeviceStoreTrait::snapshot_db(&store, "backup", Some(b"extra")).await.unwrap();

    let snapshot = dir.path().join("session.db.snapshot.backup");
    let extra = dir.path().join("session.db.snapshot.backup.extra");
    assert!(snapshot.exists());
    assert_eq!(std::fs::read(extra).unwrap(), b"extra");
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn snapshot_db_without_extra_content_only_copies_database() {
    use wa_rs_core::store::traits::DeviceStore as DeviceStoreTrait;

    let (dir, store) = test_store();

    DeviceStoreTrait::snapshot_db(&store, "plain", None).await.unwrap();

    let snapshot = dir.path().join("session.db.snapshot.plain");
    let extra = dir.path().join("session.db.snapshot.plain.extra");
    assert!(snapshot.exists());
    assert!(!extra.exists());
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn snapshot_db_returns_error_when_source_database_is_missing() {
    use wa_rs_core::store::traits::DeviceStore as DeviceStoreTrait;

    let (_dir, store) = test_store();
    let mut missing_db = store.clone();
    missing_db.db_path = "/path/that/does/not/exist/session.db".to_string();

    let err = match DeviceStoreTrait::snapshot_db(&missing_db, "broken", Some(b"extra")).await {
        Ok(_) => panic!("expected snapshot to fail for missing database"),
        Err(err) => err,
    };
    assert!(err.to_string().contains("No such file") || err.to_string().contains("os error 2"));
}
// 本文件验证 WhatsApp 设备记录的隔离、替换、反序列化与数据库快照。
