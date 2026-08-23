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
fn signal_store_is_disabled_without_whatsapp_web_feature() {
    assert!(!cfg!(feature = "whatsapp-web"));
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn identity_session_and_sender_key_round_trips() {
    use wa_rs_core::store::traits::SignalStore;

    let (_tmp, store) = test_store();

    SignalStore::put_identity(&store, "user@s.whatsapp.net", [7u8; 32]).await.unwrap();
    assert_eq!(
        SignalStore::load_identity(&store, "user@s.whatsapp.net").await.unwrap(),
        Some(vec![7u8; 32])
    );
    SignalStore::delete_identity(&store, "user@s.whatsapp.net").await.unwrap();
    assert!(SignalStore::load_identity(&store, "user@s.whatsapp.net").await.unwrap().is_none());

    SignalStore::put_session(&store, "user@s.whatsapp.net", &[1, 2, 3]).await.unwrap();
    assert!(SignalStore::has_session(&store, "user@s.whatsapp.net").await.unwrap());
    assert_eq!(
        SignalStore::get_session(&store, "user@s.whatsapp.net").await.unwrap(),
        Some(vec![1, 2, 3])
    );
    SignalStore::delete_session(&store, "user@s.whatsapp.net").await.unwrap();
    assert!(!SignalStore::has_session(&store, "user@s.whatsapp.net").await.unwrap());

    SignalStore::put_sender_key(&store, "group::sender", &[9, 8]).await.unwrap();
    assert_eq!(
        SignalStore::get_sender_key(&store, "group::sender").await.unwrap(),
        Some(vec![9, 8])
    );
    SignalStore::delete_sender_key(&store, "group::sender").await.unwrap();
    assert!(SignalStore::get_sender_key(&store, "group::sender").await.unwrap().is_none());
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn prekeys_and_signed_prekeys_round_trip_and_delete() {
    use wa_rs_core::store::traits::SignalStore;

    let (_tmp, store) = test_store();

    SignalStore::store_prekey(&store, 10, &[1, 1, 2], false).await.unwrap();
    assert_eq!(SignalStore::load_prekey(&store, 10).await.unwrap(), Some(vec![1, 1, 2]));
    SignalStore::remove_prekey(&store, 10).await.unwrap();
    assert!(SignalStore::load_prekey(&store, 10).await.unwrap().is_none());

    SignalStore::store_signed_prekey(&store, 20, &[3, 5, 8]).await.unwrap();
    SignalStore::store_signed_prekey(&store, 21, &[13, 21]).await.unwrap();
    assert_eq!(SignalStore::load_signed_prekey(&store, 20).await.unwrap(), Some(vec![3, 5, 8]));

    let mut all = SignalStore::load_all_signed_prekeys(&store).await.unwrap();
    all.sort_by_key(|(id, _)| *id);
    assert_eq!(all, vec![(20, vec![3, 5, 8]), (21, vec![13, 21])]);

    SignalStore::remove_signed_prekey(&store, 20).await.unwrap();
    assert!(SignalStore::load_signed_prekey(&store, 20).await.unwrap().is_none());
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn signal_records_are_replaced_and_accept_empty_values() {
    use wa_rs_core::store::traits::SignalStore;

    let (_tmp, store) = test_store();

    SignalStore::put_identity(&store, "", [1u8; 32]).await.unwrap();
    SignalStore::put_identity(&store, "", [2u8; 32]).await.unwrap();
    assert_eq!(SignalStore::load_identity(&store, "").await.unwrap(), Some(vec![2u8; 32]));

    SignalStore::put_session(&store, "", &[]).await.unwrap();
    SignalStore::put_session(&store, "", &[4, 5]).await.unwrap();
    assert_eq!(SignalStore::get_session(&store, "").await.unwrap(), Some(vec![4, 5]));

    SignalStore::store_prekey(&store, u32::MAX, &[], true).await.unwrap();
    SignalStore::store_prekey(&store, u32::MAX, &[8], false).await.unwrap();
    assert_eq!(SignalStore::load_prekey(&store, u32::MAX).await.unwrap(), Some(vec![8]));

    SignalStore::store_signed_prekey(&store, 0, &[]).await.unwrap();
    SignalStore::store_signed_prekey(&store, 0, &[13]).await.unwrap();
    assert_eq!(SignalStore::load_signed_prekey(&store, 0).await.unwrap(), Some(vec![13]));

    SignalStore::put_sender_key(&store, "", &[]).await.unwrap();
    SignalStore::put_sender_key(&store, "", &[21]).await.unwrap();
    assert_eq!(SignalStore::get_sender_key(&store, "").await.unwrap(), Some(vec![21]));
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn signal_records_are_scoped_by_device_id() {
    use wa_rs_core::store::traits::SignalStore;

    let (_tmp, store) = test_store();
    let mut other_device = store.clone();
    other_device.device_id = 2;

    SignalStore::put_identity(&store, "user@s.whatsapp.net", [3u8; 32]).await.unwrap();
    SignalStore::put_session(&store, "user@s.whatsapp.net", &[1]).await.unwrap();
    SignalStore::store_prekey(&store, 7, &[2], false).await.unwrap();
    SignalStore::store_signed_prekey(&store, 8, &[3]).await.unwrap();
    SignalStore::put_sender_key(&store, "group::sender", &[4]).await.unwrap();

    assert!(
        SignalStore::load_identity(&other_device, "user@s.whatsapp.net").await.unwrap().is_none()
    );
    assert!(
        SignalStore::get_session(&other_device, "user@s.whatsapp.net").await.unwrap().is_none()
    );
    assert!(SignalStore::load_prekey(&other_device, 7).await.unwrap().is_none());
    assert!(SignalStore::load_signed_prekey(&other_device, 8).await.unwrap().is_none());
    assert!(SignalStore::get_sender_key(&other_device, "group::sender").await.unwrap().is_none());
    assert!(SignalStore::load_all_signed_prekeys(&other_device).await.unwrap().is_empty());
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn signal_loads_return_database_errors_when_tables_are_missing() {
    use wa_rs_core::store::traits::SignalStore;

    let (_tmp, store) = test_store();
    {
        let conn = store.conn.lock();
        conn.execute_batch(
            "DROP TABLE identities;
             DROP TABLE sessions;
             DROP TABLE prekeys;
             DROP TABLE signed_prekeys;
             DROP TABLE sender_keys;",
        )
        .unwrap();
    }

    assert!(SignalStore::load_identity(&store, "user@s.whatsapp.net").await.is_err());
    assert!(SignalStore::get_session(&store, "user@s.whatsapp.net").await.is_err());
    assert!(SignalStore::load_prekey(&store, 1).await.is_err());
    assert!(SignalStore::load_signed_prekey(&store, 1).await.is_err());
    assert!(SignalStore::load_all_signed_prekeys(&store).await.is_err());
    assert!(SignalStore::get_sender_key(&store, "group::sender").await.is_err());
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn signal_writes_return_database_errors_when_tables_are_missing() {
    use wa_rs_core::store::traits::SignalStore;

    let (_tmp, store) = test_store();
    {
        let conn = store.conn.lock();
        conn.execute_batch(
            "DROP TABLE identities;
             DROP TABLE sessions;
             DROP TABLE prekeys;
             DROP TABLE signed_prekeys;
             DROP TABLE sender_keys;",
        )
        .unwrap();
    }

    assert!(SignalStore::put_identity(&store, "user@s.whatsapp.net", [7u8; 32]).await.is_err());
    assert!(SignalStore::delete_identity(&store, "user@s.whatsapp.net").await.is_err());

    assert!(SignalStore::put_session(&store, "user@s.whatsapp.net", &[1, 2, 3]).await.is_err());
    assert!(SignalStore::delete_session(&store, "user@s.whatsapp.net").await.is_err());

    assert!(SignalStore::store_prekey(&store, 10, &[1, 1, 2], false).await.is_err());
    assert!(SignalStore::remove_prekey(&store, 10).await.is_err());

    assert!(SignalStore::store_signed_prekey(&store, 20, &[3, 5, 8]).await.is_err());
    assert!(SignalStore::remove_signed_prekey(&store, 20).await.is_err());

    assert!(SignalStore::put_sender_key(&store, "group::sender", &[9, 8]).await.is_err());
    assert!(SignalStore::delete_sender_key(&store, "group::sender").await.is_err());
}
// 本文件验证 WhatsApp Signal 身份、会话及各类密钥记录的设备隔离和错误传播。
