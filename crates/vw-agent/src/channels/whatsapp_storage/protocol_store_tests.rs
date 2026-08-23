#[cfg(feature = "whatsapp-web")]
use super::RusqliteStore;

#[cfg(feature = "whatsapp-web")]
fn test_store() -> (tempfile::TempDir, RusqliteStore) {
    let tmp = tempfile::tempdir().unwrap();
    let store = RusqliteStore::new(&tmp.path().join("store.db")).unwrap();
    (tmp, store)
}

#[test]
#[cfg(not(feature = "whatsapp-web"))]
fn protocol_store_is_disabled_without_whatsapp_web_feature() {
    assert!(!cfg!(feature = "whatsapp-web"));
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn skdm_recipients_are_inserted_deduplicated_and_cleared() {
    use wa_rs_binary::jid::Jid;
    use wa_rs_core::store::traits::ProtocolStore;

    let (_tmp, store) = test_store();
    let device_a = Jid::pn_device("15550000001", 1);
    let device_b = Jid::pn_device("15550000002", 2);

    ProtocolStore::add_skdm_recipients(
        &store,
        "120363@g.us",
        &[device_a.clone(), device_a.clone(), device_b.clone()],
    )
    .await
    .unwrap();

    let mut loaded = ProtocolStore::get_skdm_recipients(&store, "120363@g.us")
        .await
        .unwrap()
        .into_iter()
        .map(|jid| jid.to_string())
        .collect::<Vec<_>>();
    loaded.sort();
    let mut expected = vec![device_a.to_string(), device_b.to_string()];
    expected.sort();
    assert_eq!(loaded, expected);

    ProtocolStore::clear_skdm_recipients(&store, "120363@g.us").await.unwrap();
    assert!(ProtocolStore::get_skdm_recipients(&store, "120363@g.us").await.unwrap().is_empty());
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn skdm_recipients_skip_invalid_stored_jids() {
    use rusqlite::params;
    use wa_rs_binary::jid::Jid;
    use wa_rs_core::store::traits::ProtocolStore;

    let (_tmp, store) = test_store();
    let device = Jid::pn_device("15550000001", 1);
    ProtocolStore::add_skdm_recipients(&store, "120363@g.us", std::slice::from_ref(&device))
        .await
        .unwrap();
    {
        let conn = store.conn.lock();
        conn.execute(
            "INSERT INTO skdm_recipients (group_jid, device_jid, device_id, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params!["120363@g.us", "not-a-jid", store.device_id, 1],
        )
        .unwrap();
    }

    let loaded = ProtocolStore::get_skdm_recipients(&store, "120363@g.us").await.unwrap();
    assert_eq!(loaded, vec![device]);
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn lid_mapping_round_trip_all_and_latest_phone_mapping() {
    use wa_rs_core::store::traits::{LidPnMappingEntry, ProtocolStore};

    let (_tmp, store) = test_store();
    let older = LidPnMappingEntry {
        lid: "lid-old".to_string(),
        phone_number: "15550000001".to_string(),
        created_at: 1,
        updated_at: 10,
        learning_source: "peer".to_string(),
    };
    let newer = LidPnMappingEntry {
        lid: "lid-new".to_string(),
        phone_number: "15550000001".to_string(),
        created_at: 2,
        updated_at: 20,
        learning_source: "usync".to_string(),
    };

    ProtocolStore::put_lid_mapping(&store, &older).await.unwrap();
    ProtocolStore::put_lid_mapping(&store, &newer).await.unwrap();

    assert_eq!(
        ProtocolStore::get_lid_mapping(&store, "lid-old").await.unwrap().unwrap().learning_source,
        "peer"
    );
    assert_eq!(
        ProtocolStore::get_pn_mapping(&store, "15550000001").await.unwrap().unwrap().lid,
        "lid-new"
    );

    let all = ProtocolStore::get_all_lid_mappings(&store).await.unwrap();
    assert_eq!(all.len(), 2);
    assert!(ProtocolStore::get_lid_mapping(&store, "missing").await.unwrap().is_none());
    assert!(ProtocolStore::get_pn_mapping(&store, "missing").await.unwrap().is_none());
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn lid_mapping_replaces_existing_lid() {
    use wa_rs_core::store::traits::{LidPnMappingEntry, ProtocolStore};

    let (_tmp, store) = test_store();
    let first = LidPnMappingEntry {
        lid: "lid".to_string(),
        phone_number: "15550000001".to_string(),
        created_at: 1,
        updated_at: 10,
        learning_source: "peer".to_string(),
    };
    let replacement = LidPnMappingEntry {
        lid: "lid".to_string(),
        phone_number: "15550000002".to_string(),
        created_at: 1,
        updated_at: 20,
        learning_source: "usync".to_string(),
    };

    ProtocolStore::put_lid_mapping(&store, &first).await.unwrap();
    ProtocolStore::put_lid_mapping(&store, &replacement).await.unwrap();

    let loaded = ProtocolStore::get_lid_mapping(&store, "lid").await.unwrap().unwrap();
    assert_eq!(loaded.phone_number, "15550000002");
    assert_eq!(loaded.learning_source, "usync");
    assert_eq!(ProtocolStore::get_all_lid_mappings(&store).await.unwrap().len(), 1);
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn base_key_collision_detection_compares_saved_key_and_deletes() {
    use wa_rs_core::store::traits::ProtocolStore;

    let (_tmp, store) = test_store();
    assert!(!ProtocolStore::has_same_base_key(&store, "addr", "msg", &[1, 2]).await.unwrap());

    ProtocolStore::save_base_key(&store, "addr", "msg", &[1, 2]).await.unwrap();
    assert!(ProtocolStore::has_same_base_key(&store, "addr", "msg", &[1, 2]).await.unwrap());
    assert!(!ProtocolStore::has_same_base_key(&store, "addr", "msg", &[9, 9]).await.unwrap());

    ProtocolStore::delete_base_key(&store, "addr", "msg").await.unwrap();
    assert!(!ProtocolStore::has_same_base_key(&store, "addr", "msg", &[1, 2]).await.unwrap());
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn device_registry_round_trip_and_missing_user() {
    use wa_rs_core::store::traits::{DeviceInfo, DeviceListRecord, ProtocolStore};

    let (_tmp, store) = test_store();
    let record = DeviceListRecord {
        user: "15550000001".to_string(),
        devices: vec![
            DeviceInfo { device_id: 0, key_index: None },
            DeviceInfo { device_id: 3, key_index: Some(99) },
        ],
        timestamp: 1234,
        phash: Some("hash".to_string()),
    };

    ProtocolStore::update_device_list(&store, record).await.unwrap();

    let loaded = ProtocolStore::get_devices(&store, "15550000001").await.unwrap().unwrap();
    assert_eq!(loaded.user, "15550000001");
    assert_eq!(loaded.devices.len(), 2);
    assert_eq!(loaded.devices[1].key_index, Some(99));
    assert_eq!(loaded.phash.as_deref(), Some("hash"));
    assert!(ProtocolStore::get_devices(&store, "missing").await.unwrap().is_none());
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn device_registry_replaces_existing_record_and_reports_bad_json() {
    use rusqlite::params;
    use wa_rs_core::store::traits::{DeviceInfo, DeviceListRecord, ProtocolStore};

    let (_tmp, store) = test_store();
    let first = DeviceListRecord {
        user: "15550000001".to_string(),
        devices: vec![DeviceInfo { device_id: 1, key_index: Some(1) }],
        timestamp: 1,
        phash: Some("old".to_string()),
    };
    let replacement = DeviceListRecord {
        user: "15550000001".to_string(),
        devices: vec![],
        timestamp: 2,
        phash: None,
    };

    ProtocolStore::update_device_list(&store, first).await.unwrap();
    ProtocolStore::update_device_list(&store, replacement).await.unwrap();

    let loaded = ProtocolStore::get_devices(&store, "15550000001").await.unwrap().unwrap();
    assert!(loaded.devices.is_empty());
    assert_eq!(loaded.timestamp, 2);
    assert_eq!(loaded.phash, None);

    {
        let conn = store.conn.lock();
        conn.execute(
            "UPDATE device_registry SET devices_json = ?1 WHERE user_id = ?2 AND device_id = ?3",
            params!["not-json", "15550000001", store.device_id],
        )
        .unwrap();
    }
    assert!(ProtocolStore::get_devices(&store, "15550000001").await.is_err());
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn forget_marks_are_consumed_once() {
    use wa_rs_core::store::traits::ProtocolStore;

    let (_tmp, store) = test_store();
    ProtocolStore::mark_forget_sender_key(&store, "group@g.us", "alice").await.unwrap();
    ProtocolStore::mark_forget_sender_key(&store, "group@g.us", "bob").await.unwrap();

    let mut marks = ProtocolStore::consume_forget_marks(&store, "group@g.us").await.unwrap();
    marks.sort();
    assert_eq!(marks, vec!["alice".to_string(), "bob".to_string()]);
    assert!(ProtocolStore::consume_forget_marks(&store, "group@g.us").await.unwrap().is_empty());
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn delete_operations_allow_missing_entries() {
    use wa_rs_core::store::traits::ProtocolStore;

    let (_tmp, store) = test_store();

    ProtocolStore::clear_skdm_recipients(&store, "missing@g.us").await.unwrap();
    ProtocolStore::delete_base_key(&store, "missing", "missing").await.unwrap();
    ProtocolStore::delete_tc_token(&store, "missing").await.unwrap();
    assert_eq!(ProtocolStore::delete_expired_tc_tokens(&store, 1).await.unwrap(), 0);
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn tc_tokens_round_trip_list_delete_and_expire() {
    use wa_rs_core::store::traits::{ProtocolStore, TcTokenEntry};

    let (_tmp, store) = test_store();
    let expired =
        TcTokenEntry { token: vec![1, 2], token_timestamp: 10, sender_timestamp: Some(11) };
    let fresh = TcTokenEntry { token: vec![3, 4], token_timestamp: 100, sender_timestamp: None };

    ProtocolStore::put_tc_token(&store, "jid-old", &expired).await.unwrap();
    ProtocolStore::put_tc_token(&store, "jid-new", &fresh).await.unwrap();

    assert_eq!(
        ProtocolStore::get_tc_token(&store, "jid-old").await.unwrap().unwrap().sender_timestamp,
        Some(11)
    );
    let mut jids = ProtocolStore::get_all_tc_token_jids(&store).await.unwrap();
    jids.sort();
    assert_eq!(jids, vec!["jid-new".to_string(), "jid-old".to_string()]);

    assert_eq!(ProtocolStore::delete_expired_tc_tokens(&store, 50).await.unwrap(), 1);
    assert!(ProtocolStore::get_tc_token(&store, "jid-old").await.unwrap().is_none());
    assert!(ProtocolStore::get_tc_token(&store, "jid-new").await.unwrap().is_some());

    ProtocolStore::delete_tc_token(&store, "jid-new").await.unwrap();
    assert!(ProtocolStore::get_all_tc_token_jids(&store).await.unwrap().is_empty());
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn protocol_records_are_scoped_by_device_id() {
    use wa_rs_binary::jid::Jid;
    use wa_rs_core::store::traits::{
        DeviceInfo, DeviceListRecord, LidPnMappingEntry, ProtocolStore, TcTokenEntry,
    };

    let (_tmp, store) = test_store();
    let mut other = store.clone();
    other.device_id = 2;
    let jid_one = Jid::pn_device("15550000001", 1);
    let jid_two = Jid::pn_device("15550000002", 1);

    ProtocolStore::add_skdm_recipients(&store, "group@g.us", std::slice::from_ref(&jid_one))
        .await
        .unwrap();
    ProtocolStore::add_skdm_recipients(&other, "group@g.us", std::slice::from_ref(&jid_two))
        .await
        .unwrap();
    ProtocolStore::clear_skdm_recipients(&other, "group@g.us").await.unwrap();
    assert_eq!(
        ProtocolStore::get_skdm_recipients(&store, "group@g.us").await.unwrap(),
        vec![jid_one]
    );

    let mapping_one = LidPnMappingEntry {
        lid: "lid".to_string(),
        phone_number: "15550000001".to_string(),
        created_at: 1,
        updated_at: 1,
        learning_source: "one".to_string(),
    };
    let mapping_two = LidPnMappingEntry {
        lid: "lid".to_string(),
        phone_number: "15550000002".to_string(),
        created_at: 2,
        updated_at: 2,
        learning_source: "two".to_string(),
    };
    ProtocolStore::put_lid_mapping(&store, &mapping_one).await.unwrap();
    ProtocolStore::put_lid_mapping(&other, &mapping_two).await.unwrap();
    assert_eq!(
        ProtocolStore::get_lid_mapping(&store, "lid").await.unwrap().unwrap().phone_number,
        "15550000001"
    );

    ProtocolStore::save_base_key(&store, "addr", "msg", &[1]).await.unwrap();
    ProtocolStore::save_base_key(&other, "addr", "msg", &[2]).await.unwrap();
    assert!(ProtocolStore::has_same_base_key(&store, "addr", "msg", &[1]).await.unwrap());
    ProtocolStore::delete_base_key(&other, "addr", "msg").await.unwrap();
    assert!(ProtocolStore::has_same_base_key(&store, "addr", "msg", &[1]).await.unwrap());

    ProtocolStore::update_device_list(
        &store,
        DeviceListRecord {
            user: "user".to_string(),
            devices: vec![DeviceInfo { device_id: 1, key_index: None }],
            timestamp: 1,
            phash: None,
        },
    )
    .await
    .unwrap();
    ProtocolStore::update_device_list(
        &other,
        DeviceListRecord {
            user: "user".to_string(),
            devices: vec![DeviceInfo { device_id: 2, key_index: Some(2) }],
            timestamp: 2,
            phash: Some("other".to_string()),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        ProtocolStore::get_devices(&store, "user").await.unwrap().unwrap().devices[0].device_id,
        1
    );

    ProtocolStore::mark_forget_sender_key(&store, "group@g.us", "alice").await.unwrap();
    ProtocolStore::mark_forget_sender_key(&other, "group@g.us", "bob").await.unwrap();
    assert_eq!(
        ProtocolStore::consume_forget_marks(&other, "group@g.us").await.unwrap(),
        vec!["bob".to_string()]
    );
    assert_eq!(
        ProtocolStore::consume_forget_marks(&store, "group@g.us").await.unwrap(),
        vec!["alice".to_string()]
    );

    ProtocolStore::put_tc_token(
        &store,
        "jid",
        &TcTokenEntry { token: vec![1], token_timestamp: 1, sender_timestamp: None },
    )
    .await
    .unwrap();
    ProtocolStore::put_tc_token(
        &other,
        "jid",
        &TcTokenEntry { token: vec![2], token_timestamp: 2, sender_timestamp: Some(2) },
    )
    .await
    .unwrap();
    ProtocolStore::delete_tc_token(&other, "jid").await.unwrap();
    assert_eq!(ProtocolStore::get_tc_token(&store, "jid").await.unwrap().unwrap().token, vec![1]);
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn protocol_reads_return_database_errors_when_tables_are_missing() {
    use wa_rs_core::store::traits::{LidPnMappingEntry, ProtocolStore, TcTokenEntry};

    let (_tmp, store) = test_store();
    ProtocolStore::add_skdm_recipients(
        &store,
        "group@g.us",
        &[wa_rs_binary::jid::Jid::pn_device("15550000001", 1)],
    )
    .await
    .unwrap();
    ProtocolStore::put_lid_mapping(
        &store,
        &LidPnMappingEntry {
            lid: "lid".to_string(),
            phone_number: "15550000001".to_string(),
            created_at: 1,
            updated_at: 2,
            learning_source: "peer".to_string(),
        },
    )
    .await
    .unwrap();
    ProtocolStore::save_base_key(&store, "addr", "msg", &[1, 2]).await.unwrap();
    ProtocolStore::update_device_list(
        &store,
        wa_rs_core::store::traits::DeviceListRecord {
            user: "user".to_string(),
            devices: vec![],
            timestamp: 1,
            phash: None,
        },
    )
    .await
    .unwrap();
    ProtocolStore::mark_forget_sender_key(&store, "group@g.us", "alice").await.unwrap();
    ProtocolStore::put_tc_token(
        &store,
        "jid",
        &TcTokenEntry { token: vec![1], token_timestamp: 1, sender_timestamp: None },
    )
    .await
    .unwrap();

    {
        let conn = store.conn.lock();
        conn.execute_batch(
            "DROP TABLE skdm_recipients;
             DROP TABLE lid_pn_mapping;
             DROP TABLE base_keys;
             DROP TABLE device_registry;
             DROP TABLE sender_key_status;
             DROP TABLE tc_tokens;",
        )
        .unwrap();
    }

    assert!(ProtocolStore::get_skdm_recipients(&store, "group@g.us").await.is_err());
    assert!(ProtocolStore::get_lid_mapping(&store, "lid").await.is_err());
    assert!(ProtocolStore::get_pn_mapping(&store, "15550000001").await.is_err());
    assert!(ProtocolStore::get_all_lid_mappings(&store).await.is_err());
    assert!(ProtocolStore::has_same_base_key(&store, "addr", "msg", &[1, 2]).await.is_err());
    assert!(ProtocolStore::get_devices(&store, "user").await.is_err());
    assert!(ProtocolStore::consume_forget_marks(&store, "group@g.us").await.is_err());
    assert!(ProtocolStore::get_tc_token(&store, "jid").await.is_err());
    assert!(ProtocolStore::get_all_tc_token_jids(&store).await.is_err());
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn lid_mapping_list_reports_invalid_stored_column_type() {
    use rusqlite::params;
    use wa_rs_core::store::traits::ProtocolStore;

    let (_tmp, store) = test_store();
    {
        let conn = store.conn.lock();
        conn.execute(
            "INSERT INTO lid_pn_mapping
             (lid, phone_number, created_at, learning_source, updated_at, device_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params!["lid", "15550000001", vec![1_u8], "peer", 2, store.device_id],
        )
        .unwrap();
    }

    let err = ProtocolStore::get_all_lid_mappings(&store)
        .await
        .expect_err("invalid created_at type should be reported");

    assert!(err.to_string().contains("created_at") || err.to_string().contains("column index 2"));
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn protocol_writes_return_database_errors_when_tables_are_missing() {
    use wa_rs_core::store::traits::{
        DeviceListRecord, LidPnMappingEntry, ProtocolStore, TcTokenEntry,
    };

    let (_tmp, store) = test_store();
    {
        let conn = store.conn.lock();
        conn.execute_batch(
            "DROP TABLE skdm_recipients;
             DROP TABLE lid_pn_mapping;
             DROP TABLE base_keys;
             DROP TABLE device_registry;
             DROP TABLE sender_key_status;
             DROP TABLE tc_tokens;",
        )
        .unwrap();
    }

    assert!(
        ProtocolStore::add_skdm_recipients(
            &store,
            "group@g.us",
            &[wa_rs_binary::jid::Jid::pn_device("15550000001", 1)],
        )
        .await
        .is_err()
    );
    assert!(ProtocolStore::clear_skdm_recipients(&store, "group@g.us").await.is_err());
    assert!(
        ProtocolStore::put_lid_mapping(
            &store,
            &LidPnMappingEntry {
                lid: "lid".to_string(),
                phone_number: "15550000001".to_string(),
                created_at: 1,
                updated_at: 2,
                learning_source: "peer".to_string(),
            },
        )
        .await
        .is_err()
    );
    assert!(ProtocolStore::save_base_key(&store, "addr", "msg", &[1, 2]).await.is_err());
    assert!(ProtocolStore::delete_base_key(&store, "addr", "msg").await.is_err());
    assert!(
        ProtocolStore::update_device_list(
            &store,
            DeviceListRecord {
                user: "user".to_string(),
                devices: vec![],
                timestamp: 1,
                phash: None
            },
        )
        .await
        .is_err()
    );
    assert!(ProtocolStore::mark_forget_sender_key(&store, "group@g.us", "alice").await.is_err());
    assert!(
        ProtocolStore::put_tc_token(
            &store,
            "jid",
            &TcTokenEntry { token: vec![1], token_timestamp: 1, sender_timestamp: None },
        )
        .await
        .is_err()
    );
    assert!(ProtocolStore::delete_tc_token(&store, "jid").await.is_err());
    assert!(ProtocolStore::delete_expired_tc_tokens(&store, 1).await.is_err());
}
// 本文件验证 WhatsApp 协议存储中的收件人、映射、密钥及令牌生命周期。
