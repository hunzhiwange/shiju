//! 响应缓存模块的单元测试
//!
//! 本模块包含针对 `ResponseCache` 的全面测试套件，验证缓存系统的各项功能：
//!
//! - **缓存键生成**：测试键的确定性、唯一性和多因素影响
//! - **基础操作**：测试存储、检索、清空等基本功能
//! - **过期机制**：验证 TTL（生存时间）过期逻辑
//! - **统计功能**：测试命中次数和节省的令牌数统计
//! - **LRU 淘汰**：验证最近最少使用淘汰策略的正确性
//! - **并发安全**：测试多线程并发读取的安全性
//!
//! # 测试依赖
//!
//! - `tempfile`：用于创建临时目录，确保测试隔离且不污染文件系统
//!
//! # 示例
//!
//! ```ignore
//! // 运行所有测试
//! cargo test --package vibe-window --lib app::agent::memory::response_cache::tests
//! ```

use super::*;
use tempfile::TempDir;

/// 创建临时缓存实例的辅助函数
///
/// 在临时目录中创建一个 `ResponseCache` 实例，用于测试。
/// 使用完毕后，临时目录及其内容会自动清理。
///
/// # 参数
///
/// - `ttl_minutes`：缓存条目的生存时间（分钟）
///
/// # 返回值
///
/// 返回元组 `(TempDir, ResponseCache)`：
/// - `TempDir`：临时目录句柄，需要保持在作用域内以防止过早清理
/// - `ResponseCache`：配置好的缓存实例，最大条目数为 1000
///
/// # 示例
///
/// ```ignore
/// let (_tmp, cache) = temp_cache(60);
/// // 使用 cache 进行测试...
/// // _tmp 离开作用域时自动清理
/// ```
fn temp_cache(ttl_minutes: u32) -> (TempDir, ResponseCache) {
    let tmp = TempDir::new().unwrap();
    let cache = ResponseCache::new(tmp.path(), ttl_minutes, 1000).unwrap();
    (tmp, cache)
}

#[test]
fn response_cache_uses_user_scoped_data_dir() {
    let workspace = TempDir::new().unwrap();
    let storage = paths::workspace_data_dir(workspace.path()).unwrap();

    let _cache = ResponseCache::new(workspace.path(), 60, 1000).unwrap();

    assert!(!workspace.path().join("memory").join("response_cache.db").exists());
    assert!(storage.join("memory").join("response_cache.db").exists());
}

/// 测试缓存键的确定性生成
///
/// 验证相同的输入参数总是生成相同的缓存键。
/// 缓存键应该是确定性的，确保相同请求能够命中缓存。
///
/// # 验证点
///
/// - 相同的模型、系统提示和用户提示应生成相同的键
/// - 键的长度应为 64 个字符（SHA-256 哈希的十六进制表示）
#[test]
fn cache_key_deterministic() {
    let k1 = ResponseCache::cache_key("gpt-4", Some("sys"), "hello");
    let k2 = ResponseCache::cache_key("gpt-4", Some("sys"), "hello");
    assert_eq!(k1, k2);
    assert_eq!(k1.len(), 64); // SHA-256 哈希的十六进制表示长度为 64
}

/// 测试缓存键随模型名称变化
///
/// 验证不同的模型名称会生成不同的缓存键。
/// 即使其他参数相同，不同的模型可能有不同的响应，因此需要独立的缓存条目。
#[test]
fn cache_key_varies_by_model() {
    let k1 = ResponseCache::cache_key("gpt-4", None, "hello");
    let k2 = ResponseCache::cache_key("claude-3", None, "hello");
    assert_ne!(k1, k2);
}

/// 测试缓存键随系统提示变化
///
/// 验证不同的系统提示会生成不同的缓存键。
/// 系统提示会影响模型的响应方式，因此需要分别缓存。
#[test]
fn cache_key_varies_by_system_prompt() {
    let k1 = ResponseCache::cache_key("gpt-4", Some("You are helpful"), "hello");
    let k2 = ResponseCache::cache_key("gpt-4", Some("You are rude"), "hello");
    assert_ne!(k1, k2);
}

/// 测试缓存键随用户提示变化
///
/// 验证不同的用户提示会生成不同的缓存键。
/// 不同的用户问题应该产生不同的缓存条目。
#[test]
fn cache_key_varies_by_prompt() {
    let k1 = ResponseCache::cache_key("gpt-4", None, "hello");
    let k2 = ResponseCache::cache_key("gpt-4", None, "goodbye");
    assert_ne!(k1, k2);
}

