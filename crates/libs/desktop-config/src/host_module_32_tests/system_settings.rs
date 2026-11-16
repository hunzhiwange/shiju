use config_ui::{GatewayClientServerConfig, GatewayClientSystemSettingsConfig};
use std::sync::{LazyLock, Mutex};
static LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
fn temp_home() -> (std::sync::MutexGuard<'static, ()>, tempfile::TempDir, EnvGuard) {
    let lock = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let home = tempfile::tempdir().unwrap();
    let previous = std::env::var_os("VIBEWINDOW_CONFIG_DIR");
    unsafe {
        std::env::set_var("VIBEWINDOW_CONFIG_DIR", home.path());
    }
    (lock, home, EnvGuard(previous))
}
struct EnvGuard(Option<std::ffi::OsString>);
impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            match &self.0 {
                Some(value) => std::env::set_var("VIBEWINDOW_CONFIG_DIR", value),
                None => std::env::remove_var("VIBEWINDOW_CONFIG_DIR"),
            }
        }
    }
}
fn gateway_config(host: &str, port: u16) -> GatewayClientSystemSettingsConfig {
    let mut config = GatewayClientSystemSettingsConfig::default();
    let server = GatewayClientServerConfig { host: host.into(), port, ..Default::default() };
    config.set_servers(vec![server], "local".into());
    config
}
const SOURCE: &str = include_str!("../config_system_settings.rs");
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
#[cfg(not(target_arch = "wasm32"))]
fn update_gateway_client_config_persists_without_a_gateway() {
    let (_lock, _home, _guard) = temp_home();
    let store = crate::config_system_settings::BootstrapStore { legacy_config_dir: None };
    let bootstrap = gateway_config("127.0.0.1", 1);
    store.save_gateway_client_bootstrap_config(&bootstrap).unwrap();

    let snapshot = store
        .update_gateway_client_config(|config| {
            config.username = "desktop-user".to_string();
        })
        .expect("local save should succeed with no Gateway");
    assert_eq!(snapshot.username, "desktop-user");
    assert_eq!(store.load_gateway_client_bootstrap_config(), snapshot);
}
