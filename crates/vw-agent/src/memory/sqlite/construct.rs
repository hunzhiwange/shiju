//! SQLite 记忆后端的构造入口。
//!
//! 本文件负责根据工作区路径创建 `SqliteMemory`，初始化数据库目录、连接参数和 schema。
//! 嵌入模型由调用方注入；默认构造函数使用空嵌入器以保留纯关键词检索能力。

use super::SqliteMemory;
use crate::app::agent::memory::embeddings::{EmbeddingProvider, NoopEmbedding};
use crate::app::agent::memory::paths;
use std::path::Path;
use std::sync::Arc;

impl SqliteMemory {
    /// 使用默认嵌入器创建 SQLite 记忆后端。
    ///
    /// # 参数
    ///
    /// - `workspace_dir`: 工作区根目录，仅用于派生用户态数据目录。
    ///
    /// # 错误
    ///
    /// 当数据库目录创建、连接打开或 schema 初始化失败时返回错误。
    pub fn new(workspace_dir: &Path) -> anyhow::Result<Self> {
        Self::with_embedder(workspace_dir, Arc::new(NoopEmbedding), 0.7, 0.3, 10_000, None)
    }

    /// 使用指定嵌入器和检索权重创建 SQLite 记忆后端。
    ///
    /// # 参数
    ///
    /// - `workspace_dir`: 工作区根目录。
    /// - `embedder`: 文本嵌入提供者；维度为 0 时跳过向量检索。
    /// - `vector_weight`: 混合检索中的向量分数权重。
    /// - `keyword_weight`: 混合检索中的关键词分数权重。
    /// - `cache_max`: 嵌入缓存最大条数。
    /// - `open_timeout_secs`: 可选数据库打开超时，超过上限会被截断。
    ///
    /// # 错误
    ///
    /// 当文件系统准备、数据库打开或 schema 初始化失败时返回错误。
    pub fn with_embedder(
        workspace_dir: &Path,
        embedder: Arc<dyn EmbeddingProvider>,
        vector_weight: f32,
        keyword_weight: f32,
        cache_max: usize,
        open_timeout_secs: Option<u64>,
    ) -> anyhow::Result<Self> {
        let storage_dir = paths::project_data_dir(workspace_dir)?;
        let db_path = storage_dir.join("memory").join("brain.db");

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let conn = Self::open_connection(&db_path, open_timeout_secs)?;
            conn.execute_batch(
                "PRAGMA journal_mode = WAL;
                 PRAGMA synchronous  = NORMAL;
                 PRAGMA mmap_size    = 8388608;
                 PRAGMA cache_size   = -2000;
                 PRAGMA temp_store   = MEMORY;",
            )?;
            Self::init_schema(&conn)?;

            Ok(Self {
                conn: std::sync::Arc::new(parking_lot::Mutex::new(conn)),
                db_path,
                embedder,
                vector_weight,
                keyword_weight,
                cache_max,
            })
        }

        #[cfg(target_arch = "wasm32")]
        {
            // WASM 目标当前仅保留结构体配置，不打开本地 SQLite 连接。
            let _ = open_timeout_secs;
            Ok(Self { db_path, embedder, vector_weight, keyword_weight, cache_max })
        }
    }
}

#[cfg(test)]
#[path = "construct_tests.rs"]
mod construct_tests;
