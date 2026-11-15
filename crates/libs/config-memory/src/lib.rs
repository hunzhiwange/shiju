use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// 持久化存储配置（`[storage]` 配置段）。
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub struct StorageConfig {
    /// 存储提供方配置，例如 `sqlite`、`postgres`、`mariadb`。
    #[serde(default)]
    pub provider: StorageProviderSection,
}

/// 存储提供方配置段的包装结构。
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub struct StorageProviderSection {
    /// 存储提供方后端设置。
    #[serde(default)]
    pub config: StorageProviderConfig,
}

/// 存储提供方后端配置，例如 postgres 或 mariadb 的连接参数。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StorageProviderConfig {
    /// 存储引擎标识，例如 `postgres`、`mariadb`、`sqlite`。
    #[serde(default)]
    pub provider: String,

    /// 远端存储提供方的连接 URL。
    /// 兼容旧别名：`dbURL`、`database_url`、`databaseUrl`。
    #[serde(default, alias = "dbURL", alias = "database_url", alias = "databaseUrl")]
    pub db_url: Option<String>,

    /// SQL 后端使用的数据库 schema。
    #[serde(default = "default_storage_schema")]
    pub schema: String,

    /// 记忆条目使用的数据表名称。
    #[serde(default = "default_storage_table")]
    pub table: String,

    /// 远端存储提供方的可选连接超时，单位为秒。
    #[serde(default)]
    pub connect_timeout_secs: Option<u64>,

    /// 是否为 SQL 远程连接启用 TLS。
    ///
    /// `true` 表示向后端请求 TLS；对于 PostgreSQL 还会跳过证书校验，适合
    /// 自签名证书及部分托管数据库。
    /// `false`（默认）表示使用纯 TCP，以保持向后兼容。
    #[serde(default)]
    pub tls: bool,
}

fn default_storage_schema() -> String {
    "public".into()
}

fn default_storage_table() -> String {
    "memories".into()
}

impl Default for StorageProviderConfig {
    fn default() -> Self {
        Self {
            provider: String::new(),
            db_url: None,
            schema: default_storage_schema(),
            table: default_storage_table(),
            connect_timeout_secs: None,
            tls: false,
        }
    }
}

/// Qdrant 向量数据库后端配置（`[memory.qdrant]`）。
/// 当 `[memory].backend = "qdrant"` 时使用。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QdrantConfig {
    /// Qdrant 服务 URL，例如 `http://localhost:6333`。
    /// 未设置时回退到环境变量 `QDRANT_URL`。
    #[serde(default)]
    pub url: Option<String>,
    /// 用于存储记忆的 Qdrant collection 名称。
    /// 未设置时回退到环境变量 `QDRANT_COLLECTION`，否则默认值为 `vibewindow_memories`。
    #[serde(default = "default_qdrant_collection")]
    pub collection: String,
    /// Qdrant Cloud 或受保护实例使用的可选 API Key。
    /// 未设置时回退到环境变量 `QDRANT_API_KEY`。
    #[serde(default)]
    pub api_key: Option<String>,
}

fn default_qdrant_collection() -> String {
    "vibewindow_memories".into()
}

