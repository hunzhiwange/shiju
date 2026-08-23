//! 基于 rusqlite 的 wa-rs 存储后端，统一实现 WhatsApp Web 所需存储接口。
//!
//! 后端直接复用项目的 rusqlite 版本，避免引入 wa-rs 默认存储的 SQLite 依赖冲突。
//!
//! Custom wa-rs storage backend using VibeWindow's rusqlite
//!
//! This module implements all 4 wa-rs storage traits using rusqlite directly,
//! avoiding the Diesel/libsqlite3-sys dependency conflict from wa-rs-sqlite-storage.
//!
//! # Traits Implemented
//!
//! - [`SignalStore`]: Signal protocol cryptographic operations
//! - [`AppSyncStore`]: WhatsApp app state synchronization
//! - [`ProtocolStore`]: WhatsApp Web protocol alignment
//! - [`DeviceStore`]: Device persistence operations

/// Helper macro to convert rusqlite errors to StoreError
macro_rules! to_store_err {
    (execute: $expr:expr) => {
        $expr.map(|_| ()).map_err(|e| wa_rs_core::store::error::StoreError::Database(e.to_string()))
    };
    ($expr:expr) => {
        $expr.map_err(|e| wa_rs_core::store::error::StoreError::Database(e.to_string()))
    };
}

#[cfg(feature = "whatsapp-web")]
mod app_sync_store;
#[cfg(feature = "whatsapp-web")]
mod device_store;
#[cfg(feature = "whatsapp-web")]
mod protocol_store;
#[cfg(feature = "whatsapp-web")]
mod schema;
#[cfg(feature = "whatsapp-web")]
mod signal_store;

#[cfg(test)]
#[path = "app_sync_store_tests.rs"]
mod app_sync_store_tests;
#[cfg(test)]
#[path = "device_store_tests.rs"]
mod device_store_tests;
#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "protocol_store_tests.rs"]
mod protocol_store_tests;
#[cfg(test)]
#[path = "schema_tests.rs"]
mod schema_tests;
#[cfg(test)]
#[path = "signal_store_tests.rs"]
mod signal_store_tests;

#[cfg(feature = "whatsapp-web")]
use parking_lot::Mutex;
#[cfg(feature = "whatsapp-web")]
use rusqlite::Connection;
#[cfg(feature = "whatsapp-web")]
use std::path::Path;
#[cfg(feature = "whatsapp-web")]
use std::sync::Arc;

/// 使用 rusqlite 实现 wa-rs 四类存储 trait 的共享后端。
///
/// This implements all 4 storage traits required by wa-rs.
/// The backend uses VibeWindow's existing rusqlite setup, avoiding the
/// Diesel/libsqlite3-sys conflict from wa-rs-sqlite-storage.
#[cfg(feature = "whatsapp-web")]
#[derive(Clone)]
pub struct RusqliteStore {
    /// Database file path
    db_path: String,
    /// SQLite connection (thread-safe via Mutex)
    conn: Arc<Mutex<Connection>>,
    /// Device ID for this session
    device_id: i32,
}

#[cfg(feature = "whatsapp-web")]
impl RusqliteStore {
    /// 打开或创建会话数据库，初始化 WAL 参数和协议表结构。
    ///
    /// # Arguments
    ///
    /// * `db_path` - Path to the SQLite database file (will be created if needed)
    pub fn new<P: AsRef<Path>>(db_path: P) -> anyhow::Result<Self> {
        let db_path = db_path.as_ref().to_string_lossy().to_string();

        if let Some(parent) =
            Path::new(&db_path).parent().filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;

        to_store_err!(conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;",
        ))?;

        let store = Self { db_path, conn: Arc::new(Mutex::new(conn)), device_id: 1 };

        store.init_schema()?;

        Ok(store)
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
