use crate::config_gateway::{
    load_config_value_at_path, load_vibewindow_root_json, project_config_override_path,
};
use serde_json::json;
use std::path::Path;
use std::sync::Mutex;
fn env_lock() -> &'static Mutex<()> {
    &super::ENV_LOCK
}

struct EnvVarGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
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

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

fn write_json(path: &Path, value: serde_json::Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent dir should be created");
    }
    std::fs::write(path, serde_json::to_string(&value).expect("json should serialize"))
        .expect("json file should be written");
}

#[test]
fn project_config_override_path_uses_explicit_workspace() {
    let _env_guard = env_lock().lock().expect("env lock should be acquired");
    let workspace = tempfile::tempdir().expect("temp workspace should be created");
    let _workspace = EnvVarGuard::set("VIBEWINDOW_WORKSPACE", workspace.path());

    assert_eq!(project_config_override_path(), Some(workspace.path().join(".vibewindow.json")));
}

#[test]
fn project_config_override_path_ignores_legacy_user_config_env() {
    let _env_guard = env_lock().lock().expect("env lock should be acquired");
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let workspace = temp.path().join("project");
    let _workspace = EnvVarGuard::set("VIBEWINDOW_WORKSPACE", &workspace);
    let _config = EnvVarGuard::set("VIBEWINDOW_CONFIG", temp.path().join("vibewindow.json"));
    let _config_dir = EnvVarGuard::set("VIBEWINDOW_CONFIG_DIR", temp.path().join("config"));

    assert_eq!(project_config_override_path(), Some(workspace.join(".vibewindow.json")));
}

#[test]
fn project_config_override_path_falls_back_to_current_dir_for_blank_workspace() {
    let _env_guard = env_lock().lock().expect("env lock should be acquired");
    let _workspace = EnvVarGuard::set("VIBEWINDOW_WORKSPACE", " \t ");
    let current_dir = std::env::current_dir().expect("current dir should be available");

    assert_eq!(project_config_override_path(), Some(current_dir.join(".vibewindow.json")));
}

#[test]
fn load_vibewindow_root_json_reads_only_project_override() {
    let _env_guard = env_lock().lock().expect("env lock should be acquired");
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let workspace = temp.path().join("project");
    write_json(&workspace.join(".vibewindow.json"), json!({"from_project": true}));
    write_json(&temp.path().join("vibewindow.json"), json!({"from_legacy": true}));
    let _workspace = EnvVarGuard::set("VIBEWINDOW_WORKSPACE", &workspace);
    let _config = EnvVarGuard::set("VIBEWINDOW_CONFIG", temp.path().join("vibewindow.json"));

    assert_eq!(load_vibewindow_root_json(), json!({"from_project": true}));
}

#[test]
fn load_vibewindow_root_json_returns_empty_object_without_readable_candidates() {
    let _env_guard = env_lock().lock().expect("env lock should be acquired");
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let _workspace = EnvVarGuard::set("VIBEWINDOW_WORKSPACE", temp.path().join("missing"));

    assert_eq!(load_vibewindow_root_json(), json!({}));
}

#[test]
fn load_vibewindow_root_json_returns_empty_object_for_invalid_json() {
    let _env_guard = env_lock().lock().expect("env lock should be acquired");
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let workspace = temp.path().join("project");
    std::fs::create_dir_all(&workspace).expect("workspace should be created");
    std::fs::write(workspace.join(".vibewindow.json"), "{not-json")
        .expect("invalid json fixture should be written");
    let _workspace = EnvVarGuard::set("VIBEWINDOW_WORKSPACE", &workspace);

    assert_eq!(load_vibewindow_root_json(), json!({}));
}

#[test]
fn load_config_value_at_path_deserializes_existing_value() {
    let _env_guard = env_lock().lock().expect("env lock should be acquired");
    let temp = tempfile::tempdir().expect("temp dir should be created");
    let workspace = temp.path().join("project");
    let config = workspace.join(".vibewindow.json");
    write_json(
        &config,
        json!({"app_ui": {"system_settings": {"gateway_client": {"host": "gw", "port": 9000}}}}),
    );
    let _workspace = EnvVarGuard::set("VIBEWINDOW_WORKSPACE", &workspace);

    let value = load_config_value_at_path::<GatewayClientSystemSettingsConfig>(&[
        "app_ui",
        "system_settings",
        "gateway_client",
    ])
    .expect("gateway client config should deserialize");

    assert_eq!(value.host, "gw");
    assert_eq!(value.port, 9000);
    assert!(load_config_value_at_path::<String>(&["missing"]).is_none());
    assert!(
        load_config_value_at_path::<u16>(&["app_ui", "system_settings", "gateway_client", "host"])
            .is_none()
    );
}

use config_ui::GatewayClientSystemSettingsConfig;
