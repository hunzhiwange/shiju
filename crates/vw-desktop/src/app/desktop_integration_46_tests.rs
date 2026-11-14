//! Inject clipboard completion results without accessing the system clipboard.
use super::{App, Message};
use super::state::ToastKind;

#[test]
fn desktop_integration_46_preserves_expected_behavior() {
    let mut app = App::new().0;
    app.last_copied_code_hash = Some(46);
    app.last_copy_time = Some(web_time::SystemTime::now());
    let _ = app.update(Message::ClipboardCopyFinished {
        hash: 46,
        result: Err("denied".into()),
    });
    assert_eq!(app.last_copied_code_hash, None);
    assert!(app.last_copy_time.is_none());
    assert_eq!(app.active_toast.as_ref().unwrap().kind, ToastKind::Error);
}

#[test]
fn desktop_integration_46_late_failure_preserves_newer_copy_feedback() {
    let mut app = App::new().0;
    app.last_copied_code_hash = Some(47);
    let _ = app.update(Message::ClipboardCopyFinished {
        hash: 46,
        result: Err("unavailable".into()),
    });
    assert_eq!(app.last_copied_code_hash, Some(47));
}
