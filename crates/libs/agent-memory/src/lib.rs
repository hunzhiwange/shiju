//! 记忆后端组合、嵌入与命令行服务。

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
pub mod paths;
pub mod qdrant;
pub mod response_cache;
#[cfg(not(target_arch = "wasm32"))]
pub mod snapshot;
#[cfg(not(target_arch = "wasm32"))]
pub mod sqlite;
pub mod vector;
mod factory;
pub use factory::*;
pub use qdrant::QdrantMemory;
pub use response_cache::ResponseCache;
#[cfg(feature = "memory-postgres")]
pub mod postgres;

pub use memory_types::memory::traits;
pub use traits::{Memory, MemoryCategory, MemoryEntry};
pub use backend::*;
pub use markdown::MarkdownMemory;
pub use none::NoneMemory;
#[cfg(not(target_arch = "wasm32"))]
pub use lucid::LucidMemory;
#[cfg(not(target_arch = "wasm32"))]
pub use memory_sqlite::memory::sqlite::SqliteMemory;
#[cfg(feature = "memory-postgres")]
pub use postgres::PostgresMemory;
#[cfg(feature = "memory-mariadb")]
pub use mariadb::MariadbMemory;

use agent_config::schema::StorageProviderConfig;
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
pub fn create_memory_with_builders<F, G>(
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

#[cfg(target_arch = "wasm32")]
pub fn create_memory_with_builders<F, G>(
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

#[cfg(target_arch = "wasm32")]
pub fn create_memory_for_migration(
    _backend: &str,
    _workspace_dir: &Path,
) -> anyhow::Result<Box<dyn Memory>> {
    anyhow::bail!("Memory migration is not supported on WASM")
}
