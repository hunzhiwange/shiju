//! SQLite 持久化记忆存储模块。
//!
//! 保留 `memory::sqlite::SqliteMemory` 作为外部入口，并将构造、建表、
//! 嵌入缓存、检索和 trait 实现拆分到同目录私有文件，避免单文件继续膨胀。

mod construct;
#[cfg(not(target_arch = "wasm32"))]
mod db;
#[cfg(not(target_arch = "wasm32"))]
mod embedding_cache;
#[cfg(not(target_arch = "wasm32"))]
mod helpers;
#[cfg(not(target_arch = "wasm32"))]
mod memory_impl;
#[cfg(not(target_arch = "wasm32"))]
mod search;
mod wasm;

use super::embeddings::EmbeddingProvider;
#[cfg(not(target_arch = "wasm32"))]
use parking_lot::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Arc;

/// SQLite 数据库打开操作的最大允许超时时间（秒）。
const SQLITE_OPEN_TIMEOUT_CAP_SECS: u64 = 300;

/// 基于 SQLite 的持久化记忆存储。
pub struct SqliteMemory {
    #[cfg(not(target_arch = "wasm32"))]
    conn: Arc<Mutex<Connection>>,
    db_path: PathBuf,
    embedder: Arc<dyn EmbeddingProvider>,
    vector_weight: f32,
    keyword_weight: f32,
    cache_max: usize,
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
