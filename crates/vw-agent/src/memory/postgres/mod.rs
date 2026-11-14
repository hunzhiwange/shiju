//! PostgreSQL 后端的持久化内存存储模块
//!
//! 本模块提供了基于 PostgreSQL 数据库的内存存储实现，是 [`Memory`] trait 的具体实现之一。
//! 该后端专注于提供可靠的 CRUD（创建、读取、更新、删除）操作和基于关键词的记忆召回功能，
//! 使用标准 SQL 实现，无需安装额外的数据库扩展（例如 pgvector）。
//!
//! # 核心特性
//!
//! - **持久化存储**：所有记忆数据存储在 PostgreSQL 数据库中，确保数据持久性和可靠性
//! - **线程安全**：使用 `Arc<Mutex<Client>>` 包装数据库连接，支持多线程安全访问
//! - **异步接口**：所有公开方法均为异步实现，与 tokio 运行时兼容
//! - **TLS 支持**：支持可选的 TLS 加密连接，保护数据传输安全
//! - **自动初始化**：首次连接时自动创建所需的 schema 和表结构
//!
//! # 架构设计
//!
//! ```text
//! PostgresMemory
//! ├── 客户端管理：Arc<Mutex<Client>>（线程安全的数据库连接）
//! ├── 表名管理：schema.table（带引号的限定表名）
//! └── 实现了 Memory trait 的所有方法
//! ```
//!
//! # 使用示例
//!
//! ```no_run
//! use vibe_agent::memory::postgres::PostgresMemory;
//! use vibe_agent::memory::traits::{Memory, MemoryCategory};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // 创建 PostgreSQL 内存后端实例
//!     let memory = PostgresMemory::new(
//!         "postgresql://user:pass@localhost:5432/dbname",
//!         "agent_memory",
//!         "memories",
//!         Some(30),    // 连接超时 30 秒
//!         false,       // 不使用 TLS
//!     )?;
//!
//!     // 存储记忆
//!     memory.store(
//!         "user_preference",
//!         "用户偏好深色主题",
//!         MemoryCategory::Core,
//!         None,
//!     ).await?;
//!
//!     // 召回记忆
//!     let results = memory.recall("偏好", 10, None).await?;
//!     println!("找到 {} 条相关记忆", results.len());
//!
//!     Ok(())
//! }
//! ```
//!
//! # 数据库表结构
//!
//! 模块会自动创建以下表结构（如果不存在）：
//!
//! ```sql
//! CREATE TABLE schema.table (
//!     id TEXT PRIMARY KEY,           -- 记录唯一标识符（UUID）
//!     key TEXT UNIQUE NOT NULL,      -- 记忆键名（唯一）
//!     content TEXT NOT NULL,         -- 记忆内容
//!     category TEXT NOT NULL,        -- 记忆分类
//!     created_at TIMESTAMPTZ NOT NULL,  -- 创建时间
//!     updated_at TIMESTAMPTZ NOT NULL,  -- 更新时间
//!     session_id TEXT                -- 会话标识符（可选）
//! );
//! ```
//!
//! # 安全考虑
//!
//! - **SQL 注入防护**：所有用户输入通过参数化查询传递，schema 和 table 名称经过严格验证
//! - **标识符验证**：schema 和 table 名称必须符合 PostgreSQL 标识符规范
//! - **连接超时**：支持配置连接超时，防止长时间阻塞
//! - **TLS 选项**：支持 TLS 加密连接，但默认的证书验证器接受所有证书（适用于自签名证书场景）

mod backend;
mod helpers;
mod memory_impl;
mod tls;

use anyhow::Result;
use helpers::{quote_identifier, validate_identifier};
use parking_lot::Mutex;
use postgres::Client;
use std::sync::Arc;

