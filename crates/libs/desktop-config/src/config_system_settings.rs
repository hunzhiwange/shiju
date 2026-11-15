use config_ui::AppSystemSettingsConfig;
use serde_json::Value;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemSettingsSection {
    Theme,
    Timezone,
    DialogueFlow,
    GatewayClient,
}

pub fn normalize_editor_metrics(
    mut font_size: f32,
    mut line_height: f32,
    auto_line_height: bool,
) -> (f32, f32) {
    if !font_size.is_finite() || font_size <= 0.0 {
        font_size = AppSystemSettingsConfig::default().editor_font_size;
    }
    if auto_line_height {
        line_height = font_size * 1.4;
    }
    if !line_height.is_finite() || line_height <= 0.0 {
        line_height = AppSystemSettingsConfig::default().editor_line_height;
    }
    (font_size, line_height)
}

pub fn normalize_system_settings_config(
    mut cfg: AppSystemSettingsConfig,
) -> AppSystemSettingsConfig {
    (cfg.editor_font_size, cfg.editor_line_height) = normalize_editor_metrics(
        cfg.editor_font_size,
        cfg.editor_line_height,
        cfg.editor_auto_line_height,
    );
    cfg
}

// The gateway merges objects recursively. Null deletes an object key, while arrays
// and scalars are replaced as a whole.
pub fn retain_changed_settings(before: &Value, after: &mut Value) -> bool {
    match (before, after) {
        (Value::Object(before), Value::Object(after)) => {
            for key in before.keys() {
                after.entry(key.clone()).or_insert(Value::Null);
            }
            after.retain(|key, value| {
                before.get(key).is_none_or(|previous| retain_changed_settings(previous, value))
            });
            !after.is_empty()
        }
        (before, after) => before != after,
    }
}

use config_ui::GatewayClientSystemSettingsConfig;
#[cfg(target_arch = "wasm32")]
const GATEWAY_CLIENT_BOOTSTRAP_STORAGE_KEY: &str = "vibe-window.gateway-client-bootstrap";

