//! 配置读取测试覆盖项目覆盖路径、缺失或损坏文件的默认回退，以及同步读取入口。
//!
//! 用例通过临时工作区构造真实 `.vibewindow.json`，并验证 provider 覆盖项只接收对象值；
//! 所有环境变量修改均串行执行且在守卫析构时恢复，避免污染同进程中的其他测试。

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

static ENV_LOCK: Mutex<()> = Mutex::new(());
static TEST_ROOT: OnceLock<PathBuf> = OnceLock::new();

struct EnvGuard {
    _lock: MutexGuard<'static, ()>,
    saved: Vec<(&'static str, Option<OsString>)>,
}

impl EnvGuard {
    fn new(keys: &[&'static str]) -> Self {
        let lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let saved = keys.iter().map(|key| (*key, std::env::var_os(key))).collect();
        Self { _lock: lock, saved }
    }

    fn set_path(&self, key: &str, value: &Path) {
        unsafe { std::env::set_var(key, value) };
    }

    fn set(&self, key: &str, value: &str) {
        unsafe { std::env::set_var(key, value) };
    }

    fn remove(&self, key: &str) {
        unsafe { std::env::remove_var(key) };
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.saved {
            match value {
                Some(value) => unsafe { std::env::set_var(key, value) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
    }
}

fn unique_dir(prefix: &str) -> PathBuf {
    let root = TEST_ROOT
        .get_or_init(|| {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be valid")
                .as_nanos();
            std::env::temp_dir()
                .join(format!("vw-provider-config-tests-{}-{nanos}", std::process::id()))
        })
        .clone();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();
    root.join(format!("{prefix}-{nanos}"))
}

fn write_config(dir: &Path, value: serde_json::Value) {
    std::fs::create_dir_all(dir).expect("config dir should be created");
    std::fs::write(
        dir.join(".vibewindow.json"),
        serde_json::to_string_pretty(&value).expect("config should serialize"),
    )
    .expect("config should be written");
}

fn valid_config(
    default_model: serde_json::Value,
    providers: serde_json::Value,
) -> serde_json::Value {
    json!({
        "default_temperature": 0.7,
        "default_model": default_model,
        "providers": providers
    })
}

#[test]
fn project_config_override_path_uses_explicit_workspace() {
    let env = EnvGuard::new(&["VIBEWINDOW_WORKSPACE"]);
    let dir = unique_dir("path");
    env.set_path("VIBEWINDOW_WORKSPACE", &dir);

    assert_eq!(super::project_config_override_path(), Some(dir.join(".vibewindow.json")));
}

#[test]
fn config_path_matches_project_override_compatibility_alias() {
    let _env = EnvGuard::new(&["VIBEWINDOW_WORKSPACE"]);
    let workspace = unique_dir("config-path-alias");
    _env.set_path("VIBEWINDOW_WORKSPACE", &workspace);

    assert_eq!(super::config_path(), super::project_config_override_path());
    assert_eq!(super::config_path(), Some(workspace.join(".vibewindow.json")));
}

#[test]
fn project_config_override_path_ignores_blank_workspace() {
    let env = EnvGuard::new(&["VIBEWINDOW_WORKSPACE"]);
    env.set("VIBEWINDOW_WORKSPACE", "  ");

    assert_eq!(
        super::project_config_override_path(),
        std::env::current_dir().ok().map(|dir| dir.join(".vibewindow.json"))
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn get_returns_default_when_config_file_is_missing_empty_or_invalid() {
    let env = EnvGuard::new(&["VIBEWINDOW_WORKSPACE"]);
    let missing = unique_dir("missing");
    env.set_path("VIBEWINDOW_WORKSPACE", &missing);
    assert_eq!(
        super::get().await.default_model,
        vw_config_types::config::Config::default().default_model
    );

    let empty = unique_dir("empty");
    std::fs::create_dir_all(&empty).expect("empty dir should be created");
    std::fs::write(empty.join(".vibewindow.json"), "   ").expect("empty config should be written");
    env.set_path("VIBEWINDOW_WORKSPACE", &empty);
    assert_eq!(
        super::get().await.default_model,
        vw_config_types::config::Config::default().default_model
    );

    let invalid = unique_dir("invalid");
    std::fs::create_dir_all(&invalid).expect("invalid dir should be created");
    std::fs::write(invalid.join(".vibewindow.json"), "{")
        .expect("invalid config should be written");
    env.set_path("VIBEWINDOW_WORKSPACE", &invalid);
    assert_eq!(
        super::get().await.default_model,
        vw_config_types::config::Config::default().default_model
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn get_reads_valid_config_file() {
    let env = EnvGuard::new(&["VIBEWINDOW_WORKSPACE"]);
    let dir = unique_dir("valid");
    env.set_path("VIBEWINDOW_WORKSPACE", &dir);
    write_config(
        &dir,
        valid_config(
            json!("provider/model"),
            json!({
                "openai": {"api": "https://example.com", "models": {}}
            }),
        ),
    );

    let config = super::get().await;

    assert_eq!(config.default_model.as_deref(), Some("provider/model"));
    assert!(config.providers.contains_key("openai"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn get_blocking_reads_config_without_existing_runtime() {
    let env = EnvGuard::new(&["VIBEWINDOW_WORKSPACE"]);
    let dir = unique_dir("blocking");
    env.set_path("VIBEWINDOW_WORKSPACE", &dir);
    write_config(&dir, valid_config(json!("blocking/model"), json!({})));

    let config = super::get_blocking();

    assert_eq!(config.default_model.as_deref(), Some("blocking/model"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn get_blocking_reads_config_from_existing_runtime() {
    let env = EnvGuard::new(&["VIBEWINDOW_WORKSPACE"]);
    let dir = unique_dir("blocking-runtime");
    env.set_path("VIBEWINDOW_WORKSPACE", &dir);
    write_config(&dir, valid_config(json!("runtime/model"), json!({})));

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime should build");
    let config = runtime.block_on(async { super::get_blocking() });

    assert_eq!(config.default_model.as_deref(), Some("runtime/model"));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn load_provider_overrides_keeps_only_object_provider_values() {
    let env = EnvGuard::new(&["VIBEWINDOW_WORKSPACE"]);
    let dir = unique_dir("providers");
    env.set_path("VIBEWINDOW_WORKSPACE", &dir);
    write_config(
        &dir,
        valid_config(
            json!("provider/model"),
            json!({
                "object": {"api": "https://example.com"},
                "nullish": null,
                "array": []
            }),
        ),
    );

    let overrides = super::load_provider_overrides().await;

    assert_eq!(overrides.len(), 1);
    assert_eq!(overrides[0].0, "object");
    assert_eq!(overrides[0].1["api"], "https://example.com");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn read_default_model_returns_configured_value() {
    let env = EnvGuard::new(&["VIBEWINDOW_WORKSPACE"]);
    let dir = unique_dir("default-model");
    env.set_path("VIBEWINDOW_WORKSPACE", &dir);
    write_config(&dir, valid_config(json!("configured/model"), json!({})));

    assert_eq!(super::read_default_model().await.as_deref(), Some("configured/model"));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn read_default_model_can_return_none_when_config_sets_null() {
    let env = EnvGuard::new(&["VIBEWINDOW_WORKSPACE"]);
    let dir = unique_dir("default-model-null");
    env.set_path("VIBEWINDOW_WORKSPACE", &dir);
    write_config(&dir, valid_config(serde_json::Value::Null, json!({})));

    assert_eq!(super::read_default_model().await, None);
}

#[test]
fn project_config_override_path_falls_back_to_current_directory() {
    let env = EnvGuard::new(&["VIBEWINDOW_WORKSPACE"]);
    env.remove("VIBEWINDOW_WORKSPACE");

    let path = super::project_config_override_path();

    assert!(path.as_ref().is_some_and(|value| value.ends_with(".vibewindow.json")));
}
