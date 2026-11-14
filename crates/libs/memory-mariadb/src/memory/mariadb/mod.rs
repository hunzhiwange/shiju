//! MariaDB 内存存储后端实现
//!
//! 本模块提供了基于 MariaDB 数据库的内存存储功能，实现了 [`Memory`] trait。
//! 支持持久化存储代理的对话历史、知识条目和会话状态等信息。
//!
//! # 核心功能
//!
//! - **存储与检索**: 支持基于键值和内容的模糊匹配检索
//! - **分类管理**: 支持多种内存分类（core、daily、conversation、custom）
//! - **会话隔离**: 支持按会话 ID 隔离内存条目
//! - **评分排序**: 检索结果基于键和内容的匹配度进行评分排序
//!
//! # 架构设计
//!
//! - 使用连接池管理数据库连接，提升并发性能
//! - 通过 `spawn_blocking` 将同步数据库操作包装为异步接口
//! - 支持可选的 schema 和 TLS 加密连接
//!
//! # 示例
//!
//! ```rust,no_run
//! use memory_mariadb::memory::mariadb::MariadbMemory;
//! use memory_mariadb::memory::traits::{Memory, MemoryCategory};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // 创建 MariaDB 内存存储实例
//!     let memory = MariadbMemory::new(
//!         "mysql://user:pass@localhost:3306",
//!         "agent_memory",  // schema 名称
//!         "memories",      // 表名
//!         Some(30),        // 连接超时（秒）
//!         false,           // 不启用 TLS
//!     )?;
//!
//!     // 存储一条内存
//!     memory.store("user_name", "Alice", MemoryCategory::Core, None).await?;
//!
//!     // 检索内存
//!     let results = memory.recall("Alice", 10, None).await?;
//!     println!("Found {} memories", results.len());
//!
//!     Ok(())
//! }
//! ```

use super::traits::{Memory, MemoryCategory, MemoryEntry};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use mysql::prelude::Queryable;
use mysql::{Opts, OptsBuilder, Pool, SslOpts, params};
use std::time::Duration;
use uuid::Uuid;

/// MariaDB 连接超时时间上限（秒）
///
/// 为防止用户配置过长超时导致资源浪费，设置此上限。
/// 用户指定的超时时间会被限制在此值以内。
const MARIADB_CONNECT_TIMEOUT_CAP_SECS: u64 = 300;

/// MariaDB 内存存储后端
///
/// 该结构体实现了 [`Memory`] trait，提供基于 MariaDB 数据库的持久化内存存储。
/// 内部使用连接池管理数据库连接，支持高并发访问。
///
/// # 字段说明
///
/// - `pool`: MariaDB 连接池，用于管理数据库连接
/// - `qualified_table`: 完整的表标识符，格式为 `schema.table` 或 `table`
pub struct MariadbMemory {
    /// MariaDB 连接池实例
    pool: Pool,
    /// 完整限定的表名（包含 schema 前缀，如 `my_schema.memories`）
    qualified_table: String,
}

