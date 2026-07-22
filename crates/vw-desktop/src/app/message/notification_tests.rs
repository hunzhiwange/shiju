//! 覆盖通知消息处理的本地列表、编辑器和反馈状态。

use super::{NotificationMessage, update};
use crate::app::App;
use iced::widget::text_editor;

fn test_app() -> App {
    App::new().0
}

#[test]
fn add_preserves_message_id_and_collapsed_state() {
    let mut app = test_app();
    app.next_notification_id = 41;

    let _ = update(&mut app, NotificationMessage::Add("notice text".to_string()));

    assert_eq!(app.next_notification_id, 42);
    assert_eq!(app.notifications.len(), 1);
    assert_eq!(app.notifications[0].id, 41);
    assert_eq!(app.notifications[0].message, "notice text");
    assert_eq!(app.notification_editors[&41].text(), "notice text");
    assert!(!app.notifications_expanded);
}

#[test]
fn add_remove_toggle_and_clear_keep_editors_in_sync() {
    let mut app = test_app();

    let _ = update(&mut app, NotificationMessage::Add("first".to_string()));
    let _ = update(&mut app, NotificationMessage::Add("second".to_string()));

    assert_eq!(app.notifications.len(), 2);
    assert_eq!(app.notifications[0].id + 1, app.notifications[1].id);
    assert_eq!(app.notification_editors[&app.notifications[0].id].text(), "first");

    let first_id = app.notifications[0].id;
    let _ = update(&mut app, NotificationMessage::ToggleExpanded);
    assert!(app.notifications_expanded);

    let _ = update(&mut app, NotificationMessage::Remove(first_id));
    assert_eq!(app.notifications.len(), 1);
    assert!(!app.notification_editors.contains_key(&first_id));
    assert!(app.notifications_expanded);

    let remaining_id = app.notifications[0].id;
    let _ = update(&mut app, NotificationMessage::Remove(remaining_id));
    assert!(app.notifications.is_empty());
    assert!(!app.notifications_expanded);

    let _ = update(&mut app, NotificationMessage::Add("again".to_string()));
    let _ = update(&mut app, NotificationMessage::ClearAll);
    assert!(app.notifications.is_empty());
    assert!(app.notification_editors.is_empty());
    assert_eq!(app.copied_notification_id, None);
    assert!(!app.notifications_expanded);
}

#[test]
fn remove_copied_notification_clears_matching_feedback_and_preserves_others() {
    let mut app = test_app();
    let _ = update(&mut app, NotificationMessage::Add("first".to_string()));
    let _ = update(&mut app, NotificationMessage::Add("second".to_string()));
    let first_id = app.notifications[0].id;
    let second_id = app.notifications[1].id;
    app.copied_notification_id = Some(first_id);
    app.notifications_expanded = true;

    let _ = update(&mut app, NotificationMessage::Remove(first_id));

    assert_eq!(app.copied_notification_id, None);
    assert!(!app.notification_editors.contains_key(&first_id));
    assert_eq!(app.notifications.len(), 1);
    assert_eq!(app.notifications[0].id, second_id);
    assert!(app.notification_editors.contains_key(&second_id));
    assert!(app.notifications_expanded);
}

#[test]
fn copy_reset_and_missing_copy_update_only_feedback_state() {
    let mut app = test_app();
    let _ = update(&mut app, NotificationMessage::Add("copy me".to_string()));
    let id = app.notifications[0].id;

    let _ = update(&mut app, NotificationMessage::Copy(usize::MAX));
    assert_eq!(app.copied_notification_id, None);

    let _ = update(&mut app, NotificationMessage::Copy(id));
    assert_eq!(app.copied_notification_id, Some(id));

    let _ = update(&mut app, NotificationMessage::ResetCopied(id + 1));
    assert_eq!(app.copied_notification_id, Some(id));

    let _ = update(&mut app, NotificationMessage::ResetCopied(id));
    assert_eq!(app.copied_notification_id, None);
}

#[test]
fn editor_action_allows_selection_but_ignores_text_edits() {
    let mut app = test_app();
    let _ = update(&mut app, NotificationMessage::Add("readonly".to_string()));
    let id = app.notifications[0].id;

    let _ = update(
        &mut app,
        NotificationMessage::EditorAction(
            id,
            text_editor::Action::Edit(text_editor::Edit::Paste(std::sync::Arc::new(
                " changed".to_string(),
            ))),
        ),
    );
    assert_eq!(app.notification_editors[&id].text(), "readonly");

    let _ = update(&mut app, NotificationMessage::EditorAction(id, text_editor::Action::SelectAll));
    assert_eq!(app.notification_editors[&id].selection().as_deref(), Some("readonly"));

    let _ = update(
        &mut app,
        NotificationMessage::EditorAction(usize::MAX, text_editor::Action::SelectAll),
    );
    assert_eq!(app.notification_editors[&id].selection().as_deref(), Some("readonly"));
}

#[test]
fn hide_toast_only_clears_matching_toast() {
    let mut app = test_app();
    app.active_toast = Some(crate::app::state::Toast {
        id: 7,
        message: "toast".to_string(),
        kind: crate::app::state::ToastKind::Info,
    });

    let _ = update(&mut app, NotificationMessage::HideToast(8));
    assert!(app.active_toast.is_some());

    let _ = update(&mut app, NotificationMessage::HideToast(7));
    assert!(app.active_toast.is_none());
}
