//! 响应缓存模块 —— 避免在重复提示上浪费令牌。
//!
//! # 功能概述
//!
//! 本模块提供了一个基于 SQLite 的 LLM 响应缓存系统，用于存储和复用大语言模型（LLM）的响应，
//! 从而减少重复请求的令牌消耗和响应延迟。
//!
//! # 核心机制
//!
//! - **缓存键生成**：基于 `(model, system_prompt_hash, user_prompt)` 的 SHA-256 哈希值作为缓存键
//! - **自动过期**：缓存条目在可配置的 TTL（生存时间）后自动失效，默认为 1 小时
//! - **独立存储**：使用独立的 `response_cache.db` 数据库文件，与 `brain.db` 内存记忆分离，
//!   可以独立清理而不影响记忆数据
//! - **容量管理**：支持最大条目数限制，采用 LRU（最近最少使用）策略进行驱逐
//! - **可选启用**：缓存默认禁用，用户需通过 `[memory] response_cache_enabled = true` 显式启用
//!
//! # 使用场景
//!
//! - 减少相同或相似查询的 API 调用成本
//! - 加快常见请求的响应速度
//! - 在离线或网络受限环境下提供有限的响应能力
//!
//! # 示例
//!
//! ```ignore
//! use std::path::Path;
//! use vibe_agent::memory::response_cache::ResponseCache;
//!
//! // 创建缓存实例（TTL 60 分钟，最大 1000 条记录）
//! let cache = ResponseCache::new(Path::new("./workspace"), 60, 1000)?;
//!
//! // 生成缓存键
//! let key = ResponseCache::cache_key("gpt-4", Some("You are helpful"), "What is Rust?");
//!
//! // 尝试从缓存获取响应
//! if let Some(response) = cache.get(&key)? {
//!     println!("缓存命中: {}", response);
//! } else {
//!     // 执行实际的 LLM 调用，然后存入缓存
//!     let response = call_llm(...);
//!     cache.put(&key, "gpt-4", &response, 150)?;
//! }
//! ```

use super::paths;
use anyhow::Result;
use chrono::{Duration, Local};
use parking_lot::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// 基于 SQLite 的响应缓存存储结构。
///
/// 该结构体管理一个专用的 SQLite 数据库，用于缓存 LLM 响应以减少令牌消耗和响应延迟。
/// 缓存数据存储在 `response_cache.db` 文件中，与主记忆数据库 `brain.db` 分离，
/// 允许独立管理和清理。
///
/// # 线程安全
///
/// 内部使用 `parking_lot::Mutex` 保护数据库连接，确保多线程环境下的安全访问。
///
/// # 字段说明
///
/// - `conn`: SQLite 数据库连接（WASM 目标不可用）
/// - `db_path`: 数据库文件路径，用于调试和日志记录
/// - `ttl_minutes`: 缓存条目的生存时间（分钟），过期条目将被自动清理
/// - `max_entries`: 最大缓存条目数，超出时采用 LRU 策略驱逐旧条目
///
/// # 示例
///
/// ```ignore
/// let cache = ResponseCache::new(
///     Path::new("./workspace"),  // 工作区目录
///     60,                         // TTL: 60 分钟
///     1000                        // 最多 1000 条缓存
/// )?;
/// ```
pub struct ResponseCache {
    /// SQLite 数据库连接，使用 Mutex 保护以支持多线程访问
    /// 注意：在 wasm32 目标架构下不可用
    #[cfg(not(target_arch = "wasm32"))]
    conn: Mutex<Connection>,

    /// 数据库文件的完整路径
    /// 用于调试、日志记录和路径显示
    #[allow(dead_code)]
    db_path: PathBuf,

    /// 缓存条目的生存时间（分钟）
    /// 超过此时间的条目将被视为过期，在查询时被忽略并在写入时被清理
    ttl_minutes: i64,

    /// 缓存的最大条目数
    /// 当条目数超过此限制时，使用 LRU 策略删除最少访问的条目
    max_entries: usize,
}

