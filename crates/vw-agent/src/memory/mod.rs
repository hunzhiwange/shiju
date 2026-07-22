//! 记忆模块 - 代理长期记忆存储与检索系统
//!
//! 本模块提供了多后端的记忆存储系统，支持不同类型的存储介质和检索策略。
//! 记忆系统是代理实现长期学习和上下文保持的核心组件。
//!
//! # 核心功能
//!
//! - **多后端支持**: SQLite、Lucid、PostgreSQL、MariaDB、Qdrant、Markdown 等多种存储后端
//! - **向量检索**: 基于嵌入向量的语义相似度检索
//! - **关键词检索**: 基于 BM25 算法的关键词匹配
//! - **混合检索**: 向量和关键词加权混合的检索策略
//! - **记忆卫生**: 自动清理过期或冗余的记忆条目
//! - **快照机制**: 支持记忆的导出和恢复
//! - **响应缓存**: 缓存常用响应以提升性能
//!
//! # 架构设计
//!
//! 模块采用 trait 驱动的架构设计：
//! - `Memory` trait 定义了记忆存储的统一接口
//! - 各种后端实现该 trait 提供具体的存储逻辑
//! - 工厂函数根据配置动态创建相应的后端实例
//!
//! # 示例
//!
//! ```no_run
//! use std::path::Path;
//! use vibe_agent::app::agent::config::MemoryConfig;
//! use vibe_agent::app::agent::memory::{create_memory, Memory};
//!
//! let config = MemoryConfig::default();
//! let workspace_dir = Path::new("./workspace");
//! let memory = create_memory(&config, workspace_dir, None)?;
//!
//! // 存储记忆
//! memory.remember("user_preference", "用户偏好使用中文对话", "core")?;
//!
//! // 检索记忆
//! let entries = memory.recall("用户偏好", 10)?;
//! ```

pub mod backend;
pub mod chunker;
pub mod cli;
pub mod embeddings;
pub mod hygiene;
#[cfg(not(target_arch = "wasm32"))]
pub mod lucid;
#[cfg(feature = "memory-mariadb")]
pub mod mariadb;
pub mod markdown;
pub mod none;
pub(crate) mod paths;
#[cfg(feature = "memory-postgres")]
pub mod postgres;
pub mod qdrant;
pub mod response_cache;
#[cfg(not(target_arch = "wasm32"))]
pub mod snapshot;
#[cfg(not(target_arch = "wasm32"))]
pub mod sqlite;
pub mod traits;
pub mod vector;

/// 从 backend 模块重新导出的类型和函数
#[allow(unused_imports)]
pub use backend::{
    MemoryBackendKind, MemoryBackendProfile, classify_memory_backend, default_memory_backend_key,
    memory_backend_profile, selectable_memory_backends,
};

#[cfg(not(target_arch = "wasm32"))]
pub use lucid::LucidMemory;
#[cfg(feature = "memory-mariadb")]
pub use mariadb::MariadbMemory;
pub use markdown::MarkdownMemory;
pub use none::NoneMemory;
#[cfg(feature = "memory-postgres")]
pub use postgres::PostgresMemory;
pub use qdrant::QdrantMemory;
pub use response_cache::ResponseCache;
#[cfg(not(target_arch = "wasm32"))]
pub use sqlite::SqliteMemory;
pub use traits::Memory;
#[allow(unused_imports)]
pub use traits::{MemoryCategory, MemoryEntry};

use crate::app::agent::config::{EmbeddingRouteConfig, MemoryConfig, StorageProviderConfig};
use anyhow::Context;
use std::path::Path;
use std::sync::Arc;

