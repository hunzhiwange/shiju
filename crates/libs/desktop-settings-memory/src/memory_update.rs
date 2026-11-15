use crate::{state::{MemorySettingsState, MemorySettingsTab}, messages::MemoryMessage};
use config_memory::{MemoryConfig, QdrantConfig};
pub fn normalize_backend(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "sqlite" | "postgres" | "qdrant" | "chroma" | "markdown" | "none" => {
            raw.trim().to_ascii_lowercase()
        }
        "null" => "none".to_string(),
        _ => "sqlite".to_string(),
    }
}

pub fn config(s: &MemorySettingsState) -> MemoryConfig {
    let defaults = MemoryConfig::default();
    let backend = normalize_backend(&s.backend);
    let auto_save = s.auto_save;
    let hygiene_enabled = s.hygiene_enabled;
    let archive_after_days = s.archive_after_days.clamp(0, 3650);
    let purge_after_days = s.purge_after_days.clamp(0, 3650);
    let conversation_retention_days = s.conversation_retention_days.clamp(0, 3650);
    let embedding_provider = if s.embedding_provider.trim().is_empty() {
        defaults.embedding_provider.clone()
    } else {
        s.embedding_provider.trim().to_string()
    };
    let embedding_model = if s.embedding_model.trim().is_empty() {
        defaults.embedding_model.clone()
    } else {
        s.embedding_model.trim().to_string()
    };
    let embedding_dimensions = s.embedding_dimensions.max(1) as usize;
    let vector_weight = s.vector_weight.clamp(0.0, 1.0) as f64;
    let keyword_weight = s.keyword_weight.clamp(0.0, 1.0) as f64;
    let min_relevance_score = s.min_relevance_score.clamp(0.0, 1.0) as f64;
    let embedding_cache_size = s.embedding_cache_size as usize;
    let chunk_max_tokens = s.chunk_max_tokens.max(1) as usize;
    let response_cache_enabled = s.response_cache_enabled;
    let response_cache_ttl_minutes = s.response_cache_ttl_minutes;
    let response_cache_max_entries = s.response_cache_max_entries as usize;
    let snapshot_enabled = s.snapshot_enabled;
    let snapshot_on_hygiene = s.snapshot_on_hygiene;
    let auto_hydrate = s.auto_hydrate;
    let sqlite_open_timeout_secs = if s.sqlite_open_timeout_secs == 0 {
        None
    } else {
        Some(s.sqlite_open_timeout_secs as u64)
    };
    let qdrant_url = if s.qdrant_url_input.trim().is_empty() {
        None
    } else {
        Some(s.qdrant_url_input.trim().to_string())
    };
    let qdrant_collection = if s.qdrant_collection.trim().is_empty() {
        "vibewindow_memories".to_string()
    } else {
        s.qdrant_collection.trim().to_string()
    };
    let qdrant_api_key = if s.qdrant_api_key_input.trim().is_empty() {
        None
    } else {
        Some(s.qdrant_api_key_input.trim().to_string())
    };

    MemoryConfig {
        backend,
        auto_save,
        hygiene_enabled,
        archive_after_days,
        purge_after_days,
        conversation_retention_days,
        embedding_provider,
        embedding_model,
        embedding_dimensions,
        vector_weight,
        keyword_weight,
        min_relevance_score,
        embedding_cache_size,
        chunk_max_tokens,
        response_cache_enabled,
        response_cache_ttl_minutes,
        response_cache_max_entries,
        snapshot_enabled,
        snapshot_on_hygiene,
        auto_hydrate,
        sqlite_open_timeout_secs,
        qdrant: QdrantConfig {
            url: qdrant_url,
            collection: qdrant_collection,
            api_key: qdrant_api_key,
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect { None, Persist, ClearEmbeddingRouteFeedback }

pub fn update(state: &mut MemorySettingsState, message: MemoryMessage) -> Effect {

    match message {
        MemoryMessage::Refresh => {
            state.save_error = None;
            return Effect::None;
        }
        MemoryMessage::TabSelected(tab) => {
            if tab == MemorySettingsTab::Qdrant && state.backend != "qdrant" {
                return Effect::None;
            }

            state.active_tab = tab;
            if tab == MemorySettingsTab::EmbeddingRoutes {
                return Effect::ClearEmbeddingRouteFeedback;
            }
            return Effect::None;
        }
        MemoryMessage::BackendChanged(value) => {
            state.backend = normalize_backend(&value);
            if state.backend != "qdrant"
                && state.active_tab == MemorySettingsTab::Qdrant
            {
                state.active_tab = MemorySettingsTab::General;
            }
        }
        MemoryMessage::AutoSaveToggled(value) => state.auto_save = value,
        MemoryMessage::HygieneEnabledToggled(value) => state.hygiene_enabled = value,
        MemoryMessage::ArchiveAfterDaysChanged(value) => {
            state.archive_after_days = value
        }
        MemoryMessage::PurgeAfterDaysChanged(value) => state.purge_after_days = value,
        MemoryMessage::ConversationRetentionDaysChanged(value) => {
            state.conversation_retention_days = value
        }
        MemoryMessage::EmbeddingProviderChanged(value) => {
            state.embedding_provider = value
        }
        MemoryMessage::EmbeddingModelChanged(value) => state.embedding_model = value,
        MemoryMessage::EmbeddingDimensionsChanged(value) => {
            state.embedding_dimensions = value.max(1)
        }
        MemoryMessage::VectorWeightChanged(value) => {
            state.vector_weight = value.clamp(0.0, 1.0)
        }
        MemoryMessage::KeywordWeightChanged(value) => {
            state.keyword_weight = value.clamp(0.0, 1.0)
        }
        MemoryMessage::MinRelevanceScoreChanged(value) => {
            state.min_relevance_score = value.clamp(0.0, 1.0)
        }
        MemoryMessage::EmbeddingCacheSizeChanged(value) => {
            state.embedding_cache_size = value
        }
        MemoryMessage::ChunkMaxTokensChanged(value) => {
            state.chunk_max_tokens = value.max(1)
        }
        MemoryMessage::ResponseCacheEnabledToggled(value) => {
            state.response_cache_enabled = value
        }
        MemoryMessage::ResponseCacheTtlMinutesChanged(value) => {
            state.response_cache_ttl_minutes = value
        }
        MemoryMessage::ResponseCacheMaxEntriesChanged(value) => {
            state.response_cache_max_entries = value
        }
        MemoryMessage::SnapshotEnabledToggled(value) => {
            state.snapshot_enabled = value
        }
        MemoryMessage::SnapshotOnHygieneToggled(value) => {
            state.snapshot_on_hygiene = value
        }
        MemoryMessage::AutoHydrateToggled(value) => state.auto_hydrate = value,
        MemoryMessage::SqliteOpenTimeoutSecsChanged(value) => {
            state.sqlite_open_timeout_secs = value
        }
        MemoryMessage::QdrantUrlChanged(value) => state.qdrant_url_input = value,
        MemoryMessage::QdrantCollectionChanged(value) => {
            state.qdrant_collection = value
        }
        MemoryMessage::QdrantApiKeyChanged(value) => {
            state.qdrant_api_key_input = value
        }
    }

    state.save_error = None;
    Effect::Persist
}
