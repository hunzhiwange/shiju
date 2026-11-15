//! 原生桌面完成提醒；系统调用在阻塞线程执行，不携带对话内容。

use crate::app::message::{GeneralSettingsMessage, SettingsMessage};
use crate::app::{App, Message, i18n::I18n};
use iced::Task;
use config_ui::DesktopLocale;

pub(crate) use desktop_platform::desktop_notification::SUPPORTED;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use desktop_platform::desktop_notification::show;
#[cfg(target_os = "macos")]
use desktop_platform::desktop_notification::ensure_macos_permission;

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
