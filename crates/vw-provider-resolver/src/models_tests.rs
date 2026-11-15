//! 模型目录运行时入口与缓存测试。
use super::*;
use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
#[path = "models_cache_tests.rs"]
mod cache;

fn provider_json(id: &str) -> String {
    format!(
        r#"{{
            "id": "{id}",
            "name": "{id} provider",
            "env": ["{id}_KEY"],
            "models": {{
                "{id}-model": {{
                    "id": "{id}-model",
                    "name": "{id} model",
                    "limit": {{"context": 8, "output": 4}}
                }}
            }}
        }}"#
    )
}

fn provider_map_json(id: &str) -> String {
    format!(r#"{{"{id}": {}}}"#, provider_json(id))
}

fn assert_provider(map: &HashMap<String, Provider>, id: &str) {
    let provider = map.get(id).expect("provider should be parsed");
    assert_eq!(provider.id, id);
    assert_eq!(provider.name, format!("{id} provider"));
    assert!(provider.models.contains_key(&format!("{id}-model")));
}

#[test]
fn url_returns_default_models_dev_url() {
    assert_eq!(models_url(None), "https://models.dev");
    assert_eq!(models_url(Some("https://models.example.test")), "https://models.example.test");
}

#[test]
fn cache_path_uses_global_cache_directory() {
    assert_eq!(cache_path(), global::paths().cache.join("models.json"));
}

#[test]
fn init_is_idempotent() {
    init();
    init();
}