/// 基于 PostgreSQL 的持久化内存存储
///
/// 该结构体是 [`Memory`] trait 的 PostgreSQL 后端实现，提供可靠的记忆存储和检索功能。
/// 所有数据存储在 PostgreSQL 数据库中，支持跨会话持久化和高效的查询操作。
///
/// # 线程安全性
///
/// 内部使用 `Arc<Mutex<Client>>` 包装数据库连接，确保：
/// - 多个线程可以安全地共享同一个 `PostgresMemory` 实例
/// - 数据库操作自动串行化，避免并发冲突
/// - 连接在整个生命周期内保持活跃
///
/// # 表结构
///
/// 每条记忆记录包含以下字段：
/// - `id`: 唯一标识符（UUID 字符串）
/// - `key`: 记忆键名（必须唯一）
/// - `content`: 记忆内容
/// - `category`: 记忆分类（core/daily/conversation 或自定义）
/// - `created_at`: 创建时间（带时区的时间戳）
/// - `updated_at`: 更新时间（带时区的时间戳）
/// - `session_id`: 可选的会话标识符
///
/// # 示例
///
/// ```no_run
/// use vibe_agent::memory::postgres::PostgresMemory;
///
/// // 创建实例
/// let memory = PostgresMemory::new(
///     "postgresql://localhost/mydb",
///     "agent",
///     "memories",
///     None,
///     false,
/// )?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub struct PostgresMemory {
    /// 线程安全的数据库客户端连接
    ///
    /// 使用 `Arc` 允许在多个克隆实例间共享所有权，
    /// 使用 `Mutex` 确保同一时间只有一个线程可以访问底层连接。
    client: Arc<Mutex<Client>>,

    /// 带引号的限定表名
    ///
    /// 格式为 `"schema"."table"`，其中 schema 和 table 名称已被引号包裹，
    /// 用于在 SQL 查询中安全地引用表，即使名称包含特殊字符或保留字也能正常工作。
    qualified_table: String,
}

impl PostgresMemory {
    /// 创建新的 PostgreSQL 内存存储实例
    ///
    /// 该方法会建立到 PostgreSQL 数据库的连接，并确保所需的 schema 和表结构存在。
    /// 如果表不存在，会自动创建；如果已存在，则直接使用现有表。
    ///
    /// # 参数
    ///
    /// - `db_url`: PostgreSQL 连接字符串，格式为 `postgresql://user:pass@host:port/dbname`
    /// - `schema`: 数据库 schema 名称，用于组织表结构
    /// - `table`: 表名称，用于存储记忆数据
    /// - `connect_timeout_secs`: 可选的连接超时时间（秒），超时上限为 300 秒
    /// - `tls_mode`: 是否启用 TLS 加密连接
    ///   - `true`: 使用 TLS 加密（跳过证书验证，适用于自签名证书）
    ///   - `false`: 不使用 TLS（明文传输）
    ///
    /// # 返回值
    ///
    /// 成功时返回 `PostgresMemory` 实例，失败时返回错误信息。
    ///
    /// # 错误
    ///
    /// 以下情况会导致错误：
    /// - 连接字符串格式无效
    /// - 无法连接到数据库服务器
    /// - schema 或 table 名称不符合标识符规范
    /// - 执行初始化 SQL 语句失败
    ///
    /// # 安全性
    ///
    /// - schema 和 table 名称会经过严格验证，防止 SQL 注入
    /// - 只有符合标识符规范（字母/下划线开头，仅含字母/数字/下划线）的名称才会被接受
    ///
    /// # 示例
    ///
    /// ```no_run
    /// use vibe_agent::memory::postgres::PostgresMemory;
    ///
    /// // 基本用法
    /// let memory = PostgresMemory::new(
    ///     "postgresql://localhost/mydb",
    ///     "agent",
    ///     "memories",
    ///     None,
    ///     false,
    /// )?;
    ///
    /// // 带连接超时和 TLS
    /// let memory_tls = PostgresMemory::new(
    ///     "postgresql://user:pass@remote.host:5432/db",
    ///     "production",
    ///     "agent_memory",
    ///     Some(30),  // 30 秒超时
    ///     true,      // 启用 TLS
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
        // 验证 schema 和 table 名称，防止 SQL 注入
        validate_identifier(schema, "storage schema")?;
        validate_identifier(table, "storage table")?;

        // 将标识符用引号包裹，生成安全的 SQL 标识符
        let schema_ident = quote_identifier(schema);
        let table_ident = quote_identifier(table);
        let qualified_table = format!("{schema_ident}.{table_ident}");

        // 初始化数据库连接和表结构
        let client = Self::initialize_client(
            db_url.to_string(),
            connect_timeout_secs,
            tls_mode,
            schema_ident.clone(),
            qualified_table.clone(),
        )?;

        // 返回实例，客户端连接使用 Arc<Mutex> 包装以支持线程安全共享
        Ok(Self { client: Arc::new(Mutex::new(client)), qualified_table })
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
