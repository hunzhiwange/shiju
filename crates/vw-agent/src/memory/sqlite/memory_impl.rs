//! SQLite 记忆后端的 `Memory` trait 实现。
//!
//! 本文件承接高层记忆操作，并组合 FTS5 关键词检索、可选向量检索和会话过滤。
//! 所有 SQLite 访问都放入 `spawn_blocking`，避免同步数据库调用阻塞异步执行器。

use super::SqliteMemory;
use crate::memory::traits::{Memory, MemoryCategory, MemoryEntry};
use crate::memory::vector;
use async_trait::async_trait;
use chrono::Local;
use rusqlite::params;
use uuid::Uuid;

#[async_trait]
impl Memory for SqliteMemory {
    /// 返回该记忆后端的稳定名称。
    fn name(&self) -> &str {
        "sqlite"
    }

    /// 写入或更新一条记忆。
    ///
    /// # 参数
    ///
    /// - `key`: 记忆唯一键。
    /// - `content`: 记忆正文。
    /// - `category`: 记忆分类。
    /// - `session_id`: 可选会话范围。
    ///
    /// # 错误
    ///
    /// 当嵌入计算、SQLite 写入或阻塞任务调度失败时返回错误。
    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        let embedding_bytes =
            self.get_or_compute_embedding(content).await?.map(|emb| vector::vec_to_bytes(&emb));

        let conn = self.conn.clone();
        let key = key.to_string();
        let content = content.to_string();
        let sid = session_id.map(String::from);

        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let conn = conn.lock();
            let now = Local::now().to_rfc3339();
            let cat = Self::category_to_str(&category);
            let id = Uuid::new_v4().to_string();

