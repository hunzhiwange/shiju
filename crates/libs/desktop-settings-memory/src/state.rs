#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemorySettingsTab {
    General,
    RetentionAndCache,
    Embedding,
    EmbeddingRoutes,
    Storage,
    Qdrant,
}

#[derive(Debug, Clone)]
pub struct MemorySettingsState {
    /// 当前显示的记忆配置页签
    pub active_tab: MemorySettingsTab,
    pub backend: String,
    pub auto_save: bool,
    pub hygiene_enabled: bool,
    pub archive_after_days: u32,
    pub purge_after_days: u32,
    pub conversation_retention_days: u32,
    pub embedding_provider: String,
    pub embedding_model: String,
    pub embedding_dimensions: u32,
    pub vector_weight: f32,
    pub keyword_weight: f32,
    pub min_relevance_score: f32,
    pub embedding_cache_size: u32,
    pub chunk_max_tokens: u32,
    pub response_cache_enabled: bool,
    pub response_cache_ttl_minutes: u32,
    pub response_cache_max_entries: u32,
    pub snapshot_enabled: bool,
    pub snapshot_on_hygiene: bool,
    pub auto_hydrate: bool,
    pub sqlite_open_timeout_secs: u32,
    pub qdrant_url_input: String,
    pub qdrant_collection: String,
    pub qdrant_api_key_input: String,
    pub save_error: Option<String>,
}

impl Default for MemorySettingsState {
    fn default() -> Self {
        Self {
            active_tab: MemorySettingsTab::General,
            backend: "sqlite".to_string(),
            auto_save: true,
            hygiene_enabled: true,
            archive_after_days: 7,
            purge_after_days: 30,
            conversation_retention_days: 30,
            embedding_provider: "none".to_string(),
            embedding_model: "text-embedding-3-small".to_string(),
            embedding_dimensions: 1536,
            vector_weight: 0.7,
            keyword_weight: 0.3,
            min_relevance_score: 0.4,
            embedding_cache_size: 10_000,
            chunk_max_tokens: 512,
            response_cache_enabled: false,
            response_cache_ttl_minutes: 60,
            response_cache_max_entries: 5_000,
            snapshot_enabled: false,
            snapshot_on_hygiene: false,
            auto_hydrate: true,
            sqlite_open_timeout_secs: 0,
            qdrant_url_input: String::new(),
            qdrant_collection: "vibewindow_memories".to_string(),
            qdrant_api_key_input: String::new(),
            save_error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StorageSettingsState {
    /// 存储 provider 标识
    pub provider: String,
    /// 数据库连接 URL 输入
    pub db_url_input: String,
    /// schema 输入
    pub schema: String,
    /// table 输入
    pub table: String,
    /// 连接超时输入（秒）
    pub connect_timeout_secs_input: String,
    /// 是否启用 TLS
    pub tls: bool,
    /// 保存错误信息
    pub save_error: Option<String>,
}

impl Default for StorageSettingsState {
    fn default() -> Self {
        Self {
            provider: String::new(),
            db_url_input: String::new(),
            schema: "public".to_string(),
            table: "memories".to_string(),
            connect_timeout_secs_input: String::new(),
            tls: false,
            save_error: None,
        }
    }
}
