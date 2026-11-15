//! 从已加载配置构造本领域设置；不执行网络或宿主操作。

pub fn build_memory_settings(
    full_agent_cfg: &config_aggregate::Config,
) -> crate::state::MemorySettingsState {
    let memory_cfg = &full_agent_cfg.memory;

    crate::state::MemorySettingsState {
        active_tab: crate::state::MemorySettingsTab::General,
        backend: match memory_cfg.backend.trim().to_ascii_lowercase().as_str() {
            "sqlite" | "postgres" | "qdrant" | "markdown" | "none" => {
                memory_cfg.backend.trim().to_ascii_lowercase()
            }
            "null" => "none".to_string(),
            _ => "sqlite".to_string(),
        },
        auto_save: memory_cfg.auto_save,
        hygiene_enabled: memory_cfg.hygiene_enabled,
        archive_after_days: memory_cfg.archive_after_days,
        purge_after_days: memory_cfg.purge_after_days,
        conversation_retention_days: memory_cfg.conversation_retention_days,
        embedding_provider: memory_cfg.embedding_provider.clone(),
        embedding_model: memory_cfg.embedding_model.clone(),
        embedding_dimensions: memory_cfg.embedding_dimensions.min(u32::MAX as usize) as u32,
        vector_weight: memory_cfg.vector_weight.clamp(0.0, 1.0) as f32,
        keyword_weight: memory_cfg.keyword_weight.clamp(0.0, 1.0) as f32,
        min_relevance_score: memory_cfg.min_relevance_score.clamp(0.0, 1.0) as f32,
        embedding_cache_size: memory_cfg.embedding_cache_size.min(u32::MAX as usize) as u32,
        chunk_max_tokens: memory_cfg.chunk_max_tokens.min(u32::MAX as usize) as u32,
        response_cache_enabled: memory_cfg.response_cache_enabled,
        response_cache_ttl_minutes: memory_cfg.response_cache_ttl_minutes,
        response_cache_max_entries: memory_cfg.response_cache_max_entries.min(u32::MAX as usize)
            as u32,
        snapshot_enabled: memory_cfg.snapshot_enabled,
        snapshot_on_hygiene: memory_cfg.snapshot_on_hygiene,
        auto_hydrate: memory_cfg.auto_hydrate,
        sqlite_open_timeout_secs: memory_cfg
            .sqlite_open_timeout_secs
            .unwrap_or_default()
            .min(u32::MAX as u64) as u32,
        qdrant_url_input: memory_cfg.qdrant.url.clone().unwrap_or_default(),
        qdrant_collection: {
            let value = memory_cfg.qdrant.collection.trim().to_string();
            if value.is_empty() { "vibewindow_memories".to_string() } else { value }
        },
        qdrant_api_key_input: memory_cfg.qdrant.api_key.clone().unwrap_or_default(),
        save_error: None,
    }
}

pub fn build_storage_settings(
    full_agent_cfg: &config_aggregate::Config,
) -> crate::state::StorageSettingsState {
    let storage_cfg = &full_agent_cfg.storage;

    crate::state::StorageSettingsState {
        provider: storage_cfg.provider.config.provider.clone(),
        db_url_input: storage_cfg.provider.config.db_url.clone().unwrap_or_default(),
        schema: {
            let value = storage_cfg.provider.config.schema.trim().to_string();
            if value.is_empty() { "public".to_string() } else { value }
        },
        table: {
            let value = storage_cfg.provider.config.table.trim().to_string();
            if value.is_empty() { "memories".to_string() } else { value }
        },
        connect_timeout_secs_input: storage_cfg
            .provider
            .config
            .connect_timeout_secs
            .map(|value| value.to_string())
            .unwrap_or_default(),
        tls: storage_cfg.provider.config.tls,
        save_error: None,
    }
}
