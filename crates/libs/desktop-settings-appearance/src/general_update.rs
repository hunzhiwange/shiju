//! General appearance persistence, with gateway access supplied by the shell.
use config_ui::{DesktopLocale, QuickMessageConfig};
use desktop_config::ConfigAccess;

pub async fn save_locale(access: ConfigAccess, locale: DesktopLocale) -> Result<(), String> {
    access.update_system_settings_config_result_async(move |system| system.desktop_locale = locale).await
}
pub async fn save_task_completion_notification(access: ConfigAccess, enabled: bool) -> Result<(), String> {
    access.update_system_settings_config_result_async(move |system| system.task_completion_notification = enabled).await
}
pub async fn save_quick_messages(access: ConfigAccess, items: Vec<QuickMessageConfig>) -> Result<(), String> {
    access.update_system_settings_config_result_async(move |system| system.quick_messages = items).await
}
