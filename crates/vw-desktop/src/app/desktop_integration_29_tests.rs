//! Host contract: permission failure disables the setting and preserves existing feedback.
use super::message::{GeneralSettingsMessage, SettingsMessage};
use super::{App, Message};
#[test]
fn notification_permission_failure_disables_setting_without_replacing_feedback() {
    let mut app = App::new().0;
    let _ = app.show_success_toast("业务已完成");
    let id = app.toast_state.active_toast.as_ref().unwrap().id;
    app.general_settings.task_completion_notification = true;
    app.general_settings.task_completion_notification_saving = true;
    let _ = app.update(Message::Settings(SettingsMessage::General(
        GeneralSettingsMessage::TaskCompletionNotificationPermissionChecked(Err("denied".into())),
    )));
    assert_eq!(app.toast_state.active_toast.as_ref().unwrap().id, id);
    if super::desktop_notification::SUPPORTED {
        assert!(!app.general_settings.task_completion_notification);
    }
}
