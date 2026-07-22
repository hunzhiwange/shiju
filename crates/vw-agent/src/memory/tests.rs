//! 内存后端工厂与嵌入配置解析的回归测试。
//!
//! 本文件覆盖 `memory` 模块对不同持久化后端的选择、迁移入口的安全约束，
//! 以及嵌入路由覆盖规则。测试重点是确认外部配置被解析为明确、可预期的运行时行为。

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use crate::app::agent::config::{EmbeddingRouteConfig, StorageProviderConfig};
    use tempfile::TempDir;

    #[test]
    fn factory_sqlite() {
        let tmp = TempDir::new().unwrap();
        let cfg = MemoryConfig { backend: "sqlite".into(), ..MemoryConfig::default() };
        let mem = create_memory(&cfg, tmp.path(), None).unwrap();
        assert_eq!(mem.name(), "sqlite");
    }

    #[test]
    fn assistant_autosave_key_detection_matches_legacy_patterns() {
        assert!(is_assistant_autosave_key("assistant_resp"));
        assert!(is_assistant_autosave_key("assistant_resp_1234"));
        assert!(is_assistant_autosave_key("ASSISTANT_RESP_abcd"));
        assert!(!is_assistant_autosave_key("assistant_response"));
        assert!(!is_assistant_autosave_key("user_msg_1234"));
    }

    #[test]
    fn factory_markdown() {
        let tmp = TempDir::new().unwrap();
        let cfg = MemoryConfig { backend: "markdown".into(), ..MemoryConfig::default() };
        let mem = create_memory(&cfg, tmp.path(), None).unwrap();
        assert_eq!(mem.name(), "markdown");
    }

    #[test]
    fn factory_lucid() {
        let tmp = TempDir::new().unwrap();
        let cfg = MemoryConfig { backend: "lucid".into(), ..MemoryConfig::default() };
        let mem = create_memory(&cfg, tmp.path(), None).unwrap();
        assert_eq!(mem.name(), "lucid");
    }

    #[test]
    fn factory_none_uses_noop_memory() {
        let tmp = TempDir::new().unwrap();
        let cfg = MemoryConfig { backend: "none".into(), ..MemoryConfig::default() };
        let mem = create_memory(&cfg, tmp.path(), None).unwrap();
        assert_eq!(mem.name(), "none");
    }

    #[test]
    fn factory_unknown_falls_back_to_markdown() {
        let tmp = TempDir::new().unwrap();
        let cfg = MemoryConfig { backend: "redis".into(), ..MemoryConfig::default() };
        let mem = create_memory(&cfg, tmp.path(), None).unwrap();
        // 兼容旧配置：未知后端仍回退到 markdown，避免升级后直接丢失记忆能力。
        assert_eq!(mem.name(), "markdown");
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn injected_memory_builders_propagate_local_and_postgres_errors() {
        let tmp = TempDir::new().unwrap();

        for backend in ["sqlite", "lucid"] {
            let error = create_memory_with_builders(
                backend,
                tmp.path(),
                || Err(anyhow::anyhow!("local builder failed")),
                || -> anyhow::Result<Box<dyn Memory>> {
                    panic!("postgres builder should not be called")
                },
                " in test",
            )
            .err()
            .expect("local builder error should be preserved");

            assert_eq!(error.to_string(), "local builder failed");
        }

        let error = create_memory_with_builders(
            "postgres",
            tmp.path(),
            || -> anyhow::Result<SqliteMemory> { panic!("sqlite builder should not be called") },
            || Err(anyhow::anyhow!("postgres builder failed")),
            " in test",
        )
        .err()
        .expect("postgres builder error should be preserved");

        assert_eq!(error.to_string(), "postgres builder failed");
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn injected_none_backend_skips_storage_builders() {
        let tmp = TempDir::new().unwrap();

        let memory = create_memory_with_builders(
            "none",
            tmp.path(),
            || -> anyhow::Result<SqliteMemory> { panic!("sqlite builder should not be called") },
            || -> anyhow::Result<Box<dyn Memory>> {
                panic!("postgres builder should not be called")
            },
            " in test",
        )
        .unwrap();

        assert_eq!(memory.name(), "none");
    }

    #[test]
    fn migration_factory_lucid() {
        let tmp = TempDir::new().unwrap();
        let mem = create_memory_for_migration("lucid", tmp.path()).unwrap();
        assert_eq!(mem.name(), "lucid");
    }

    #[test]
    fn migration_factory_none_is_rejected() {
        let tmp = TempDir::new().unwrap();
        let error = create_memory_for_migration("none", tmp.path())
            .err()
            .expect("backend=none should be rejected for migration");
        // 迁移需要真实持久化目标；noop 后端会让导入结果不可恢复，因此必须显式失败。
        assert!(error.to_string().contains("disables persistence"));
    }

    #[test]
    fn effective_backend_name_prefers_storage_override() {
        let storage = StorageProviderConfig {
            provider: "postgres".into(),
            ..StorageProviderConfig::default()
        };

        assert_eq!(effective_memory_backend_name("sqlite", Some(&storage)), "postgres");
    }

    #[test]
    fn factory_postgres_without_db_url_is_rejected() {
        let tmp = TempDir::new().unwrap();
        let cfg = MemoryConfig { backend: "postgres".into(), ..MemoryConfig::default() };

        let storage = StorageProviderConfig {
            provider: "postgres".into(),
            db_url: None,
            ..StorageProviderConfig::default()
        };

        let error = create_memory_with_storage(&cfg, Some(&storage), tmp.path(), None)
            .err()
            .expect("postgres without db_url should be rejected");
        if cfg!(feature = "memory-postgres") {
            assert!(error.to_string().contains("db_url"));
        } else {
            assert!(error.to_string().contains("memory-postgres"));
        }
    }

    #[test]
    fn factory_mariadb_without_db_url_is_rejected() {
        let tmp = TempDir::new().unwrap();
        let cfg = MemoryConfig { backend: "mariadb".into(), ..MemoryConfig::default() };

        let storage = StorageProviderConfig {
            provider: "mariadb".into(),
            db_url: None,
            ..StorageProviderConfig::default()
        };

        let error = create_memory_with_storage(&cfg, Some(&storage), tmp.path(), None)
            .err()
            .expect("mariadb without db_url should be rejected");
        if cfg!(feature = "memory-mariadb") {
            assert!(error.to_string().contains("db_url"));
        } else {
            assert!(error.to_string().contains("memory-mariadb"));
        }
    }

    #[test]
    fn resolve_embedding_config_uses_base_config_when_model_is_not_hint() {
        let cfg = MemoryConfig {
            embedding_provider: "openai".into(),
            embedding_model: "text-embedding-3-small".into(),
            embedding_dimensions: 1536,
            ..MemoryConfig::default()
        };

        let resolved = resolve_embedding_config(&cfg, &[], Some("base-key"));
        assert_eq!(
            resolved,
            ResolvedEmbeddingConfig {
                provider: "openai".into(),
                model: "text-embedding-3-small".into(),
                dimensions: 1536,
                api_key: Some("base-key".into()),
            }
        );
    }

    #[test]
    fn resolve_embedding_config_uses_matching_route_with_api_key_override() {
        let cfg = MemoryConfig {
            embedding_provider: "none".into(),
            embedding_model: "hint:semantic".into(),
            embedding_dimensions: 1536,
            ..MemoryConfig::default()
        };
        let routes = vec![EmbeddingRouteConfig {
            hint: "semantic".into(),
            provider: "custom:https://api.example.com/v1".into(),
            model: "custom-embed-v2".into(),
            dimensions: Some(1024),
            api_key: Some("route-key".into()),
        }];

        let resolved = resolve_embedding_config(&cfg, &routes, Some("base-key"));
        // 命中的路由可以独立覆盖 key，确保多模型路由不会误用全局凭据。
        assert_eq!(
            resolved,
            ResolvedEmbeddingConfig {
                provider: "custom:https://api.example.com/v1".into(),
                model: "custom-embed-v2".into(),
                dimensions: 1024,
                api_key: Some("route-key".into()),
            }
        );
    }

    #[test]
    fn resolve_embedding_config_falls_back_when_hint_is_missing() {
        let cfg = MemoryConfig {
            embedding_provider: "openai".into(),
            embedding_model: "hint:semantic".into(),
            embedding_dimensions: 1536,
            ..MemoryConfig::default()
        };

        let resolved = resolve_embedding_config(&cfg, &[], Some("base-key"));
        assert_eq!(
            resolved,
            ResolvedEmbeddingConfig {
                provider: "openai".into(),
                model: "hint:semantic".into(),
                dimensions: 1536,
                api_key: Some("base-key".into()),
            }
        );
    }

    #[test]
    fn resolve_embedding_config_falls_back_when_route_is_invalid() {
        let cfg = MemoryConfig {
            embedding_provider: "openai".into(),
            embedding_model: "hint:semantic".into(),
            embedding_dimensions: 1536,
            ..MemoryConfig::default()
        };
        let routes = vec![EmbeddingRouteConfig {
            hint: "semantic".into(),
            provider: String::new(),
            model: "text-embedding-3-small".into(),
            dimensions: Some(0),
            api_key: None,
        }];

        let resolved = resolve_embedding_config(&cfg, &routes, Some("base-key"));
        // 无效路由保持局部失败，不污染基础配置解析结果。
        assert_eq!(
            resolved,
            ResolvedEmbeddingConfig {
                provider: "openai".into(),
                model: "hint:semantic".into(),
                dimensions: 1536,
                api_key: Some("base-key".into()),
            }
        );
    }

    #[test]
    fn resolve_embedding_config_rejects_each_invalid_route_field() {
        let cfg = MemoryConfig {
            embedding_provider: "openai".into(),
            embedding_model: "hint:semantic".into(),
            embedding_dimensions: 1536,
            ..MemoryConfig::default()
        };
        let invalid_routes = [
            EmbeddingRouteConfig {
                hint: "semantic".into(),
                provider: " ".into(),
                model: "text-embedding-3-small".into(),
                dimensions: Some(1536),
                api_key: Some("route-key".into()),
            },
            EmbeddingRouteConfig {
                hint: "semantic".into(),
                provider: "openai".into(),
                model: "\t".into(),
                dimensions: Some(1536),
                api_key: Some("route-key".into()),
            },
            EmbeddingRouteConfig {
                hint: "semantic".into(),
                provider: "openai".into(),
                model: "text-embedding-3-small".into(),
                dimensions: Some(0),
                api_key: Some("route-key".into()),
            },
        ];

        for route in invalid_routes {
            let resolved = resolve_embedding_config(&cfg, &[route], Some("base-key"));

            assert_eq!(resolved.provider, "openai");
            assert_eq!(resolved.model, "hint:semantic");
            assert_eq!(resolved.dimensions, 1536);
            assert_eq!(resolved.api_key(), Some("base-key"));
        }
    }
}