/// 使用构建器函数创建记忆后端（非 WASM 环境）
///
/// 这个内部函数通过传入的构建器函数来创建具体的记忆后端实例，
/// 提供了灵活的依赖注入机制，便于测试和配置管理。
///
/// # 类型参数
///
/// - `F`: SQLite 后端构建器函数类型，返回 `SqliteMemory`
/// - `G`: PostgreSQL 后端构建器函数类型，返回 `Box<dyn Memory>`
///
/// # 参数
///
/// - `backend_name`: 后端名称字符串，如 "sqlite"、"lucid"、"postgres" 等
/// - `workspace_dir`: 工作空间目录路径，用于存储本地记忆文件
/// - `sqlite_builder`: SQLite 后端构建器函数
/// - `postgres_builder`: PostgreSQL 后端构建器函数
/// - `unknown_context`: 用于错误消息的上下文信息
///
/// # 返回值
///
/// 返回装箱的 `Memory` trait 对象，具体类型由 `backend_name` 决定
///
/// # 错误
///
/// 当后端不可用或构建失败时返回错误
#[cfg(not(target_arch = "wasm32"))]
fn create_memory_with_builders<F, G>(
    backend_name: &str,
    workspace_dir: &Path,
    mut sqlite_builder: F,
    mut postgres_builder: G,
    unknown_context: &str,
) -> anyhow::Result<Box<dyn Memory>>
where
    F: FnMut() -> anyhow::Result<SqliteMemory>,
    G: FnMut() -> anyhow::Result<Box<dyn Memory>>,
{
    match classify_memory_backend(backend_name) {
        MemoryBackendKind::Sqlite => Ok(Box::new(sqlite_builder()?)),
        MemoryBackendKind::Lucid => {
            // Lucid 后端基于 SQLite，需要先创建 SQLite 后端
            let local = sqlite_builder()?;
            Ok(Box::new(LucidMemory::new(workspace_dir, local)))
        }
        MemoryBackendKind::Postgres => postgres_builder(),
        MemoryBackendKind::Mariadb => {
            anyhow::bail!("memory backend 'mariadb' is not available in this build context")
        }
        MemoryBackendKind::Qdrant | MemoryBackendKind::Markdown => {
            // Qdrant 和 Markdown 都使用 Markdown 作为本地缓存
            Ok(Box::new(MarkdownMemory::new(workspace_dir)))
        }
        MemoryBackendKind::None => Ok(Box::new(NoneMemory::new())),
        MemoryBackendKind::Unknown => {
            // 未知后端类型，降级到 Markdown 并记录警告
            tracing::warn!(
                "Unknown memory backend '{backend_name}'{unknown_context}, falling back to markdown"
            );
            Ok(Box::new(MarkdownMemory::new(workspace_dir)))
        }
    }
}

/// 使用构建器函数创建记忆后端（WASM 环境）
///
/// 在 WebAssembly 环境中，某些后端（如 SQLite、Lucid）不可用，
/// 此函数提供了受限的后端选择逻辑。
///
/// # 类型参数
///
/// - `F`: SQLite 后端构建器函数类型（WASM 环境下不可用，仅作为占位符）
/// - `G`: PostgreSQL 后端构建器函数类型，返回 `Box<dyn Memory>`
///
/// # 参数
///
/// - `backend_name`: 后端名称字符串
/// - `workspace_dir`: 工作空间目录路径
/// - `_sqlite_builder`: SQLite 后端构建器函数（WASM 环境下忽略）
/// - `postgres_builder`: PostgreSQL 后端构建器函数
/// - `unknown_context`: 用于错误消息的上下文信息
///
/// # 返回值
///
/// 返回装箱的 `Memory` trait 对象
///
/// # 错误
///
/// 当请求不支持的后端（如 SQLite、Lucid）时返回错误
#[cfg(target_arch = "wasm32")]
fn create_memory_with_builders<F, G>(
    backend_name: &str,
    workspace_dir: &Path,
    mut _sqlite_builder: F,
    mut postgres_builder: G,
    unknown_context: &str,
) -> anyhow::Result<Box<dyn Memory>>
where
    F: FnMut() -> anyhow::Result<()>,
    G: FnMut() -> anyhow::Result<Box<dyn Memory>>,
{
    match classify_memory_backend(backend_name) {
        MemoryBackendKind::Sqlite | MemoryBackendKind::Lucid => {
            anyhow::bail!("Sqlite/Lucid memory backend is not supported on WASM")
        }
        MemoryBackendKind::Postgres => postgres_builder(),
        MemoryBackendKind::Mariadb => {
            anyhow::bail!("memory backend 'mariadb' is not available in this build context")
        }
        MemoryBackendKind::Qdrant | MemoryBackendKind::Markdown => {
            // WASM 环境支持 Markdown 后端
            Ok(Box::new(MarkdownMemory::new(workspace_dir)))
        }
        MemoryBackendKind::None => Ok(Box::new(NoneMemory::new())),
        MemoryBackendKind::Unknown => {
            tracing::warn!(
                "Unknown memory backend '{backend_name}'{unknown_context}, falling back to markdown"
            );
            Ok(Box::new(MarkdownMemory::new(workspace_dir)))
        }
    }
}

