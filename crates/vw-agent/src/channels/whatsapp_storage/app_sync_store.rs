//! WhatsApp 应用状态同步存储，持久化同步密钥、版本和变更校验码。
//!
//! AppSyncStore trait implementation for WhatsApp storage
//!
//! Implements WhatsApp app state synchronization operations including:
//! - Sync key management
//! - Version tracking
//! - Mutation MAC storage

#[cfg(feature = "whatsapp-web")]
use async_trait::async_trait;
#[cfg(feature = "whatsapp-web")]
use rusqlite::params;

#[cfg(feature = "whatsapp-web")]
use wa_rs_core::appstate::hash::HashState;
#[cfg(feature = "whatsapp-web")]
use wa_rs_core::appstate::processor::AppStateMutationMAC;
#[cfg(feature = "whatsapp-web")]
use wa_rs_core::store::traits::{AppStateSyncKey, AppSyncStore};

use super::RusqliteStore;

#[cfg(feature = "whatsapp-web")]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AppSyncStore for RusqliteStore {
    async fn get_sync_key(
        &self,
        key_id: &[u8],
    ) -> wa_rs_core::store::error::Result<Option<AppStateSyncKey>> {
        let conn = self.conn.lock();
        let result = conn.query_row(
            "SELECT key_data FROM app_state_keys WHERE key_id = ?1 AND device_id = ?2",
            params![key_id, self.device_id],
            |row| {
                let key_data: Vec<u8> = row.get(0)?;
                serde_json::from_slice(&key_data)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
            },
        );

        match result {
            Ok(key) => Ok(Some(key)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(wa_rs_core::store::error::StoreError::Database(e.to_string())),
        }
    }

    async fn set_sync_key(
        &self,
        key_id: &[u8],
        key: AppStateSyncKey,
    ) -> wa_rs_core::store::error::Result<()> {
        let conn = self.conn.lock();
        let key_data = to_store_err!(serde_json::to_vec(&key))?;

        to_store_err!(execute: conn.execute(
            "INSERT OR REPLACE INTO app_state_keys (key_id, key_data, device_id)
             VALUES (?1, ?2, ?3)",
            params![key_id, key_data, self.device_id],
        ))
    }

    async fn get_version(&self, name: &str) -> wa_rs_core::store::error::Result<HashState> {
        let conn = self.conn.lock();
        let state_data: Vec<u8> = to_store_err!(conn.query_row(
            "SELECT state_data FROM app_state_versions WHERE name = ?1 AND device_id = ?2",
            params![name, self.device_id],
            |row| row.get(0),
        ))?;

        to_store_err!(serde_json::from_slice(&state_data))
    }

    async fn set_version(
        &self,
        name: &str,
        state: HashState,
    ) -> wa_rs_core::store::error::Result<()> {
        let conn = self.conn.lock();
        let state_data = to_store_err!(serde_json::to_vec(&state))?;

        to_store_err!(execute: conn.execute(
            "INSERT OR REPLACE INTO app_state_versions (name, state_data, device_id)
             VALUES (?1, ?2, ?3)",
            params![name, state_data, self.device_id],
        ))
    }

    async fn put_mutation_macs(
        &self,
        name: &str,
        version: u64,
        mutations: &[AppStateMutationMAC],
    ) -> wa_rs_core::store::error::Result<()> {
        let conn = self.conn.lock();

        for mutation in mutations {
            let index_mac = to_store_err!(serde_json::to_vec(&mutation.index_mac))?;
            let value_mac = to_store_err!(serde_json::to_vec(&mutation.value_mac))?;

            to_store_err!(execute: conn.execute(
                "INSERT OR REPLACE INTO app_state_mutation_macs
                 (name, version, index_mac, value_mac, device_id)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![name, i64::try_from(version).unwrap_or(i64::MAX), index_mac, value_mac, self.device_id],
            ))?;
        }

        Ok(())
    }

    async fn get_mutation_mac(
        &self,
        name: &str,
        index_mac: &[u8],
    ) -> wa_rs_core::store::error::Result<Option<Vec<u8>>> {
        let conn = self.conn.lock();
        let index_mac_json = to_store_err!(serde_json::to_vec(index_mac))?;

        let result = conn.query_row(
            "SELECT value_mac FROM app_state_mutation_macs
             WHERE name = ?1 AND index_mac = ?2 AND device_id = ?3",
            params![name, index_mac_json, self.device_id],
            |row| row.get::<_, Vec<u8>>(0),
        );

        match result {
            Ok(mac) => Ok(Some(mac)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(wa_rs_core::store::error::StoreError::Database(e.to_string())),
        }
    }

    async fn delete_mutation_macs(
        &self,
        name: &str,
        index_macs: &[Vec<u8>],
    ) -> wa_rs_core::store::error::Result<()> {
        let conn = self.conn.lock();

        for index_mac in index_macs {
            let index_mac_json = to_store_err!(serde_json::to_vec(index_mac))?;

            to_store_err!(execute: conn.execute(
                "DELETE FROM app_state_mutation_macs
                 WHERE name = ?1 AND index_mac = ?2 AND device_id = ?3",
                params![name, index_mac_json, self.device_id],
            ))?;
        }

        Ok(())
    }
}
