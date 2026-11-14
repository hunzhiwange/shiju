//! 注入系统结果验证消息处理，不请求真实系统权限。

use super::message::{GeneralSettingsMessage, SettingsMessage};
use super::state::ToastKind;
use super::{App, Message};

#[test]
fn desktop_integration_29_preserves_expected_behavior() {
    let mut app = App::new().0;
    let _ = app.show_success_toast("业务已完成");
    let original = app.active_toast.clone().unwrap();
    let next_id = app.next_toast_id;
    app.task_completion_notification = true;
    // 避免产生持久化任务；授权失败仍需关闭应用内通知开关。
    app.task_completion_notification_saving = true;

    for result in [
        GeneralSettingsMessage::TaskCompletionNotificationPermissionChecked(Err(
            "通知权限未获允许".into(),
        )),
        GeneralSettingsMessage::TaskCompletionNotificationSent(Err("通知权限未获允许".into())),
        GeneralSettingsMessage::TaskCompletionNotificationSent(Ok(())),
    ] {
        let _ = app.update(Message::Settings(SettingsMessage::General(result)));
        let toast = app.active_toast.as_ref().unwrap();
        assert_eq!(toast.id, original.id);
        assert_eq!(toast.kind, ToastKind::Success);
        assert_eq!(toast.message, original.message);
        assert_eq!(app.next_toast_id, next_id);
    }
    if super::desktop_notification::SUPPORTED {
        assert!(!app.task_completion_notification);
    }
}

#[test]
fn desktop_integration_29_reports_failure_when_no_toast_is_visible() {
    let mut app = App::new().0;
    app.active_toast = None;
    let _ = app.update(Message::Settings(SettingsMessage::General(
        GeneralSettingsMessage::TaskCompletionNotificationSent(Err("permission denied".into())),
    )));
    let toast = app.active_toast.as_ref().unwrap();
    assert_eq!(toast.kind, ToastKind::Error);
    assert!(toast.message.contains("permission denied"));
}
