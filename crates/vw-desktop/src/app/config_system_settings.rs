//! 读写桌面系统设置并维护网关客户端启动缓存。
//! 本模块负责配置归一化；系统设置通过 Gateway 读写，启动阶段仅单独缓存 Gateway 连接信息。

use crate::app::Message;
use iced::Task;
use vw_config_types::ui::{AppSystemSettingsConfig, GatewayClientSystemSettingsConfig};

use super::gateway::{gateway_client, run_gateway_call, spawn_gateway_task};

async fn fetch_desktop_system_settings_via_gateway()
-> Result<Option<AppSystemSettingsConfig>, String> {
    let client = gateway_client()?;
    client.desktop_system_settings_get::<AppSystemSettingsConfig>().await
}

async fn patch_desktop_system_settings_via_gateway(
    config: &AppSystemSettingsConfig,
) -> Result<(), String> {
    let client = gateway_client()?;
    client.desktop_system_settings_patch(config).await
}

fn normalize_system_settings_config(mut cfg: AppSystemSettingsConfig) -> AppSystemSettingsConfig {
    if !cfg.editor_font_size.is_finite() || cfg.editor_font_size <= 0.0 {
        cfg.editor_font_size = 14.0;
    }
    if cfg.editor_auto_line_height {
        cfg.editor_line_height = cfg.editor_font_size * 1.4;
    }
    if !cfg.editor_line_height.is_finite() || cfg.editor_line_height <= 0.0 {
        cfg.editor_line_height = 20.0;
    }
    cfg
}

#[cfg(not(target_arch = "wasm32"))]
fn gateway_client_bootstrap_cache_path() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(std::path::PathBuf::from).map(|home| {
        vw_config_types::paths::home_config_dir(home).join("gateway-client-bootstrap.json")
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn legacy_gateway_client_bootstrap_cache_path() -> Option<std::path::PathBuf> {
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

/// 模块内可见函数，执行 save_gateway_client_bootstrap_config 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn save_gateway_client_bootstrap_config(config: &GatewayClientSystemSettingsConfig) {
    #[cfg(test)]
    assert!(WINDOW_TEST_BOOTSTRAP.get().is_none(), "window fixture must not write bootstrap files");
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(storage) =
            web_sys::window().and_then(|window| window.local_storage().ok().flatten())
            && let Ok(content) = serde_json::to_string(config)
        {
            let _ = storage.set_item(GATEWAY_CLIENT_BOOTSTRAP_STORAGE_KEY, &content);
        }
        return;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let Some(path) = gateway_client_bootstrap_cache_path() else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(content) = serde_json::to_string(config) {
            let _ = std::fs::write(path, content);
        }
    }
}

/// 公开函数，执行 load_gateway_client_config 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn load_gateway_client_config() -> GatewayClientSystemSettingsConfig {
    load_gateway_client_bootstrap_config()
}

/// 公开函数，执行 update_gateway_client_config 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn update_gateway_client_config(
    update: impl FnOnce(&mut GatewayClientSystemSettingsConfig) + Send,
) {
    let mut gateway_client = load_gateway_client_bootstrap_config();
    update(&mut gateway_client);
    save_gateway_client_bootstrap_config(&gateway_client);
    update_system_settings_config(move |system| {
        system.gateway_client = gateway_client;
    });
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
    update(&mut cfg);
    cfg = normalize_system_settings_config(cfg);
    patch_desktop_system_settings_via_gateway(&cfg).await?;
    save_gateway_client_bootstrap_config(&cfg.gateway_client);
    Ok(())
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

/// 公开函数，执行 update_system_settings_config 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn update_system_settings_config(update: impl FnOnce(&mut AppSystemSettingsConfig) + Send) {
    let outcome = run_gateway_call(update_system_settings_config_result_async(update));

    if let Err(err) = outcome {
        if vw_gateway_client::is_gateway_unavailable_error(&err) {
            tracing::debug!(target: "vw_desktop", error = %err, "desktop system settings patch skipped because gateway is unavailable");
        } else {
            tracing::warn!(target: "vw_desktop", error = %err, "failed to patch desktop system settings via gateway");
        }
    }
}

#[cfg(test)]
#[path = "config_system_settings_tests.rs"]
mod config_system_settings_tests;

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
