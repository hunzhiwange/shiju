// 本文件验证 WhatsApp 应用同步密钥、版本状态与变更校验码的存储语义。

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
fn app_sync_store_is_disabled_without_whatsapp_web_feature() {
    assert!(!cfg!(feature = "whatsapp-web"));
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn sync_key_round_trip_and_missing_key_returns_none() {
    use wa_rs_core::store::traits::{AppStateSyncKey, AppSyncStore};

    let (_tmp, store) = test_store();
    assert!(AppSyncStore::get_sync_key(&store, b"missing").await.unwrap().is_none());

    let key =
        AppStateSyncKey { key_data: vec![1, 2, 3], fingerprint: vec![4, 5, 6], timestamp: 123 };
    AppSyncStore::set_sync_key(&store, b"key-1", key.clone()).await.unwrap();

    let loaded = AppSyncStore::get_sync_key(&store, b"key-1").await.unwrap().unwrap();
    assert_eq!(loaded.key_data, key.key_data);
    assert_eq!(loaded.fingerprint, key.fingerprint);
    assert_eq!(loaded.timestamp, key.timestamp);
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn sync_key_replaces_existing_value_and_rejects_invalid_stored_key() {
    use rusqlite::params;
    use wa_rs_core::store::traits::{AppStateSyncKey, AppSyncStore};

    let (_tmp, store) = test_store();
    let first = AppStateSyncKey { key_data: vec![1], fingerprint: vec![2], timestamp: 1 };
    let second = AppStateSyncKey { key_data: vec![3], fingerprint: vec![4], timestamp: 2 };

    AppSyncStore::set_sync_key(&store, b"replace", first).await.unwrap();
    AppSyncStore::set_sync_key(&store, b"replace", second.clone()).await.unwrap();

    let loaded = AppSyncStore::get_sync_key(&store, b"replace").await.unwrap().unwrap();
    assert_eq!(loaded.key_data, second.key_data);
    assert_eq!(loaded.fingerprint, second.fingerprint);
    assert_eq!(loaded.timestamp, second.timestamp);

    {
        let conn = store.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO app_state_keys (key_id, key_data, device_id) VALUES (?1, ?2, ?3)",
            params![b"invalid".as_slice(), b"not-json".as_slice(), store.device_id],
        )
        .unwrap();
    }

    assert!(AppSyncStore::get_sync_key(&store, b"invalid").await.is_err());
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn version_round_trip_preserves_hash_state() {
    use std::collections::HashMap;
    use wa_rs_core::appstate::hash::HashState;
    use wa_rs_core::store::traits::AppSyncStore;

    let (_tmp, store) = test_store();
    let mut index_value_map = HashMap::new();
    index_value_map.insert("idx".to_string(), vec![9, 8, 7]);
    let mut hash = [0u8; 128];
    hash[0] = 42;
    hash[127] = 24;
    let state = HashState { version: 77, hash, index_value_map };

    AppSyncStore::set_version(&store, "regular_high", state.clone()).await.unwrap();

    let loaded = AppSyncStore::get_version(&store, "regular_high").await.unwrap();
    assert_eq!(loaded.version, 77);
    assert_eq!(loaded.hash[0], 42);
    assert_eq!(loaded.hash[127], 24);
    assert_eq!(loaded.index_value_map.get("idx"), Some(&vec![9, 8, 7]));
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn version_missing_and_invalid_stored_state_return_errors() {
    use rusqlite::params;
    use wa_rs_core::store::traits::AppSyncStore;

    let (_tmp, store) = test_store();
    assert!(AppSyncStore::get_version(&store, "missing").await.is_err());

    {
        let conn = store.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO app_state_versions (name, state_data, device_id) VALUES (?1, ?2, ?3)",
            params!["invalid", b"not-json".as_slice(), store.device_id],
        )
        .unwrap();
    }

    assert!(AppSyncStore::get_version(&store, "invalid").await.is_err());
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn mutation_macs_can_be_inserted_loaded_and_deleted() {
    use wa_rs_core::appstate::processor::AppStateMutationMAC;
    use wa_rs_core::store::traits::AppSyncStore;

    let (_tmp, store) = test_store();
    let mutation = AppStateMutationMAC { index_mac: vec![1, 2], value_mac: vec![3, 4] };

    AppSyncStore::put_mutation_macs(&store, "critical_block", 5, &[mutation.clone()])
        .await
        .unwrap();

    let loaded = AppSyncStore::get_mutation_mac(&store, "critical_block", &mutation.index_mac)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded, serde_json::to_vec(&mutation.value_mac).unwrap());

    AppSyncStore::delete_mutation_macs(&store, "critical_block", &[mutation.index_mac])
        .await
        .unwrap();
    assert!(
        AppSyncStore::get_mutation_mac(&store, "critical_block", &[1, 2]).await.unwrap().is_none()
    );
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
// 空变更集合和版本饱和不能产生回绕版本。
async fn mutation_macs_handle_empty_inputs_and_saturating_version() {
    use rusqlite::params;
    use wa_rs_core::appstate::processor::AppStateMutationMAC;
    use wa_rs_core::store::traits::AppSyncStore;

    let (_tmp, store) = test_store();
    AppSyncStore::put_mutation_macs(&store, "regular_low", 1, &[]).await.unwrap();
    AppSyncStore::delete_mutation_macs(&store, "regular_low", &[]).await.unwrap();

    let mutation = AppStateMutationMAC { index_mac: vec![9], value_mac: vec![8] };
    AppSyncStore::put_mutation_macs(&store, "regular_low", u64::MAX, &[mutation]).await.unwrap();

    let conn = store.conn.lock();
    let version: i64 = conn
        .query_row(
            "SELECT version FROM app_state_mutation_macs WHERE name = ?1",
            params!["regular_low"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(version, i64::MAX);
}

#[tokio::test]
#[cfg(feature = "whatsapp-web")]
async fn mutation_mac_operations_return_database_errors() {
    use wa_rs_core::appstate::processor::AppStateMutationMAC;
    use wa_rs_core::store::traits::AppSyncStore;

    let (_tmp, store) = test_store();
    {
        let conn = store.conn.lock();
        conn.execute("DROP TABLE app_state_mutation_macs", []).unwrap();
    }

    let mutation = AppStateMutationMAC { index_mac: vec![1], value_mac: vec![2] };

    assert!(AppSyncStore::put_mutation_macs(&store, "regular_low", 1, &[mutation]).await.is_err());
    assert!(AppSyncStore::get_mutation_mac(&store, "regular_low", &[1]).await.is_err());
    assert!(AppSyncStore::delete_mutation_macs(&store, "regular_low", &[vec![1]]).await.is_err());
}
