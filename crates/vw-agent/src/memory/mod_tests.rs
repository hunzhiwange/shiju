// 验证记忆后端工厂、配置选择与统一接口行为。

// 环境配置中的密钥只通过脱敏调试输出观察，空覆盖不会抹掉有效基础配置。
use super::*;
use crate::app::agent::config::{EmbeddingRouteConfig, MemoryConfig, StorageProviderConfig};
use std::sync::{Mutex, MutexGuard};
use tempfile::TempDir;

static ENV_LOCK: Mutex<()> = Mutex::new(());

// 环境覆盖守卫在析构时恢复原值，避免嵌入配置测试污染进程状态。
struct EnvOverride {
    previous: Vec<(&'static str, Option<String>)>,
    _guard: MutexGuard<'static, ()>,
}

impl EnvOverride {
    fn new(vars: &[(&'static str, Option<&str>)]) -> Self {
        let guard = ENV_LOCK.lock().unwrap();
        let previous = vars.iter().map(|(key, _)| (*key, std::env::var(key).ok())).collect();
        for (key, value) in vars {
            match value {
                Some(value) => unsafe { std::env::set_var(key, value) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
        Self { previous, _guard: guard }
    }
}

impl Drop for EnvOverride {
    fn drop(&mut self) {
        for (key, value) in &self.previous {
            match value {
                Some(value) => unsafe { std::env::set_var(key, value) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
    }
}

fn memory_config(backend: &str) -> MemoryConfig {
    MemoryConfig { backend: backend.to_string(), ..MemoryConfig::default() }
}

#[test]
fn effective_backend_name_trims_backend_and_blank_storage_override() {
    let blank_override =
        StorageProviderConfig { provider: "   ".to_string(), ..StorageProviderConfig::default() };
    let mysql_override = StorageProviderConfig {
        provider: " MySQL ".to_string(),
        ..StorageProviderConfig::default()
    };

    assert_eq!(effective_memory_backend_name(" SQLite ", None), "sqlite");
    assert_eq!(effective_memory_backend_name(" Lucid ", Some(&blank_override)), "lucid");
    assert_eq!(effective_memory_backend_name("sqlite", Some(&mysql_override)), "mysql");
}

#[test]
fn assistant_autosave_key_detection_trims_input() {
    assert!(is_assistant_autosave_key(" assistant_resp "));
    assert!(is_assistant_autosave_key("\tASSISTANT_RESP_42\n"));
    assert!(is_assistant_autosave_key(" assistant_resp_extra"));
}

#[test]
fn resolved_embedding_debug_redacts_api_key() {
    let resolved = ResolvedEmbeddingConfig {
        provider: "openai".to_string(),
        model: "text-embedding-3-small".to_string(),
        dimensions: 1536,
        api_key: Some("secret-key".to_string()),
    };

    let debug = format!("{resolved:?}");

    assert!(debug.contains("openai"));
    assert!(debug.contains("text-embedding-3-small"));
    assert!(debug.contains("1536"));
    assert!(!debug.contains("secret-key"));
}

#[test]
fn resolved_embedding_api_key_helpers_trim_and_clear_values() {
    let mut resolved = ResolvedEmbeddingConfig {
        provider: "openai".to_string(),
        model: "text-embedding-3-small".to_string(),
        dimensions: 1536,
        api_key: Some(" key ".to_string()),
    };

    assert!(resolved.api_key_configured());
    assert_eq!(resolved.api_key(), Some("key"));

    resolved.set_api_key(Some(" \n ".to_string()));
    assert!(!resolved.api_key_configured());
    assert_eq!(resolved.api_key(), None);

    resolved.set_api_key(Some(" next-key ".to_string()));
    assert_eq!(resolved.api_key(), Some("next-key"));
}

#[test]
fn resolve_embedding_config_falls_back_for_plain_model_and_empty_hint() {
    let plain = MemoryConfig {
        embedding_provider: " openai ".to_string(),
        embedding_model: " text-embedding-3-small ".to_string(),
        embedding_dimensions: 1536,
        ..MemoryConfig::default()
    };
    let empty_hint = MemoryConfig {
        embedding_provider: " none ".to_string(),
        embedding_model: "hint:   ".to_string(),
        embedding_dimensions: 12,
        ..MemoryConfig::default()
    };

    let plain_resolved = resolve_embedding_config(&plain, &[], Some(" key "));
    let empty_hint_resolved = resolve_embedding_config(&empty_hint, &[], Some(" \t "));

    assert_eq!(plain_resolved.provider, "openai");
    assert_eq!(plain_resolved.model, "text-embedding-3-small");
    assert_eq!(plain_resolved.api_key(), Some("key"));
    assert_eq!(empty_hint_resolved.provider, "none");
    assert_eq!(empty_hint_resolved.model, "hint:");
    assert_eq!(empty_hint_resolved.api_key(), None);
}

#[test]
fn resolve_embedding_config_trims_routes_and_falls_back_to_base_key() {
    let cfg = MemoryConfig {
        embedding_provider: " none ".to_string(),
        embedding_model: "hint: semantic ".to_string(),
        embedding_dimensions: 1536,
        ..MemoryConfig::default()
    };
    let routes = vec![EmbeddingRouteConfig {
        hint: " semantic ".to_string(),
        provider: " alibaba-cn ".to_string(),
        model: " text-embedding-v4 ".to_string(),
        dimensions: None,
        api_key: Some("   ".to_string()),
    }];

    let resolved = resolve_embedding_config(&cfg, &routes, Some(" base-key "));

    assert_eq!(
        resolved,
        ResolvedEmbeddingConfig {
            provider: "alibaba-cn".to_string(),
            model: "text-embedding-v4".to_string(),
            dimensions: 1536,
            api_key: Some("base-key".to_string()),
        }
    );
}

#[test]
// 未知路由或无效配置回退到基础嵌入设置，但不会回显 API 密钥。
fn resolve_embedding_config_uses_fallback_for_unknown_or_invalid_routes() {
    let cfg = MemoryConfig {
        embedding_provider: "openai".to_string(),
        embedding_model: "hint:semantic".to_string(),
        embedding_dimensions: 1536,
        ..MemoryConfig::default()
    };
    let invalid_routes = vec![EmbeddingRouteConfig {
        hint: "semantic".to_string(),
        provider: " ".to_string(),
        model: "text-embedding-v4".to_string(),
        dimensions: Some(0),
        api_key: Some("route-key".to_string()),
    }];

    let unknown = resolve_embedding_config(&cfg, &[], Some("base-key"));
    let invalid = resolve_embedding_config(&cfg, &invalid_routes, Some("base-key"));

    assert_eq!(unknown.provider, "openai");
    assert_eq!(unknown.model, "hint:semantic");
    assert_eq!(unknown.api_key(), Some("base-key"));
    assert_eq!(invalid, unknown);
}

#[test]
fn create_embedding_provider_with_routes_returns_resolved_config() {
    let cfg = MemoryConfig {
        embedding_provider: "none".to_string(),
        embedding_model: "hint:test".to_string(),
        embedding_dimensions: 32,
        ..MemoryConfig::default()
    };
    let routes = vec![EmbeddingRouteConfig {
        hint: "test".to_string(),
        provider: "none".to_string(),
        model: "noop".to_string(),
        dimensions: Some(12),
        api_key: None,
    }];

    let (_provider, resolved) = create_embedding_provider_with_routes(&cfg, &routes, None);

    assert_eq!(resolved.provider, "none");
    assert_eq!(resolved.model, "noop");
    assert_eq!(resolved.dimensions, 12);
    assert_eq!(resolved.api_key, None);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn create_memory_with_builders_dispatches_postgres_and_rejects_mariadb() {
    let tmp = TempDir::new().unwrap();
    let postgres = create_memory_with_builders(
        "postgres",
        tmp.path(),
        || -> anyhow::Result<SqliteMemory> { panic!("sqlite builder should not be called") },
        || Ok(Box::new(NoneMemory::new()) as Box<dyn Memory>),
        " in test",
    )
    .unwrap();

    assert_eq!(postgres.name(), "none");

    let error = match create_memory_with_builders(
        "mariadb",
        tmp.path(),
        || -> anyhow::Result<SqliteMemory> { panic!("sqlite builder should not be called") },
        || Ok(Box::new(NoneMemory::new()) as Box<dyn Memory>),
        " in test",
    ) {
        Ok(_) => panic!("mariadb should be rejected"),
        Err(error) => error.to_string(),
    };

    assert!(error.contains("mariadb"));
    assert!(error.contains("not available"));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn create_memory_with_builders_dispatches_sqlite_lucid_and_file_backends() {
    let tmp = TempDir::new().unwrap();
    let mut sqlite_calls = 0usize;

    let sqlite = create_memory_with_builders(
        "sqlite",
        tmp.path(),
        || {
            sqlite_calls += 1;
            SqliteMemory::new(tmp.path())
        },
        || -> anyhow::Result<Box<dyn Memory>> { panic!("postgres builder should not be called") },
        " in test",
    )
    .unwrap();
    assert_eq!(sqlite.name(), "sqlite");
    assert_eq!(sqlite_calls, 1);

    let lucid = create_memory_with_builders(
        "lucid",
        tmp.path(),
        || {
            sqlite_calls += 1;
            SqliteMemory::new(tmp.path())
        },
        || -> anyhow::Result<Box<dyn Memory>> { panic!("postgres builder should not be called") },
        " in test",
    )
    .unwrap();
    assert_eq!(lucid.name(), "lucid");
    assert_eq!(sqlite_calls, 2);

    for backend in ["markdown", "qdrant", "unknown-custom"] {
        let memory = create_memory_with_builders(
            backend,
            tmp.path(),
            || -> anyhow::Result<SqliteMemory> { panic!("sqlite builder should not be called") },
            || -> anyhow::Result<Box<dyn Memory>> {
                panic!("postgres builder should not be called")
            },
            " in test",
        )
        .unwrap();
        assert_eq!(memory.name(), "markdown");
    }
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn migration_factory_rejects_sql_backends_and_allows_file_backends() {
    let tmp = TempDir::new().unwrap();

    let sqlite = create_memory_for_migration("sqlite", tmp.path()).unwrap();
    assert_eq!(sqlite.name(), "sqlite");

    let markdown = create_memory_for_migration("markdown", tmp.path()).unwrap();
    assert_eq!(markdown.name(), "markdown");

    let unknown = create_memory_for_migration("custom-experimental", tmp.path()).unwrap();
    assert_eq!(unknown.name(), "markdown");

    for backend in ["postgres", "mariadb", "mysql"] {
        let error = match create_memory_for_migration(backend, tmp.path()) {
            Ok(_) => panic!("{backend} should be rejected"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("unsupported"));
    }
}

#[test]
fn create_memory_with_storage_ignores_blank_storage_override() {
    let tmp = TempDir::new().unwrap();
    let cfg = memory_config("markdown");
    let storage =
        StorageProviderConfig { provider: " \t ".to_string(), ..StorageProviderConfig::default() };

    let memory = create_memory_with_storage(&cfg, Some(&storage), tmp.path(), None).unwrap();

    assert_eq!(memory.name(), "markdown");
}

#[test]
fn create_memory_with_storage_prefers_nonblank_storage_override() {
    let tmp = TempDir::new().unwrap();
    let cfg = memory_config("markdown");
    let storage = StorageProviderConfig {
        provider: " none ".to_string(),
        ..StorageProviderConfig::default()
    };

    let memory = create_memory_with_storage(&cfg, Some(&storage), tmp.path(), None).unwrap();

    assert_eq!(memory.name(), "none");
}

#[test]
#[cfg(feature = "memory-postgres")]
fn postgres_factory_rejects_missing_storage_settings_before_connecting() {
    let tmp = TempDir::new().unwrap();
    let cfg = memory_config("postgres");

    let error = create_memory(&cfg, tmp.path(), None)
        .err()
        .expect("postgres without storage settings should be rejected");

    assert!(error.to_string().contains("requires [storage.provider.config] settings"));
}

#[test]
#[cfg(feature = "memory-mariadb")]
fn mariadb_factory_rejects_missing_storage_settings_before_connecting() {
    let tmp = TempDir::new().unwrap();
    let cfg = memory_config("mariadb");

    let error = create_memory(&cfg, tmp.path(), None)
        .err()
        .expect("mariadb without storage settings should be rejected");

    assert!(error.to_string().contains("requires [storage.provider.config] settings"));
}

#[test]
fn create_memory_rejects_qdrant_without_config_or_env_url() {
    let _env = EnvOverride::new(&[("QDRANT_URL", None)]);
    let tmp = TempDir::new().unwrap();
    let cfg = memory_config("qdrant");

    let error = match create_memory(&cfg, tmp.path(), None) {
        Ok(_) => panic!("qdrant without url should be rejected"),
        Err(error) => error.to_string(),
    };

    assert!(error.contains("Qdrant memory backend requires url"));
}

#[test]
fn qdrant_factory_uses_lazy_backend_when_url_is_configured() {
    let tmp = TempDir::new().unwrap();
    let mut cfg = memory_config("qdrant");
    cfg.qdrant.url = Some("http://127.0.0.1:6333/".to_string());
    cfg.embedding_provider = "none".to_string();

    let memory = create_memory(&cfg, tmp.path(), None).unwrap();

    assert_eq!(memory.name(), "qdrant");
}

#[test]
fn qdrant_factory_uses_env_fallbacks_for_blank_config_values() {
    let _env = EnvOverride::new(&[
        ("QDRANT_URL", Some("http://127.0.0.1:6333/")),
        ("QDRANT_COLLECTION", Some("env_memories")),
        ("QDRANT_API_KEY", Some("env-key")),
    ]);
    let tmp = TempDir::new().unwrap();
    let mut cfg = memory_config("qdrant");
    cfg.qdrant.url = Some(" \t ".to_string());
    cfg.embedding_provider = "none".to_string();

    let memory = create_memory(&cfg, tmp.path(), None).unwrap();

    assert_eq!(memory.name(), "qdrant");
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn create_memory_runs_snapshot_export_and_auto_hydrate_paths() {
    let export_tmp = TempDir::new().unwrap();
    let export_cfg = MemoryConfig {
        backend: "sqlite".to_string(),
        embedding_provider: "none".to_string(),
        snapshot_enabled: true,
        snapshot_on_hygiene: true,
        auto_hydrate: false,
        ..MemoryConfig::default()
    };

    let exported = create_memory(&export_cfg, export_tmp.path(), None).unwrap();
    assert_eq!(exported.name(), "sqlite");

    let hydrate_tmp = TempDir::new().unwrap();
    let storage_dir = paths::project_data_dir(hydrate_tmp.path()).unwrap();
    std::fs::create_dir_all(&storage_dir).unwrap();
    std::fs::write(storage_dir.join(snapshot::SNAPSHOT_FILENAME), "# empty snapshot\n").unwrap();
    let hydrate_cfg = MemoryConfig {
        backend: "sqlite".to_string(),
        embedding_provider: "none".to_string(),
        auto_hydrate: true,
        ..MemoryConfig::default()
    };

    let hydrated = create_memory(&hydrate_cfg, hydrate_tmp.path(), None).unwrap();

    assert_eq!(hydrated.name(), "sqlite");
    assert!(storage_dir.join("memory").join("brain.db").exists());
    let _ = std::fs::remove_dir_all(storage_dir);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn create_memory_continues_when_hygiene_snapshot_or_hydration_fails() {
    let hygiene_tmp = TempDir::new().unwrap();
    let hygiene_state_dir = paths::workspace_data_dir(hygiene_tmp.path()).unwrap().join("state");
    std::fs::create_dir_all(hygiene_state_dir.parent().unwrap()).unwrap();
    std::fs::write(&hygiene_state_dir, "not a directory").unwrap();
    let hygiene_cfg = MemoryConfig {
        backend: "none".to_string(),
        hygiene_enabled: true,
        ..MemoryConfig::default()
    };
    let hygiene_memory = create_memory(&hygiene_cfg, hygiene_tmp.path(), None).unwrap();
    assert_eq!(hygiene_memory.name(), "none");

    let snapshot_tmp = TempDir::new().unwrap();
    let snapshot_storage = paths::project_data_dir(snapshot_tmp.path()).unwrap();
    let snapshot_memory_dir = snapshot_storage.join("memory");
    std::fs::create_dir_all(&snapshot_memory_dir).unwrap();
    let conn = rusqlite::Connection::open(snapshot_memory_dir.join("brain.db")).unwrap();
    drop(conn);
    let snapshot_cfg = MemoryConfig {
        backend: "sqlite".to_string(),
        embedding_provider: "none".to_string(),
        hygiene_enabled: false,
        snapshot_enabled: true,
        snapshot_on_hygiene: true,
        auto_hydrate: false,
        ..MemoryConfig::default()
    };
    let snapshot_memory = create_memory(&snapshot_cfg, snapshot_tmp.path(), None).unwrap();
    assert_eq!(snapshot_memory.name(), "sqlite");

    let hydrate_tmp = TempDir::new().unwrap();
    let hydrate_storage = paths::project_data_dir(hydrate_tmp.path()).unwrap();
    std::fs::create_dir_all(hydrate_storage.join(snapshot::SNAPSHOT_FILENAME)).unwrap();
    let hydrate_cfg = MemoryConfig {
        backend: "sqlite".to_string(),
        embedding_provider: "none".to_string(),
        hygiene_enabled: false,
        auto_hydrate: true,
        ..MemoryConfig::default()
    };
    let hydrate_memory = create_memory(&hydrate_cfg, hydrate_tmp.path(), None).unwrap();
    assert_eq!(hydrate_memory.name(), "sqlite");

    let _ = std::fs::remove_dir_all(paths::workspace_data_dir(hygiene_tmp.path()).unwrap());
    let _ = std::fs::remove_dir_all(snapshot_storage);
    let _ = std::fs::remove_dir_all(hydrate_storage);
}

#[test]
fn create_response_cache_respects_enabled_flag() {
    let tmp = TempDir::new().unwrap();
    let disabled = MemoryConfig { response_cache_enabled: false, ..MemoryConfig::default() };
    let enabled = MemoryConfig {
        response_cache_enabled: true,
        response_cache_ttl_minutes: 5,
        response_cache_max_entries: 2,
        ..MemoryConfig::default()
    };

    assert!(create_response_cache(&disabled, tmp.path()).is_none());
    assert!(create_response_cache(&enabled, tmp.path()).is_some());
}

#[test]
fn create_response_cache_returns_none_when_storage_path_is_blocked() {
    let tmp = TempDir::new().unwrap();
    let storage_dir = paths::workspace_data_dir(tmp.path()).unwrap();
    std::fs::create_dir_all(&storage_dir).unwrap();
    std::fs::write(storage_dir.join("memory"), "not a directory").unwrap();
    let cfg = MemoryConfig { response_cache_enabled: true, ..MemoryConfig::default() };

    assert!(create_response_cache(&cfg, tmp.path()).is_none());

    let _ = std::fs::remove_dir_all(storage_dir);
}
