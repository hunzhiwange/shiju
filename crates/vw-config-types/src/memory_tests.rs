#[test]
fn storage_provider_supports_legacy_db_url_aliases() {
    let parsed: super::StorageProviderConfig = serde_json::from_value(serde_json::json!({
        "provider": "postgres",
        "dbURL": "postgres://example"
    }))
    .unwrap();

    assert_eq!(parsed.db_url.as_deref(), Some("postgres://example"));
    assert_eq!(parsed.schema, "public");
    assert_eq!(parsed.table, "memories");
}

#[test]
fn memory_defaults_and_qdrant_defaults_are_stable() {
    let memory = super::MemoryConfig::default();
    assert_eq!(memory.backend, "sqlite");
    assert!(memory.auto_save);
    assert_eq!(memory.embedding_provider, "none");
    assert_eq!(memory.embedding_model, "text-embedding-3-small");
    assert_eq!(memory.embedding_dimensions, 1536);
    assert!(memory.auto_hydrate);

    let qdrant = super::QdrantConfig::default();
    assert_eq!(qdrant.collection, "vibewindow_memories");
}

#[test]
fn storage_provider_supports_remaining_db_url_aliases() {
    for alias in ["database_url", "databaseUrl"] {
        let parsed: super::StorageProviderConfig = serde_json::from_value(serde_json::json!({
            "provider": "mariadb",
            (alias): "mysql://example",
            "schema": "archive",
            "table": "facts",
            "connect_timeout_secs": 8,
            "tls": true
        }))
        .unwrap();

        assert_eq!(parsed.db_url.as_deref(), Some("mysql://example"));
        assert_eq!(parsed.schema, "archive");
        assert_eq!(parsed.table, "facts");
        assert_eq!(parsed.connect_timeout_secs, Some(8));
        assert!(parsed.tls);
    }
}

#[test]
fn memory_default_covers_retention_cache_and_snapshot_values() {
    let memory = super::MemoryConfig::default();

    assert!(memory.hygiene_enabled);
    assert_eq!(memory.archive_after_days, 7);
    assert_eq!(memory.purge_after_days, 30);
    assert_eq!(memory.conversation_retention_days, 30);
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
    assert!(memory.sqlite_open_timeout_secs.is_none());
}

#[test]
fn memory_deserialization_defaults_auto_hydrate_to_true() {
    let memory: super::MemoryConfig = serde_json::from_value(serde_json::json!({
        "backend": "none",
        "auto_save": false
    }))
    .unwrap();

    assert!(memory.auto_hydrate);
    assert_eq!(memory.qdrant.collection, "vibewindow_memories");
}
