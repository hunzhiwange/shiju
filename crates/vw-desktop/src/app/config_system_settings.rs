//! 读写桌面系统设置并维护网关客户端启动缓存。
//! 本模块负责配置归一化；系统设置通过 Gateway 读写，启动阶段仅单独缓存 Gateway 连接信息。

use crate::app::i18n::settings_models::messages::Feedback;
use crate::app::message::SettingsMessage;
use crate::app::{App, Message};
use iced::Task;
use serde_json::{Map, Value};
use config_ui::{AppSystemSettingsConfig, GatewayClientSystemSettingsConfig};
use gateway_endpoint::GatewayEndpoint;

use super::desktop::{PreferencesScope, preferences_scope};
use super::gateway::{
    gateway_client_endpoint,
    spawn_gateway_task,
};

pub use desktop_config::config_system_settings::SystemSettingsSection;

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
        ).map(super::desktop::preferences_message)
    }

    pub(in crate::app) fn queue_gateway_client_snapshot(
        &mut self,
        config: GatewayClientSystemSettingsConfig,
    ) -> (GatewayEndpoint, Task<Message>) {
        let (endpoint, task) = self.desktop_preferences_persistence.enqueue_gateway_client_snapshot(config);
        #[cfg(not(target_arch = "wasm32"))]
        self.agent_apps_auth.select_endpoint(&endpoint);
        (endpoint, task.map(super::desktop::preferences_message))
    }

    pub(in crate::app) fn system_settings_edited(&self, section: SystemSettingsSection) -> bool {
        self.desktop_preferences_persistence.system_settings_edited(
            &preferences_scope(&gateway_client_endpoint()), section)
    }

    pub(in crate::app) fn system_settings_saved(
        &mut self,
        scope: PreferencesScope,
        section: SystemSettingsSection,
        result: Result<(), String>,
    ) -> Task<Message> {
        let next = self
            .desktop_preferences_persistence
            .finish_system_settings_save(scope.clone(), section).map(super::desktop::preferences_message);
        use desktop_config::config_desktop::SystemSettingsSaveFeedback;
        match self.desktop_preferences_persistence.system_settings_save_feedback(
            &scope, &preferences_scope(&gateway_client_endpoint()), section,
            result.is_err(), self.gateway_client_settings.save_error.is_some()) {
            SystemSettingsSaveFeedback::GatewaySyncFailed => {
                let error = Feedback::owned("网关连接已在本机保存，远端同步失败。", vec![]);
                let toast = self.show_error_toast(error.display(self.general_settings.desktop_locale));
                self.gateway_client_settings.save_error = Some(error);
                Task::batch([next, toast])
            }
            SystemSettingsSaveFeedback::DialogueFlow => Task::batch([
                next,
                crate::app::message::settings::update(self, SettingsMessage::DialogueFlowUiSettingsSaved(result)),
            ]),
            SystemSettingsSaveFeedback::Failed => {
                let error = result.expect_err("failed save feedback");
                Task::batch([next, self.show_error_toast(format!("系统设置未保存：{error}"))])
            }
            SystemSettingsSaveFeedback::None => next,
        }
    }
}


fn bootstrap_store() -> desktop_config::config_system_settings::BootstrapStore {
    desktop_config::config_system_settings::BootstrapStore {
        legacy_config_dir: crate::app::project_dirs().map(|d| d.config_dir().to_path_buf()),
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn gateway_client_bootstrap_cache_path() -> Option<std::path::PathBuf> {
    bootstrap_store().gateway_client_bootstrap_cache_path()
}
pub(super) fn load_gateway_client_bootstrap_config() -> GatewayClientSystemSettingsConfig {
    #[cfg(test)]
    if let Some(config) = WINDOW_TEST_BOOTSTRAP.get() { return config.clone(); }
    bootstrap_store().load_gateway_client_bootstrap_config()
}
#[cfg(test)]
pub(super) fn save_gateway_client_bootstrap_config(config: &GatewayClientSystemSettingsConfig) -> Result<(), String> {
    #[cfg(test)]
    assert!(WINDOW_TEST_BOOTSTRAP.get().is_none(), "window fixture must not write bootstrap files");
    bootstrap_store().save_gateway_client_bootstrap_config(config)
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
    #[cfg(test)]
    assert!(WINDOW_TEST_BOOTSTRAP.get().is_none(), "window fixture must not write bootstrap files");
    bootstrap_store().update_gateway_client_config(update)
}

/// 公开函数，执行 load_system_settings_config_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn load_system_settings_config_async() -> Result<AppSystemSettingsConfig, String> {
    config_access().load_system_settings_config_async().await
}

/// 公开函数，执行 update_system_settings_config_result_async 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub async fn update_system_settings_config_result_async(
    update: impl FnOnce(&mut AppSystemSettingsConfig) + Send,
) -> Result<(), String> {
    config_access().update_system_settings_config_result_async(update).await
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
    config_access().load_system_settings_config()
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
    let server = config_ui::GatewayClientServerConfig {
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

pub(in crate::app) fn config_access() -> desktop_config::ConfigAccess {
    desktop_config::ConfigAccess::new(gateway_client_endpoint(), None)
        .with_bootstrap_config(load_gateway_client_bootstrap_config())
}

#[cfg(test)]
pub(in crate::app) use desktop_config::config_system_settings::normalize_editor_metrics;
#[cfg(test)]
pub(in crate::app) use desktop_config::config_system_settings::normalize_system_settings_config;
