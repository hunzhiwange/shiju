// Tests migrated from desktop host state; production types retain their original owners.
mod behavior_0 {
use crate::state::MemorySettingsState;
#[test]
fn memory_defaults_match_runtime_config() {
    let memory = MemorySettingsState::default();
    assert_eq!(memory.backend, "sqlite");
    assert!(memory.auto_save);
    assert!(memory.hygiene_enabled);
    assert_eq!(memory.archive_after_days, 7);
    assert_eq!(memory.purge_after_days, 30);
    assert_eq!(memory.conversation_retention_days, 30);
    assert_eq!(memory.embedding_provider, "none");
    assert_eq!(memory.embedding_model, "text-embedding-3-small");
    assert_eq!(memory.embedding_dimensions, 1536);
    assert_eq!(memory.vector_weight, 0.7);
    assert_eq!(memory.keyword_weight, 0.3);
    assert_eq!(memory.min_relevance_score, 0.4);
    assert_eq!(memory.embedding_cache_size, 10_000);
    assert_eq!(memory.chunk_max_tokens, 512);
    assert!(!memory.response_cache_enabled);
    assert_eq!(memory.response_cache_ttl_minutes, 60);
    assert_eq!(memory.response_cache_max_entries, 5_000);
    assert!(!memory.snapshot_enabled);
    assert!(!memory.snapshot_on_hygiene);
    assert!(memory.auto_hydrate);
    assert_eq!(memory.sqlite_open_timeout_secs, 0);
    assert!(memory.qdrant_url_input.is_empty());
    assert_eq!(memory.qdrant_collection, "vibewindow_memories");
    assert!(memory.qdrant_api_key_input.is_empty());
    assert!(memory.save_error.is_none());
}
}

mod behavior_1 {
use crate::state::StorageSettingsState;
#[test]
fn storage_proxy_and_tunnel_defaults_are_stable_0() {
    let storage = StorageSettingsState::default();
    assert!(storage.provider.is_empty());
    assert!(storage.db_url_input.is_empty());
    assert_eq!(storage.schema, "public");
    assert_eq!(storage.table, "memories");
    assert!(storage.connect_timeout_secs_input.is_empty());
    assert!(!storage.tls);
    assert!(storage.save_error.is_none());


}
}
