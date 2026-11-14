//! SQLite 记忆后端的检索实现。
//!
//! 本文件提供 FTS5 关键词检索、内存内向量相似度检索，以及重建索引的维护入口。
//! 检索函数只返回 ID 和分数，完整记录组装由 `memory_impl` 负责。

use super::SqliteMemory;
use crate::memory::vector;
use rusqlite::{Connection, params};
use std::fmt::Write as _;

impl SqliteMemory {
    /// 使用 FTS5 按关键词检索候选记忆。
    ///
    /// # 参数
    ///
    /// - `conn`: SQLite 连接。
    /// - `query`: 原始查询文本，会按空白切分并组合为 FTS 查询。
    /// - `limit`: 最大候选条数。
    ///
    /// # 错误
    ///
    /// 当 FTS 查询准备、执行或结果读取失败时返回错误。
    pub(super) fn fts5_search(
        conn: &Connection,
        query: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<(String, f32)>> {
        let fts_query = query
            .split_whitespace()
            .map(|word| format!("\"{word}\""))
            .collect::<Vec<_>>()
            .join(" OR ");

        if fts_query.is_empty() {
            return Ok(Vec::new());
        }

        let sql = "SELECT m.id, bm25(memories_fts) as score
                   FROM memories_fts f
                   JOIN memories m ON m.rowid = f.rowid
                   WHERE memories_fts MATCH ?1
                   ORDER BY score
                   LIMIT ?2";

        let mut stmt = conn.prepare(sql)?;
        #[allow(clippy::cast_possible_wrap)]
        let limit_i64 = limit as i64;
        let rows = stmt.query_map(params![fts_query, limit_i64], |row| {
            let id: String = row.get(0)?;
            let score: f64 = row.get(1)?;
            // bm25 分数越小相关性越高，转成正向分数便于后续混合排序。
            #[allow(clippy::cast_possible_truncation)]
            Ok((id, (-score) as f32))
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// 使用余弦相似度按向量检索候选记忆。
    ///
    /// # 参数
    ///
    /// - `conn`: SQLite 连接。
    /// - `query_embedding`: 查询文本的嵌入向量。
    /// - `limit`: 最大候选条数。
    /// - `category`: 可选分类过滤。
    /// - `session_id`: 可选会话过滤。
    ///
    /// # 错误
    ///
    /// 当 SQL 准备、记录读取或向量反序列化相关操作失败时返回错误。
    pub(super) fn vector_search(
        conn: &Connection,
        query_embedding: &[f32],
        limit: usize,
        category: Option<&str>,
        session_id: Option<&str>,
    ) -> anyhow::Result<Vec<(String, f32)>> {
        let mut sql = "SELECT id, embedding FROM memories WHERE embedding IS NOT NULL".to_string();
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut idx = 1;

        if let Some(cat) = category {
            let _ = write!(sql, " AND category = ?{idx}");
            param_values.push(Box::new(cat.to_string()));
            idx += 1;
        }
        if let Some(sid) = session_id {
            let _ = write!(sql, " AND session_id = ?{idx}");
            param_values.push(Box::new(sid.to_string()));
        }

        let mut stmt = conn.prepare(&sql)?;
        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(AsRef::as_ref).collect();
        let rows = stmt.query_map(params_ref.as_slice(), |row| {
            let id: String = row.get(0)?;
            let blob: Vec<u8> = row.get(1)?;
            Ok((id, blob))
        })?;

        let mut scored = Vec::new();
        for row in rows {
            let (id, blob) = row?;
            let emb = vector::bytes_to_vec(&blob);
            let sim = vector::cosine_similarity(query_embedding, &emb);
            if sim > 0.0 {
                // 负相似度通常表示语义方向相反，保留它会污染混合排序。
                scored.push((id, sim));
            }
        }

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        Ok(scored)
    }

    /// 重建 FTS 索引，并为缺失向量的记录补齐嵌入。
    ///
    /// # 返回值
    ///
    /// 返回成功写入嵌入向量的记录数；嵌入器不可用时返回 0。
    ///
    /// # 错误
    ///
    /// 当 FTS 重建、记录读取、嵌入写入或阻塞任务调度失败时返回错误。
    #[allow(dead_code)]
    pub async fn reindex(&self) -> anyhow::Result<usize> {
        {
            let conn = self.conn.clone();
            tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
                let conn = conn.lock();
                conn.execute_batch("INSERT INTO memories_fts(memories_fts) VALUES('rebuild');")?;
                Ok(())
            })
            .await??;
        }

        if self.embedder.dimensions() == 0 {
            // 没有可用嵌入器时仍已完成 FTS 重建，向量补齐部分显式跳过。
            return Ok(0);
        }

        let conn = self.conn.clone();
        let entries: Vec<(String, String)> = tokio::task::spawn_blocking(move || {
            let conn = conn.lock();
            let mut stmt =
                conn.prepare("SELECT id, content FROM memories WHERE embedding IS NULL")?;
            let rows =
                stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;
            Ok::<_, anyhow::Error>(rows.filter_map(std::result::Result::ok).collect())
        })
        .await??;

        let mut count = 0;
        for (id, content) in &entries {
            if let Ok(Some(emb)) = self.get_or_compute_embedding(content).await {
                let bytes = vector::vec_to_bytes(&emb);
                let conn = self.conn.clone();
                let id = id.clone();
                tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
                    let conn = conn.lock();
                    conn.execute(
                        "UPDATE memories SET embedding = ?1 WHERE id = ?2",
                        params![bytes, id],
                    )?;
                    Ok(())
                })
                .await??;
                count += 1;
            }
        }

        Ok(count)
    }
}

#[cfg(test)]
#[path = "search_tests.rs"]
mod search_tests;
