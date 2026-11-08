//! 读写桌面系统设置并维护网关客户端启动缓存。
//! 本模块负责配置归一化；系统设置通过 Gateway 读写，启动阶段仅单独缓存 Gateway 连接信息。

use crate::app::i18n::settings_models::messages::Feedback;
use crate::app::message::SettingsMessage;
use crate::app::{App, Message};
use iced::Task;
use serde_json::{Map, Value};
use vw_config_types::ui::{AppSystemSettingsConfig, GatewayClientSystemSettingsConfig};
use vw_gateway_client::GatewayEndpoint;

use super::desktop::{PreferencesScope, preferences_scope};
use super::gateway::{
    gateway_client, gateway_client_endpoint, gateway_client_endpoint_from_config, run_gateway_call,
    spawn_gateway_task,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemSettingsSection {
    Theme,
    Timezone,
    DialogueFlow,
    GatewayClient,
}

impl App {
    pub(in crate::app) fn save_system_settings_fields(
        &mut self,
        section: SystemSettingsSection,
        fields: Map<String, Value>,
    ) -> Task<Message> {
        self.desktop_preferences_persistence.enqueue_system_settings(
            gateway_client_endpoint(),
            section,
            fields,
        )
    }

    pub(in crate::app) fn queue_gateway_client_snapshot(
        &mut self,
        config: GatewayClientSystemSettingsConfig,
    ) -> (GatewayEndpoint, Task<Message>) {
        let endpoint = gateway_client_endpoint_from_config(&config);
        let mut snapshot = serde_json::json!(config);
        // The Gateway merges objects. Explicitly remove cleared or migrated credentials.
        for key in ["skey", "bearer_token", "username", "password"] {
            if snapshot.get(key).is_none() {
                snapshot[key] = Value::Null;
            }
        }
        let task = self.desktop_preferences_persistence.enqueue_system_settings(
            endpoint.clone(),
            SystemSettingsSection::GatewayClient,
            Map::from_iter([("gateway_client".to_string(), snapshot)]),
        );
        (endpoint, task)
    }

    pub(in crate::app) fn system_settings_edited(&self, section: SystemSettingsSection) -> bool {
        self.desktop_preferences_persistence.edited_system_settings.contains(&section)
    }

    pub(in crate::app) fn system_settings_saved(
        &mut self,
        scope: PreferencesScope,
        section: SystemSettingsSection,
        result: Result<(), String>,
    ) -> Task<Message> {
        let next = self
            .desktop_preferences_persistence
            .finish_system_settings_save(scope.clone(), section);
        if section == SystemSettingsSection::GatewayClient {
            if result.is_err()
                && scope == preferences_scope(&gateway_client_endpoint())
                && !self.desktop_preferences_persistence.has_system_settings_write(&scope, section)
                && self.gateway_client_settings.save_error.is_none()
            {
                let error = Feedback::owned("网关连接已在本机保存，远端同步失败。", vec![]);
                let toast = self.show_error_toast(error.display(self.desktop_locale));
                self.gateway_client_settings.save_error = Some(error);
                return Task::batch([next, toast]);
            }
            return next;
        }
        if section == SystemSettingsSection::DialogueFlow
            && scope == preferences_scope(&gateway_client_endpoint())
        {
            return Task::batch([
                next,
                crate::app::message::settings::update(
                    self,
                    SettingsMessage::DialogueFlowUiSettingsSaved(result),
                ),
            ]);
        }
        match result {
            Ok(()) => next,
            Err(error) => {
                Task::batch([next, self.show_error_toast(format!("系统设置未保存：{error}"))])
            }
        }
    }
}

async fn fetch_desktop_system_settings_via_gateway()
-> Result<Option<AppSystemSettingsConfig>, String> {
    let client = gateway_client()?;
    client.desktop_system_settings_get::<AppSystemSettingsConfig>().await
}

async fn patch_desktop_system_settings_via_gateway(
    patch: &impl serde::Serialize,
) -> Result<(), String> {
    let client = gateway_client()?;
    client.desktop_system_settings_patch(patch).await
}

pub(in crate::app) fn normalize_editor_metrics(
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

fn normalize_system_settings_config(mut cfg: AppSystemSettingsConfig) -> AppSystemSettingsConfig {
    (cfg.editor_font_size, cfg.editor_line_height) = normalize_editor_metrics(
        cfg.editor_font_size,
        cfg.editor_line_height,
        cfg.editor_auto_line_height,
    );
    cfg
}

// The gateway merges objects recursively. Null deletes an object key, while arrays
// and scalars are replaced as a whole.
fn retain_changed_settings(before: &Value, after: &mut Value) -> bool {
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

#[cfg(not(target_arch = "wasm32"))]
fn gateway_client_bootstrap_cache_path() -> Option<std::path::PathBuf> {
    if let Some(directory) =
        std::env::var_os("VIBEWINDOW_CONFIG_DIR").filter(|value| !value.is_empty())
    {
        return Some(std::path::PathBuf::from(directory).join("gateway-client-bootstrap.json"));
    }
    std::env::var_os("HOME").map(std::path::PathBuf::from).map(|home| {
        vw_config_types::paths::home_config_dir(home).join("gateway-client-bootstrap.json")
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn legacy_gateway_client_bootstrap_cache_path() -> Option<std::path::PathBuf> {
    if std::env::var_os("VIBEWINDOW_CONFIG_DIR").is_some_and(|value| !value.is_empty()) {
        return None;
    }
    crate::app::project_dirs().map(|d| d.config_dir().join("gateway-client-bootstrap.json"))
}

#[cfg(target_arch = "wasm32")]
const GATEWAY_CLIENT_BOOTSTRAP_STORAGE_KEY: &str = "vibe-window.gateway-client-bootstrap";

/// 模块内可见函数，执行 load_gateway_client_bootstrap_config 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn load_gateway_client_bootstrap_config() -> GatewayClientSystemSettingsConfig {
    #[cfg(test)]
    if let Some(config) = WINDOW_TEST_BOOTSTRAP.get() {
        return config.clone();
    }
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
    gateway_client_bootstrap_cache_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .or_else(|| {
            legacy_gateway_client_bootstrap_cache_path()
                .and_then(|path| std::fs::read_to_string(path).ok())
        })
        .and_then(|content| {
            serde_json::from_str::<GatewayClientSystemSettingsConfig>(&content).ok()
        })
        .unwrap_or_default()
}

/// 保存供原生启动或浏览器重载使用的本机连接配置。
pub(super) fn save_gateway_client_bootstrap_config(
    config: &GatewayClientSystemSettingsConfig,
) -> Result<(), String> {
    #[cfg(test)]
    assert!(WINDOW_TEST_BOOTSTRAP.get().is_none(), "window fixture must not write bootstrap files");

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
        let path = gateway_client_bootstrap_cache_path().ok_or("missing gateway bootstrap path")?;
        let parent = path.parent().ok_or("missing gateway bootstrap directory")?;
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|err| err.to_string())?;
        serde_json::to_writer(file.as_file_mut(), config).map_err(|err| err.to_string())?;
        file.persist(path).map_err(|err| err.to_string())?;
        Ok(())
    }
}

/// 公开函数，执行 load_gateway_client_config 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn load_gateway_client_config() -> GatewayClientSystemSettingsConfig {
    load_gateway_client_bootstrap_config()
}

/// 先持久化本机连接，返回该次保存的快照供远端任务使用。
pub fn update_gateway_client_config(
    update: impl FnOnce(&mut GatewayClientSystemSettingsConfig),
) -> Result<GatewayClientSystemSettingsConfig, String> {
    let mut config = load_gateway_client_bootstrap_config();
    update(&mut config);
    save_gateway_client_bootstrap_config(&config)?;
    Ok(config)
}

/// 公开函数，执行 load_system_settings_config_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn load_system_settings_config_async() -> Result<AppSystemSettingsConfig, String> {
    match fetch_desktop_system_settings_via_gateway().await {
        Ok(Some(config)) => {
            let mut config = normalize_system_settings_config(config);
            config.gateway_client = load_gateway_client_bootstrap_config();
            Ok(config)
        }
        Ok(None) => {
            let mut config = normalize_system_settings_config(AppSystemSettingsConfig::default());
            config.gateway_client = load_gateway_client_bootstrap_config();
            Ok(config)
        }
        Err(err) => {
            if vw_gateway_client::is_gateway_unavailable_error(&err) {
                tracing::debug!(target: "vw_desktop", error = %err, "failed to load desktop system settings via gateway");
            } else {
                tracing::warn!(target: "vw_desktop", error = %err, "failed to load desktop system settings via gateway");
            }
            let mut config = normalize_system_settings_config(AppSystemSettingsConfig::default());
            config.gateway_client = load_gateway_client_bootstrap_config();
            Ok(config)
        }
    }
}

/// 公开函数，执行 update_system_settings_config_result_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn update_system_settings_config_result_async(
    update: impl FnOnce(&mut AppSystemSettingsConfig) + Send,
) -> Result<(), String> {
    let mut cfg = load_system_settings_config_async().await?;
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
    patch_desktop_system_settings_via_gateway(&patch).await
}