impl ResponseCache {
    /// 创建或打开响应缓存数据库实例。
    ///
    /// 该方法会在用户态项目数据目录下创建 `memory/response_cache.db` 文件（如果不存在），
    /// 并初始化必要的数据库表结构和索引。
    ///
    /// # 参数
    ///
    /// - `workspace_dir`: 工作区根目录路径，仅用于派生用户态项目数据目录
    /// - `ttl_minutes`: 缓存条目的生存时间（分钟），过期条目将被自动清理
    /// - `max_entries`: 缓存的最大条目数，超出时使用 LRU 策略驱逐旧条目
    ///
    /// # 返回值
    ///
    /// - `Ok(ResponseCache)`: 成功创建或打开的缓存实例
    /// - `Err(anyhow::Error)`: 数据库创建或初始化失败
    ///
    /// # 数据库配置
    ///
    /// 使用以下 SQLite PRAGMA 设置以优化性能：
    /// - `journal_mode = WAL`: 使用预写日志模式，提高并发性能
    /// - `synchronous = NORMAL`: 平衡安全性和性能
    /// - `temp_store = MEMORY`: 临时表存储在内存中
    ///
    /// # 数据库表结构
    ///
    /// 创建 `response_cache` 表，包含以下字段：
    /// - `prompt_hash`: 提示哈希值（主键）
    /// - `model`: 模型名称
    /// - `response`: 缓存的响应文本
    /// - `token_count`: 令牌数量
    /// - `created_at`: 创建时间
    /// - `accessed_at`: 最后访问时间
    /// - `hit_count`: 命中次数
    ///
    /// # 平台差异
    ///
    /// - 非 WASM 目标：创建完整的 SQLite 数据库实例
    /// - WASM 目标：创建空实例（缓存功能不可用，所有操作返回空结果）
    ///
    /// # 错误
    ///
    /// - 目录创建失败
    /// - 数据库连接失败
    /// - SQL 执行失败
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use std::path::Path;
    ///
    /// // 创建缓存实例
    /// let cache = ResponseCache::new(
    ///     Path::new("./workspace"),
    ///     60,    // 60 分钟 TTL
    ///     1000   // 最多 1000 条记录
    /// )?;
    /// ```
    pub fn new(workspace_dir: &Path, ttl_minutes: u32, max_entries: usize) -> Result<Self> {
        let storage_dir = paths::workspace_data_dir(workspace_dir)?;
        let db_dir = storage_dir.join("memory");

        // 在非 WASM 目标上创建目录（如果不存在）
        #[cfg(not(target_arch = "wasm32"))]
        std::fs::create_dir_all(&db_dir)?;

        let db_path = db_dir.join("response_cache.db");

        // 非 WASM 目标的完整实现
        #[cfg(not(target_arch = "wasm32"))]
        {
            // 打开或创建 SQLite 数据库文件
            let conn = Connection::open(&db_path)?;

            // 设置 SQLite PRAGMA 选项以优化性能
            // - WAL 模式：提高并发读写性能
            // - NORMAL 同步：平衡数据安全性和写入性能
            // - 内存临时存储：加快临时表操作
            conn.execute_batch(
                "PRAGMA journal_mode = WAL;
                 PRAGMA synchronous  = NORMAL;
                 PRAGMA temp_store   = MEMORY;",
            )?;

            // 创建缓存表和索引
            // - 主键：prompt_hash（基于提示内容的 SHA-256 哈希）
            // - 索引：accessed_at 和 created_at 用于过期清理和 LRU 驱逐
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS response_cache (
                    prompt_hash TEXT PRIMARY KEY,
                    model       TEXT NOT NULL,
                    response    TEXT NOT NULL,
                    token_count INTEGER NOT NULL DEFAULT 0,
                    created_at  TEXT NOT NULL,
                    accessed_at TEXT NOT NULL,
                    hit_count   INTEGER NOT NULL DEFAULT 0
                );
                CREATE INDEX IF NOT EXISTS idx_rc_accessed ON response_cache(accessed_at);
                CREATE INDEX IF NOT EXISTS idx_rc_created ON response_cache(created_at);",
            )?;

            // 返回初始化完成的缓存实例
            Ok(Self {
                conn: Mutex::new(conn),
                db_path,
                ttl_minutes: i64::from(ttl_minutes),
                max_entries,
            })
        }