impl MariadbMemory {
    /// 创建新的 MariaDB 内存存储实例
    ///
    /// 该方法会自动执行以下初始化操作：
    /// 1. 验证 schema 和表名的合法性
    /// 2. 创建数据库连接池
    /// 3. 创建指定的 schema（如果不存在）
    /// 4. 创建内存存储表（如果不存在）
    ///
    /// # 参数
    ///
    /// - `db_url`: MariaDB 连接 URL，格式为 `mysql://user:pass@host:port`
    /// - `schema`: 数据库名称（schema），空字符串或 "public" 表示使用默认数据库
    /// - `table`: 存储内存数据的表名
    /// - `connect_timeout_secs`: 可选的连接超时时间（秒），会被限制在 300 秒以内
    /// - `tls_mode`: 是否启用 TLS 加密连接
    ///
    /// # 返回值
    ///
    /// 成功返回 `MariadbMemory` 实例，失败返回错误信息
    ///
    /// # 错误
    ///
    /// - 如果表名或 schema 名包含非法字符，返回验证错误
    /// - 如果数据库连接失败，返回连接错误
    /// - 如果表创建失败，返回 SQL 错误
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use memory_mariadb::memory::mariadb::MariadbMemory;
    ///
    /// let memory = MariadbMemory::new(
    ///     "mysql://root@localhost:3306",
    ///     "my_db",
    ///     "memories",
    ///     Some(30),
    ///     true,
    /// )?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn new(
        db_url: &str,
        schema: &str,
        table: &str,
        connect_timeout_secs: Option<u64>,
        tls_mode: bool,
    ) -> Result<Self> {
        // 验证表名合法性，防止 SQL 注入
        validate_identifier(table, "storage table")?;

        // 标准化 schema 名称（处理空值和 "public" 特殊值）
        let schema = normalize_schema(schema);
        if let Some(schema_name) = schema.as_deref() {
            validate_identifier(schema_name, "storage schema")?;
        }

        // 构建完整限定的表标识符
        let table_ident = quote_identifier(table);
        let qualified_table = match schema.as_deref() {
            Some(schema_name) => format!("{}.{}", quote_identifier(schema_name), table_ident),
            None => table_ident,
        };

        // 初始化连接池并创建表结构
        let pool = Self::initialize_pool(
            db_url,
            connect_timeout_secs,
            tls_mode,
            schema.as_deref(),
            &qualified_table,
        )?;

        Ok(Self { pool, qualified_table })
    }

    /// 初始化 MariaDB 连接池
    ///
    /// 在独立线程中执行同步的连接池初始化操作，包括：
    /// - 配置连接参数（超时、TLS 等）
    /// - 创建连接池
    /// - 执行 schema 和表的初始化
    ///
    /// # 参数
    ///
    /// - `db_url`: 数据库连接 URL
    /// - `connect_timeout_secs`: 可选的连接超时时间（秒）
    /// - `tls_mode`: 是否启用 TLS
    /// - `schema`: 可选的数据库名称
    /// - `qualified_table`: 完整限定的表名
    ///
    /// # 返回值
    ///
    /// 成功返回连接池实例，失败返回错误
    fn initialize_pool(
        db_url: &str,
        connect_timeout_secs: Option<u64>,
        tls_mode: bool,
        schema: Option<&str>,
        qualified_table: &str,
    ) -> Result<Pool> {
        // 将引用转换为所有权类型，以便在闭包中移动
        let db_url = db_url.to_string();
        let schema = schema.map(str::to_string);
        let qualified_table = qualified_table.to_string();

        // 在独立线程中执行同步初始化
        // 这样可以避免阻塞异步运行时
        let init_handle = std::thread::Builder::new()
            .name("mariadb-memory-init".to_string())
            .spawn(move || -> Result<Pool> {
                // 解析连接 URL 并构建选项
                let mut builder = OptsBuilder::from_opts(
                    Opts::from_url(&db_url).context("invalid MariaDB connection URL")?,
                );

                // 配置连接超时，限制最大值防止资源浪费
                if let Some(timeout_secs) = connect_timeout_secs {
                    let bounded = timeout_secs.min(MARIADB_CONNECT_TIMEOUT_CAP_SECS);
                    builder = builder.tcp_connect_timeout(Some(Duration::from_secs(bounded)));
                }

                // 配置 TLS 加密连接
                if tls_mode {
                    builder = builder.ssl_opts(Some(SslOpts::default()));
                }

                // 创建连接池
                let pool = Pool::new(builder).context("failed to create MariaDB pool")?;
                let mut conn =
                    pool.get_conn().context("failed to connect to MariaDB memory backend")?;

                // 初始化 schema 和表结构
                Self::init_schema(&mut conn, schema.as_deref(), &qualified_table)?;
                drop(conn);
                Ok(pool)
            })
            .context("failed to spawn MariaDB initializer thread")?;

        // 等待初始化线程完成并获取结果
        init_handle.join().map_err(|_| anyhow::anyhow!("MariaDB initializer thread panicked"))?
    }

    /// 初始化数据库 schema 和表结构
    ///
    /// 如果指定的 schema 不存在则创建，然后创建内存存储表。
    /// 表结构包含 id、key、content、category、时间戳和 session_id 等字段。
    ///
    /// # 参数
    ///
    /// - `conn`: 数据库连接
    /// - `schema`: 可选的数据库名称
    /// - `qualified_table`: 完整限定的表名
    ///
    /// # 表结构
    ///
    /// ```sql
    /// CREATE TABLE memories (
    ///     id VARCHAR(64) PRIMARY KEY,           -- 唯一标识符
    ///     `key` VARCHAR(255) NOT NULL UNIQUE,   -- 内存键名
    ///     content LONGTEXT NOT NULL,            -- 内存内容
    ///     category VARCHAR(64) NOT NULL,        -- 分类（core/daily/conversation/custom）
    ///     created_at VARCHAR(40) NOT NULL,      -- 创建时间（RFC3339 格式）
    ///     updated_at VARCHAR(40) NOT NULL,      -- 更新时间（RFC3339 格式）
    ///     session_id VARCHAR(255) NULL,         -- 会话 ID（可选）
    ///     INDEX idx_memories_category (category),
    ///     INDEX idx_memories_session_id (session_id),
    ///     INDEX idx_memories_updated_at (updated_at)
    /// )
    /// ```
    fn init_schema(
        conn: &mut mysql::PooledConn,
        schema: Option<&str>,
        qualified_table: &str,
    ) -> Result<()> {
        // 如果指定了 schema，则创建数据库（如果不存在）
        if let Some(schema_name) = schema {
            let create_schema =
                format!("CREATE DATABASE IF NOT EXISTS {}", quote_identifier(schema_name));
            conn.query_drop(create_schema)?;
        }

        // 创建内存存储表
        // 使用 utf8mb4 字符集以支持完整的 Unicode 字符（包括 emoji）
        let create_table = format!(
            "
            CREATE TABLE IF NOT EXISTS {qualified_table} (
                id VARCHAR(64) PRIMARY KEY,
                `key` VARCHAR(255) NOT NULL UNIQUE,
                content LONGTEXT NOT NULL,
                category VARCHAR(64) NOT NULL,
                created_at VARCHAR(40) NOT NULL,
                updated_at VARCHAR(40) NOT NULL,
                session_id VARCHAR(255) NULL,
                INDEX idx_memories_category (category),
                INDEX idx_memories_session_id (session_id),
                INDEX idx_memories_updated_at (updated_at)
            ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
            "
        );
        conn.query_drop(create_table)?;

        Ok(())
    }

    /// 将内存分类枚举转换为字符串
    ///
    /// 用于数据库存储时的序列化。
    ///
    /// # 参数
    ///
    /// - `category`: 内存分类枚举引用
    ///
    /// # 返回值
    ///
    /// 对应的字符串值：
    /// - `Core` -> `"core"`
    /// - `Daily` -> `"daily"`
    /// - `Conversation` -> `"conversation"`
    /// - `Custom(name)` -> `name`
    fn category_to_str(category: &MemoryCategory) -> String {
        match category {
            MemoryCategory::Core => "core".to_string(),
            MemoryCategory::Daily => "daily".to_string(),
            MemoryCategory::Conversation => "conversation".to_string(),
            MemoryCategory::Custom(name) => name.clone(),
        }
    }

    /// 将字符串解析为内存分类枚举
    ///
    /// 用于从数据库读取时的反序列化。
    /// 未知分类会被映射为 `MemoryCategory::Custom`。
    ///
    /// # 参数
    ///
    /// - `value`: 分类字符串
    ///
    /// # 返回值
    ///
    /// 对应的 [`MemoryCategory`] 枚举值
    ///
    /// # 示例
    ///
    /// ```rust
    /// use memory_mariadb::memory::traits::MemoryCategory;
    /// use memory_mariadb::memory::mariadb::MariadbMemory;
    ///
    /// assert!(matches!(MariadbMemory::parse_category("core"), MemoryCategory::Core));
    /// assert!(matches!(MariadbMemory::parse_category("daily"), MemoryCategory::Daily));
    /// assert!(matches!(MariadbMemory::parse_category("custom_tag"), MemoryCategory::Custom(_)));
    /// ```
    pub fn parse_category(value: &str) -> MemoryCategory {
        match value {
            "core" => MemoryCategory::Core,
            "daily" => MemoryCategory::Daily,
            "conversation" => MemoryCategory::Conversation,
            other => MemoryCategory::Custom(other.to_string()),
        }
    }

    /// 将数据库行转换为内存条目结构
    ///
    /// # 参数
    ///
    /// - `row`: MySQL 查询返回的行数据
    ///
    /// # 返回值
    ///
    /// 成功返回 [`MemoryEntry`]，失败返回错误
    ///
    /// # 期望的列顺序
    ///
    /// 0. id (VARCHAR)
    /// 1. key (VARCHAR)
    /// 2. content (LONGTEXT)
    /// 3. category (VARCHAR)
    /// 4. created_at (VARCHAR) - 映射到 timestamp 字段
    /// 5. session_id (VARCHAR, nullable)
    /// 6. score (DOUBLE, nullable) - 仅在检索时存在
    fn row_to_entry(row: mysql::Row) -> Result<MemoryEntry> {
        // 按列索引提取数据
        let id: String = row.get(0).context("missing id column in memory row")?;
        let key: String = row.get(1).context("missing key column in memory row")?;
        let content: String = row.get(2).context("missing content column in memory row")?;
        let category: String = row.get(3).context("missing category column in memory row")?;
        let timestamp: String = row.get(4).context("missing created_at column in memory row")?;
        let session_id: Option<String> = row.get(5);
        let score: Option<f64> = row.get(6);

        Ok(MemoryEntry {
            id,
            key,
            content,
            category: Self::parse_category(&category),
            timestamp,
            session_id,
            score,
        })
    }
}