/// 确定有效的记忆后端名称
///
/// 此函数解析记忆后端配置，优先使用存储提供者配置中的覆盖值，
/// 否则使用默认的记忆后端配置。
///
/// # 参数
///
/// - `memory_backend`: 配置文件中的记忆后端名称
/// - `storage_provider`: 可选的存储提供者配置，可能包含后端覆盖
///
/// # 返回值
///
/// 返回标准化的后端名称（小写、去除空白）
///
/// # 示例
///
/// ```no_run
/// use vibe_agent::app::agent::memory::effective_memory_backend_name;
/// use vibe_agent::app::agent::config::StorageProviderConfig;
///
/// let name = effective_memory_backend_name("SQLite", None);
/// assert_eq!(name, "sqlite");
/// ```
pub fn effective_memory_backend_name(
    memory_backend: &str,
    storage_provider: Option<&StorageProviderConfig>,
) -> String {
    // 如果存储提供者配置中有明确指定后端，优先使用
    if let Some(override_provider) =
        storage_provider.map(|cfg| cfg.provider.trim()).filter(|provider| !provider.is_empty())
    {
        return override_provider.to_ascii_lowercase();
    }

    // 否则使用默认的记忆后端配置
    memory_backend.trim().to_ascii_lowercase()
}

/// 判断是否为助手自动保存键名
///
/// 用于识别由模型生成的助手摘要记录。这些条目被视为不可信上下文，
/// 不应被重新注入到对话中，以避免循环引用或信息污染。
///
/// # 参数
///
/// - `key`: 记忆键名
///
/// # 返回值
///
/// 如果键名匹配 "assistant_resp" 或以 "assistant_resp_" 开头，返回 true
///
/// # 示例
///
/// ```no_run
/// use vibe_agent::app::agent::memory::is_assistant_autosave_key;
///
/// assert!(is_assistant_autosave_key("assistant_resp"));
/// assert!(is_assistant_autosave_key("assistant_resp_123"));
/// assert!(!is_assistant_autosave_key("user_preference"));
/// ```
pub fn is_assistant_autosave_key(key: &str) -> bool {
    let normalized = key.trim().to_ascii_lowercase();
    normalized == "assistant_resp" || normalized.starts_with("assistant_resp_")
}

/// 解析后的嵌入配置
///
/// 此结构体包含经过解析和验证的嵌入模型配置信息，
/// 用于创建嵌入向量提供者实例。配置可能来自默认设置或路由规则。
#[derive(Clone, PartialEq, Eq)]
pub struct ResolvedEmbeddingConfig {
    /// 嵌入提供者名称，如 "openai"、"alibaba-cn" 等
    pub provider: String,
    /// 嵌入模型名称，如 "text-embedding-v4" 或 "text-embedding-3-small"
    pub model: String,
    /// 嵌入向量维度
    pub dimensions: usize,
    /// API 密钥（可选）
    api_key: Option<String>,
}

/// 为 ResolvedEmbeddingConfig 实现安全调试输出
///
/// 使用 `finish_non_exhaustive()` 避免在调试输出中泄露 API 密钥
impl std::fmt::Debug for ResolvedEmbeddingConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedEmbeddingConfig")
            .field("provider", &self.provider)
            .field("model", &self.model)
            .field("dimensions", &self.dimensions)
            .finish_non_exhaustive()
    }
}

impl ResolvedEmbeddingConfig {
    pub fn api_key_configured(&self) -> bool {
        self.api_key.as_deref().map(str::trim).is_some_and(|value| !value.is_empty())
    }

    pub(crate) fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref().map(str::trim).filter(|value| !value.is_empty())
    }

    pub(crate) fn set_api_key(&mut self, api_key: Option<String>) {
        self.api_key = api_key.and_then(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        });
    }
}