        // WASM 目标的空实现（缓存功能不可用）
        #[cfg(target_arch = "wasm32")]
        {
            Ok(Self { db_path, ttl_minutes: i64::from(ttl_minutes), max_entries })
        }
    }

    /// 根据模型和提示内容生成确定性的缓存键。
    ///
    /// 该方法使用 SHA-256 哈希算法，基于模型名称、系统提示和用户提示生成唯一的缓存键。
    /// 相同的输入组合总是产生相同的缓存键，确保缓存的一致性和可重复性。
    ///
    /// # 参数
    ///
    /// - `model`: 模型标识符（如 "gpt-4", "claude-3-opus" 等）
    /// - `system_prompt`: 可选的系统提示内容，定义 AI 的角色和行为
    /// - `user_prompt`: 用户的实际查询或指令
    ///
    /// # 返回值
    ///
    /// 返回 64 个十六进制字符组成的字符串（256 位哈希值的十六进制表示）
    ///
    /// # 哈希计算逻辑
    ///
    /// 哈希输入格式：`{model}|{system_prompt}|{user_prompt}`
    /// - 各部分用 `|` 分隔符连接
    /// - 系统提示为 `None` 时，该部分为空字符串
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let key1 = ResponseCache::cache_key(
    ///     "gpt-4",
    ///     Some("You are a helpful assistant"),
    ///     "What is Rust?"
    /// );
    /// // key1 = "a1b2c3d4e5..." (64个字符的十六进制字符串)
    ///
    /// let key2 = ResponseCache::cache_key(
    ///     "gpt-4",
    ///     None,
    ///     "What is Rust?"
    /// );
    /// // key2 与 key1 不同，因为系统提示不同
    /// ```
    pub fn cache_key(model: &str, system_prompt: Option<&str>, user_prompt: &str) -> String {
        // 创建 SHA-256 哈希器
        let mut hasher = Sha256::new();

        // 添加模型名称到哈希输入
        hasher.update(model.as_bytes());
        hasher.update(b"|"); // 分隔符

        // 添加系统提示（如果存在）
        if let Some(sys) = system_prompt {
            hasher.update(sys.as_bytes());
        }
        hasher.update(b"|"); // 分隔符

        // 添加用户提示
        hasher.update(user_prompt.as_bytes());

        // 计算最终哈希值并格式化为十六进制字符串
        let hash = hasher.finalize();
        hex::encode(hash)
    }

    /// 从缓存中查找响应。
    ///
    /// 根据缓存键查找匹配的响应。如果找到且未过期，则返回响应内容；
    /// 否则返回 `None`。命中时会自动更新访问时间和命中计数。
    ///
    /// # 参数
    ///
    /// - `key`: 由 `cache_key()` 方法生成的缓存键
    ///
    /// # 返回值
    ///
    /// - `Ok(Some(String))`: 缓存命中，返回响应文本
    /// - `Ok(None)`: 缓存未命中或条目已过期
    /// - `Err(anyhow::Error)`: 数据库查询错误
    ///
    /// # 过期检查
    ///
    /// 只返回 `created_at` 时间戳晚于 `当前时间 - TTL` 的条目。
    /// 过期条目会被自动忽略，并在下次写入操作时被清理。
    ///
    /// # 命中统计
    ///
    /// 缓存命中时会自动：
    /// - 更新 `accessed_at` 为当前时间
    /// - 增加 `hit_count` 计数器
    ///
    /// # 平台差异
    ///
    /// - 非 WASM 目标：执行实际的数据库查询
    /// - WASM 目标：始终返回 `Ok(None)`
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let key = ResponseCache::cache_key("gpt-4", None, "Hello");
    /// match cache.get(&key)? {
    ///     Some(response) => {
    ///         println!("缓存命中: {}", response);
    ///     }
    ///     None => {
    ///         println!("缓存未命中，需要调用 LLM");
    ///     }
    /// }
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub fn get(&self, key: &str) -> Result<Option<String>> {
        // 获取数据库连接的互斥锁
        let conn = self.conn.lock();

        // 计算过期截止时间：当前时间 - TTL
        let now = Local::now();
        let cutoff = (now - Duration::minutes(self.ttl_minutes)).to_rfc3339();

        // 查询未过期的缓存条目
        // SQL: 只返回 prompt_hash 匹配且 created_at 晚于截止时间的记录
        let mut stmt = conn.prepare(
            "SELECT response FROM response_cache
             WHERE prompt_hash = ?1 AND created_at > ?2",
        )?;

        // 执行查询，获取响应文本
        let result: Option<String> = stmt.query_row(params![key, cutoff], |row| row.get(0)).ok();

        // 如果缓存命中，更新访问统计信息
        if result.is_some() {
            // 更新 accessed_at 为当前时间，并增加 hit_count
            // 这有助于 LRU 驱逐策略识别热门条目
            let now_str = now.to_rfc3339();
            conn.execute(
                "UPDATE response_cache
                 SET accessed_at = ?1, hit_count = hit_count + 1
                 WHERE prompt_hash = ?2",
                params![now_str, key],
            )?;
        }

        Ok(result)
    }

    /// WASM 目标平台的空实现 —— 缓存功能不可用。
    ///
    /// 始终返回 `Ok(None)`，表示缓存未命中。
    #[cfg(target_arch = "wasm32")]
    pub fn get(&self, _key: &str) -> Result<Option<String>> {
        Ok(None)
    }

    /// 将响应存入缓存。
    ///
    /// 将 LLM 响应及其元数据存储到缓存数据库中。如果相同键的条目已存在，则替换旧条目。
    /// 写入后会自动执行过期清理和 LRU 驱逐以维护缓存容量。
    ///
    /// # 参数
    ///
    /// - `key`: 由 `cache_key()` 方法生成的缓存键
    /// - `model`: 生成响应的模型名称（用于统计和调试）
    /// - `response`: 要缓存的 LLM 响应文本
    /// - `token_count`: 响应消耗的令牌数量（用于计算节省的令牌数）
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 缓存写入成功
    /// - `Err(anyhow::Error)`: 数据库操作错误
    ///
    /// # 自动清理机制
    ///
    /// 写入操作后会自动执行以下清理：
    ///
    /// 1. **过期清理**：删除所有 `created_at` 早于 `当前时间 - TTL` 的条目
    /// 2. **LRU 驱逐**：如果条目数超过 `max_entries`，删除最少访问的条目
    ///
    /// # 数据库操作
    ///
    /// 使用 `INSERT OR REPLACE` 语义，确保相同键的重复写入不会产生冲突。
    /// 新条目的 `hit_count` 初始化为 0，`created_at` 和 `accessed_at` 设置为当前时间。
    ///
    /// # 平台差异
    ///
    /// - 非 WASM 目标：执行实际的数据库写入和清理
    /// - WASM 目标：无操作，直接返回成功
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let key = ResponseCache::cache_key("gpt-4", None, "Hello");
    /// let response = "Hello! How can I help you?";
    /// cache.put(&key, "gpt-4", response, 10)?;
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub fn put(&self, key: &str, model: &str, response: &str, token_count: u32) -> Result<()> {
        // 获取数据库连接的互斥锁
        let conn = self.conn.lock();

        // 获取当前时间戳（RFC 3339 格式）
        let now = Local::now().to_rfc3339();

        // 插入或替换缓存条目
        // 使用 INSERT OR REPLACE 确保相同键的重复写入不产生冲突
        conn.execute(
            "INSERT OR REPLACE INTO response_cache
             (prompt_hash, model, response, token_count, created_at, accessed_at, hit_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
            params![key, model, response, token_count, now, now],
        )?;

        // 步骤 1：清理过期条目
        // 删除所有 created_at 早于 (当前时间 - TTL) 的记录
        let cutoff = (Local::now() - Duration::minutes(self.ttl_minutes)).to_rfc3339();
        conn.execute("DELETE FROM response_cache WHERE created_at <= ?1", params![cutoff])?;

        // 步骤 2：LRU 驱逐 —— 如果超过最大条目数限制
        // 删除最少访问的条目，直到条目数不超过 max_entries
        // 使用 accessed_at 排序确定 LRU 顺序
        #[allow(clippy::cast_possible_wrap)]
        let max = self.max_entries as i64;
        conn.execute(
            "DELETE FROM response_cache WHERE prompt_hash IN (
                SELECT prompt_hash FROM response_cache
                ORDER BY accessed_at ASC
                LIMIT MAX(0, (SELECT COUNT(*) FROM response_cache) - ?1)
            )",
            params![max],
        )?;

        Ok(())
    }

    /// WASM 目标平台的空实现 —— 缓存功能不可用。
    ///
    /// 不执行任何操作，直接返回成功。
    #[cfg(target_arch = "wasm32")]
    pub fn put(&self, _key: &str, _model: &str, _response: &str, _token_count: u32) -> Result<()> {
        Ok(())
    }

    /// 获取缓存统计信息。
    ///
    /// 返回缓存的整体使用统计，包括总条目数、总命中次数和节省的令牌总数。
    /// 这些统计数据可用于监控缓存效果和优化缓存配置。
    ///
    /// # 返回值
    ///
    /// 返回元组 `(total_entries, total_hits, total_tokens_saved)`：
    ///
    /// - `total_entries`: 当前缓存中的总条目数
    /// - `total_hits`: 所有条目的命中次数总和
    /// - `total_tokens_saved`: 节省的令牌总数（`token_count * hit_count` 的总和）
    ///
    /// # 数据库查询
    ///
    /// 执行三个聚合查询：
    /// 1. `COUNT(*)`: 计算总条目数
    /// 2. `SUM(hit_count)`: 计算总命中次数
    /// 3. `SUM(token_count * hit_count)`: 计算节省的令牌总数
    ///
    /// # 平台差异
    ///
    /// - 非 WASM 目标：执行实际的数据库查询
    /// - WASM 目标：返回 `(0, 0, 0)`
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let (entries, hits, tokens_saved) = cache.stats()?;
    /// println!("缓存条目: {}", entries);
    /// println!("总命中次数: {}", hits);
    /// println!("节省令牌: {}", tokens_saved);
    ///
    /// // 计算命中率（需要跟踪总请求数）
    /// let total_requests = 1000;
    /// let hit_rate = hits as f64 / total_requests as f64;
    /// println!("命中率: {:.2}%", hit_rate * 100.0);
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub fn stats(&self) -> Result<(usize, u64, u64)> {
        // 获取数据库连接的互斥锁
        let conn = self.conn.lock();

        // 查询 1：获取总条目数
        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM response_cache", [], |row| row.get(0))?;

        // 查询 2：获取总命中次数
        // 使用 COALESCE 处理空表情况，返回 0 而不是 NULL
        let hits: i64 =
            conn.query_row("SELECT COALESCE(SUM(hit_count), 0) FROM response_cache", [], |row| {
                row.get(0)
            })?;

        // 查询 3：计算节省的令牌总数
        // 每个条目节省的令牌数 = token_count * hit_count
        // 使用 COALESCE 处理空表情况
        let tokens_saved: i64 = conn.query_row(
            "SELECT COALESCE(SUM(token_count * hit_count), 0) FROM response_cache",
            [],
            |row| row.get(0),
        )?;

        // 转换类型并返回结果
        // 注意：这里允许有符号到无符号的转换，因为值不应为负
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        Ok((count as usize, hits as u64, tokens_saved as u64))
    }

    /// WASM 目标平台的空实现 —— 缓存功能不可用。
    ///
    /// 返回零统计值 `(0, 0, 0)`。
    #[cfg(target_arch = "wasm32")]
    pub fn stats(&self) -> Result<(usize, u64, u64)> {
        Ok((0, 0, 0))
    }

    /// 清空整个缓存。
    ///
    /// 删除缓存中的所有条目，通常用于手动清理缓存或重置缓存状态。
    /// 适用于 `shiju cache clear` 等命令行工具。
    ///
    /// # 返回值
    ///
    /// - `Ok(usize)`: 被删除的条目数量
    /// - `Err(anyhow::Error)`: 数据库操作错误
    ///
    /// # 注意事项
    ///
    /// - 此操作不可逆，所有缓存数据将被永久删除
    /// - 不影响记忆数据库 `brain.db` 中的数据
    /// - 清空后，后续请求将需要重新调用 LLM 并重新建立缓存
    ///
    /// # 平台差异
    ///
    /// - 非 WASM 目标：执行实际的数据库删除操作
    /// - WASM 目标：返回 `Ok(0)`
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 清空缓存并显示删除的条目数
    /// let deleted = cache.clear()?;
    /// println!("已清空 {} 条缓存记录", deleted);
    ///
    /// // 验证缓存已清空
    /// let (entries, _, _) = cache.stats()?;
    /// assert_eq!(entries, 0);
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub fn clear(&self) -> Result<usize> {
        // 获取数据库连接的互斥锁
        let conn = self.conn.lock();

        // 删除所有缓存条目
        // execute() 返回受影响的行数，即被删除的条目数
        let affected = conn.execute("DELETE FROM response_cache", [])?;
        Ok(affected)
    }

    /// WASM 目标平台的空实现 —— 缓存功能不可用。
    ///
    /// 返回 `Ok(0)`，表示没有删除任何条目。
    #[cfg(target_arch = "wasm32")]
    pub fn clear(&self) -> Result<usize> {
        Ok(0)
    }
}

// 单元测试模块（仅在非 WASM 目标且测试配置下编译）
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "tests.rs"]
mod tests;
