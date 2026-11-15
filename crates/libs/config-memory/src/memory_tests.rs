//! 验证记忆存储后端的默认参数、数据库地址别名与自动加载兼容行为。
//! 数据库地址覆盖多代命名格式，而缓存、保留和快照字段保持统一默认值。

// 驼峰旧字段仍存在于历史配置，必须映射到统一的数据库地址字段。
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

// 后端选择与向量维度是跨组件契约，升级依赖时不能随库默认值变化。
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

// 其余历史别名与主别名共享相同解析路径，并保留显式表结构设置。
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

// 将长期保留、查询缓存和快照设置作为一组锁定，便于发现默认策略漂移。
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

// 旧配置缺少该字段时仍自动恢复记忆，维持升级前的启动行为。
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
