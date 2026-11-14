//! SQLite 记忆后端的嵌入缓存。
//!
//! 本文件负责复用已计算的文本向量，减少重复调用嵌入模型的成本。缓存按内容哈希寻址，
//! 并通过访问时间做简单的容量淘汰。

use super::SqliteMemory;
use crate::memory::vector;
use chrono::Local;
use rusqlite::params;

impl SqliteMemory {
    /// 从缓存读取文本嵌入，未命中时计算并写回缓存。
    ///
    /// # 参数
    ///
    /// - `text`: 需要向量化的文本。
    ///
    /// # 返回值
    ///
    /// 嵌入器维度为 0 时返回 `Ok(None)`；否则返回缓存或新计算的向量。
    ///
    /// # 错误
    ///
    /// 当缓存查询、嵌入计算、缓存写入或阻塞任务调度失败时返回错误。
    pub(super) async fn get_or_compute_embedding(
        &self,
        text: &str,
    ) -> anyhow::Result<Option<Vec<f32>>> {
        if self.embedder.dimensions() == 0 {
            // NoopEmbedding 使用 0 维作为显式哨兵，表示当前后端只启用关键词检索。
            return Ok(None);
        }

        let hash = Self::content_hash(text);
        let now = Local::now().to_rfc3339();

        let conn = self.conn.clone();
        let hash_c = hash.clone();
        let now_c = now.clone();
        let cached = tokio::task::spawn_blocking(move || -> anyhow::Result<Option<Vec<f32>>> {
            let conn = conn.lock();
            let mut stmt =
                conn.prepare("SELECT embedding FROM embedding_cache WHERE content_hash = ?1")?;
            let blob: Option<Vec<u8>> = stmt.query_row(params![hash_c], |row| row.get(0)).ok();
            if let Some(bytes) = blob {
                // 命中后刷新访问时间，让容量淘汰更接近 LRU 行为。
                conn.execute(
                    "UPDATE embedding_cache SET accessed_at = ?1 WHERE content_hash = ?2",
                    params![now_c, hash_c],
                )?;
                return Ok(Some(vector::bytes_to_vec(&bytes)));
            }
            Ok(None)
        })
        .await??;

        if cached.is_some() {
            return Ok(cached);
        }

        let embedding = self.embedder.embed_one(text).await?;
        let bytes = vector::vec_to_bytes(&embedding);

        let conn = self.conn.clone();
        #[allow(clippy::cast_possible_wrap)]
        let cache_max = self.cache_max as i64;
        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let conn = conn.lock();
            conn.execute(
                "INSERT OR REPLACE INTO embedding_cache (content_hash, embedding, created_at, accessed_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![hash, bytes, now, now],
            )?;
            conn.execute(
                "DELETE FROM embedding_cache WHERE content_hash IN (
                    SELECT content_hash FROM embedding_cache
                    ORDER BY accessed_at ASC
                    LIMIT MAX(0, (SELECT COUNT(*) FROM embedding_cache) - ?1)
                )",
                params![cache_max],
            )?;
            Ok(())
        })
        .await??;

        Ok(Some(embedding))
    }
}

#[cfg(test)]
#[path = "embedding_cache_tests.rs"]
mod embedding_cache_tests;