/// 测试缓存的基本存取操作
///
/// 验证向缓存中存储数据后能够正确检索。
/// 这是最基础的缓存功能测试。
///
/// # 测试流程
///
/// 1. 创建缓存实例（TTL 为 60 分钟）
/// 2. 生成缓存键
/// 3. 存储响应内容及其令牌数
/// 4. 检索并验证内容是否一致
#[test]
fn put_and_get() {
    let (_tmp, cache) = temp_cache(60);
    let key = ResponseCache::cache_key("gpt-4", None, "What is Rust?");

    cache.put(&key, "gpt-4", "Rust is a systems programming language.", 25).unwrap();

    let result = cache.get(&key).unwrap();
    assert_eq!(result.as_deref(), Some("Rust is a systems programming language."));
}

/// 测试缓存未命中场景
///
/// 验证查询不存在的键时返回 `None`。
/// 这是正常的缓存未命中行为。
#[test]
fn miss_returns_none() {
    let (_tmp, cache) = temp_cache(60);
    let result = cache.get("nonexistent_key").unwrap();
    assert!(result.is_none());
}

/// 测试过期条目的处理
///
/// 验证超过 TTL 的缓存条目不会被返回。
/// 当条目的创建时间早于截止时间时，应视为已过期。
///
/// # 测试策略
///
/// - 设置 TTL 为 0 分钟，意味着所有条目立即过期
/// - 创建条目后立即查询
/// - 截止时间为 now() - 0 = now()
/// - 条目的创建时间不大于截止时间，因此应返回 `None`
#[test]
fn expired_entry_returns_none() {
    let (_tmp, cache) = temp_cache(0); // 0 分钟 TTL，所有内容立即过期
    let key = ResponseCache::cache_key("gpt-4", None, "test");

    cache.put(&key, "gpt-4", "response", 10).unwrap();

    // 条目创建时 created_at = now()，但 TTL 为 0 分钟
    // 因此 cutoff = now() - 0 = now()
    // 条目的 created_at 不大于 cutoff，所以会被视为已过期
    let result = cache.get(&key).unwrap();
    assert!(result.is_none());
}

/// 测试缓存命中计数的递增
///
/// 验证每次成功从缓存检索数据时，命中计数器都会递增。
/// 命中计数是衡量缓存效果的重要指标。
#[test]
fn hit_count_incremented() {
    let (_tmp, cache) = temp_cache(60);
    let key = ResponseCache::cache_key("gpt-4", None, "hello");

    cache.put(&key, "gpt-4", "Hi!", 5).unwrap();

    // 执行 3 次缓存命中
    for _ in 0..3 {
        let _ = cache.get(&key).unwrap();
    }

    let (_, total_hits, _) = cache.stats().unwrap();
    assert_eq!(total_hits, 3);
}

/// 测试节省令牌数的计算
///
/// 验证缓存节省的令牌数正确累积。
/// 节省的令牌数 = 缓存命中次数 × 每次响应的令牌数
#[test]
fn tokens_saved_calculated() {
    let (_tmp, cache) = temp_cache(60);
    let key = ResponseCache::cache_key("gpt-4", None, "explain rust");

    cache.put(&key, "gpt-4", "Rust is...", 100).unwrap();

    // 5 次缓存命中 × 100 令牌 = 500 令牌节省
    for _ in 0..5 {
        let _ = cache.get(&key).unwrap();
    }

    let (_, _, tokens_saved) = cache.stats().unwrap();
    assert_eq!(tokens_saved, 500);
}

/// 测试 LRU（最近最少使用）淘汰机制
///
/// 验证当缓存条目数超过最大限制时，最旧的条目会被淘汰。
/// LRU 策略确保缓存不会无限增长。
///
/// # 测试场景
///
/// - 最大条目数设置为 3
/// - 插入 5 个条目
/// - 验证最终条目数不超过 3
#[test]
fn lru_eviction() {
    let tmp = TempDir::new().unwrap();
    let cache = ResponseCache::new(tmp.path(), 60, 3).unwrap(); // 最多 3 个条目

    for i in 0..5 {
        let key = ResponseCache::cache_key("gpt-4", None, &format!("prompt {i}"));
        cache.put(&key, "gpt-4", &format!("response {i}"), 10).unwrap();
    }

    let (count, _, _) = cache.stats().unwrap();
    assert!(count <= 3, "淘汰后应最多有 3 个条目");
}

/// 测试清空缓存功能
///
/// 验证 `clear()` 方法能够删除所有缓存条目并返回删除的数量。
#[test]
fn clear_wipes_all() {
    let (_tmp, cache) = temp_cache(60);

    for i in 0..10 {
        let key = ResponseCache::cache_key("gpt-4", None, &format!("prompt {i}"));
        cache.put(&key, "gpt-4", &format!("response {i}"), 10).unwrap();
    }

    let cleared = cache.clear().unwrap();
    assert_eq!(cleared, 10);

    let (count, _, _) = cache.stats().unwrap();
    assert_eq!(count, 0);
}

