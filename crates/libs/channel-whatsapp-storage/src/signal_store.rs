//! WhatsApp Signal 协议存储，管理身份密钥、会话、预密钥与发送者密钥。
//!
//! 存取操作均绑定当前设备，缺失记录返回 `None`，数据库故障转换为 wa-rs 存储错误。
//!
//! SignalStore trait implementation for WhatsApp storage
//!
//! Implements Signal protocol cryptographic operations including:
//! - Identity key management
//! - Session management
//! - PreKey operations
//! - Signed PreKey operations
//! - Sender Key operations

#[cfg(feature = "whatsapp-web")]
use async_trait::async_trait;
#[cfg(feature = "whatsapp-web")]
use rusqlite::params;

#[cfg(feature = "whatsapp-web")]
use wa_rs_core::store::traits::SignalStore;

use super::RusqliteStore;

#[cfg(feature = "whatsapp-web")]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl SignalStore for RusqliteStore {
    async fn put_identity(
        &self,
        address: &str,
        key: [u8; 32],
    ) -> wa_rs_core::store::error::Result<()> {
        let conn = self.conn.lock();
        to_store_err!(execute: conn.execute(
            "INSERT OR REPLACE INTO identities (address, key, device_id)
             VALUES (?1, ?2, ?3)",
            params![address, key.to_vec(), self.device_id],
        ))
    }

    async fn load_identity(
        &self,
        address: &str,
    ) -> wa_rs_core::store::error::Result<Option<Vec<u8>>> {
        let conn = self.conn.lock();
        let result = conn.query_row(
            "SELECT key FROM identities WHERE address = ?1 AND device_id = ?2",
            params![address, self.device_id],
            |row| row.get::<_, Vec<u8>>(0),
        );

        match result {
            Ok(key) => Ok(Some(key)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(wa_rs_core::store::error::StoreError::Database(e.to_string())),
        }
    }

    async fn delete_identity(&self, address: &str) -> wa_rs_core::store::error::Result<()> {
        let conn = self.conn.lock();
        to_store_err!(execute: conn.execute(
            "DELETE FROM identities WHERE address = ?1 AND device_id = ?2",
            params![address, self.device_id],
        ))
    }

    async fn get_session(
        &self,
        address: &str,
    ) -> wa_rs_core::store::error::Result<Option<Vec<u8>>> {
        let conn = self.conn.lock();
        let result = conn.query_row(
            "SELECT record FROM sessions WHERE address = ?1 AND device_id = ?2",
            params![address, self.device_id],
            |row| row.get::<_, Vec<u8>>(0),
        );

        match result {
            Ok(record) => Ok(Some(record)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(wa_rs_core::store::error::StoreError::Database(e.to_string())),
        }
    }

    async fn put_session(
        &self,
        address: &str,
        session: &[u8],
    ) -> wa_rs_core::store::error::Result<()> {
        let conn = self.conn.lock();
        to_store_err!(execute: conn.execute(
            "INSERT OR REPLACE INTO sessions (address, record, device_id)
             VALUES (?1, ?2, ?3)",
            params![address, session, self.device_id],
        ))
    }

    async fn delete_session(&self, address: &str) -> wa_rs_core::store::error::Result<()> {
        let conn = self.conn.lock();
        to_store_err!(execute: conn.execute(
            "DELETE FROM sessions WHERE address = ?1 AND device_id = ?2",
            params![address, self.device_id],
        ))
    }

    async fn store_prekey(
        &self,
        id: u32,
        record: &[u8],
        uploaded: bool,
    ) -> wa_rs_core::store::error::Result<()> {
        let conn = self.conn.lock();
        to_store_err!(execute: conn.execute(
            "INSERT OR REPLACE INTO prekeys (id, key, uploaded, device_id)
             VALUES (?1, ?2, ?3, ?4)",
            params![id, record, uploaded, self.device_id],
        ))
    }

    async fn load_prekey(&self, id: u32) -> wa_rs_core::store::error::Result<Option<Vec<u8>>> {
        let conn = self.conn.lock();
        let result = conn.query_row(
            "SELECT key FROM prekeys WHERE id = ?1 AND device_id = ?2",
            params![id, self.device_id],
            |row| row.get::<_, Vec<u8>>(0),
        );

        match result {
            Ok(key) => Ok(Some(key)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(wa_rs_core::store::error::StoreError::Database(e.to_string())),
        }
    }

    async fn remove_prekey(&self, id: u32) -> wa_rs_core::store::error::Result<()> {
        let conn = self.conn.lock();
        to_store_err!(execute: conn.execute(
            "DELETE FROM prekeys WHERE id = ?1 AND device_id = ?2",
            params![id, self.device_id],
        ))
    }

    async fn store_signed_prekey(
        &self,
        id: u32,
        record: &[u8],
    ) -> wa_rs_core::store::error::Result<()> {
        let conn = self.conn.lock();
        to_store_err!(execute: conn.execute(
            "INSERT OR REPLACE INTO signed_prekeys (id, record, device_id)
             VALUES (?1, ?2, ?3)",
            params![id, record, self.device_id],
        ))
    }

    async fn load_signed_prekey(
        &self,
        id: u32,
    ) -> wa_rs_core::store::error::Result<Option<Vec<u8>>> {
        let conn = self.conn.lock();
        let result = conn.query_row(
            "SELECT record FROM signed_prekeys WHERE id = ?1 AND device_id = ?2",
            params![id, self.device_id],
            |row| row.get::<_, Vec<u8>>(0),
        );

        match result {
            Ok(record) => Ok(Some(record)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(wa_rs_core::store::error::StoreError::Database(e.to_string())),
        }
    }

    async fn load_all_signed_prekeys(
        &self,
    ) -> wa_rs_core::store::error::Result<Vec<(u32, Vec<u8>)>> {
        let conn = self.conn.lock();
        let mut stmt = to_store_err!(
            conn.prepare("SELECT id, record FROM signed_prekeys WHERE device_id = ?1")
        )?;

        let rows = to_store_err!(stmt.query_map(params![self.device_id], |row| {
            Ok((row.get::<_, u32>(0)?, row.get::<_, Vec<u8>>(1)?))
        }))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(to_store_err!(row)?);
        }

        Ok(result)
    }

    async fn remove_signed_prekey(&self, id: u32) -> wa_rs_core::store::error::Result<()> {
        let conn = self.conn.lock();
        to_store_err!(execute: conn.execute(
            "DELETE FROM signed_prekeys WHERE id = ?1 AND device_id = ?2",
            params![id, self.device_id],
        ))
    }

    async fn put_sender_key(
        &self,
        address: &str,
        record: &[u8],
    ) -> wa_rs_core::store::error::Result<()> {
        let conn = self.conn.lock();
        to_store_err!(execute: conn.execute(
            "INSERT OR REPLACE INTO sender_keys (address, record, device_id)
             VALUES (?1, ?2, ?3)",
            params![address, record, self.device_id],
        ))
    }

    async fn get_sender_key(
        &self,
        address: &str,
    ) -> wa_rs_core::store::error::Result<Option<Vec<u8>>> {
        let conn = self.conn.lock();
        let result = conn.query_row(
            "SELECT record FROM sender_keys WHERE address = ?1 AND device_id = ?2",
            params![address, self.device_id],
            |row| row.get::<_, Vec<u8>>(0),
        );

        match result {
            Ok(record) => Ok(Some(record)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(wa_rs_core::store::error::StoreError::Database(e.to_string())),
        }
    }

    async fn delete_sender_key(&self, address: &str) -> wa_rs_core::store::error::Result<()> {
        let conn = self.conn.lock();
        to_store_err!(execute: conn.execute(
            "DELETE FROM sender_keys WHERE address = ?1 AND device_id = ?2",
            params![address, self.device_id],
        ))
    }
}
