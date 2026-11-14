//! 内存后端配置模块
//!
//! 本模块提供内存后端的类型定义、配置配置和分类功能。
//! 支持多种存储后端，包括 SQLite、Markdown、PostgreSQL、MariaDB、Qdrant 等。
//!
//! # 主要功能
//!
//! - 定义内存后端类型枚举（`MemoryBackendKind`）
//! - 提供内存后端配置信息（`MemoryBackendProfile`）
//! - 后端类型分类和识别
//! - 获取可选和默认的后端配置
//!
//! # 示例
//!
//! ```rust
//! use memory_types::memory::backend::{classify_memory_backend, memory_backend_profile};
//!
//! let kind = classify_memory_backend("sqlite");
//! let profile = memory_backend_profile("sqlite");
//! println!("Backend: {}", profile.label);
//! ```

/// 内存后端类型枚举
///
/// 定义所有支持的内存存储后端类型。
/// 用于在运行时识别和分类不同的存储实现。
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MemoryBackendKind {
    /// SQLite 数据库后端（推荐）
    /// 支持向量搜索、混合查询、嵌入式存储
    Sqlite,

    /// Lucid 内存桥接后端
    /// 与本地 lucid-memory CLI 同步，保留 SQLite 回退
    Lucid,

    /// PostgreSQL 远程数据库后端
    /// 通过 [storage.provider.config] 配置远程持久化存储
    Postgres,

    /// MariaDB/MySQL 远程数据库后端
    /// 通过 [storage.provider.config] 配置远程持久化存储
    Mariadb,

    /// Qdrant 向量数据库后端
    /// 通过 [memory.qdrant] 配置语义搜索
    Qdrant,

    /// Markdown 文件后端
    /// 简单、人类可读、无依赖
    Markdown,

    /// 禁用持久化内存
    None,

    /// 未知/自定义后端
    Unknown,
}

/// 内存后端配置配置
///
/// 描述特定内存后端的特性和默认行为。
/// 用于在配置和运行时决策中提供后端的元信息。
///
/// # 字段说明
///
/// - `key`: 后端的唯一标识符（用于配置文件）
/// - `label`: 用户友好的显示标签
/// - `auto_save_default`: 自动保存的默认设置
/// - `uses_sqlite_hygiene`: 是否使用 SQLite 卫生机制
/// - `sqlite_based`: 是否基于 SQLite 实现
/// - `optional_dependency`: 是否需要可选依赖
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct MemoryBackendProfile {
    /// 后端的唯一标识符键
    /// 用于配置文件中指定后端类型
    pub key: &'static str,

    /// 用户友好的显示标签
    /// 包含后端描述和特性说明
    pub label: &'static str,

    /// 自动保存的默认值
    /// 指定是否默认启用自动保存功能
    pub auto_save_default: bool,

    /// 是否使用 SQLite 卫生机制
    /// 某些基于 SQLite 的后端需要额外的卫生检查
    pub uses_sqlite_hygiene: bool,

    /// 是否基于 SQLite 实现
    /// 用于判断是否需要 SQLite 相关的初始化
    pub sqlite_based: bool,

    /// 是否为可选依赖
    /// 标识该后端是否需要额外的可选依赖包
    pub optional_dependency: bool,
}

/// SQLite 后端配置配置
///
/// 推荐的默认后端，支持向量搜索和混合查询。
const SQLITE_PROFILE: MemoryBackendProfile = MemoryBackendProfile {
    key: "sqlite",
    label: "SQLite with Vector Search (recommended) — fast, hybrid search, embeddings",
    auto_save_default: true,
    uses_sqlite_hygiene: true,
    sqlite_based: true,
    optional_dependency: false,
};

/// Lucid 内存桥接后端配置配置
///
/// 与本地 lucid-memory CLI 同步，保留 SQLite 回退机制。
const LUCID_PROFILE: MemoryBackendProfile = MemoryBackendProfile {
    key: "lucid",
    label: "Lucid Memory bridge — sync with local lucid-memory CLI, keep SQLite fallback",
    auto_save_default: true,
    uses_sqlite_hygiene: true,
    sqlite_based: true,
    optional_dependency: true,
};

/// Markdown 文件后端配置配置
///
/// 简单的文件存储方式，人类可读，无需额外依赖。
const MARKDOWN_PROFILE: MemoryBackendProfile = MemoryBackendProfile {
    key: "markdown",
    label: "Markdown Files — simple, human-readable, no dependencies",
    auto_save_default: true,
    uses_sqlite_hygiene: false,
    sqlite_based: false,
    optional_dependency: false,
};

/// PostgreSQL 远程数据库后端配置配置
///
/// 通过 [storage.provider.config] 配置的远程持久化存储。
const POSTGRES_PROFILE: MemoryBackendProfile = MemoryBackendProfile {
    key: "postgres",
    label: "PostgreSQL — remote durable storage via [storage.provider.config]",
    auto_save_default: true,
    uses_sqlite_hygiene: false,
    sqlite_based: false,
    optional_dependency: true,
};

/// MariaDB/MySQL 远程数据库后端配置配置
///
/// 通过 [storage.provider.config] 配置的远程持久化存储。
const MARIADB_PROFILE: MemoryBackendProfile = MemoryBackendProfile {
    key: "mariadb",
    label: "MariaDB/MySQL — remote durable storage via [storage.provider.config]",
    auto_save_default: true,
    uses_sqlite_hygiene: false,
    sqlite_based: false,
    optional_dependency: true,
};

