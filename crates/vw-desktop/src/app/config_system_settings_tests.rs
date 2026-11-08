//! 验证系统设置配置在应用状态与持久化结构之间的转换。
//!
//! 往返夹具比较持久化配置与应用设置状态，确保缺省值和可选字段不漂移。

use std::ffi::OsStr;
#[cfg(not(target_arch = "wasm32"))]
use std::io::{Read, Write};
#[cfg(not(target_arch = "wasm32"))]
use std::net::{TcpListener, TcpStream};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{self, Receiver};
use std::sync::{LazyLock, Mutex, MutexGuard};

use tempfile::TempDir;
use vw_config_types::ui::{
    AppSystemSettingsConfig, GatewayClientServerConfig, GatewayClientSystemSettingsConfig,
};

const SOURCE: &str = include_str!("config_system_settings.rs");
static CONFIG_SYSTEM_SETTINGS_ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

pub(in crate::app) struct EnvGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set_os(key: &'static str, value: &OsStr) -> Self {
        let previous = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, previous }
    }

    fn remove(key: &'static str) -> Self {
        let previous = std::env::var_os(key);
        unsafe { std::env::remove_var(key) };
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

fn env_lock() -> MutexGuard<'static, ()> {
    CONFIG_SYSTEM_SETTINGS_ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner())
}

fn temp_home() -> (MutexGuard<'static, ()>, TempDir, EnvGuard) {
    let lock = env_lock();
    let home = tempfile::tempdir().expect("temp home should be created");
    let guard = EnvGuard::set_os("HOME", home.path().as_os_str());
    (lock, home, guard)
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn isolated_settings() -> (MutexGuard<'static, ()>, TempDir, EnvGuard) {
    let lock = env_lock();
    let root = tempfile::tempdir().expect("temp settings directory");
    let guard = EnvGuard::set_os("VIBEWINDOW_CONFIG_DIR", root.path().as_os_str());
    (lock, root, guard)
}

fn source_declares_symbol(name: &str) -> bool {
    let needles = [
        format!("fn {name}"),
        format!("pub fn {name}"),
        format!("struct {name}"),
        format!("pub struct {name}"),
        format!("enum {name}"),
        format!("pub enum {name}"),
        format!("type {name}"),
        format!("pub type {name}"),
        format!("const {name}"),
        format!("pub const {name}"),
        format!("static {name}"),
        format!("pub static {name}"),
        format!("impl {name}"),
    ];

    needles.iter().any(|needle| SOURCE.contains(needle))
}

fn gateway_config(host: &str, port: u16) -> GatewayClientSystemSettingsConfig {
    let mut config = GatewayClientSystemSettingsConfig::default();
    let mut server = GatewayClientServerConfig::default();
    server.host = host.to_string();
    server.port = port;
    config.set_servers(vec![server], "local".to_string());
    config
}

#[cfg(not(target_arch = "wasm32"))]
fn serve_global_config(
    responses: Vec<(u16, String)>,
) -> (u16, Receiver<String>, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("stub server should bind");
    let port = listener.local_addr().expect("stub address should exist").port();
    let (requests_tx, requests_rx) = mpsc::channel();
    let join = std::thread::spawn(move || {
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().expect("gateway request should connect");
            let request = read_http_request(&mut stream);
            requests_tx.send(request).expect("request should be captured");
            let reason = if status == 200 { "OK" } else { "Test Error" };
            let response = format!(
                "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).expect("response should be written");
        }
    });
    (port, requests_rx, join)
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn read_http_request(stream: &mut TcpStream) -> String {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 512];
    loop {
        let read = stream.read(&mut buffer).expect("request should be readable");
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..read]);
        if let Some(header_end) =
            request.windows(4).position(|window| window == b"\r\n\r\n").map(|index| index + 4)
        {
            let headers = String::from_utf8_lossy(&request[..header_end]);
            if request.len() >= header_end + http_content_length(&headers) {
                break;
            }
        }
    }
    String::from_utf8(request).expect("request should be utf8")
}