pub struct BootstrapStore { pub legacy_config_dir: Option<std::path::PathBuf> }
impl BootstrapStore {
#[cfg(not(target_arch = "wasm32"))]
pub fn gateway_client_bootstrap_cache_path(&self) -> Option<std::path::PathBuf> {
    if let Some(directory) =
        std::env::var_os("VIBEWINDOW_CONFIG_DIR").filter(|value| !value.is_empty())
    {
        return Some(std::path::PathBuf::from(directory).join("gateway-client-bootstrap.json"));
    }
    std::env::var_os("HOME").map(std::path::PathBuf::from).map(|home| {
        config_runtime_paths::home_config_dir(home).join("gateway-client-bootstrap.json")
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn legacy_gateway_client_bootstrap_cache_path(&self) -> Option<std::path::PathBuf> {
    if std::env::var_os("VIBEWINDOW_CONFIG_DIR").is_some_and(|value| !value.is_empty()) {
        return None;
    }
    self.legacy_config_dir.as_ref().map(|d| d.join("gateway-client-bootstrap.json"))
}

/// 模块内可见函数，执行 load_gateway_client_bootstrap_config 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn load_gateway_client_bootstrap_config(&self) -> GatewayClientSystemSettingsConfig {
    #[cfg(target_arch = "wasm32")]
    {
        return web_sys::window()
            .and_then(|window| window.local_storage().ok().flatten())
            .and_then(|storage| {
                storage.get_item(GATEWAY_CLIENT_BOOTSTRAP_STORAGE_KEY).ok().flatten()
            })
            .and_then(|content| {
                serde_json::from_str::<GatewayClientSystemSettingsConfig>(&content).ok()
            })
            .unwrap_or_default();
    }

    #[cfg(not(target_arch = "wasm32"))]
    self.gateway_client_bootstrap_cache_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .or_else(|| {
            self.legacy_gateway_client_bootstrap_cache_path()
                .and_then(|path| std::fs::read_to_string(path).ok())
        })
        .and_then(|content| {
            serde_json::from_str::<GatewayClientSystemSettingsConfig>(&content).ok()
        })
        .unwrap_or_default()
}

/// 保存供原生启动或浏览器重载使用的本机连接配置。
pub fn save_gateway_client_bootstrap_config(&self,
    config: &GatewayClientSystemSettingsConfig,
) -> Result<(), String> {

    #[cfg(target_arch = "wasm32")]
    {
        let storage = web_sys::window()
            .and_then(|window| window.local_storage().ok().flatten())
            .ok_or("gateway bootstrap storage is unavailable")?;
        let content = serde_json::to_string(config).map_err(|err| err.to_string())?;
        storage
            .set_item(GATEWAY_CLIENT_BOOTSTRAP_STORAGE_KEY, &content)
            .map_err(|_| "gateway bootstrap storage write failed".to_string())
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = self.gateway_client_bootstrap_cache_path().ok_or("missing gateway bootstrap path")?;
        let parent = path.parent().ok_or("missing gateway bootstrap directory")?;
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|err| err.to_string())?;
        serde_json::to_writer(file.as_file_mut(), config).map_err(|err| err.to_string())?;
        file.persist(path).map_err(|err| err.to_string())?;
        Ok(())
    }
}

}

use crate::access::run_gateway_call;
impl crate::ConfigAccess {
pub async fn fetch_desktop_system_settings_via_gateway(&self)
-> Result<Option<AppSystemSettingsConfig>, String> {
    let client = self.gateway_client()?;
    client.desktop_system_settings_get::<AppSystemSettingsConfig>().await
}
pub async fn patch_desktop_system_settings_via_gateway(&self,
    patch: &impl serde::Serialize,
) -> Result<(), String> {
    let client = self.gateway_client()?;
    client.desktop_system_settings_patch(patch).await
}
/// 公开函数，执行 load_system_settings_config_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn load_system_settings_config_async(&self) -> Result<AppSystemSettingsConfig, String> {
    match self.fetch_desktop_system_settings_via_gateway().await {
        Ok(Some(config)) => {
            let mut config = normalize_system_settings_config(config);
            config.gateway_client = self.bootstrap_config.clone();
            Ok(config)
        }
        Ok(None) => {
            let mut config = normalize_system_settings_config(AppSystemSettingsConfig::default());
            config.gateway_client = self.bootstrap_config.clone();
            Ok(config)
        }
        Err(err) => {
            if gateway_http::is_gateway_unavailable_error(&err) {
                tracing::debug!(target: "vw_desktop", error = %err, "failed to load desktop system settings via gateway");
            } else {
                tracing::warn!(target: "vw_desktop", error = %err, "failed to load desktop system settings via gateway");
            }
            let mut config = normalize_system_settings_config(AppSystemSettingsConfig::default());
            config.gateway_client = self.bootstrap_config.clone();
            Ok(config)
        }
    }
}
/// 公开函数，执行 update_system_settings_config_result_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn update_system_settings_config_result_async(&self,
    update: impl FnOnce(&mut AppSystemSettingsConfig) + Send,
) -> Result<(), String> {
    let mut cfg = self.load_system_settings_config_async().await?;
    let before = serde_json::to_value(&cfg).map_err(|err| err.to_string())?;
    update(&mut cfg);
    cfg = normalize_system_settings_config(cfg);
    let mut patch = serde_json::to_value(&cfg).map_err(|err| err.to_string())?;
    if !retain_changed_settings(&before, &mut patch) {
        return Ok(());
    }
    if patch.get("gateway_client").is_some() {
        return Err(
            "gateway connections must be saved through the desktop connection settings".into()
        );
    }
    self.patch_desktop_system_settings_via_gateway(&patch).await
}
/// 公开函数，执行 load_system_settings_config 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn load_system_settings_config(&self) -> AppSystemSettingsConfig {
    run_gateway_call(self.load_system_settings_config_async()).unwrap_or_else(|err| {
        if gateway_http::is_gateway_unavailable_error(&err) {
            tracing::debug!(target: "vw_desktop", error = %err, "failed to load desktop system settings via gateway");
        } else {
            tracing::warn!(target: "vw_desktop", error = %err, "failed to load desktop system settings via gateway");
        }
        let mut config = normalize_system_settings_config(AppSystemSettingsConfig::default());
        config.gateway_client = self.bootstrap_config.clone();
        config
    })
}
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "config_system_settings_tests.rs"]
mod tests;
