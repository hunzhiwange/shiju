use super::*;
use std::collections::HashSet;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;

#[cfg(not(target_arch = "wasm32"))]
static MODELS_STATE_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

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

#[cfg(not(target_arch = "wasm32"))]
fn spawn_http_response(status: &str, body: String) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
    let addr = listener.local_addr().expect("test server address should be available");
    let status = status.to_string();
    let handle = thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let mut request = [0_u8; 1024];
        let _ = stream.read(&mut request);
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(response.as_bytes());
    });

    (format!("http://{addr}/api.json"), handle)
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_incomplete_http_body() -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
    let addr = listener.local_addr().expect("test server address should be available");
    let handle = thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let mut request = [0_u8; 1024];
        let _ = stream.read(&mut request);
        let response = "HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\nshort";
        let _ = stream.write_all(response.as_bytes());
    });

    (format!("http://{addr}/api.json"), handle)
}

#[cfg(not(target_arch = "wasm32"))]
fn unused_local_url() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test port should bind");
    let addr = listener.local_addr().expect("test server address should be available");
    drop(listener);
    format!("http://{addr}/api.json")
}

#[cfg(not(target_arch = "wasm32"))]
fn refresh_test_path(name: &str) -> PathBuf {
    global::paths().cache.join(format!("{name}-{}.json", std::process::id()))
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn refresh_from_url_writes_successful_body_and_invalidates_cache() {
    let _state_lock = MODELS_STATE_TEST_LOCK.lock().await;
    let path = refresh_test_path("models-refresh-success");
    let body = provider_map_json("remote");
    let (url, handle) = spawn_http_response("200 OK", body.clone());

    {
        let mut lock = CACHE.lock().expect("cache lock should be available");
        *lock = Some(HashMap::new());
    }

    refresh_from_url(path.clone(), url).await;
    handle.join().expect("test server should finish");

    assert_eq!(fs::read_to_string(&path).unwrap(), body);
    let lock = CACHE.lock().expect("cache lock should be available");
    assert!(lock.is_none());
    drop(lock);
    let _ = fs::remove_file(path);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn refresh_from_url_keeps_file_when_status_is_not_successful() {
    let path = refresh_test_path("models-refresh-status");
    let original = "original";
    fs::write(&path, original).expect("refresh test file should be written");
    let (url, handle) = spawn_http_response("500 Internal Server Error", "ignored".to_string());

    refresh_from_url(path.clone(), url).await;
    handle.join().expect("test server should finish");

    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    let _ = fs::remove_file(path);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn refresh_from_url_returns_when_request_fails() {
    let path = refresh_test_path("models-refresh-error");

    refresh_from_url(path.clone(), unused_local_url()).await;

    assert!(!path.exists());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn refresh_from_url_returns_when_body_read_fails() {
    let path = refresh_test_path("models-refresh-body");
    let (url, handle) = spawn_incomplete_http_body();

    refresh_from_url(path.clone(), url).await;
    handle.join().expect("test server should finish");

    assert!(!path.exists());
}

#[cfg(not(target_arch = "wasm32"))]
fn active_models_path() -> PathBuf {
    flag::VIBEWINDOW_MODELS_PATH.clone().map(PathBuf::from).unwrap_or_else(cache_path)
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
fn parse_models_text_accepts_direct_provider_map() {
    let parsed = parse_models_text("direct", &provider_map_json("direct")).unwrap();

    assert_provider(&parsed, "direct");
}

#[test]
fn parse_models_text_accepts_providers_wrapper() {
    let text = format!(r#"{{"providers": {}}}"#, provider_map_json("wrapped"));
    let parsed = parse_models_text("wrapped", &text).unwrap();

    assert_provider(&parsed, "wrapped");
}

#[test]
fn parse_models_text_accepts_data_wrapper() {
    let text = format!(r#"{{"data": {}}}"#, provider_map_json("data"));
    let parsed = parse_models_text("data", &text).unwrap();

    assert_provider(&parsed, "data");
}

#[test]
fn parse_models_text_accepts_provider_array_and_filters_empty_ids() {
    let text = format!(r#"[{}, {}]"#, provider_json("array"), provider_json(""));
    let parsed = parse_models_text("array", &text).unwrap();

    assert_provider(&parsed, "array");
    assert!(!parsed.contains_key(""));
}

#[test]
fn parse_models_text_accepts_empty_provider_map_and_array() {
    assert!(parse_models_text("empty_map", "{}").unwrap().is_empty());
    assert!(parse_models_text("empty_array", "[]").unwrap().is_empty());
}

#[test]
fn parse_models_text_rejects_invalid_or_unsupported_shapes() {
    assert!(parse_models_text("invalid", "{").is_none());
    assert!(parse_models_text("other", "42").is_none());
    assert!(parse_models_text("bad_wrapper", r#"{"providers": 42}"#).is_none());
}

#[test]
fn parse_models_text_falls_back_from_bad_providers_wrapper_to_data_wrapper() {
    let text = format!(
        r#"{{
            "providers": 42,
            "data": {}
        }}"#,
        provider_map_json("fallback")
    );

    let parsed = parse_models_text("fallback", &text).unwrap();

    assert_provider(&parsed, "fallback");
}

#[test]
fn parse_models_text_prefers_providers_wrapper_over_data_wrapper() {
    let text = format!(
        r#"{{"providers": {}, "data": {}}}"#,
        provider_map_json("preferred"),
        provider_map_json("fallback")
    );

    let parsed = parse_models_text("wrapper_precedence", &text).unwrap();

    assert_provider(&parsed, "preferred");
    assert!(!parsed.contains_key("fallback"));
}

#[test]
fn parse_models_text_rejects_array_items_with_invalid_provider_shape() {
    let text = r#"[{"id": "bad", "models": []}]"#;

    assert!(parse_models_text("bad_array", text).is_none());
}

#[test]
fn bundled_models_loads_included_metadata() {
    let bundled = bundled_models("bundled");

    assert!(!bundled.is_empty());
}

#[test]
fn bundled_models_include_shiju_static_catalog() {
    let bundled = bundled_models("bundled_shiju");
    let shiju = bundled.get("shiju").expect("bundled models should include shiju");
    let expected = HashSet::from([
        "qwen-coder-turbo",
        "gte-rerank-v2",
        "qwen3-max-preview",
        "qwen-mt-turbo",
        "qwen3-32b",
        "qwen3-235b-a22b",
        "qvq-max",
        "deepseek-v3",
        "glm-4.7",
        "text-embedding-v3",
        "qwen3-30b-a3b",
        "MiniMax-M2.5",
        "qwen-plus-latest",
        "qwen-plus",
        "glm-5.2",
        "deepseek-v3.2",
        "glm-5",
        "qwen3-235b-a22b-instruct-2507",
        "glm-5.1",
        "qwen3.5-plus",
        "deepseek-v4-pro",
        "qwen-image-edit-max",
        "qwen-math-plus",
        "qwen3.7-text-embedding",
        "deepseek-r1-distill-qwen-14b",
        "qwen3.5-flash",
        "deepseek-r1-distill-qwen-32b",
        "qwen3-next-80b-a3b-thinking",
        "text-embedding-v4",
        "qwen-image-2.0",
        "qwen-turbo",
        "qwen3-235b-a22b-thinking-2507",
        "qwen3-vl-flash",
        "deepseek-v4-flash",
    ]);
    let actual = shiju.models.keys().map(String::as_str).collect::<HashSet<_>>();

    assert_eq!(shiju.id, "shiju");
    assert_eq!(shiju.name, "识矩");
    assert_eq!(shiju.env, vec!["SHIJU_API_KEY"]);
    assert_eq!(shiju.api.as_deref(), Some("https://router.shiju.cloud/v1"));
    assert_eq!(shiju.adapter.as_deref(), Some("openai-compatible"));
    assert_eq!(actual, expected);
    assert_eq!(shiju.models.len(), 34);
    assert!(shiju.models.contains_key("qwen-coder-turbo"));
    assert!(shiju.models.contains_key("deepseek-v4-flash"));
    let vision = &shiju.models["qwen3-vl-flash"];
    assert!(vision.attachment);
    assert_eq!(
        vision.modalities.as_ref().map(|modalities| modalities.input.as_slice()),
        Some(["text".to_string(), "image".to_string()].as_slice())
    );
    assert!(bundled.contains_key("openai"));
}

#[tokio::test]
async fn get_returns_process_cache_until_invalidated() {
    let _state_lock = MODELS_STATE_TEST_LOCK.lock().await;
    let mut cached = HashMap::new();
    let provider = Provider {
        api: None,
        name: "cached provider".to_string(),
        env: Vec::new(),
        id: "cached".to_string(),
        adapter: None,
        models: HashMap::new(),
    };
    cached.insert(provider.id.clone(), provider);

    {
        let mut lock = CACHE.lock().expect("cache lock should be available");
        *lock = Some(cached);
    }

    let data = get().await;
    assert_eq!(data.len(), 1);
    assert_eq!(data["cached"].name, "cached provider");

    invalidate_cache();
    let lock = CACHE.lock().expect("cache lock should be available");
    assert!(lock.is_none());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn get_loads_and_stores_data_when_process_cache_is_empty() {
    let _state_lock = MODELS_STATE_TEST_LOCK.lock().await;
    let path = active_models_path();
    let original = fs::read(&path).ok();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("cache directory should be created");
    }
    fs::write(&path, provider_map_json("loaded")).expect("cache file should be written");
    invalidate_cache();

    let data = get().await;
    assert_provider(&data, "loaded");

    let lock = CACHE.lock().expect("cache lock should be available");
    assert!(lock.as_ref().is_some_and(|cache| cache.contains_key("loaded")));
    drop(lock);
    invalidate_cache();

    match original {
        Some(bytes) => fs::write(&path, bytes).expect("cache file should be restored"),
        None => {
            let _ = fs::remove_file(&path);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn load_reads_non_empty_cache_before_bundled_fallback() {
    let _state_lock = MODELS_STATE_TEST_LOCK.lock().await;
    let path = active_models_path();
    let original = fs::read(&path).ok();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("cache directory should be created");
    }
    fs::write(&path, provider_map_json("disk")).expect("cache file should be written");

    let data = load().await;
    assert_provider(&data, "disk");
    assert!(data.contains_key("shiju"));

    match original {
        Some(bytes) => fs::write(&path, bytes).expect("cache file should be restored"),
        None => {
            let _ = fs::remove_file(&path);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn load_falls_back_to_bundled_models_for_empty_or_invalid_cache() {
    let _state_lock = MODELS_STATE_TEST_LOCK.lock().await;
    let path = active_models_path();
    let original = fs::read(&path).ok();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("cache directory should be created");
    }
    fs::write(&path, "{}").expect("cache file should be written");

    let data = load().await;
    assert!(!data.is_empty());
    assert_eq!(fs::read_to_string(&path).unwrap(), BUNDLED_MODELS_TEXT);

    fs::write(&path, "{").expect("cache file should be written");

    let data = load().await;
    assert!(!data.is_empty());
    assert_eq!(fs::read_to_string(&path).unwrap(), BUNDLED_MODELS_TEXT);

    match original {
        Some(bytes) => fs::write(&path, bytes).expect("cache file should be restored"),
        None => {
            let _ = fs::remove_file(&path);
        }
    }
}

#[test]
fn init_is_idempotent() {
    init();
    init();
}