/// 测试空缓存的统计信息
///
/// 验证新创建的缓存返回全零的统计数据。
/// 条目数、命中次数和节省令牌数都应为 0。
#[test]
fn stats_empty_cache() {
    let (_tmp, cache) = temp_cache(60);
    let (count, hits, tokens) = cache.stats().unwrap();
    assert_eq!(count, 0);
    assert_eq!(hits, 0);
    assert_eq!(tokens, 0);
}

/// 测试覆盖相同键的值
///
/// 验证使用相同的键存储新值时，会覆盖旧值而不增加条目数。
/// 更新操作应替换现有条目而非创建新条目。
#[test]
fn overwrite_same_key() {
    let (_tmp, cache) = temp_cache(60);
    let key = ResponseCache::cache_key("gpt-4", None, "question");

    cache.put(&key, "gpt-4", "answer v1", 20).unwrap();
    cache.put(&key, "gpt-4", "answer v2", 25).unwrap();

    let result = cache.get(&key).unwrap();
    assert_eq!(result.as_deref(), Some("answer v2"));

    let (count, _, _) = cache.stats().unwrap();
    assert_eq!(count, 1);
}

/// 测试 Unicode 提示的处理
///
/// 验证缓存系统能够正确处理 Unicode 字符（如日语和表情符号）。
/// 缓存键生成和内容存储都应支持 UTF-8 编码。
#[test]
fn unicode_prompt_handling() {
    let (_tmp, cache) = temp_cache(60);
    let key = ResponseCache::cache_key("gpt-4", None, "日本語のテスト 🦀");

    cache.put(&key, "gpt-4", "はい、Rustは素晴らしい", 30).unwrap();

    let result = cache.get(&key).unwrap();
    assert_eq!(result.as_deref(), Some("はい、Rustは素晴らしい"));
}

// ── §4.4 缓存压力淘汰测试 ─────────────

/// 测试 LRU 淘汰保留最近访问的条目
///
/// 验证 LRU 策略正确识别并保留最近访问的条目。
/// 当需要淘汰时，应该淘汰最久未使用的条目。
///
/// # 测试流程
///
/// 1. 插入 3 个条目（填满缓存）
/// 2. 访问条目 0，使其成为最近使用的
/// 3. 插入第 4 个条目（触发淘汰）
/// 4. 验证条目 0 仍然存在（因为它被最近访问过）
#[test]
fn lru_eviction_keeps_most_recent() {
    let tmp = TempDir::new().unwrap();
    let cache = ResponseCache::new(tmp.path(), 60, 3).unwrap();

    // 插入 3 个条目
    for i in 0..3 {
        let key = ResponseCache::cache_key("gpt-4", None, &format!("prompt {i}"));
        cache.put(&key, "gpt-4", &format!("response {i}"), 10).unwrap();
    }

    // 访问条目 0，使其成为最近使用的
    let key0 = ResponseCache::cache_key("gpt-4", None, "prompt 0");
    let _ = cache.get(&key0).unwrap();

    // 插入条目 3（触发淘汰）
    let key3 = ResponseCache::cache_key("gpt-4", None, "prompt 3");
    cache.put(&key3, "gpt-4", "response 3", 10).unwrap();

    let (count, _, _) = cache.stats().unwrap();
    assert!(count <= 3, "缓存条目数不得超过 max_entries");

    // 条目 0 被最近访问过，应该存活
    let entry0 = cache.get(&key0).unwrap();
    assert!(entry0.is_some(), "最近访问的条目应在 LRU 淘汰中存活");
}

/// 测试缓存处理最大条目数为零的情况
///
/// 验证当 max_entries 设置为 0 时，缓存不会崩溃，
/// 并且会立即淘汰所有条目。
#[test]
fn cache_handles_zero_max_entries() {
    let tmp = TempDir::new().unwrap();
    let cache = ResponseCache::new(tmp.path(), 60, 0).unwrap();

    let key = ResponseCache::cache_key("gpt-4", None, "test");
    // 即使 max_entries=0 也不应崩溃
    cache.put(&key, "gpt-4", "response", 10).unwrap();

    let (count, _, _) = cache.stats().unwrap();
    assert_eq!(count, 0, "max_entries=0 的缓存应淘汰所有条目");
}

#[test]
fn zero_ttl_does_not_retain_new_entries() {
    let (_tmp, cache) = temp_cache(0);

    cache.put("key", "gpt-4", "response", 10).unwrap();

    assert!(cache.get("key").unwrap().is_none());
    assert_eq!(cache.stats().unwrap(), (0, 0, 0));
}

