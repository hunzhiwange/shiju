use super::*;
use crate::app::App;
use crate::app::state::MemorySettingsTab;

fn app() -> App {
    App::new().0
}

fn has_scheduled_action(task: Task<Message>) -> bool {
    task.units() > 0
}

fn apply(app: &mut App, message: MemoryMessage) {
    let _ = update(app, SettingsMessage::Memory(message));
}

#[test]
fn normalize_backend_accepts_supported_values_and_normalizes_aliases() {
    for backend in ["sqlite", "postgres", "qdrant", "chroma", "markdown", "none"] {
        assert_eq!(normalize_backend(backend), backend);
    }

    assert_eq!(normalize_backend("  QDRANT  "), "qdrant");
    assert_eq!(normalize_backend(" NULL "), "none");
    assert_eq!(normalize_backend("unknown"), "sqlite");
    assert_eq!(normalize_backend("  "), "sqlite");
}

#[test]
fn update_ignores_unrelated_messages_and_refresh_clears_only_the_error() {
    let mut app = app();
    app.memory_settings.backend = "postgres".to_string();
    app.memory_settings.save_error = Some("old".to_string());

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);
    assert_eq!(app.memory_settings.backend, "postgres");
    assert_eq!(app.memory_settings.save_error.as_deref(), Some("old"));

    apply(&mut app, MemoryMessage::Refresh);
    assert_eq!(app.memory_settings.backend, "postgres");
    assert!(app.memory_settings.save_error.is_none());
}

#[test]
fn tab_selection_updates_only_visible_memory_section() {
    let mut app = app();
    app.memory_settings.save_error = Some("keep".to_string());

    apply(&mut app, MemoryMessage::TabSelected(MemorySettingsTab::Embedding));
    assert_eq!(app.memory_settings.active_tab, MemorySettingsTab::Embedding);
    assert_eq!(app.memory_settings.save_error.as_deref(), Some("keep"));

    apply(&mut app, MemoryMessage::TabSelected(MemorySettingsTab::Qdrant));
    assert_eq!(app.memory_settings.active_tab, MemorySettingsTab::Embedding);

    apply(&mut app, MemoryMessage::BackendChanged("qdrant".to_string()));
    apply(&mut app, MemoryMessage::TabSelected(MemorySettingsTab::Qdrant));
    assert_eq!(app.memory_settings.active_tab, MemorySettingsTab::Qdrant);

    apply(&mut app, MemoryMessage::BackendChanged("sqlite".to_string()));
    assert_eq!(app.memory_settings.active_tab, MemorySettingsTab::General);
}

#[test]
fn update_applies_backend_retention_and_embedding_text_fields() {
    let mut app = app();
    app.memory_settings.save_error = Some("old".to_string());

    apply(&mut app, MemoryMessage::BackendChanged(" NULL ".to_string()));
    apply(&mut app, MemoryMessage::ArchiveAfterDaysChanged(12));
    apply(&mut app, MemoryMessage::PurgeAfterDaysChanged(34));
    apply(&mut app, MemoryMessage::ConversationRetentionDaysChanged(56));
    apply(&mut app, MemoryMessage::EmbeddingProviderChanged(" provider ".to_string()));
    apply(&mut app, MemoryMessage::EmbeddingModelChanged(" model ".to_string()));

    assert_eq!(app.memory_settings.backend, "none");
    assert_eq!(app.memory_settings.archive_after_days, 12);
    assert_eq!(app.memory_settings.purge_after_days, 34);
    assert_eq!(app.memory_settings.conversation_retention_days, 56);
    assert_eq!(app.memory_settings.embedding_provider, " provider ");
    assert_eq!(app.memory_settings.embedding_model, " model ");
    assert!(app.memory_settings.save_error.is_none());
}

