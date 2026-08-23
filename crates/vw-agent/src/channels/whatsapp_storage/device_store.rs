//! WhatsApp 设备存储，负责设备身份材料、注册状态与数据库快照。
//!
//! 私钥与公钥按协议格式序列化后写入同一设备作用域，恢复时重新构建设备对象。
//!
//! DeviceStoreTrait implementation for WhatsApp storage
//!
//! Implements device persistence operations including:
//! - Device save/load
//! - Device existence check
//! - Device creation
//! - Database snapshots

#[cfg(feature = "whatsapp-web")]
use async_trait::async_trait;
#[cfg(feature = "whatsapp-web")]
use prost::Message;
#[cfg(feature = "whatsapp-web")]
use rusqlite::params;

#[cfg(feature = "whatsapp-web")]
use wa_rs_core::store::Device as CoreDevice;
#[cfg(feature = "whatsapp-web")]
use wa_rs_core::store::traits::DeviceStore as DeviceStoreTrait;

use super::RusqliteStore;

#[cfg(feature = "whatsapp-web")]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl DeviceStoreTrait for RusqliteStore {
    async fn save(&self, device: &CoreDevice) -> wa_rs_core::store::error::Result<()> {
        let conn = self.conn.lock();

        let noise_key = {
            let mut bytes = Vec::new();
            let priv_key = device.noise_key.private_key.serialize();
            bytes.extend_from_slice(priv_key.as_slice());
            bytes.extend_from_slice(device.noise_key.public_key.public_key_bytes());
            bytes
        };

        let identity_key = {
            let mut bytes = Vec::new();
            let priv_key = device.identity_key.private_key.serialize();
            bytes.extend_from_slice(priv_key.as_slice());
            bytes.extend_from_slice(device.identity_key.public_key.public_key_bytes());
            bytes
        };

        let signed_pre_key = {
            let mut bytes = Vec::new();
            let priv_key = device.signed_pre_key.private_key.serialize();
            bytes.extend_from_slice(priv_key.as_slice());
            bytes.extend_from_slice(device.signed_pre_key.public_key.public_key_bytes());
            bytes
        };

        let account = device.account.as_ref().map(|a| a.encode_to_vec());

        to_store_err!(execute: conn.execute(
            "INSERT OR REPLACE INTO device (
                id, lid, pn, registration_id, noise_key, identity_key,
                signed_pre_key, signed_pre_key_id, signed_pre_key_signature,
                adv_secret_key, account, push_name, app_version_primary,
                app_version_secondary, app_version_tertiary, app_version_last_fetched_ms,
                edge_routing_info, props_hash
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                self.device_id,
                device.lid.as_ref().map(|j| j.to_string()),
                device.pn.as_ref().map(|j| j.to_string()),
                device.registration_id,
                noise_key,
                identity_key,
                signed_pre_key,
                device.signed_pre_key_id,
                device.signed_pre_key_signature.to_vec(),
                device.adv_secret_key.to_vec(),
                account,
                &device.push_name,
                device.app_version_primary,
                device.app_version_secondary,
                device.app_version_tertiary,
                device.app_version_last_fetched_ms,
                device.edge_routing_info.as_ref().map(|v| v.clone()),
                device.props_hash.as_ref().map(|v| v.clone()),
            ],
        ))
    }

    async fn load(&self) -> wa_rs_core::store::error::Result<Option<CoreDevice>> {
        let conn = self.conn.lock();
        let result =
            conn.query_row("SELECT * FROM device WHERE id = ?1", params![self.device_id], |row| {
                fn to_rusqlite_err<E: std::error::Error + Send + Sync + 'static>(
                    e: E,
                ) -> rusqlite::Error {
                    rusqlite::Error::ToSqlConversionFailure(Box::new(e))
                }

                let noise_key_bytes: Vec<u8> = row.get("noise_key")?;
                let identity_key_bytes: Vec<u8> = row.get("identity_key")?;
                let signed_pre_key_bytes: Vec<u8> = row.get("signed_pre_key")?;

                if noise_key_bytes.len() != 64
                    || identity_key_bytes.len() != 64
                    || signed_pre_key_bytes.len() != 64
                {
                    return Err(rusqlite::Error::InvalidParameterName("key_pair".into()));
                }

                use wa_rs_core::libsignal::protocol::{KeyPair, PrivateKey, PublicKey};

                let noise_key = KeyPair::new(
                    PublicKey::from_djb_public_key_bytes(&noise_key_bytes[32..64])
                        .map_err(to_rusqlite_err)?,
                    PrivateKey::deserialize(&noise_key_bytes[0..32]).map_err(to_rusqlite_err)?,
                );

                let identity_key = KeyPair::new(
                    PublicKey::from_djb_public_key_bytes(&identity_key_bytes[32..64])
                        .map_err(to_rusqlite_err)?,
                    PrivateKey::deserialize(&identity_key_bytes[0..32]).map_err(to_rusqlite_err)?,
                );

                let signed_pre_key = KeyPair::new(
                    PublicKey::from_djb_public_key_bytes(&signed_pre_key_bytes[32..64])
                        .map_err(to_rusqlite_err)?,
                    PrivateKey::deserialize(&signed_pre_key_bytes[0..32])
                        .map_err(to_rusqlite_err)?,
                );

                let lid_str: Option<String> = row.get("lid")?;
                let pn_str: Option<String> = row.get("pn")?;
                let signature_bytes: Vec<u8> = row.get("signed_pre_key_signature")?;
                let adv_secret_bytes: Vec<u8> = row.get("adv_secret_key")?;
                let account_bytes: Option<Vec<u8>> = row.get("account")?;

                let mut signature = [0u8; 64];
                let mut adv_secret = [0u8; 32];
                signature.copy_from_slice(&signature_bytes);
                adv_secret.copy_from_slice(&adv_secret_bytes);

                let account = if let Some(bytes) = account_bytes {
                    Some(
                        wa_rs_proto::whatsapp::AdvSignedDeviceIdentity::decode(&*bytes)
                            .map_err(to_rusqlite_err)?,
                    )
                } else {
                    None
                };

                Ok(CoreDevice {
                    lid: lid_str.and_then(|s| s.parse().ok()),
                    pn: pn_str.and_then(|s| s.parse().ok()),
                    registration_id: row.get("registration_id")?,
                    noise_key,
                    identity_key,
                    signed_pre_key,
                    signed_pre_key_id: row.get("signed_pre_key_id")?,
                    signed_pre_key_signature: signature,
                    adv_secret_key: adv_secret,
                    account,
                    push_name: row.get("push_name")?,
                    app_version_primary: row.get("app_version_primary")?,
                    app_version_secondary: row.get("app_version_secondary")?,
                    app_version_tertiary: row.get("app_version_tertiary")?,
                    app_version_last_fetched_ms: row.get("app_version_last_fetched_ms")?,
                    edge_routing_info: row.get("edge_routing_info")?,
                    props_hash: row.get("props_hash")?,
                    ..Default::default()
                })
            });

        match result {
            Ok(device) => Ok(Some(device)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(wa_rs_core::store::error::StoreError::Database(e.to_string())),
        }
    }

    async fn exists(&self) -> wa_rs_core::store::error::Result<bool> {
        let conn = self.conn.lock();
        let count: i64 = to_store_err!(conn.query_row(
            "SELECT COUNT(*) FROM device WHERE id = ?1",
            params![self.device_id],
            |row| row.get(0),
        ))?;

        Ok(count > 0)
    }

    async fn create(&self) -> wa_rs_core::store::error::Result<i32> {
        Ok(self.device_id)
    }

    async fn snapshot_db(
        &self,
        name: &str,
        extra_content: Option<&[u8]>,
    ) -> wa_rs_core::store::error::Result<()> {
        let snapshot_path = format!("{}.snapshot.{}", self.db_path, name);

        to_store_err!(std::fs::copy(&self.db_path, &snapshot_path))?;

        if let Some(content) = extra_content {
            let content_path = format!("{}.extra", snapshot_path);
            to_store_err!(std::fs::write(&content_path, content))?;
        }

        Ok(())
    }
}
