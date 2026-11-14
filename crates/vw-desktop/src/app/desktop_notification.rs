//! 原生桌面完成提醒；系统调用在阻塞线程执行，不携带对话内容。

use crate::app::message::{GeneralSettingsMessage, SettingsMessage};
use crate::app::{App, Message, i18n::I18n};
use iced::Task;
use vw_config_types::ui::DesktopLocale;

pub(crate) const SUPPORTED: bool = cfg!(any(target_os = "macos", target_os = "windows"));

/// 通知只是附加提醒；权限或发送失败不能覆盖当前业务反馈。
pub(crate) fn failure_feedback(app: &mut App, error: String) -> Task<Message> {
    if app.active_toast.is_some() {
        return Task::none();
    }
    app.show_error_toast(error)
}

pub(crate) fn completion_task(locale: DesktopLocale) -> Task<Message> {
    let i18n = I18n::new(locale);
    let title = i18n.__("任务完成").to_owned();
    let body = i18n.__("本轮对话已完成，请返回识矩查看。").to_owned();
    Task::perform(
        async move {
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            {
                tokio::task::spawn_blocking(move || show(&title, &body))
                    .await
                    .map_err(|_| "通知线程异常退出".to_owned())?
            }
            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
            {
                let _ = (title, body);
                Err("当前平台暂不支持原生桌面通知".to_owned())
            }
        },
        |result| {
            Message::Settings(SettingsMessage::General(
                GeneralSettingsMessage::TaskCompletionNotificationSent(result),
            ))
        },
    )
}

/// 开启时立即检查系统权限，避免把应用内开关误当成系统授权。
pub(crate) fn permission_task(locale: DesktopLocale) -> Task<Message> {
    Task::perform(
        async move {
            #[cfg(target_os = "macos")]
            {
                let result = tokio::task::spawn_blocking(ensure_macos_permission)
                    .await
                    .map_err(|_| "通知线程异常退出".to_owned())?;
                result.map_err(|error| I18n::new(locale).__(error).to_owned())
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = locale;
                Ok(())
            }
        },
        |result| {
            Message::Settings(SettingsMessage::General(
                GeneralSettingsMessage::TaskCompletionNotificationPermissionChecked(result),
            ))
        },
    )
}

#[cfg(target_os = "macos")]
fn ensure_macos_permission() -> Result<(), &'static str> {
    use mac_usernotifications::{AuthorizationStatus, blocking};
    mac_usernotifications::check_bundle()
        .map_err(|_| "macOS 通知需要从已签名的 ShiJu.app 启动，开发命令直接运行不支持。")?;
    let mut settings = blocking::get_notification_settings()
        .map_err(|_| "无法读取 macOS 通知权限，请检查系统设置。")?;
    if settings.authorization_status == AuthorizationStatus::NotDetermined {
        if !blocking::request_auth().map_err(|_| "无法请求 macOS 通知权限，请检查应用签名。")?
        {
            return Err("通知权限未获允许，请在系统设置 → 通知中允许识矩通知。");
        }
        settings = blocking::get_notification_settings()
            .map_err(|_| "无法读取 macOS 通知权限，请检查系统设置。")?;
    }
    validate_macos_settings(settings)
}

#[cfg(target_os = "macos")]
fn validate_macos_settings(
    settings: mac_usernotifications::NotificationSettings,
) -> Result<(), &'static str> {
    use mac_usernotifications::{AuthorizationStatus, NotificationSettingStatus};
    if settings.authorization_status != AuthorizationStatus::Authorized {
        return Err("通知权限未获允许，请在系统设置 → 通知中允许识矩通知。");
    }
    if settings.alert_enabled != NotificationSettingStatus::Enabled {
        return Err("通知横幅未开启，请在系统设置 → 通知中为识矩选择横幅或提醒。");
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn show(title: &str, body: &str) -> Result<(), String> {
    ensure_macos_permission().map_err(str::to_owned)?;
    mac_usernotifications::Notification::new()
        .title(title)
        .message(body)
        .default_sound()
        .send_blocking()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[cfg(target_os = "windows")]
fn show(title: &str, body: &str) -> Result<(), String> {
    const APP_ID: &str = "com.rsung.shiju";
    register_windows_application(APP_ID).map_err(|error| error.to_string())?;
    notify_rust::Notification::new()
        .summary(title)
        .body(body)
        .app_id(APP_ID)
        .show()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// 为免安装版本注册当前用户的通知身份，无需管理员权限。
#[cfg(target_os = "windows")]
fn register_windows_application(app_id: &str) -> std::io::Result<()> {
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};
    let root = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = root.create_subkey(format!("Software\\Classes\\AppUserModelId\\{app_id}"))?;
    key.set_value("DisplayName", &"识矩")?;
    Ok(())
}

#[cfg(all(test, target_os = "macos"))]
#[path = "desktop_notification_tests.rs"]
mod tests;