/// 公开函数，执行 update_system_settings_config_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(not(target_arch = "wasm32"))]
pub fn update_system_settings_config_async(
    update: impl FnOnce(&mut AppSystemSettingsConfig) + Send + 'static,
) -> Task<Message> {
    spawn_gateway_task("system_settings", async move {
        update_system_settings_config_result_async(update).await
    })
}

/// 公开函数，执行 update_system_settings_config_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
#[cfg(target_arch = "wasm32")]
pub fn update_system_settings_config_async(
    update: impl FnOnce(&mut AppSystemSettingsConfig) + Send + 'static,
) -> Task<Message> {
    spawn_gateway_task("system_settings", async move {
        update_system_settings_config_result_async(update).await
    })
}

/// 公开函数，执行 load_system_settings_config 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn load_system_settings_config() -> AppSystemSettingsConfig {
    run_gateway_call(load_system_settings_config_async()).unwrap_or_else(|err| {
        if vw_gateway_client::is_gateway_unavailable_error(&err) {
            tracing::debug!(target: "vw_desktop", error = %err, "failed to load desktop system settings via gateway");
        } else {
            tracing::warn!(target: "vw_desktop", error = %err, "failed to load desktop system settings via gateway");
        }
        let mut config = normalize_system_settings_config(AppSystemSettingsConfig::default());
        config.gateway_client = load_gateway_client_bootstrap_config();
        config
    })
}

#[cfg(test)]
#[path = "config_system_settings_tests.rs"]
pub(in crate::app) mod config_system_settings_tests;

/// 仅自定义窗口测试目标使用；端点必须是无生产凭据的本机临时服务。
#[cfg(test)]
static WINDOW_TEST_BOOTSTRAP: std::sync::OnceLock<GatewayClientSystemSettingsConfig> =
    std::sync::OnceLock::new();

#[cfg(test)]
#[allow(dead_code)] // 由独立的 recovery-window 测试目标调用。
pub(crate) fn install_window_test_bootstrap(port: u16) {
    assert_ne!(port, 0);
    let mut config = GatewayClientSystemSettingsConfig::default();
    let server = vw_config_types::ui::GatewayClientServerConfig {
        host: "127.0.0.1".into(),
        skey: "window-fixture-token".into(),
        port,
        platform_id: "recovery-window".into(),
        company_id: "validation".into(),
        ..Default::default()
    };
    config.set_servers(vec![server], "local".into());
    WINDOW_TEST_BOOTSTRAP.set(config).expect("install window test endpoint once");
}