/// 标准化 schema 名称
///
/// 处理空值和 "public" 特殊值，将其统一转换为 `None`。
/// 这符合 PostgreSQL 的命名惯例，其中 "public" 是默认 schema。
///
/// # 参数
///
/// - `schema`: 原始 schema 名称字符串
///
/// # 返回值
///
/// - 如果 schema 为空或为 "public"（不区分大小写），返回 `None`
/// - 否则返回修剪后的字符串
///
/// # 示例
///
/// ```rust
/// use memory_mariadb::memory::mariadb::normalize_schema;
///
/// assert_eq!(normalize_schema(""), None);
/// assert_eq!(normalize_schema("public"), None);
/// assert_eq!(normalize_schema("PUBLIC"), None);
/// assert_eq!(normalize_schema("  my_schema  "), Some("my_schema".to_string()));
/// ```
pub fn normalize_schema(schema: &str) -> Option<String> {
    let trimmed = schema.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("public") {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// 验证标识符的合法性
///
/// 检查数据库标识符（表名、schema 名等）是否符合命名规范，
/// 防止 SQL 注入攻击。
///
/// # 参数
///
/// - `value`: 要验证的标识符字符串
/// - `field_name`: 字段名称（用于错误消息）
///
/// # 返回值
///
/// 验证通过返回 `Ok(())`，失败返回错误信息
///
/// # 验证规则
///
/// 1. 不能为空
/// 2. 首字符必须是 ASCII 字母或下划线
/// 3. 后续字符只能是 ASCII 字母、数字或下划线
///
/// # 示例
///
/// ```rust
/// use memory_mariadb::memory::mariadb::validate_identifier;
///
/// assert!(validate_identifier("my_table", "table name").is_ok());
/// assert!(validate_identifier("_users", "table name").is_ok());
/// assert!(validate_identifier("table1", "table name").is_ok());
///
/// assert!(validate_identifier("", "table name").is_err());
/// assert!(validate_identifier("1table", "table name").is_err());
/// assert!(validate_identifier("my-table", "table name").is_err());
/// assert!(validate_identifier("my.table", "table name").is_err());
/// ```
pub fn validate_identifier(value: &str, field_name: &str) -> Result<()> {
    // 检查是否为空
    if value.is_empty() {
        anyhow::bail!("{field_name} must not be empty");
    }

    // 检查首字符
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        anyhow::bail!("{field_name} must not be empty");
    };

    // 首字符必须是 ASCII 字母或下划线
    if !(first.is_ascii_alphabetic() || first == '_') {
        anyhow::bail!("{field_name} must start with an ASCII letter or underscore; got '{value}'");
    }

    // 后续字符只能是 ASCII 字母、数字或下划线
    if !chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
        anyhow::bail!(
            "{field_name} can only contain ASCII letters, numbers, and underscores; got '{value}'"
        );
    }

    Ok(())
}