/// 测试缓存在并发读取时的安全性
///
/// 验证多个线程同时读取缓存时不会发生数据竞争或崩溃。
/// 这是一个并发安全性的压力测试。
///
/// # 测试策略
///
/// - 使用 `Arc` 包装缓存以便跨线程共享
/// - 启动 10 个线程并发读取同一键
/// - 验证所有线程都能成功完成
/// - 验证命中次数正确统计
#[test]
fn cache_concurrent_reads_no_panic() {
    let tmp = TempDir::new().unwrap();
    let cache = std::sync::Arc::new(ResponseCache::new(tmp.path(), 60, 100).unwrap());

    let key = ResponseCache::cache_key("gpt-4", None, "concurrent");
    cache.put(&key, "gpt-4", "response", 10).unwrap();

    let mut handles = Vec::new();
    for _ in 0..10 {
        let cache = std::sync::Arc::clone(&cache);
        let key = key.clone();
        handles.push(std::thread::spawn(move || {
            let _ = cache.get(&key).unwrap();
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let (_, hits, _) = cache.stats().unwrap();
    assert_eq!(hits, 10, "所有并发读取都应计入命中");
}

#[test]
fn cache_key_none_and_empty_system_prompt_are_equivalent() {
    let without_system = ResponseCache::cache_key("gpt-4", None, "hello");
    let empty_system = ResponseCache::cache_key("gpt-4", Some(""), "hello");

    assert_eq!(without_system, empty_system);
}

#[test]
fn put_cleans_expired_entries() {
    let (_tmp, cache) = temp_cache(60);
    let old_key = ResponseCache::cache_key("gpt-4", None, "old");
    let new_key = ResponseCache::cache_key("gpt-4", None, "new");
    let old_time = (Local::now() - Duration::minutes(120)).to_rfc3339();

    {
        let conn = cache.conn.lock();
        conn.execute(
            "INSERT INTO response_cache
             (prompt_hash, model, response, token_count, created_at, accessed_at, hit_count)
             VALUES (?1, 'gpt-4', 'old response', 5, ?2, ?3, 0)",
            params![old_key, old_time, old_time],
        )
        .unwrap();
    }

    cache.put(&new_key, "gpt-4", "new response", 10).unwrap();

    assert!(cache.get(&old_key).unwrap().is_none());
    assert_eq!(cache.get(&new_key).unwrap().as_deref(), Some("new response"));
    assert_eq!(cache.stats().unwrap().0, 1);
}

#[test]
fn clear_empty_cache_reports_zero() {
    let (_tmp, cache) = temp_cache(60);

    assert_eq!(cache.clear().unwrap(), 0);
}

#[test]
fn new_initializes_expected_schema_and_indexes() {
    let (_tmp, cache) = temp_cache(60);
    let conn = cache.conn.lock();

    let table_sql: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'response_cache'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(table_sql.contains("prompt_hash TEXT PRIMARY KEY"));
    assert!(table_sql.contains("hit_count"));

    let index_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'index' AND name IN ('idx_rc_accessed', 'idx_rc_created')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(index_count, 2);
}

#[test]
fn new_errors_when_memory_path_is_file() {
    let workspace = TempDir::new().unwrap();
    let storage = paths::workspace_data_dir(workspace.path()).unwrap();
    std::fs::create_dir_all(&storage).unwrap();
    std::fs::write(storage.join("memory"), b"not a directory").unwrap();

    assert!(ResponseCache::new(workspace.path(), 60, 1000).is_err());
}

#[test]
fn new_errors_when_database_path_is_directory() {
    let workspace = TempDir::new().unwrap();
    let storage = paths::workspace_data_dir(workspace.path()).unwrap();
    let db_path = storage.join("memory").join("response_cache.db");
    std::fs::create_dir_all(&db_path).unwrap();

    assert!(ResponseCache::new(workspace.path(), 60, 1000).is_err());
}

#[test]
fn new_errors_when_existing_schema_cannot_be_indexed() {
    let workspace = TempDir::new().unwrap();
    let storage = paths::workspace_data_dir(workspace.path()).unwrap();
    let db_dir = storage.join("memory");
    std::fs::create_dir_all(&db_dir).unwrap();

    let conn = Connection::open(db_dir.join("response_cache.db")).unwrap();
    conn.execute("CREATE TABLE response_cache (prompt_hash TEXT PRIMARY KEY)", []).unwrap();
    drop(conn);

    assert!(ResponseCache::new(workspace.path(), 60, 1000).is_err());
}

#[test]
fn operations_return_errors_when_table_is_missing() {
    let (_tmp, cache) = temp_cache(60);

    {
        let conn = cache.conn.lock();
        conn.execute("DROP TABLE response_cache", []).unwrap();
    }

    assert!(cache.get("missing").is_err());
    assert!(cache.put("key", "gpt-4", "response", 10).is_err());
    assert!(cache.stats().is_err());
    assert!(cache.clear().is_err());
}
