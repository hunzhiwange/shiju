//! 验证响应缓存旧入口的类型身份、工厂接线及本地持久化。

#[test]
fn memory_response_cache_production_boundary() {
    use crate::config::MemoryConfig;
    use crate::memory::{ResponseCache, create_response_cache};

    let workspace = tempfile::TempDir::new().unwrap();
    let storage = crate::memory::paths::workspace_data_dir(workspace.path()).unwrap();
    assert!(!storage.exists(), "boundary fixture must own a fresh workspace data directory");
    std::fs::create_dir_all(&storage).unwrap();
    struct StorageCleanup(std::path::PathBuf);
    impl Drop for StorageCleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = StorageCleanup(storage.clone());
    let config = MemoryConfig {
        response_cache_enabled: true,
        response_cache_ttl_minutes: 60,
        response_cache_max_entries: 2,
        ..MemoryConfig::default()
    };
    let cache = create_response_cache(&config, workspace.path()).unwrap();
    let _: &memory_response_cache::memory::response_cache::ResponseCache = &cache;
    let _: &crate::memory::response_cache::ResponseCache = &cache;
    let key = ResponseCache::cache_key("boundary-model", Some("system"), "prompt");

    cache.put(&key, "boundary-model", "persisted response", 7).unwrap();
    assert_eq!(cache.get(&key).unwrap().as_deref(), Some("persisted response"));
    assert_eq!(cache.stats().unwrap(), (1, 1, 7));
    assert!(storage.join("memory/response_cache.db").is_file());
    assert!(!workspace.path().join("memory/response_cache.db").exists());
    drop(cache);

    let reopened = ResponseCache::new(workspace.path(), 60, 2).unwrap();
    assert_eq!(reopened.get(&key).unwrap().as_deref(), Some("persisted response"));
    assert_eq!(reopened.clear().unwrap(), 1);
    assert_eq!(reopened.stats().unwrap(), (0, 0, 0));
}