            // key 是逻辑唯一键；冲突时更新内容与范围，保留“写入即最新”的语义。
            conn.execute(
                "INSERT INTO memories (id, key, content, category, embedding, created_at, updated_at, session_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(key) DO UPDATE SET
                    content = excluded.content,
                    category = excluded.category,
                    embedding = excluded.embedding,
                    updated_at = excluded.updated_at,
                    session_id = excluded.session_id",
                params![id, key, content, cat, embedding_bytes, now, now, sid],
            )?;
            Ok(())
        })
        .await?
    }

    /// 按查询文本召回相关记忆。
    ///
    /// # 参数
    ///
    /// - `query`: 用户查询文本；空白查询直接返回空结果。
    /// - `limit`: 最大返回条数。
    /// - `session_id`: 可选会话范围过滤。
    ///
    /// # 错误
    ///
    /// 当嵌入计算、SQLite 查询、结果转换或阻塞任务调度失败时返回错误。
    async fn recall(
        &self,
        query: &str,
        limit: usize,
        session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        let query_embedding = self.get_or_compute_embedding(query).await?;

        let conn = self.conn.clone();
        let query = query.to_string();
        let sid = session_id.map(String::from);
        let vector_weight = self.vector_weight;
        let keyword_weight = self.keyword_weight;

        tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<MemoryEntry>> {
            let conn = conn.lock();
            let session_ref = sid.as_deref();

            // 先扩大候选集再合并，给混合排序留出足够空间，最后统一截断到调用方限制。
            let keyword_results = Self::fts5_search(&conn, &query, limit * 2).unwrap_or_default();
            let vector_results = if let Some(ref qe) = query_embedding {
                Self::vector_search(&conn, qe, limit * 2, None, session_ref).unwrap_or_default()
            } else {
                Vec::new()
            };

            let merged = if vector_results.is_empty() {
                keyword_results
                    .iter()
                    .map(|(id, score)| vector::ScoredResult {
                        id: id.clone(),
                        vector_score: None,
                        keyword_score: Some(*score),
                        final_score: *score,
                    })
                    .collect::<Vec<_>>()
            } else {
                vector::hybrid_merge(
                    &vector_results,
                    &keyword_results,
                    vector_weight,
                    keyword_weight,
                    limit,
                )
            };

            let mut results = Vec::new();
            if !merged.is_empty() {
                let placeholders: String =
                    (1..=merged.len()).map(|i| format!("?{i}")).collect::<Vec<_>>().join(", ");
                let sql = format!(
                    "SELECT id, key, content, category, created_at, session_id \
                     FROM memories WHERE id IN ({placeholders})"
                );
                let mut stmt = conn.prepare(&sql)?;
                let id_params: Vec<Box<dyn rusqlite::types::ToSql>> = merged
                    .iter()
                    .map(|s| Box::new(s.id.clone()) as Box<dyn rusqlite::types::ToSql>)
                    .collect();
                let params_ref: Vec<&dyn rusqlite::types::ToSql> =
                    id_params.iter().map(AsRef::as_ref).collect();
                let rows = stmt.query_map(params_ref.as_slice(), |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                })?;

                let mut entry_map = std::collections::HashMap::new();
                for row in rows {
                    let (id, key, content, cat, ts, sid) = row?;
                    entry_map.insert(id, (key, content, cat, ts, sid));
                }

                // IN 查询不保证返回顺序，按 merged 顺序重建结果以保留评分排序。
                for scored in &merged {
                    if let Some((key, content, cat, ts, sid)) = entry_map.remove(&scored.id) {
                        let entry = MemoryEntry {
                            id: scored.id.clone(),
                            key,
                            content,
                            category: Self::str_to_category(&cat),
                            timestamp: ts,
                            session_id: sid,
                            score: Some(f64::from(scored.final_score)),
                        };
                        if let Some(filter_sid) = session_ref
                            && entry.session_id.as_deref() != Some(filter_sid)
                        {
                            continue;
                        }
                        results.push(entry);
                    }
                }
            }

            if results.is_empty() {
                const MAX_LIKE_KEYWORDS: usize = 8;
                // FTS5 可能因为查询语法或分词失败无结果；LIKE 兜底保持基础召回能力。
                let keywords: Vec<String> = query
                    .split_whitespace()
                    .take(MAX_LIKE_KEYWORDS)
                    .map(|word| format!("%{word}%"))
                    .collect();
                if !keywords.is_empty() {
                    let conditions: Vec<String> = keywords
                        .iter()
                        .enumerate()
                        .map(|(i, _)| {
                            format!("(content LIKE ?{} OR key LIKE ?{})", i * 2 + 1, i * 2 + 2)
                        })
                        .collect();
                    let where_clause = conditions.join(" OR ");
                    let sql = format!(
                        "SELECT id, key, content, category, created_at, session_id FROM memories
                         WHERE {where_clause}
                         ORDER BY updated_at DESC
                         LIMIT ?{}",
                        keywords.len() * 2 + 1
                    );
                    let mut stmt = conn.prepare(&sql)?;
                    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
                    for kw in &keywords {
                        param_values.push(Box::new(kw.clone()));
                        param_values.push(Box::new(kw.clone()));
                    }
                    #[allow(clippy::cast_possible_wrap)]
                    param_values.push(Box::new(limit as i64));
                    let params_ref: Vec<&dyn rusqlite::types::ToSql> =
                        param_values.iter().map(AsRef::as_ref).collect();
                    let rows = stmt.query_map(params_ref.as_slice(), |row| {
                        Ok(MemoryEntry {
                            id: row.get(0)?,
                            key: row.get(1)?,
                            content: row.get(2)?,
                            category: Self::str_to_category(&row.get::<_, String>(3)?),
                            timestamp: row.get(4)?,
                            session_id: row.get(5)?,
                            score: Some(1.0),
                        })
                    })?;
                    for row in rows {
                        let entry = row?;
                        if let Some(sid) = session_ref
                            && entry.session_id.as_deref() != Some(sid)
                        {
                            continue;
                        }
                        results.push(entry);
                    }
                }
            }

            results.truncate(limit);
            Ok(results)
        })
        .await?
    }

    /// 按唯一键读取单条记忆。
    ///
    /// # 错误
    ///
    /// 当 SQLite 查询、结果转换或阻塞任务调度失败时返回错误。
    async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        let conn = self.conn.clone();
        let key = key.to_string();

        tokio::task::spawn_blocking(move || -> anyhow::Result<Option<MemoryEntry>> {
            let conn = conn.lock();
            let mut stmt = conn.prepare(
                "SELECT id, key, content, category, created_at, session_id FROM memories WHERE key = ?1",
            )?;
            let mut rows = stmt.query_map(params![key], |row| {
                Ok(MemoryEntry {
                    id: row.get(0)?,
                    key: row.get(1)?,
                    content: row.get(2)?,
                    category: Self::str_to_category(&row.get::<_, String>(3)?),
                    timestamp: row.get(4)?,
                    session_id: row.get(5)?,
                    score: None,
                })
            })?;

            match rows.next() {
                Some(Ok(entry)) => Ok(Some(entry)),
                _ => Ok(None),
            }
        })
        .await?
    }

    /// 列出记忆，可按分类和会话过滤。
    ///
    /// # 参数
    ///
    /// - `category`: 可选分类过滤。
    /// - `session_id`: 可选会话过滤。
    ///
    /// # 错误
    ///
    /// 当 SQLite 查询、结果转换或阻塞任务调度失败时返回错误。
    async fn list(
        &self,
        category: Option<&MemoryCategory>,
        session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        const DEFAULT_LIST_LIMIT: i64 = 1000;

        let conn = self.conn.clone();
        let category = category.cloned();
        let sid = session_id.map(String::from);

        tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<MemoryEntry>> {
            let conn = conn.lock();
            let session_ref = sid.as_deref();
            let mut results = Vec::new();

            // 分类条件能交给 SQLite 索引处理；会话过滤保持在本地以兼容旧库和缺失会话的记录。
            let row_mapper = |row: &rusqlite::Row| -> rusqlite::Result<MemoryEntry> {
                Ok(MemoryEntry {
                    id: row.get(0)?,
                    key: row.get(1)?,
                    content: row.get(2)?,
                    category: Self::str_to_category(&row.get::<_, String>(3)?),
                    timestamp: row.get(4)?,
                    session_id: row.get(5)?,
                    score: None,
                })
            };

            if let Some(ref cat) = category {
                let cat_str = Self::category_to_str(cat);
                let mut stmt = conn.prepare(
                    "SELECT id, key, content, category, created_at, session_id FROM memories
                     WHERE category = ?1 ORDER BY updated_at DESC LIMIT ?2",
                )?;
                let rows = stmt.query_map(params![cat_str, DEFAULT_LIST_LIMIT], row_mapper)?;
                for row in rows {
                    let entry = row?;
                    if let Some(sid) = session_ref
                        && entry.session_id.as_deref() != Some(sid)
                    {
                        continue;
                    }
                    results.push(entry);
                }
            } else {
                let mut stmt = conn.prepare(
                    "SELECT id, key, content, category, created_at, session_id FROM memories
                     ORDER BY updated_at DESC LIMIT ?1",
                )?;
                let rows = stmt.query_map(params![DEFAULT_LIST_LIMIT], row_mapper)?;
                for row in rows {
                    let entry = row?;
                    if let Some(sid) = session_ref
                        && entry.session_id.as_deref() != Some(sid)
                    {
                        continue;
                    }
                    results.push(entry);
                }
            }

            Ok(results)
        })
        .await?
    }

    /// 删除指定键对应的记忆，并返回是否实际删除了记录。
    ///
    /// # 错误
    ///
    /// 当 SQLite 删除或阻塞任务调度失败时返回错误。
    async fn forget(&self, key: &str) -> anyhow::Result<bool> {
        let conn = self.conn.clone();
        let key = key.to_string();

        tokio::task::spawn_blocking(move || -> anyhow::Result<bool> {
            let conn = conn.lock();
            let affected = conn.execute("DELETE FROM memories WHERE key = ?1", params![key])?;
            Ok(affected > 0)
        })
        .await?
    }

    /// 统计 SQLite 记忆表中的记录数。
    ///
    /// # 错误
    ///
    /// 当 SQLite 查询或阻塞任务调度失败时返回错误。
    async fn count(&self) -> anyhow::Result<usize> {
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || -> anyhow::Result<usize> {
            let conn = conn.lock();
            let count: i64 =
                conn.query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))?;
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            Ok(count as usize)
        })
        .await?
    }

    /// 执行轻量健康检查。
    ///
    /// 返回 `false` 表示连接不可用、查询失败或阻塞任务未能完成。
    async fn health_check(&self) -> bool {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || conn.lock().execute_batch("SELECT 1").is_ok())
            .await
            .unwrap_or(false)
    }
}

#[cfg(test)]
#[path = "memory_impl_tests.rs"]
mod memory_impl_tests;