/// 用反引号引用标识符
///
/// 为 SQL 标识符添加反引号包裹，用于在 SQL 语句中安全引用。
/// 这是 MariaDB/MySQL 的标准标识符引用方式。
///
/// # 参数
///
/// - `value`: 原始标识符
///
/// # 返回值
///
/// 反引号包裹的标识符字符串
///
/// # 示例
///
/// ```rust
/// use memory_mariadb::memory::mariadb::quote_identifier;
///
/// assert_eq!(quote_identifier("my_table"), "`my_table`");
/// assert_eq!(quote_identifier("users"), "`users`");
/// ```
fn quote_identifier(value: &str) -> String {
    format!("`{value}`")
}

/// 为 [`MariadbMemory`] 实现 [`Memory`] trait
///
/// 提供完整的内存存储、检索、删除和健康检查功能。
/// 所有数据库操作都通过 `spawn_blocking` 在独立线程池中执行，
/// 避免阻塞异步运行时。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Memory for MariadbMemory {
    /// 返回存储后端名称
    ///
    /// # 返回值
    ///
    /// 固定返回 `"mariadb"`
    fn name(&self) -> &str {
        "mariadb"
    }

    /// 存储一条内存条目
    ///
    /// 如果指定 key 已存在，则更新内容、分类、更新时间和会话 ID。
    /// 使用 `INSERT ... ON DUPLICATE KEY UPDATE` 实现 upsert 语义。
    ///
    /// # 参数
    ///
    /// - `key`: 内存条目的唯一键名
    /// - `content`: 内存内容文本
    /// - `category`: 内存分类
    /// - `session_id`: 可选的会话 ID，用于隔离不同会话的内存
    ///
    /// # 返回值
    ///
    /// 成功返回 `Ok(())`，失败返回错误
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use memory_mariadb::memory::traits::{Memory, MemoryCategory};
    /// # use memory_mariadb::memory::mariadb::MariadbMemory;
    /// # let memory = MariadbMemory::new("mysql://localhost", "", "memories", None, false)?;
    ///
    /// // 存储核心记忆
    /// memory.store("user_name", "Alice", MemoryCategory::Core, None).await?;
    ///
    /// // 存储会话相关记忆
    /// memory.store("last_query", "hello", MemoryCategory::Conversation, Some("session-123")).await?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
    ) -> Result<()> {
        // 克隆数据以便在 spawn_blocking 闭包中移动
        let pool = self.pool.clone();
        let qualified_table = self.qualified_table.clone();
        let key = key.to_string();
        let content = content.to_string();
        let category = Self::category_to_str(&category);
        let session_id = session_id.map(str::to_string);

        // 在阻塞线程池中执行同步数据库操作
        tokio::task::spawn_blocking(move || -> Result<()> {
            let mut conn = pool.get_conn()?;
            let now = Utc::now().to_rfc3339();

            // 使用 upsert 语义：插入新记录或更新已存在的记录
            let sql = format!(
                "
                INSERT INTO {qualified_table}
                    (id, `key`, content, category, created_at, updated_at, session_id)
                VALUES
                    (:id, :key, :content, :category, :created_at, :updated_at, :session_id)
                ON DUPLICATE KEY UPDATE
                    content = VALUES(content),
                    category = VALUES(category),
                    updated_at = VALUES(updated_at),
                    session_id = VALUES(session_id)
                "
            );

            conn.exec_drop(
                sql,
                params! {
                    "id" => Uuid::new_v4().to_string(),
                    "key" => key,
                    "content" => content,
                    "category" => category,
                    "created_at" => now.clone(),
                    "updated_at" => now,
                    "session_id" => session_id,
                },
            )?;
            Ok(())
        })
        .await?
    }

    /// 检索匹配的内存条目
    ///
    /// 基于查询字符串在 key 和 content 中进行模糊匹配。
    /// 结果按匹配评分和更新时间排序，评分规则为：
    /// - key 匹配：2.0 分
    /// - content 匹配：1.0 分
    /// - 同时匹配：3.0 分
    ///
    /// # 参数
    ///
    /// - `query`: 查询字符串，为空时返回所有条目（受 session_id 限制）
    /// - `limit`: 返回结果的最大数量
    /// - `session_id`: 可选的会话 ID 过滤条件
    ///
    /// # 返回值
    ///
    /// 返回匹配的内存条目列表，按评分降序和更新时间降序排列
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use memory_mariadb::memory::traits::Memory;
    /// # use memory_mariadb::memory::mariadb::MariadbMemory;
    /// # let memory = MariadbMemory::new("mysql://localhost", "", "memories", None, false)?;
    ///
    /// // 搜索包含 "project" 的内存
    /// let results = memory.recall("project", 10, None).await?;
    ///
    /// // 搜索特定会话的内存
    /// let session_results = memory.recall("", 100, Some("session-123")).await?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    async fn recall(
        &self,
        query: &str,
        limit: usize,
        session_id: Option<&str>,
    ) -> Result<Vec<MemoryEntry>> {
        let pool = self.pool.clone();
        let qualified_table = self.qualified_table.clone();
        let query = query.trim().to_string();
        let session_id = session_id.map(str::to_string);

        tokio::task::spawn_blocking(move || -> Result<Vec<MemoryEntry>> {
            let mut conn = pool.get_conn()?;

            // 构建带评分的查询
            // 评分算法：key 匹配得 2 分，content 匹配得 1 分
            let sql = format!(
                "
                SELECT
                    id,
                    `key`,
                    content,
                    category,
                    created_at,
                    session_id,
                    (
                        CASE WHEN LOWER(`key`) LIKE CONCAT('%', LOWER(:query), '%') THEN 2.0 ELSE 0.0 END +
                        CASE WHEN LOWER(content) LIKE CONCAT('%', LOWER(:query), '%') THEN 1.0 ELSE 0.0 END
                    ) AS score
                FROM {qualified_table}
                WHERE (:session_id IS NULL OR session_id = :session_id)
                  AND (
                    :query = '' OR
                    LOWER(`key`) LIKE CONCAT('%', LOWER(:query), '%') OR
                    LOWER(content) LIKE CONCAT('%', LOWER(:query), '%')
                  )
                ORDER BY score DESC, updated_at DESC
                LIMIT :limit
                "
            );

            // 转换 limit 为 i64 以匹配 MySQL 参数类型
            #[allow(clippy::cast_possible_wrap)]
            let limit_i64 = limit as i64;

            let rows = conn.exec(
                sql,
                params! {
                    "query" => query,
                    "session_id" => session_id,
                    "limit" => limit_i64,
                },
            )?;

            // 将结果行转换为 MemoryEntry
            rows.into_iter()
                .map(Self::row_to_entry)
                .collect::<Result<Vec<MemoryEntry>>>()
        })
        .await?
    }

    /// 根据 key 获取单个内存条目
    ///
    /// # 参数
    ///
    /// - `key`: 内存条目的键名
    ///
    /// # 返回值
    ///
    /// - `Ok(Some(entry))`: 找到对应的内存条目
    /// - `Ok(None)`: 未找到对应条目
    /// - `Err(e)`: 查询过程中发生错误
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use memory_mariadb::memory::traits::Memory;
    /// # use memory_mariadb::memory::mariadb::MariadbMemory;
    /// # let memory = MariadbMemory::new("mysql://localhost", "", "memories", None, false)?;
    ///
    /// if let Some(entry) = memory.get("user_name").await? {
    ///     println!("User name: {}", entry.content);
    /// }
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    async fn get(&self, key: &str) -> Result<Option<MemoryEntry>> {
        let pool = self.pool.clone();
        let qualified_table = self.qualified_table.clone();
        let key = key.to_string();

        tokio::task::spawn_blocking(move || -> Result<Option<MemoryEntry>> {
            let mut conn = pool.get_conn()?;

            // 按 key 精确查询
            let sql = format!(
                "
                SELECT id, `key`, content, category, created_at, session_id
                FROM {qualified_table}
                WHERE `key` = :key
                LIMIT 1
                "
            );

            let row = conn.exec_first(sql, params! { "key" => key })?;
            row.map(Self::row_to_entry).transpose()
        })
        .await?
    }

    /// 列出内存条目
    ///
    /// 可按分类和会话 ID 过滤，结果按更新时间降序排列。
    ///
    /// # 参数
    ///
    /// - `category`: 可选的分类过滤条件
    /// - `session_id`: 可选的会话 ID 过滤条件
    ///
    /// # 返回值
    ///
    /// 返回符合条件的所有内存条目
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use memory_mariadb::memory::traits::{Memory, MemoryCategory};
    /// # use memory_mariadb::memory::mariadb::MariadbMemory;
    /// # let memory = MariadbMemory::new("mysql://localhost", "", "memories", None, false)?;
    ///
    /// // 列出所有核心记忆
    /// let core_memories = memory.list(Some(&MemoryCategory::Core), None).await?;
    ///
    /// // 列出特定会话的所有记忆
    /// let session_memories = memory.list(None, Some("session-123")).await?;
    ///
    /// // 列出所有记忆
    /// let all_memories = memory.list(None, None).await?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    async fn list(
        &self,
        category: Option<&MemoryCategory>,
        session_id: Option<&str>,
    ) -> Result<Vec<MemoryEntry>> {
        let pool = self.pool.clone();
        let qualified_table = self.qualified_table.clone();
        let category = category.map(Self::category_to_str);
        let session_id = session_id.map(str::to_string);

        tokio::task::spawn_blocking(move || -> Result<Vec<MemoryEntry>> {
            let mut conn = pool.get_conn()?;

            // 构建带可选过滤条件的查询
            let sql = format!(
                "
                SELECT id, `key`, content, category, created_at, session_id
                FROM {qualified_table}
                WHERE (:category IS NULL OR category = :category)
                  AND (:session_id IS NULL OR session_id = :session_id)
                ORDER BY updated_at DESC
                "
            );

            let rows = conn.exec(
                sql,
                params! {
                    "category" => category,
                    "session_id" => session_id,
                },
            )?;

            rows.into_iter().map(Self::row_to_entry).collect::<Result<Vec<MemoryEntry>>>()
        })
        .await?
    }

    /// 删除指定 key 的内存条目
    ///
    /// # 参数
    ///
    /// - `key`: 要删除的内存条目键名
    ///
    /// # 返回值
    ///
    /// - `Ok(true)`: 成功删除条目
    /// - `Ok(false)`: 条目不存在
    /// - `Err(e)`: 删除过程中发生错误
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use memory_mariadb::memory::traits::Memory;
    /// # use memory_mariadb::memory::mariadb::MariadbMemory;
    /// # let memory = MariadbMemory::new("mysql://localhost", "", "memories", None, false)?;
    ///
    /// if memory.forget("old_memory").await? {
    ///     println!("Memory deleted");
    /// } else {
    ///     println!("Memory not found");
    /// }
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    async fn forget(&self, key: &str) -> Result<bool> {
        let pool = self.pool.clone();
        let qualified_table = self.qualified_table.clone();
        let key = key.to_string();

        tokio::task::spawn_blocking(move || -> Result<bool> {
            let mut conn = pool.get_conn()?;
            let sql = format!("DELETE FROM {qualified_table} WHERE `key` = :key");
            conn.exec_drop(sql, params! { "key" => key })?;

            // 根据受影响行数判断是否删除成功
            Ok(conn.affected_rows() > 0)
        })
        .await?
    }

    /// 获取内存条目总数
    ///
    /// # 返回值
    ///
    /// 返回当前存储的所有内存条目数量
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use memory_mariadb::memory::traits::Memory;
    /// # use memory_mariadb::memory::mariadb::MariadbMemory;
    /// # let memory = MariadbMemory::new("mysql://localhost", "", "memories", None, false)?;
    ///
    /// let total = memory.count().await?;
    /// println!("Total memories: {}", total);
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    async fn count(&self) -> Result<usize> {
        let pool = self.pool.clone();
        let qualified_table = self.qualified_table.clone();

        tokio::task::spawn_blocking(move || -> Result<usize> {
            let mut conn = pool.get_conn()?;
            let sql = format!("SELECT COUNT(*) FROM {qualified_table}");
            let count: Option<i64> = conn.query_first(sql)?;
            let count = count.unwrap_or(0);

            // 将 i64 转换为 usize，负数情况理论上不应出现
            let count =
                usize::try_from(count).context("MariaDB returned a negative memory count")?;
            Ok(count)
        })
        .await?
    }

    /// 执行健康检查
    ///
    /// 通过执行简单的 `SELECT 1` 查询验证数据库连接是否正常。
    ///
    /// # 返回值
    ///
    /// - `true`: 数据库连接正常
    /// - `false`: 数据库连接异常
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use memory_mariadb::memory::traits::Memory;
    /// # use memory_mariadb::memory::mariadb::MariadbMemory;
    /// # let memory = MariadbMemory::new("mysql://localhost", "", "memories", None, false)?;
    ///
    /// if memory.health_check().await {
    ///     println!("Database is healthy");
    /// } else {
    ///     println!("Database connection failed");
    /// }
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    async fn health_check(&self) -> bool {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || -> bool {
            match pool.get_conn() {
                Ok(mut conn) => conn.query_drop("SELECT 1").is_ok(),
                Err(_) => false,
            }
        })
        .await
        .unwrap_or(false)
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