#[test]
fn update_clamps_embedding_dimensions_weights_and_chunk_size() {
    let mut app = app();

    apply(&mut app, MemoryMessage::EmbeddingDimensionsChanged(0));
    apply(&mut app, MemoryMessage::VectorWeightChanged(3.0));
    apply(&mut app, MemoryMessage::KeywordWeightChanged(-1.0));
    apply(&mut app, MemoryMessage::MinRelevanceScoreChanged(2.0));
    apply(&mut app, MemoryMessage::ChunkMaxTokensChanged(0));

    assert_eq!(app.memory_settings.embedding_dimensions, 1);
    assert_eq!(app.memory_settings.vector_weight, 1.0);
    assert_eq!(app.memory_settings.keyword_weight, 0.0);
    assert_eq!(app.memory_settings.min_relevance_score, 1.0);
    assert_eq!(app.memory_settings.chunk_max_tokens, 1);

    apply(&mut app, MemoryMessage::EmbeddingDimensionsChanged(u32::MAX));
    apply(&mut app, MemoryMessage::VectorWeightChanged(0.25));
    apply(&mut app, MemoryMessage::KeywordWeightChanged(0.75));
    apply(&mut app, MemoryMessage::MinRelevanceScoreChanged(0.5));
    apply(&mut app, MemoryMessage::ChunkMaxTokensChanged(u32::MAX));

    assert_eq!(app.memory_settings.embedding_dimensions, u32::MAX);
    assert_eq!(app.memory_settings.vector_weight, 0.25);
    assert_eq!(app.memory_settings.keyword_weight, 0.75);
    assert_eq!(app.memory_settings.min_relevance_score, 0.5);
    assert_eq!(app.memory_settings.chunk_max_tokens, u32::MAX);
}

#[test]
fn update_applies_cache_snapshot_and_hydration_fields() {
    let mut app = app();

    apply(&mut app, MemoryMessage::EmbeddingCacheSizeChanged(0));
    apply(&mut app, MemoryMessage::ResponseCacheEnabledToggled(true));
    apply(&mut app, MemoryMessage::ResponseCacheTtlMinutesChanged(0));
    apply(&mut app, MemoryMessage::ResponseCacheMaxEntriesChanged(u32::MAX));
    apply(&mut app, MemoryMessage::SnapshotEnabledToggled(true));
    apply(&mut app, MemoryMessage::SnapshotOnHygieneToggled(true));
    apply(&mut app, MemoryMessage::AutoHydrateToggled(false));

    assert_eq!(app.memory_settings.embedding_cache_size, 0);
    assert!(app.memory_settings.response_cache_enabled);
    assert_eq!(app.memory_settings.response_cache_ttl_minutes, 0);
    assert_eq!(app.memory_settings.response_cache_max_entries, u32::MAX);
    assert!(app.memory_settings.snapshot_enabled);
    assert!(app.memory_settings.snapshot_on_hygiene);
    assert!(!app.memory_settings.auto_hydrate);
}

#[test]
fn update_applies_memory_lifecycle_toggles() {
    let mut app = app();

    apply(&mut app, MemoryMessage::AutoSaveToggled(false));
    apply(&mut app, MemoryMessage::HygieneEnabledToggled(false));

    assert!(!app.memory_settings.auto_save);
    assert!(!app.memory_settings.hygiene_enabled);
}

#[test]
fn update_applies_sqlite_and_qdrant_fields_without_trimming_user_input() {
    let mut app = app();

    apply(&mut app, MemoryMessage::SqliteOpenTimeoutSecsChanged(u32::MAX));
    apply(&mut app, MemoryMessage::QdrantUrlChanged("  https://qdrant.test  ".to_string()));
    apply(&mut app, MemoryMessage::QdrantCollectionChanged("  memories  ".to_string()));
    apply(&mut app, MemoryMessage::QdrantApiKeyChanged("  secret-placeholder  ".to_string()));

    assert_eq!(app.memory_settings.sqlite_open_timeout_secs, u32::MAX);
    assert_eq!(app.memory_settings.qdrant_url_input, "  https://qdrant.test  ");
    assert_eq!(app.memory_settings.qdrant_collection, "  memories  ");
    assert_eq!(app.memory_settings.qdrant_api_key_input, "  secret-placeholder  ");
}

#[test]
fn memory_edits_schedule_persistence_while_refresh_and_unrelated_messages_do_not() {
    let mut app = app();

    let edit_task = update(
        &mut app,
        SettingsMessage::Memory(MemoryMessage::BackendChanged("qdrant".to_string())),
    );
    assert!(has_scheduled_action(edit_task));

    let refresh_task = update(&mut app, SettingsMessage::Memory(MemoryMessage::Refresh));
    assert!(!has_scheduled_action(refresh_task));

    let unrelated_task = update(&mut app, SettingsMessage::ToggleSettingsSidebar);
    assert!(!has_scheduled_action(unrelated_task));
}
