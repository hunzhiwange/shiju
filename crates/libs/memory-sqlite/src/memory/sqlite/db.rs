//! SQLite 连接打开与 schema 初始化。
//!
//! 本文件集中维护数据库连接超时、性能 PRAGMA、表结构、索引、FTS 触发器和轻量迁移。
//! 这样构造逻辑可以保持简洁，存储实现也不需要关心 schema 细节。

use super::{SQLITE_OPEN_TIMEOUT_CAP_SECS, SqliteMemory};
use anyhow::Context;
use rusqlite::Connection;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

impl SqliteMemory {
    /// 打开 SQLite 连接，并按需应用打开超时。
    ///
    /// # 参数
    ///
    /// - `db_path`: SQLite 数据库文件路径。
    /// - `open_timeout_secs`: 可选打开超时；超过模块上限时会被截断。
    ///
    /// # 错误
    ///
    /// 当 SQLite 打开失败、后台打开线程退出或等待超时时返回错误。
    pub(super) fn open_connection(
        db_path: &Path,
        open_timeout_secs: Option<u64>,
    ) -> anyhow::Result<Connection> {
        let path_buf = db_path.to_path_buf();

        let conn = if let Some(secs) = open_timeout_secs {
            let capped = secs.min(SQLITE_OPEN_TIMEOUT_CAP_SECS);
            let (tx, rx) = mpsc::channel();

            // Connection::open 是同步调用，用独立线程包住后才能给打开阶段施加外部超时。
            thread::spawn(move || {
                let result = Connection::open(&path_buf);
                let _ = tx.send(result);
            });

            match rx.recv_timeout(Duration::from_secs(capped)) {
                Ok(Ok(conn)) => conn,
                Ok(Err(err)) => return Err(err).context("SQLite 无法打开数据库"),
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    anyhow::bail!("SQLite 连接在 {} 秒后超时", capped);
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    anyhow::bail!("SQLite 打开线程意外退出");
                }
            }
        } else {
            Connection::open(&path_buf).context("SQLite 无法打开数据库")?
        };

        Ok(conn)
    }

    /// 初始化或迁移 SQLite 记忆数据库结构。
    ///
    /// # 参数
    ///
    /// - `conn`: 已打开的 SQLite 连接。
    ///
    /// # 错误
    ///
    /// 当建表、建索引、触发器创建或迁移语句执行失败时返回错误。
    pub(super) fn init_schema(conn: &Connection) -> anyhow::Result<()> {
        conn.execute_batch(
            "-- 核心记忆表
            CREATE TABLE IF NOT EXISTS memories (
                id          TEXT PRIMARY KEY,
                key         TEXT NOT NULL UNIQUE,
                content     TEXT NOT NULL,
                category    TEXT NOT NULL DEFAULT 'core',
                embedding   BLOB,
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_memories_category ON memories(category);
            CREATE INDEX IF NOT EXISTS idx_memories_key ON memories(key);

            CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
                key, content, content=memories, content_rowid=rowid
            );

            CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
                INSERT INTO memories_fts(rowid, key, content)
                VALUES (new.rowid, new.key, new.content);
            END;
            CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON memories BEGIN
                INSERT INTO memories_fts(memories_fts, rowid, key, content)
                VALUES ('delete', old.rowid, old.key, old.content);
            END;
            CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON memories BEGIN
                INSERT INTO memories_fts(memories_fts, rowid, key, content)
                VALUES ('delete', old.rowid, old.key, old.content);
                INSERT INTO memories_fts(rowid, key, content)
                VALUES (new.rowid, new.key, new.content);
            END;

            CREATE TABLE IF NOT EXISTS embedding_cache (
                content_hash TEXT PRIMARY KEY,
                embedding    BLOB NOT NULL,
                created_at   TEXT NOT NULL,
                accessed_at  TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_cache_accessed ON embedding_cache(accessed_at);",
        )?;

        let has_session_id: bool = conn
            .prepare("SELECT sql FROM sqlite_master WHERE type='table' AND name='memories'")?
            .query_row([], |row| row.get::<_, String>(0))?
            .contains("session_id");
        if !has_session_id {
            // 旧数据库可能缺少 session_id；这里做幂等迁移，避免强制用户重建记忆库。
            conn.execute_batch(
                "ALTER TABLE memories ADD COLUMN session_id TEXT;
                 CREATE INDEX IF NOT EXISTS idx_memories_session ON memories(session_id);",
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "db_tests.rs"]
mod db_tests;