/// 解析嵌入配置
///
/// 根据配置和嵌入路由规则，解析出最终的嵌入配置。
/// 支持通过 "hint:" 前缀指定路由规则，实现不同场景使用不同的嵌入模型。
///
/// # 参数
///
/// - `config`: 记忆配置，包含默认的嵌入设置
/// - `embedding_routes`: 嵌入路由配置列表，用于根据 hint 选择不同的嵌入模型
/// - `api_key`: 可选的默认 API 密钥
///
/// # 返回值
///
/// 返回解析后的嵌入配置。解析优先级：
/// 1. 如果嵌入模型名称以 "hint:" 开头，则查找匹配的路由配置
/// 2. 如果没有找到匹配的路由或路由配置无效，使用默认配置
///
/// # 示例
///
/// ```no_run
/// // 配置中使用 hint 前缀
/// config.embedding_model = "hint:high-dimensional".to_string();
/// // 将查找 embedding_routes 中 hint 为 "high-dimensional" 的配置
/// ```
pub fn resolve_embedding_config(
    config: &MemoryConfig,
    embedding_routes: &[EmbeddingRouteConfig],
    api_key: Option<&str>,
) -> ResolvedEmbeddingConfig {
    // 准备备用 API 密钥
    let fallback_api_key =
        api_key.map(str::trim).filter(|value| !value.is_empty()).map(str::to_string);

    // 构建备用配置（使用默认设置）
    let fallback = ResolvedEmbeddingConfig {
        provider: config.embedding_provider.trim().to_string(),
        model: config.embedding_model.trim().to_string(),
        dimensions: config.embedding_dimensions,
        api_key: fallback_api_key.clone(),
    };

    // 检查是否使用了 hint 前缀进行路由选择
    let Some(hint) = config
        .embedding_model
        .strip_prefix("hint:")
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return fallback;
    };

    // 在路由配置中查找匹配的 hint
    let Some(route) = embedding_routes.iter().find(|route| route.hint.trim() == hint) else {
        tracing::warn!(
            hint,
            "Unknown embedding route hint; falling back to [memory] embedding settings"
        );
        return fallback;
    };

    let provider = route.provider.trim();
    let model = route.model.trim();
    let dimensions = route.dimensions.unwrap_or(config.embedding_dimensions);

    // 验证路由配置的有效性
    if provider.is_empty() || model.is_empty() || dimensions == 0 {
        tracing::warn!(
            hint,
            "Invalid embedding route configuration; falling back to [memory] embedding settings"
        );
        return fallback;
    }

    // 使用路由配置中的 API 密钥，或回退到默认密钥
    let routed_api_key = route
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|value: &&str| !value.is_empty())
        .map(|value| value.to_string());

    ResolvedEmbeddingConfig {
        provider: provider.to_string(),
        model: model.to_string(),
        dimensions,
        api_key: routed_api_key.or(fallback_api_key),
    }
}

/// 使用 memory 配置和 embedding routes 创建 embedding provider。
pub fn create_embedding_provider_with_routes(
    config: &MemoryConfig,
    embedding_routes: &[EmbeddingRouteConfig],
    api_key: Option<&str>,
) -> (Arc<dyn embeddings::EmbeddingProvider>, ResolvedEmbeddingConfig) {
    let resolved = resolve_embedding_config(config, embedding_routes, api_key);
    let embedder: Arc<dyn embeddings::EmbeddingProvider> =
        Arc::from(embeddings::create_embedding_provider(
            &resolved.provider,
            resolved.api_key.as_deref(),
            &resolved.model,
            resolved.dimensions,
        ));
    (embedder, resolved)
}