#[cfg(not(target_arch = "wasm32"))]
fn http_content_length(headers: &str) -> usize {
    headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if !name.eq_ignore_ascii_case("content-length") {
                return None;
            }
            value.trim().parse().ok()
        })
        .unwrap_or(0)
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn request_json_body(request: &str) -> serde_json::Value {
    let (_, body) = request.split_once("\r\n\r\n").expect("request should contain a body");
    serde_json::from_str(body).expect("request body should be json")
}

#[cfg(not(target_arch = "wasm32"))]
fn test_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime should build")
}

fn normalized_default_with_gateway(
    gateway_client: GatewayClientSystemSettingsConfig,
) -> AppSystemSettingsConfig {
    let mut expected = super::normalize_system_settings_config(AppSystemSettingsConfig::default());
    expected.gateway_client = gateway_client;
    expected
}

fn assert_f32_close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.001, "expected {actual} to be close to {expected}");
}

#[test]
fn config_system_settings_tests_keeps_planned_coverage_targets() {
    for name in [
        "fetch_desktop_system_settings_via_gateway",
        "patch_desktop_system_settings_via_gateway",
        "normalize_system_settings_config",
        "gateway_client_bootstrap_cache_path",
        "load_gateway_client_bootstrap_config",
        "save_gateway_client_bootstrap_config",
        "load_gateway_client_config",
        "update_gateway_client_config",
        "load_system_settings_config_async",
    ] {
        assert!(source_declares_symbol(name), "expected source to declare coverage target {name}");
    }
}

#[test]
fn normalize_system_settings_config_replaces_invalid_font_size() {
    let cfg = AppSystemSettingsConfig {
        editor_font_size: f32::NAN,
        editor_line_height: 99.0,
        editor_auto_line_height: false,
        ..AppSystemSettingsConfig::default()
    };

    let normalized = super::normalize_system_settings_config(cfg);

    assert_eq!(normalized.editor_font_size, 16.0);
    assert_eq!(normalized.editor_line_height, 99.0);
}

#[test]
fn normalize_system_settings_config_replaces_non_positive_and_infinite_font_sizes() {
    for editor_font_size in [0.0, -1.0, f32::INFINITY, f32::NEG_INFINITY] {
        let cfg = AppSystemSettingsConfig {
            editor_font_size,
            editor_line_height: 24.0,
            editor_auto_line_height: false,
            ..AppSystemSettingsConfig::default()
        };

        let normalized = super::normalize_system_settings_config(cfg);

        assert_eq!(normalized.editor_font_size, 16.0);
        assert_eq!(normalized.editor_line_height, 24.0);
    }
}

#[test]
fn normalize_system_settings_config_auto_line_height_tracks_font_size() {
    let cfg = AppSystemSettingsConfig {
        editor_font_size: 15.0,
        editor_line_height: 1.0,
        editor_auto_line_height: true,
        ..AppSystemSettingsConfig::default()
    };

    let normalized = super::normalize_system_settings_config(cfg);

    assert_eq!(normalized.editor_font_size, 15.0);
    assert_eq!(normalized.editor_line_height, 21.0);
}

#[test]
fn normalize_system_settings_config_auto_line_height_uses_repaired_font_size() {
    let cfg = AppSystemSettingsConfig {
        editor_font_size: f32::NAN,
        editor_line_height: 1.0,
        editor_auto_line_height: true,
        ..AppSystemSettingsConfig::default()
    };

    let normalized = super::normalize_system_settings_config(cfg);

    assert_eq!(normalized.editor_font_size, 16.0);
    assert_eq!(normalized.editor_line_height, 22.4);
}

#[test]
fn normalize_system_settings_config_replaces_invalid_manual_line_height() {
    for editor_line_height in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let cfg = AppSystemSettingsConfig {
            editor_font_size: 16.0,
            editor_line_height,
            editor_auto_line_height: false,
            ..AppSystemSettingsConfig::default()
        };

        let normalized = super::normalize_system_settings_config(cfg);

        assert_eq!(normalized.editor_font_size, 16.0);
        assert_eq!(normalized.editor_line_height, 19.6);
    }
}

