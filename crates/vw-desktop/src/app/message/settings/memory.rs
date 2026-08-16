//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::config::update_memory_config_async;
use crate::app::{App, Message};
use iced::Task;
use vw_config_types::memory::{MemoryConfig, QdrantConfig};

use crate::app::state::MemorySettingsTab;

use super::messages::{MemoryMessage, SettingsMessage};

fn normalize_backend(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "sqlite" | "postgres" | "qdrant" | "chroma" | "markdown" | "none" => {
            raw.trim().to_ascii_lowercase()
        }
        "null" => "none".to_string(),
        _ => "sqlite".to_string(),
    }
}

#[cfg(test)]
#[path = "memory_tests.rs"]
mod memory_tests;

fn persist_memory_settings(app: &mut App) -> Task<Message> {
    let s = &app.memory_settings;
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

    update_memory_config_async(move |memory| {
        *memory = MemoryConfig {
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
        };
    })
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::Memory(message) = message else {
        return Task::none();
    };

    match message {
        MemoryMessage::Refresh => {
            app.memory_settings.save_error = None;
            return Task::none();
        }
        MemoryMessage::TabSelected(tab) => {
            if tab == MemorySettingsTab::Qdrant && app.memory_settings.backend != "qdrant" {
                return Task::none();
            }

            app.memory_settings.active_tab = tab;
            if tab == MemorySettingsTab::EmbeddingRoutes {
                app.embedding_routes_settings.save_error = None;
                app.embedding_routes_settings.save_success = false;
            }
            return Task::none();
        }
        MemoryMessage::BackendChanged(value) => {
            app.memory_settings.backend = normalize_backend(&value);
            if app.memory_settings.backend != "qdrant"
                && app.memory_settings.active_tab == MemorySettingsTab::Qdrant
            {
                app.memory_settings.active_tab = MemorySettingsTab::General;
            }
        }
        MemoryMessage::AutoSaveToggled(value) => app.memory_settings.auto_save = value,
        MemoryMessage::HygieneEnabledToggled(value) => app.memory_settings.hygiene_enabled = value,
        MemoryMessage::ArchiveAfterDaysChanged(value) => {
            app.memory_settings.archive_after_days = value
        }
        MemoryMessage::PurgeAfterDaysChanged(value) => app.memory_settings.purge_after_days = value,
        MemoryMessage::ConversationRetentionDaysChanged(value) => {
            app.memory_settings.conversation_retention_days = value
        }
        MemoryMessage::EmbeddingProviderChanged(value) => {
            app.memory_settings.embedding_provider = value
        }
        MemoryMessage::EmbeddingModelChanged(value) => app.memory_settings.embedding_model = value,
        MemoryMessage::EmbeddingDimensionsChanged(value) => {
            app.memory_settings.embedding_dimensions = value.max(1)
        }
        MemoryMessage::VectorWeightChanged(value) => {
            app.memory_settings.vector_weight = value.clamp(0.0, 1.0)
        }
        MemoryMessage::KeywordWeightChanged(value) => {
            app.memory_settings.keyword_weight = value.clamp(0.0, 1.0)
        }
        MemoryMessage::MinRelevanceScoreChanged(value) => {
            app.memory_settings.min_relevance_score = value.clamp(0.0, 1.0)
        }
        MemoryMessage::EmbeddingCacheSizeChanged(value) => {
            app.memory_settings.embedding_cache_size = value
        }
        MemoryMessage::ChunkMaxTokensChanged(value) => {
            app.memory_settings.chunk_max_tokens = value.max(1)
        }
        MemoryMessage::ResponseCacheEnabledToggled(value) => {
            app.memory_settings.response_cache_enabled = value
        }
        MemoryMessage::ResponseCacheTtlMinutesChanged(value) => {
            app.memory_settings.response_cache_ttl_minutes = value
        }
        MemoryMessage::ResponseCacheMaxEntriesChanged(value) => {
            app.memory_settings.response_cache_max_entries = value
        }
        MemoryMessage::SnapshotEnabledToggled(value) => {
            app.memory_settings.snapshot_enabled = value
        }
        MemoryMessage::SnapshotOnHygieneToggled(value) => {
            app.memory_settings.snapshot_on_hygiene = value
        }
        MemoryMessage::AutoHydrateToggled(value) => app.memory_settings.auto_hydrate = value,
        MemoryMessage::SqliteOpenTimeoutSecsChanged(value) => {
            app.memory_settings.sqlite_open_timeout_secs = value
        }
        MemoryMessage::QdrantUrlChanged(value) => app.memory_settings.qdrant_url_input = value,
        MemoryMessage::QdrantCollectionChanged(value) => {
            app.memory_settings.qdrant_collection = value
        }
        MemoryMessage::QdrantApiKeyChanged(value) => {
            app.memory_settings.qdrant_api_key_input = value
        }
    }

    app.memory_settings.save_error = None;
    persist_memory_settings(app)
}