/// 工厂函数：根据配置创建记忆后端
///
/// 这是最简单的记忆后端创建函数，仅使用基本配置。
/// 如需更细粒度的控制，请使用 `create_memory_with_storage` 或
/// `create_memory_with_storage_and_routes`。
///
/// # 参数
///
/// - `config`: 记忆配置，指定后端类型和嵌入设置
/// - `workspace_dir`: 工作空间目录，用于存储本地记忆文件
/// - `api_key`: 可选的 API 密钥，用于嵌入模型调用
///
/// # 返回值
///
/// 返回装箱的 `Memory` trait 对象
///
/// # 错误
///
/// 当后端配置无效或创建失败时返回错误
///
/// # 示例
///
/// ```no_run
/// use std::path::Path;
/// use vibe_agent::app::agent::config::MemoryConfig;
/// use vibe_agent::app::agent::memory::create_memory;
///
/// let config = MemoryConfig::default();
/// let workspace_dir = Path::new("./workspace");
/// let memory = create_memory(&config, workspace_dir, None)?;
/// ```
pub fn create_memory(
    config: &MemoryConfig,
    workspace_dir: &Path,
    api_key: Option<&str>,
) -> anyhow::Result<Box<dyn Memory>> {
    create_memory_with_storage_and_routes(config, &[], None, workspace_dir, api_key)
}

/// 工厂函数：创建记忆后端，支持存储提供者覆盖
///
/// 此函数允许通过存储提供者配置覆盖默认的后端选择，
/// 适用于需要动态切换后端的场景。
///
/// # 参数
///
/// - `config`: 记忆配置
/// - `storage_provider`: 可选的存储提供者配置，可覆盖后端选择
/// - `workspace_dir`: 工作空间目录
/// - `api_key`: 可选的 API 密钥
///
/// # 返回值
///
/// 返回装箱的 `Memory` trait 对象
///
/// # 示例
///
/// ```no_run
/// use std::path::Path;
/// use vibe_agent::app::agent::config::{MemoryConfig, StorageProviderConfig};
/// use vibe_agent::app::agent::memory::create_memory_with_storage;
///
/// let config = MemoryConfig::default();
/// let storage_config = StorageProviderConfig {
///     provider: "postgres".to_string(),
///     // ... 其他配置
/// };
/// let workspace_dir = Path::new("./workspace");
/// let memory = create_memory_with_storage(&config, Some(&storage_config), workspace_dir, None)?;
/// ```
pub fn create_memory_with_storage(
    config: &MemoryConfig,
    storage_provider: Option<&StorageProviderConfig>,
    workspace_dir: &Path,
    api_key: Option<&str>,
) -> anyhow::Result<Box<dyn Memory>> {
    create_memory_with_storage_and_routes(config, &[], storage_provider, workspace_dir, api_key)
}