#[test]
fn normalize_system_settings_config_replaces_overflowing_auto_line_height() {
    let cfg = AppSystemSettingsConfig {
        editor_font_size: f32::MAX,
        editor_line_height: 1.0,
        editor_auto_line_height: true,
        ..AppSystemSettingsConfig::default()
    };

    let normalized = super::normalize_system_settings_config(cfg);

    assert_eq!(normalized.editor_font_size, f32::MAX);
    assert_eq!(normalized.editor_line_height, 19.6);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn gateway_client_bootstrap_cache_path_uses_vibewindow_home_file() {
    let (_lock, home, _guard) = temp_home();

    let path = super::gateway_client_bootstrap_cache_path().expect("path should exist");

    assert_eq!(
        path,
        vw_config_types::paths::home_config_dir(home.path()).join("gateway-client-bootstrap.json")
    );
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn gateway_client_bootstrap_cache_path_requires_home() {
    let _lock = env_lock();
    let _guard = EnvGuard::remove("HOME");

    assert_eq!(super::gateway_client_bootstrap_cache_path(), None);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn save_gateway_client_bootstrap_config_writes_loadable_json() {
    let (_lock, home, _guard) = temp_home();
    let config = gateway_config("10.0.0.8", 49152);

    super::save_gateway_client_bootstrap_config(&config).unwrap();

    let path =
        vw_config_types::paths::home_config_dir(home.path()).join("gateway-client-bootstrap.json");
    let saved = std::fs::read_to_string(path).expect("bootstrap json should be written");
    assert!(saved.contains("10.0.0.8"));
    assert_eq!(super::load_gateway_client_bootstrap_config(), config);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn load_gateway_client_bootstrap_config_ignores_invalid_json() {
    let (_lock, home, _guard) = temp_home();
    let cache_dir = vw_config_types::paths::home_config_dir(home.path());
    std::fs::create_dir_all(&cache_dir).expect("cache dir should be created");
    std::fs::write(cache_dir.join("gateway-client-bootstrap.json"), "{not-json")
        .expect("invalid cache should be written");

    let config = super::load_gateway_client_bootstrap_config();

    assert_eq!(config, GatewayClientSystemSettingsConfig::default());
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn load_gateway_client_bootstrap_config_falls_back_to_legacy_cache() {
    let (_lock, home, _guard) = temp_home();
    let legacy_path =
        super::legacy_gateway_client_bootstrap_cache_path().expect("legacy path should exist");
    assert!(legacy_path.starts_with(home.path()), "legacy cache should stay in temp home");
    std::fs::create_dir_all(legacy_path.parent().expect("legacy cache should have a parent"))
        .expect("legacy cache dir should be created");
    let legacy = gateway_config("legacy.gateway", 7777);
    std::fs::write(&legacy_path, serde_json::to_string(&legacy).expect("config should serialize"))
        .expect("legacy cache should be written");

    let loaded = super::load_gateway_client_bootstrap_config();

    assert_eq!(loaded, legacy);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn load_gateway_client_bootstrap_config_prefers_home_cache_over_legacy_cache() {
    let (_lock, home, _guard) = temp_home();
    let legacy_path =
        super::legacy_gateway_client_bootstrap_cache_path().expect("legacy path should exist");
    assert!(legacy_path.starts_with(home.path()), "legacy cache should stay in temp home");
    std::fs::create_dir_all(legacy_path.parent().expect("legacy cache should have a parent"))
        .expect("legacy cache dir should be created");
    let legacy = gateway_config("legacy.gateway", 7777);
    std::fs::write(&legacy_path, serde_json::to_string(&legacy).expect("config should serialize"))
        .expect("legacy cache should be written");
    let current = gateway_config("current.gateway", 8888);
    super::save_gateway_client_bootstrap_config(&current).unwrap();

    let loaded = super::load_gateway_client_bootstrap_config();

    assert_eq!(loaded, current);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn load_gateway_client_bootstrap_config_does_not_fall_back_after_invalid_home_cache() {
    let (_lock, home, _guard) = temp_home();
    let legacy_path =
        super::legacy_gateway_client_bootstrap_cache_path().expect("legacy path should exist");
    assert!(legacy_path.starts_with(home.path()), "legacy cache should stay in temp home");
    std::fs::create_dir_all(legacy_path.parent().expect("legacy cache should have a parent"))
        .expect("legacy cache dir should be created");
    let legacy = gateway_config("legacy.gateway", 7777);
    std::fs::write(&legacy_path, serde_json::to_string(&legacy).expect("config should serialize"))
        .expect("legacy cache should be written");
    let current_dir = vw_config_types::paths::home_config_dir(home.path());
    std::fs::create_dir_all(&current_dir).expect("current cache dir should be created");
    std::fs::write(current_dir.join("gateway-client-bootstrap.json"), "{not-json")
        .expect("invalid current cache should be written");

    let loaded = super::load_gateway_client_bootstrap_config();

    assert_eq!(loaded, GatewayClientSystemSettingsConfig::default());
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn load_gateway_client_config_returns_bootstrap_cache() {
    let (_lock, _home, _guard) = temp_home();
    let config = gateway_config("gateway.internal", 9090);
    super::save_gateway_client_bootstrap_config(&config).unwrap();

    let loaded = super::load_gateway_client_config();

    assert_eq!(loaded, config);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn load_system_settings_config_async_restores_gateway_snapshot() {
    let (_lock, home, _guard) = temp_home();
    let database_settings = AppSystemSettingsConfig {
        editor_font_size: 18.0,
        editor_auto_line_height: true,
        ..AppSystemSettingsConfig::default()
    };
    let body = serde_json::json!({
        "app_ui": {"system_settings": database_settings}
    })
    .to_string();
    let (port, requests, join) = serve_global_config(vec![(200, body)]);
    let bootstrap = gateway_config("127.0.0.1", port);
    super::save_gateway_client_bootstrap_config(&bootstrap).unwrap();
    std::fs::write(
        home.path().join("vibewindow.json"),
        serde_json::json!({
            "app_ui": {"system_settings": {"editor_font_size": 9.0}}
        })
        .to_string(),
    )
    .expect("legacy config should be written");

    let loaded = test_runtime()
        .block_on(super::load_system_settings_config_async())
        .expect("gateway settings should load");
    let request = requests.recv().expect("GET request should be captured");
    join.join().expect("stub should stop");

    assert!(request.starts_with("GET /v1/global/config "));
    assert_eq!(loaded.editor_font_size, 18.0);
    assert_f32_close(loaded.editor_line_height, 25.2);
    assert_eq!(loaded.gateway_client, bootstrap);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn load_system_settings_config_async_uses_defaults_when_gateway_path_is_missing() {
    let (_lock, _home, _guard) = temp_home();
    let (port, requests, join) =
        serve_global_config(vec![(200, serde_json::json!({"app_ui": {}}).to_string())]);
    let bootstrap = gateway_config("127.0.0.1", port);
    super::save_gateway_client_bootstrap_config(&bootstrap).unwrap();

    let loaded = test_runtime()
        .block_on(super::load_system_settings_config_async())
        .expect("missing settings should use defaults");
    let request = requests.recv().expect("GET request should be captured");
    join.join().expect("stub should stop");

    assert!(request.starts_with("GET /v1/global/config "));
    assert_eq!(loaded, normalized_default_with_gateway(bootstrap));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn load_system_settings_config_async_uses_defaults_for_malformed_gateway_settings() {
    let (_lock, _home, _guard) = temp_home();
    let body = serde_json::json!({
        "app_ui": {"system_settings": "not-an-object"}
    })
    .to_string();
    let (port, requests, join) = serve_global_config(vec![(200, body)]);
    let bootstrap = gateway_config("127.0.0.1", port);
    super::save_gateway_client_bootstrap_config(&bootstrap).unwrap();

    let loaded = test_runtime()
        .block_on(super::load_system_settings_config_async())
        .expect("malformed settings should use defaults");
    let request = requests.recv().expect("GET request should be captured");
    join.join().expect("stub should stop");

    assert!(request.starts_with("GET /v1/global/config "));
    assert_eq!(loaded, normalized_default_with_gateway(bootstrap));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn load_system_settings_config_async_uses_defaults_when_gateway_is_unavailable() {
    let (_lock, _home, _guard) = temp_home();
    let listener = TcpListener::bind("127.0.0.1:0").expect("unused port should bind");
    let port = listener.local_addr().expect("unused address should exist").port();
    drop(listener);
    let bootstrap = gateway_config("127.0.0.1", port);
    super::save_gateway_client_bootstrap_config(&bootstrap).unwrap();

    let loaded = test_runtime()
        .block_on(super::load_system_settings_config_async())
        .expect("unavailable gateway should use defaults");

    assert_eq!(loaded, normalized_default_with_gateway(bootstrap));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn load_system_settings_config_runs_gateway_load_synchronously() {
    let (_lock, _home, _guard) = temp_home();
    let settings = AppSystemSettingsConfig {
        editor_font_size: 17.0,
        editor_auto_line_height: true,
        ..AppSystemSettingsConfig::default()
    };
    let body = serde_json::json!({"app_ui": {"system_settings": settings}}).to_string();
    let (port, requests, join) = serve_global_config(vec![(200, body)]);
    let bootstrap = gateway_config("127.0.0.1", port);
    super::save_gateway_client_bootstrap_config(&bootstrap).unwrap();

    let loaded = super::load_system_settings_config();
    let request = requests.recv().expect("GET request should be captured");
    join.join().expect("stub should stop");

    assert!(request.starts_with("GET /v1/global/config "));
    assert_eq!(loaded.editor_font_size, 17.0);
    assert_f32_close(loaded.editor_line_height, 23.8);
    assert_eq!(loaded.gateway_client, bootstrap);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn system_settings_update_preserves_concurrent_editor_fields() {
    let (_lock, home, _home_guard) = temp_home();
    let _config_guard = EnvGuard::set_os("VIBEWINDOW_CONFIG_DIR", home.path().as_os_str());
    let snapshot = serde_json::json!({"app_ui": {"system_settings": {
        "editor_font_size": 14.0,
        "terminal_font_size": 14.0,
    }}});
    let (port, requests, join) = serve_global_config(vec![
        (200, snapshot.to_string()),
        (200, "{}".into()),
        (200, "{}".into()),
    ]);
    super::save_gateway_client_bootstrap_config(&gateway_config("127.0.0.1", port)).unwrap();

    replay_stale_system_settings_update();
    join.join().expect("stub should stop");
    let requests = requests.into_iter().collect::<Vec<_>>();
    assert_eq!(requests.len(), 3);
    assert!(requests[0].starts_with("GET /v1/global/config "));
    assert!(requests[1].starts_with("PATCH /v1/global/config "));
    assert!(requests[2].starts_with("PATCH /v1/global/config "));
    let patch = request_json_body(&requests[2]);
    let settings = patch["app_ui"]["system_settings"].as_object().unwrap();
    assert!(!settings.contains_key("editor_font_size"), "patch included the stale editor size");
    assert_eq!(settings.len(), 2);
    assert_eq!(settings["terminal_font_size"], 19.0);
    assert_eq!(settings["lsp_enabled"], true);
}

#[cfg(not(target_arch = "wasm32"))]
fn replay_stale_system_settings_update() {
    test_runtime()
        .block_on(super::update_system_settings_config_result_async(|config| {
            assert_eq!(config.editor_font_size, 14.0);
            assert_eq!(config.terminal_font_size, 14.0);
            // Hold the fetched snapshot while a separate client completes the editor write.
            std::thread::spawn(|| {
                let editor_patch = serde_json::json!({"editor_font_size": 24.0});
                test_runtime()
                    .block_on(async {
                        super::gateway_client()?.desktop_system_settings_patch(&editor_patch).await
                    })
                    .expect("editor patch should succeed");
            })
            .join()
            .expect("editor update should finish");
            config.terminal_font_size = 19.0;
            config.lsp_enabled = true;
        }))
        .expect("legacy settings patch should succeed");
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
#[ignore = "requires an authenticated throwaway gateway seeded with both font sizes at 14"]
fn stale_system_settings_replay_on_live_gateway() {
    let (_lock, home, _home_guard) = temp_home();
    let _config_guard = EnvGuard::set_os("VIBEWINDOW_CONFIG_DIR", home.path().as_os_str());
    let fixture = std::env::var_os("VIBEWINDOW_TEST_SYSTEM_SETTINGS_BOOTSTRAP")
        .expect("the live test requires a bootstrap fixture path");
    let bootstrap: GatewayClientSystemSettingsConfig =
        serde_json::from_slice(&std::fs::read(fixture).expect("read bootstrap fixture"))
            .expect("parse bootstrap fixture");
    super::save_gateway_client_bootstrap_config(&bootstrap).unwrap();

    replay_stale_system_settings_update();
    let settings = test_runtime()
        .block_on(super::fetch_desktop_system_settings_via_gateway())
        .expect("read live gateway settings")
        .expect("system settings should exist");
    assert_eq!(settings.editor_font_size, 24.0);
    assert_eq!(settings.terminal_font_size, 19.0);
    assert!(settings.lsp_enabled);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn update_system_settings_config_result_async_normalizes_and_patches_settings() {
    let (_lock, _home, _guard) = temp_home();
    let settings = AppSystemSettingsConfig {
        editor_font_size: 18.0,
        editor_line_height: 24.0,
        ..AppSystemSettingsConfig::default()
    };
    let body = serde_json::json!({
        "app_ui": {"system_settings": settings}
    })
    .to_string();
    let (port, requests, join) =
        serve_global_config(vec![(200, body), (200, serde_json::json!({}).to_string())]);
    let bootstrap = gateway_config("127.0.0.1", port);
    super::save_gateway_client_bootstrap_config(&bootstrap).unwrap();

    test_runtime()
        .block_on(super::update_system_settings_config_result_async(|config| {
            config.editor_font_size = f32::NAN;
            config.editor_line_height = -1.0;
            config.editor_auto_line_height = false;
        }))
        .expect("settings patch should succeed");
    let get_request = requests.recv().expect("GET request should be captured");
    let patch_request = requests.recv().expect("PATCH request should be captured");
    join.join().expect("stub should stop");
    let patch = request_json_body(&patch_request);

    assert!(get_request.starts_with("GET /v1/global/config "));
    assert!(patch_request.starts_with("PATCH /v1/global/config "));
    assert_eq!(
        patch["app_ui"]["system_settings"],
        serde_json::json!({"editor_font_size": 16.0, "editor_line_height": 19.6_f32})
    );
    assert_eq!(super::load_gateway_client_bootstrap_config(), bootstrap);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn update_system_settings_config_result_async_clears_arrays_and_removes_object_keys() {
    let (_lock, _home, _guard) = temp_home();
    let body = serde_json::json!({"app_ui": {"system_settings": {
        "project_worktree_enabled": {"/remove": true, "/keep": true},
        "quick_messages": [{"text": "hello", "group": "git", "kind": "createNewSession"}],
    }}});
    let (port, requests, join) =
        serve_global_config(vec![(200, body.to_string()), (200, "{}".into())]);
    super::save_gateway_client_bootstrap_config(&gateway_config("127.0.0.1", port)).unwrap();

    test_runtime()
        .block_on(super::update_system_settings_config_result_async(|config| {
            config.project_worktree_enabled.remove("/remove");
            config.quick_messages.clear();
        }))
        .expect("settings patch should succeed");
    join.join().expect("stub should stop");
    let requests = requests.into_iter().collect::<Vec<_>>();
    assert_eq!(requests.len(), 2);
    let patch = request_json_body(&requests[1]);
    assert_eq!(
        patch["app_ui"]["system_settings"],
        serde_json::json!({"project_worktree_enabled": {"/remove": null}, "quick_messages": []})
    );
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn update_system_settings_config_result_async_skips_unchanged_settings() {
    let (_lock, _home, _guard) = temp_home();
    let (port, requests, join) = serve_global_config(vec![(200, "{}".into())]);
    super::save_gateway_client_bootstrap_config(&gateway_config("127.0.0.1", port)).unwrap();

    test_runtime()
        .block_on(super::update_system_settings_config_result_async(|_| {}))
        .expect("unchanged settings should succeed with just a read");
    join.join().expect("stub should stop");
    let requests = requests.into_iter().collect::<Vec<_>>();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].starts_with("GET /v1/global/config "));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn update_system_settings_config_result_async_patches_quick_messages() {
    use vw_config_types::ui::{QuickMessageConfig, QuickMessageGroup, QuickMessageKind};

    let (_lock, _home, _guard) = temp_home();
    let body = serde_json::json!({
        "app_ui": {"system_settings": AppSystemSettingsConfig::default()}
    })
    .to_string();
    let (port, requests, join) =
        serve_global_config(vec![(200, body), (200, serde_json::json!({}).to_string())]);
    let bootstrap = gateway_config("127.0.0.1", port);
    super::save_gateway_client_bootstrap_config(&bootstrap).unwrap();

    test_runtime()
        .block_on(super::update_system_settings_config_result_async(|config| {
            config.quick_messages = vec![QuickMessageConfig {
                text: "创建新会话".to_string(),
                group: QuickMessageGroup::Git,
                kind: QuickMessageKind::CreateNewSession,
            }];
        }))
        .expect("quick messages patch should succeed");
    let _get_request = requests.recv().expect("GET request should be captured");
    let patch_request = requests.recv().expect("PATCH request should be captured");
    join.join().expect("stub should stop");
    let patch = request_json_body(&patch_request);

    assert_eq!(
        patch["app_ui"]["system_settings"]["quick_messages"],
        serde_json::json!([{
            "text": "创建新会话",
            "group": "git",
            "kind": "createNewSession"
        }])
    );
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn update_system_settings_config_result_async_rejects_gateway_connection_edits() {
    let (_lock, _home, _guard) = temp_home();
    let (port, requests, join) = serve_global_config(vec![(200, "{}".into())]);
    let bootstrap = gateway_config("127.0.0.1", port);
    super::save_gateway_client_bootstrap_config(&bootstrap).unwrap();

    let error = test_runtime()
        .block_on(super::update_system_settings_config_result_async(|config| {
            config.gateway_client.username = "pending-user".to_string();
        }))
        .expect_err("gateway connections use the dedicated local save path");
    join.join().expect("stub should stop");

    let requests = requests.into_iter().collect::<Vec<_>>();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].starts_with("GET /v1/global/config "));
    assert!(error.contains("gateway connections"));
    assert_eq!(super::load_gateway_client_bootstrap_config(), bootstrap);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn update_gateway_client_config_persists_without_a_gateway() {
    let (_lock, _home, _guard) = temp_home();
    let bootstrap = gateway_config("127.0.0.1", 1);
    super::save_gateway_client_bootstrap_config(&bootstrap).unwrap();

    let snapshot = super::update_gateway_client_config(|config| {
        config.username = "desktop-user".to_string();
    })
    .expect("local save should succeed with no Gateway");
    assert_eq!(snapshot.username, "desktop-user");
    assert_eq!(super::load_gateway_client_bootstrap_config(), snapshot);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn load_gateway_client_bootstrap_config_does_not_read_legacy_system_settings() {
    let lock = env_lock();
    let home = tempfile::tempdir().expect("temp home should be created");
    let config_file = home.path().join("legacy-vibewindow.json");
    let legacy = gateway_config("legacy.gateway", 7777);
    std::fs::write(
        &config_file,
        serde_json::json!({
            "app_ui": {
                "system_settings": {
                    "gateway_client": legacy
                }
            }
        })
        .to_string(),
    )
    .expect("legacy config should be written");
    let _home = EnvGuard::set_os("HOME", home.path().as_os_str());
    let _config = EnvGuard::set_os("VIBEWINDOW_CONFIG", config_file.as_os_str());
    let _config_dir = EnvGuard::remove("VIBEWINDOW_CONFIG_DIR");

    let loaded = super::load_gateway_client_bootstrap_config();

    assert_eq!(loaded, GatewayClientSystemSettingsConfig::default());
    drop(lock);
}

#[test]
fn agent_unified_selection_config_override_isolates_window_bootstrap() {
    let _lock = env_lock();
    let root = TempDir::new().unwrap();
    let _guard = EnvGuard::set_os("VIBEWINDOW_CONFIG_DIR", root.path().as_os_str());
    assert_eq!(
        super::gateway_client_bootstrap_cache_path().unwrap(),
        root.path().join("gateway-client-bootstrap.json")
    );
    assert_eq!(super::legacy_gateway_client_bootstrap_cache_path(), None);
    assert!(crate::app::project_dirs().unwrap().data_local_dir().starts_with(root.path()));
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn with_editor_settings_gateway(
    test: impl FnOnce(crate::app::App, Receiver<String>, mpsc::Sender<Option<String>>),
) {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    let (_lock, home, _home_guard) = temp_home();
    let _config_guard = EnvGuard::set_os("VIBEWINDOW_CONFIG_DIR", home.path().as_os_str());
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let bootstrap = gateway_config("127.0.0.1", address.port());
    super::save_gateway_client_bootstrap_config(&bootstrap).unwrap();
    let bootstrap_path = super::gateway_client_bootstrap_cache_path().unwrap();
    let original_bootstrap = std::fs::read(&bootstrap_path).unwrap();

    let observe = Arc::new(AtomicBool::new(false));
    let observe_requests = observe.clone();
    let (request_tx, requests) = mpsc::channel();
    let (responses, response_rx) = mpsc::channel::<Option<String>>();
    let server = std::thread::spawn(move || {
        while let Ok((mut stream, _)) = listener.accept() {
            stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let request = read_http_request(&mut stream);
            if request.is_empty() {
                break;
            }
            let body = if observe_requests.load(Ordering::Relaxed) {
                if request_tx.send(request).is_err() {
                    break;
                }
                match response_rx.recv_timeout(Duration::from_secs(5)) {
                    Ok(Some(body)) => body,
                    Ok(None) => continue, // Disconnect without a response.
                    Err(_) => break,
                }
            } else {
                "{}".to_string()
            };
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let (app, _) = crate::app::App::new();
        observe.store(true, Ordering::Relaxed);
        test(app, requests, responses);
    }));
    // Wake accept on success or panic; the response channel also closes if the test panics.
    drop(TcpStream::connect(address));
    server.join().unwrap();
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
    assert_eq!(std::fs::read(bootstrap_path).unwrap(), original_bootstrap);
}

/// Poll the Iced save task while holding its HTTP reply until the caller has exercised the UI.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn drive_system_settings_patch(
    task: iced::Task<crate::app::Message>,
    requests: &Receiver<String>,
    responses: &mpsc::Sender<Option<String>>,
    expected: serde_json::Value,
    reply: Option<&str>,
    while_waiting: impl FnOnce(),
) -> crate::app::Message {
    use iced_runtime::{Action, futures::futures::StreamExt, task::into_stream};
    use std::time::Duration;

    const TIMEOUT: Duration = Duration::from_secs(4);
    let runtime =
        tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
    let completion = runtime.spawn(async move {
        let mut stream = into_stream(task).expect("Iced save task");
        let Some(Action::Output(message)) = stream.next().await else {
            panic!("expected a save result");
        };
        message
    });
    let request = requests.recv_timeout(TIMEOUT).expect("system settings PATCH");
    assert!(request.starts_with("PATCH /v1/global/config "), "{request}");
    assert_eq!(
        request_json_body(&request),
        serde_json::json!({"app_ui": {"system_settings": expected}})
    );
    while_waiting();
    responses.send(reply.map(str::to_owned)).unwrap();
    runtime.block_on(async { tokio::time::timeout(TIMEOUT, completion).await.unwrap().unwrap() })
}