/// Qdrant 向量数据库后端配置配置
///
/// 通过 [memory.qdrant] 配置的语义搜索向量数据库。
const QDRANT_PROFILE: MemoryBackendProfile = MemoryBackendProfile {
    key: "qdrant",
    label: "Qdrant — vector database for semantic search via [memory.qdrant]",
    auto_save_default: true,
    uses_sqlite_hygiene: false,
    sqlite_based: false,
    optional_dependency: false,
};

/// 禁用持久化内存的配置配置
///
/// 完全禁用内存持久化功能。
const NONE_PROFILE: MemoryBackendProfile = MemoryBackendProfile {
    key: "none",
    label: "None — disable persistent memory",
    auto_save_default: false,
    uses_sqlite_hygiene: false,
    sqlite_based: false,
    optional_dependency: false,
};

/// 自定义后端配置配置
///
/// 扩展点，用于支持自定义内存后端实现。
const CUSTOM_PROFILE: MemoryBackendProfile = MemoryBackendProfile {
    key: "custom",
    label: "Custom backend — extension point",
    auto_save_default: true,
    uses_sqlite_hygiene: false,
    sqlite_based: false,
    optional_dependency: false,
};

/// 可选择的内存后端列表
///
/// 包含用户在配置中可以选择的所有内存后端。
/// 仅包含主要和推荐的后端选项。
const SELECTABLE_MEMORY_BACKENDS: [MemoryBackendProfile; 4] =
    [SQLITE_PROFILE, LUCID_PROFILE, MARKDOWN_PROFILE, NONE_PROFILE];

/// 获取可选择的内存后端列表
///
/// 返回用户可以在配置中选择的所有内存后端配置配置。
/// 这些是经过筛选的主要后端选项。
///
/// # 返回值
///
/// 返回静态生命周期的内存后端配置配置切片。
///
/// # 示例
///
/// ```rust
/// use memory_types::memory::backend::selectable_memory_backends;
///
/// let backends = selectable_memory_backends();
/// for backend in backends {
///     println!("{}: {}", backend.key, backend.label);
/// }
/// ```
pub fn selectable_memory_backends() -> &'static [MemoryBackendProfile] {
    &SELECTABLE_MEMORY_BACKENDS
}

/// 获取默认内存后端的键
///
/// 返回系统默认使用的内存后端标识符。
///
/// # 返回值
///
/// 返回默认后端的键字符串（"sqlite"）。
///
/// # 示例
///
/// ```rust
/// use memory_types::memory::backend::default_memory_backend_key;
///
/// let default_key = default_memory_backend_key();
/// assert_eq!(default_key, "sqlite");
/// ```
pub fn default_memory_backend_key() -> &'static str {
    SQLITE_PROFILE.key
}

/// 分类内存后端类型
///
/// 根据后端标识符字符串判断并返回对应的内存后端类型枚举。
///
/// # 参数
///
/// - `backend`: 后端标识符字符串（如 "sqlite"、"postgres" 等）
///
/// # 返回值
///
/// 返回对应的 `MemoryBackendKind` 枚举变体。
/// 如果无法识别，返回 `MemoryBackendKind::Unknown`。
///
/// # 示例
///
/// ```rust
/// use memory_types::memory::backend::{classify_memory_backend, MemoryBackendKind};
///
/// assert_eq!(classify_memory_backend("sqlite"), MemoryBackendKind::Sqlite);
/// assert_eq!(classify_memory_backend("postgres"), MemoryBackendKind::Postgres);
/// assert_eq!(classify_memory_backend("mysql"), MemoryBackendKind::Mariadb);
/// assert_eq!(classify_memory_backend("unknown"), MemoryBackendKind::Unknown);
/// ```
pub fn classify_memory_backend(backend: &str) -> MemoryBackendKind {
    match backend {
        "sqlite" => MemoryBackendKind::Sqlite,
        "lucid" => MemoryBackendKind::Lucid,
        "postgres" => MemoryBackendKind::Postgres,
        "mariadb" | "mysql" => MemoryBackendKind::Mariadb,
        "qdrant" => MemoryBackendKind::Qdrant,
        "markdown" => MemoryBackendKind::Markdown,
        "none" => MemoryBackendKind::None,
        _ => MemoryBackendKind::Unknown,
    }
}

/// 获取内存后端的配置配置
///
/// 根据后端标识符字符串返回对应的配置配置信息。
///
/// # 参数
///
/// - `backend`: 后端标识符字符串（如 "sqlite"、"postgres" 等）
///
/// # 返回值
///
/// 返回对应的 `MemoryBackendProfile` 配置配置。
/// 如果后端无法识别，返回 `CUSTOM_PROFILE`。
///
/// # 示例
///
/// ```rust
/// use memory_types::memory::backend::memory_backend_profile;
///
/// let profile = memory_backend_profile("sqlite");
/// assert_eq!(profile.key, "sqlite");
/// assert!(profile.sqlite_based);
/// assert!(profile.auto_save_default);
/// ```
pub fn memory_backend_profile(backend: &str) -> MemoryBackendProfile {
    match classify_memory_backend(backend) {
        MemoryBackendKind::Sqlite => SQLITE_PROFILE,
        MemoryBackendKind::Lucid => LUCID_PROFILE,
        MemoryBackendKind::Postgres => POSTGRES_PROFILE,
        MemoryBackendKind::Mariadb => MARIADB_PROFILE,
        MemoryBackendKind::Qdrant => QDRANT_PROFILE,
        MemoryBackendKind::Markdown => MARKDOWN_PROFILE,
        MemoryBackendKind::None => NONE_PROFILE,
        MemoryBackendKind::Unknown => CUSTOM_PROFILE,
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