/// 工厂函数：创建记忆后端，支持存储提供者覆盖和嵌入路由
///
/// 这是功能最完整的记忆后端创建函数，支持所有配置选项。
/// 根据配置执行以下步骤：
///
/// 1. 解析有效的后端名称
/// 2. 执行记忆卫生清理（如配置启用）
/// 3. 执行记忆快照导出（如配置启用）
/// 4. 从快照恢复记忆（冷启动时，如配置启用）
/// 5. 创建具体的记忆后端实例
///
/// # 参数
///
/// - `config`: 记忆配置，包含后端类型、嵌入设置、卫生规则等
/// - `embedding_routes`: 嵌入路由配置列表，用于根据 hint 选择不同的嵌入模型
/// - `storage_provider`: 可选的存储提供者配置，可覆盖后端选择
/// - `workspace_dir`: 工作空间目录，用于存储本地记忆文件
/// - `api_key`: 可选的 API 密钥，用于嵌入模型调用
///
/// # 返回值
///
/// 返回装箱的 `Memory` trait 对象
///
/// # 错误
///
/// 当后端配置无效、所需配置缺失或创建失败时返回错误
///
/// # 后端支持
///
/// - **SQLite**: 本地文件数据库，支持向量检索（非 WASM）
/// - **Lucid**: 基于SQLite的高级后端，支持快照和恢复（非 WASM）
/// - **PostgreSQL**: 远程数据库，需要配置连接信息（需启用 feature）
/// - **MariaDB**: 远程数据库，需要配置连接信息（需启用 feature）
/// - **Qdrant**: 向量数据库，支持高性能向量检索
/// - **Markdown**: 纯文本文件存储，简单可靠
/// - **None**: 无操作后端，禁用记忆功能
///
/// # 示例
///
/// ```no_run
/// use std::path::Path;
/// use vibe_agent::app::agent::config::{MemoryConfig, StorageProviderConfig, EmbeddingRouteConfig};
/// use vibe_agent::app::agent::memory::create_memory_with_storage_and_routes;
///
/// let config = MemoryConfig::default();
/// let routes = vec![EmbeddingRouteConfig {
///     hint: "high-dim".to_string(),
///     provider: "openai".to_string(),
///     model: "text-embedding-3-large".to_string(),
///     dimensions: Some(3072),
///     api_key: None,
/// }];
/// let workspace_dir = Path::new("./workspace");
/// let memory = create_memory_with_storage_and_routes(
///     &config,
///     &routes,
///     None,
///     workspace_dir,
///     Some("sk-..."),
/// )?;
/// ```
pub fn create_memory_with_storage_and_routes(
    config: &MemoryConfig,
    embedding_routes: &[EmbeddingRouteConfig],
    storage_provider: Option<&StorageProviderConfig>,
    workspace_dir: &Path,
    api_key: Option<&str>,
) -> anyhow::Result<Box<dyn Memory>> {
    let backend_name = effective_memory_backend_name(&config.backend, storage_provider);
    let backend_kind = classify_memory_backend(&backend_name);
    let resolved_embedding = resolve_embedding_config(config, embedding_routes, api_key);

    // 尽力执行记忆卫生/保留清理（由状态文件控制节流）
    if let Err(e) = hygiene::run_if_due(config, workspace_dir) {
        tracing::warn!("memory hygiene skipped: {e}");
    }

    // 如果启用了 snapshot_on_hygiene，在卫生清理期间导出核心记忆
    #[cfg(not(target_arch = "wasm32"))]
    if config.snapshot_enabled
        && config.snapshot_on_hygiene
        && matches!(backend_kind, MemoryBackendKind::Sqlite | MemoryBackendKind::Lucid)
    {
        if let Err(e) = snapshot::export_snapshot(workspace_dir) {
            tracing::warn!("memory snapshot skipped: {e}");
        }
    }

    // 自动水合：如果 brain.db 丢失但 MEMORY_SNAPSHOT.md 存在，
    // 在创建后端之前从快照恢复"灵魂"
    #[cfg(not(target_arch = "wasm32"))]
    if config.auto_hydrate
        && matches!(backend_kind, MemoryBackendKind::Sqlite | MemoryBackendKind::Lucid)
        && snapshot::should_hydrate(workspace_dir)
    {
        tracing::info!("🧬 Cold boot detected — hydrating from MEMORY_SNAPSHOT.md");
        match snapshot::hydrate_from_snapshot(workspace_dir) {
            Ok(count) => {
                if count > 0 {
                    tracing::info!("🧬 Hydrated {count} core memories from snapshot");
                }
            }
            Err(e) => {
                tracing::warn!("memory hydration failed: {e}");
            }
        }
    }

    /// 构建 SQLite 记忆后端
    #[cfg(not(target_arch = "wasm32"))]
    fn build_sqlite_memory(
        config: &MemoryConfig,
        workspace_dir: &Path,
        resolved_embedding: &ResolvedEmbeddingConfig,
    ) -> anyhow::Result<SqliteMemory> {
        let embedder: Arc<dyn embeddings::EmbeddingProvider> =
            Arc::from(embeddings::create_embedding_provider(
                &resolved_embedding.provider,
                resolved_embedding.api_key.as_deref(),
                &resolved_embedding.model,
                resolved_embedding.dimensions,
            ));

        #[allow(clippy::cast_possible_truncation)]
        let mem = SqliteMemory::with_embedder(
            workspace_dir,
            embedder,
            config.vector_weight as f32,
            config.keyword_weight as f32,
            config.embedding_cache_size,
            config.sqlite_open_timeout_secs,
        )?;
        Ok(mem)
    }

    /// 构建 PostgreSQL 记忆后端（需要启用 feature）
    #[cfg(feature = "memory-postgres")]
    fn build_postgres_memory(
        storage_provider: Option<&StorageProviderConfig>,
    ) -> anyhow::Result<Box<dyn Memory>> {
        let storage_provider = storage_provider
            .context("memory backend 'postgres' requires [storage.provider.config] settings")?;
        let db_url = storage_provider
            .db_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .context(
                "memory backend 'postgres' requires [storage.provider.config].db_url (or dbURL)",
            )?;

        let memory = PostgresMemory::new(
            db_url,
            &storage_provider.schema,
            &storage_provider.table,
            storage_provider.connect_timeout_secs,
            storage_provider.tls,
        )?;
        Ok(Box::new(memory))
    }

    /// PostgreSQL 后端未启用时的占位函数
    #[cfg(not(feature = "memory-postgres"))]
    fn build_postgres_memory(
        _storage_provider: Option<&StorageProviderConfig>,
    ) -> anyhow::Result<Box<dyn Memory>> {
        anyhow::bail!(
            "memory backend 'postgres' requested but this build was compiled without `memory-postgres`; rebuild with `--features memory-postgres`"
        );
    }

    /// 构建 MariaDB 记忆后端（需要启用 feature）
    #[cfg(feature = "memory-mariadb")]
    fn build_mariadb_memory(
        storage_provider: Option<&StorageProviderConfig>,
    ) -> anyhow::Result<Box<dyn Memory>> {
        let storage_provider = storage_provider
            .context("memory backend 'mariadb' requires [storage.provider.config] settings")?;
        let db_url = storage_provider
            .db_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .context(
                "memory backend 'mariadb' requires [storage.provider.config].db_url (or dbURL)",
            )?;

        let memory = MariadbMemory::new(
            db_url,
            &storage_provider.schema,
            &storage_provider.table,
            storage_provider.connect_timeout_secs,
            storage_provider.tls,
        )?;
        Ok(Box::new(memory))
    }

    /// MariaDB 后端未启用时的占位函数
    #[cfg(not(feature = "memory-mariadb"))]
    fn build_mariadb_memory(
        _storage_provider: Option<&StorageProviderConfig>,
    ) -> anyhow::Result<Box<dyn Memory>> {
        anyhow::bail!(
            "memory backend 'mariadb' requested but this build was compiled without `memory-mariadb`; rebuild with `--features memory-mariadb`"
        );
    }

    // Qdrant 后端需要特殊配置
    if matches!(backend_kind, MemoryBackendKind::Qdrant) {
        let url = config
            .qdrant
            .url
            .clone()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| std::env::var("QDRANT_URL").ok())
            .filter(|s| !s.trim().is_empty())
            .context(
                "Qdrant memory backend requires url in [memory.qdrant] or QDRANT_URL env var",
            )?;
        let collection = std::env::var("QDRANT_COLLECTION")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| config.qdrant.collection.clone());
        let qdrant_api_key = config
            .qdrant
            .api_key
            .clone()
            .or_else(|| std::env::var("QDRANT_API_KEY").ok())
            .filter(|s| !s.trim().is_empty());
        let embedder: Arc<dyn embeddings::EmbeddingProvider> =
            Arc::from(embeddings::create_embedding_provider(
                &resolved_embedding.provider,
                resolved_embedding.api_key.as_deref(),
                &resolved_embedding.model,
                resolved_embedding.dimensions,
            ));
        tracing::info!(
            "📦 Qdrant memory backend configured (url: {}, collection: {})",
            url,
            collection
        );
        return Ok(Box::new(QdrantMemory::new_lazy(&url, &collection, qdrant_api_key, embedder)));
    }

    // MariaDB 后端需要通过构建器创建
    if matches!(backend_kind, MemoryBackendKind::Mariadb) {
        return build_mariadb_memory(storage_provider);
    }

    // 准备 SQLite 构建器（WASM 环境下为空操作）
    #[cfg(not(target_arch = "wasm32"))]
    let sqlite_builder = || build_sqlite_memory(config, workspace_dir, &resolved_embedding);
    #[cfg(target_arch = "wasm32")]
    let sqlite_builder = || Ok(());

    // 使用构建器创建后端
    create_memory_with_builders(
        &backend_name,
        workspace_dir,
        sqlite_builder,
        || build_postgres_memory(storage_provider),
        "",
    )
}

