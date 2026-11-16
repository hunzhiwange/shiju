use crate::memory_update::update;
use crate::messages::MemoryMessage;
use crate::state::MemorySettingsState;
fn apply(state: &mut MemorySettingsState, message: MemoryMessage) { let _ = update(state, message); }

#[test]
fn update_applies_backend_retention_and_embedding_text_fields() {
    let mut app = MemorySettingsState::default();
    app.save_error = Some("old".to_string());

    apply(&mut app, MemoryMessage::BackendChanged(" NULL ".to_string()));
    apply(&mut app, MemoryMessage::ArchiveAfterDaysChanged(12));
    apply(&mut app, MemoryMessage::PurgeAfterDaysChanged(34));
    apply(&mut app, MemoryMessage::ConversationRetentionDaysChanged(56));
    apply(&mut app, MemoryMessage::EmbeddingProviderChanged(" provider ".to_string()));
    apply(&mut app, MemoryMessage::EmbeddingModelChanged(" model ".to_string()));

    assert_eq!(app.backend, "none");
    assert_eq!(app.archive_after_days, 12);
    assert_eq!(app.purge_after_days, 34);
    assert_eq!(app.conversation_retention_days, 56);
    assert_eq!(app.embedding_provider, " provider ");
    assert_eq!(app.embedding_model, " model ");
    assert!(app.save_error.is_none());
}

#[test]
fn update_clamps_embedding_dimensions_weights_and_chunk_size() {
    let mut app = MemorySettingsState::default();

    apply(&mut app, MemoryMessage::EmbeddingDimensionsChanged(0));
    apply(&mut app, MemoryMessage::VectorWeightChanged(3.0));
    apply(&mut app, MemoryMessage::KeywordWeightChanged(-1.0));
    apply(&mut app, MemoryMessage::MinRelevanceScoreChanged(2.0));
    apply(&mut app, MemoryMessage::ChunkMaxTokensChanged(0));

    assert_eq!(app.embedding_dimensions, 1);
    assert_eq!(app.vector_weight, 1.0);
    assert_eq!(app.keyword_weight, 0.0);
    assert_eq!(app.min_relevance_score, 1.0);
    assert_eq!(app.chunk_max_tokens, 1);

    apply(&mut app, MemoryMessage::EmbeddingDimensionsChanged(u32::MAX));
    apply(&mut app, MemoryMessage::VectorWeightChanged(0.25));
    apply(&mut app, MemoryMessage::KeywordWeightChanged(0.75));
    apply(&mut app, MemoryMessage::MinRelevanceScoreChanged(0.5));
    apply(&mut app, MemoryMessage::ChunkMaxTokensChanged(u32::MAX));

    assert_eq!(app.embedding_dimensions, u32::MAX);
    assert_eq!(app.vector_weight, 0.25);
    assert_eq!(app.keyword_weight, 0.75);
    assert_eq!(app.min_relevance_score, 0.5);
    assert_eq!(app.chunk_max_tokens, u32::MAX);
}

#[test]
fn update_applies_cache_snapshot_and_hydration_fields() {
    let mut app = MemorySettingsState::default();

    apply(&mut app, MemoryMessage::EmbeddingCacheSizeChanged(0));
    apply(&mut app, MemoryMessage::ResponseCacheEnabledToggled(true));
    apply(&mut app, MemoryMessage::ResponseCacheTtlMinutesChanged(0));
    apply(&mut app, MemoryMessage::ResponseCacheMaxEntriesChanged(u32::MAX));
    apply(&mut app, MemoryMessage::SnapshotEnabledToggled(true));
    apply(&mut app, MemoryMessage::SnapshotOnHygieneToggled(true));
    apply(&mut app, MemoryMessage::AutoHydrateToggled(false));

    assert_eq!(app.embedding_cache_size, 0);
    assert!(app.response_cache_enabled);
    assert_eq!(app.response_cache_ttl_minutes, 0);
    assert_eq!(app.response_cache_max_entries, u32::MAX);
    assert!(app.snapshot_enabled);
    assert!(app.snapshot_on_hygiene);
    assert!(!app.auto_hydrate);
}

#[test]
fn update_applies_memory_lifecycle_toggles() {
    let mut app = MemorySettingsState::default();

    apply(&mut app, MemoryMessage::AutoSaveToggled(false));
    apply(&mut app, MemoryMessage::HygieneEnabledToggled(false));

    assert!(!app.auto_save);
    assert!(!app.hygiene_enabled);
}

#[test]
fn update_applies_sqlite_and_qdrant_fields_without_trimming_user_input() {
    let mut app = MemorySettingsState::default();

    apply(&mut app, MemoryMessage::SqliteOpenTimeoutSecsChanged(u32::MAX));
    apply(&mut app, MemoryMessage::QdrantUrlChanged("  https://qdrant.test  ".to_string()));
    apply(&mut app, MemoryMessage::QdrantCollectionChanged("  memories  ".to_string()));
    apply(&mut app, MemoryMessage::QdrantApiKeyChanged("  secret-placeholder  ".to_string()));

    assert_eq!(app.sqlite_open_timeout_secs, u32::MAX);
    assert_eq!(app.qdrant_url_input, "  https://qdrant.test  ");
    assert_eq!(app.qdrant_collection, "  memories  ");
    assert_eq!(app.qdrant_api_key_input, "  secret-placeholder  ");
}