impl Default for QdrantConfig {
    fn default() -> Self {
        Self { url: None, collection: default_qdrant_collection(), api_key: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[allow(clippy::struct_excessive_bools)]
pub struct MemoryConfig {
    /// 记忆后端类型：`sqlite`、`lucid`、`postgres`、`mariadb`、`qdrant`、`markdown`、`none`。
    ///
    /// `none` 表示显式关闭记忆能力。
    /// `postgres` / `mariadb` 需要配置 `[storage.provider.config]` 中的 `db_url`
    ///（兼容 `dbURL` 别名）。
    /// `qdrant` 使用 `[memory.qdrant]` 配置，或回退到 `QDRANT_URL` 环境变量。
    pub backend: String,
    /// 是否自动将用户在对话中明确表达的输入保存到记忆中，不包含助手输出。
    pub auto_save: bool,
    /// 是否执行记忆与会话的清理流程（归档与保留期清理）。
    #[serde(default = "default_hygiene_enabled")]
    pub hygiene_enabled: bool,
    /// 超过该天数的日记忆或会话文件会被归档。
    #[serde(default = "default_archive_after_days")]
    pub archive_after_days: u32,
    /// 超过该天数的归档文件会被清除。
    #[serde(default = "default_purge_after_days")]
    pub purge_after_days: u32,
    /// 对 sqlite 后端，超过该天数的会话记录行会被裁剪。
    #[serde(default = "default_conversation_retention_days")]
    pub conversation_retention_days: u32,
    /// Embedding 提供方：`none`、`openai`、`alibaba`、`alibaba-cn` 或 `custom:URL`。
    #[serde(default = "default_embedding_provider")]
    pub embedding_provider: String,
    /// Embedding 模型名称，例如 `text-embedding-v4` 或 `text-embedding-3-small`。
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,
    /// Embedding 向量维度。
    #[serde(default = "default_embedding_dims")]
    pub embedding_dimensions: usize,
    /// 混合检索中向量相似度的权重，范围为 `0.0–1.0`。
    #[serde(default = "default_vector_weight")]
    pub vector_weight: f64,
    /// 混合检索中关键词 BM25 的权重，范围为 `0.0–1.0`。
    #[serde(default = "default_keyword_weight")]
    pub keyword_weight: f64,
    /// 记忆进入上下文所需的最小混合分数，范围为 `0.0–1.0`。
    /// 低于该阈值的记忆会被丢弃，以避免无关上下文渗入对话。默认值为 `0.4`。
    #[serde(default = "default_min_relevance_score")]
    pub min_relevance_score: f64,
    /// Embedding 缓存触发 LRU 淘汰前允许保留的最大条目数。
    #[serde(default = "default_cache_size")]
    pub embedding_cache_size: usize,
    /// 文档切分时每个分块允许的最大 token 数。
    #[serde(default = "default_chunk_size")]
    pub chunk_max_tokens: usize,

    // ── 响应缓存（减少重复提示词的 token 消耗） ──────
    /// 是否启用 LLM 响应缓存，以避免重复提示词重复计费。
    #[serde(default)]
    pub response_cache_enabled: bool,
    /// 缓存响应的 TTL，单位为分钟，默认值为 `60`。
    #[serde(default = "default_response_cache_ttl")]
    pub response_cache_ttl_minutes: u32,
    /// 触发 LRU 淘汰前允许缓存的响应条目上限，默认值为 `5000`。
    #[serde(default = "default_response_cache_max")]
    pub response_cache_max_entries: usize,

    // ── 记忆快照（导出到 Markdown 的核心备份） ─────────────
    /// 是否定期将核心记忆导出到 `MEMORY_SNAPSHOT.md`。
    #[serde(default)]
    pub snapshot_enabled: bool,
    /// 是否在清理流程中顺带执行快照导出（由 heartbeat 驱动）。
    #[serde(default)]
    pub snapshot_on_hygiene: bool,
    /// 当 `brain.db` 缺失时，是否自动从 `MEMORY_SNAPSHOT.md` 恢复。
    #[serde(default = "default_true")]
    pub auto_hydrate: bool,

    // ── SQLite 后端选项 ─────────────────────────────────
    /// 对 sqlite 后端，打开数据库时最长等待秒数，例如文件被锁时。
    /// `None` 表示无限等待（默认）。建议最大值为 `300`。
    #[serde(default)]
    pub sqlite_open_timeout_secs: Option<u64>,

    // ── Qdrant 后端选项 ─────────────────────────────────
    /// Qdrant 向量数据库后端配置。
    /// 仅在 `backend = "qdrant"` 时使用。
    #[serde(default)]
    pub qdrant: QdrantConfig,
}

fn default_embedding_provider() -> String {
    "none".into()
}
fn default_hygiene_enabled() -> bool {
    true
}
fn default_archive_after_days() -> u32 {
    7
}
fn default_purge_after_days() -> u32 {
    30
}
fn default_conversation_retention_days() -> u32 {
    30
}
fn default_embedding_model() -> String {
    "text-embedding-3-small".into()
}
fn default_embedding_dims() -> usize {
    1536
}
fn default_vector_weight() -> f64 {
    0.7
}
fn default_keyword_weight() -> f64 {
    0.3
}
fn default_min_relevance_score() -> f64 {
    0.4
}
fn default_cache_size() -> usize {
    10_000
}
fn default_chunk_size() -> usize {
    512
}
fn default_response_cache_ttl() -> u32 {
    60
}
fn default_response_cache_max() -> usize {
    5_000
}

fn default_true() -> bool {
    true
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            backend: "sqlite".into(),
            auto_save: true,
            hygiene_enabled: default_hygiene_enabled(),
            archive_after_days: default_archive_after_days(),
            purge_after_days: default_purge_after_days(),
            conversation_retention_days: default_conversation_retention_days(),
            embedding_provider: default_embedding_provider(),
            embedding_model: default_embedding_model(),
            embedding_dimensions: default_embedding_dims(),
            vector_weight: default_vector_weight(),
            keyword_weight: default_keyword_weight(),
            min_relevance_score: default_min_relevance_score(),
            embedding_cache_size: default_cache_size(),
            chunk_max_tokens: default_chunk_size(),
            response_cache_enabled: false,
            response_cache_ttl_minutes: default_response_cache_ttl(),
            response_cache_max_entries: default_response_cache_max(),
            snapshot_enabled: false,
            snapshot_on_hygiene: false,
            auto_hydrate: true,
            sqlite_open_timeout_secs: None,
            qdrant: QdrantConfig::default(),
        }
    }
}
#[cfg(test)]
#[path = "memory_tests.rs"]
mod memory_tests;