/// 为迁移操作创建记忆后端（非 WASM 环境）
///
/// 此函数专门用于记忆迁移场景，提供简化的后端创建逻辑。
/// 某些后端（如 PostgreSQL、MariaDB）不支持直接迁移，
/// "none" 后端禁用持久化，也不支持迁移。
///
/// # 参数
///
/// - `backend`: 目标后端名称
/// - `workspace_dir`: 工作空间目录
///
/// # 返回值
///
/// 返回装箱的 `Memory` trait 对象
///
/// # 错误
///
/// 当请求不支持迁移的后端时返回错误：
/// - "none": 禁用持久化，无法迁移
/// - "postgres" / "mariadb": SQL 后端不支持直接迁移
///
/// # 支持的迁移目标
///
/// - SQLite
/// - Lucid
/// - Markdown
///
/// # 示例
///
/// ```no_run
/// use std::path::Path;
/// use vibe_agent::app::agent::memory::create_memory_for_migration;
///
/// let workspace_dir = Path::new("./workspace");
/// let memory = create_memory_for_migration("sqlite", workspace_dir)?;
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub fn create_memory_for_migration(
    backend: &str,
    workspace_dir: &Path,
) -> anyhow::Result<Box<dyn Memory>> {
    // "none" 后端禁用持久化，无法进行迁移
    if matches!(classify_memory_backend(backend), MemoryBackendKind::None) {
        anyhow::bail!(
            "memory backend 'none' disables persistence; choose sqlite, lucid, or markdown before migration"
        );
    }

    // PostgreSQL 和 MariaDB 不支持直接迁移
    if matches!(
        classify_memory_backend(backend),
        MemoryBackendKind::Postgres | MemoryBackendKind::Mariadb
    ) {
        anyhow::bail!(
            "memory migration for SQL backends ('postgres' / 'mariadb') is unsupported; migrate with sqlite or markdown first"
        );
    }

    create_memory_with_builders(
        backend,
        workspace_dir,
        || SqliteMemory::new(workspace_dir),
        || anyhow::bail!("postgres backend is not available in migration context"),
        " during migration",
    )
}

/// WASM 环境下不支持记忆迁移
///
/// WebAssembly 环境下的记忆迁移功能不可用。
#[cfg(target_arch = "wasm32")]
pub fn create_memory_for_migration(
    _backend: &str,
    _workspace_dir: &Path,
) -> anyhow::Result<Box<dyn Memory>> {
    anyhow::bail!("Memory migration is not supported on WASM")
}

/// 工厂函数：根据配置创建可选的响应缓存
///
/// 响应缓存用于缓存常用响应，减少重复计算，提升性能。
/// 当配置中禁用响应缓存时，返回 None。
///
/// # 参数
///
/// - `config`: 记忆配置，包含缓存相关设置
/// - `workspace_dir`: 工作空间目录，用于存储缓存文件
///
/// # 返回值
///
/// 如果启用了响应缓存且创建成功，返回 `Some(ResponseCache)`，否则返回 `None`
///
/// # 配置项
///
/// - `response_cache_enabled`: 是否启用响应缓存
/// - `response_cache_ttl_minutes`: 缓存有效期（分钟）
/// - `response_cache_max_entries`: 最大缓存条目数
///
/// # 示例
///
/// ```no_run
/// use std::path::Path;
/// use vibe_agent::app::agent::config::MemoryConfig;
/// use vibe_agent::app::agent::memory::create_response_cache;
///
/// let config = MemoryConfig::default();
/// let workspace_dir = Path::new("./workspace");
///
/// if let Some(cache) = create_response_cache(&config, workspace_dir) {
///     // 使用缓存
/// }
/// ```
pub fn create_response_cache(config: &MemoryConfig, workspace_dir: &Path) -> Option<ResponseCache> {
    if !config.response_cache_enabled {
        return None;
    }

    match ResponseCache::new(
        workspace_dir,
        config.response_cache_ttl_minutes,
        config.response_cache_max_entries,
    ) {
        Ok(cache) => {
            tracing::info!(
                "💾 Response cache enabled (TTL: {}min, max: {} entries)",
                config.response_cache_ttl_minutes,
                config.response_cache_max_entries
            );
            Some(cache)
        }
        Err(e) => {
            tracing::warn!("Response cache disabled due to error: {e}");
            None
        }
    }
}

/// 单元测试模块
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
