//! Inject clipboard completion results without accessing the system clipboard.
use super::state::ToastKind;
use super::{App, Message};

#[test]
fn desktop_integration_46_preserves_expected_behavior() {
    let mut app = App::new().0;
    app.clipboard_state.last_copied_code_hash = Some(46);
    app.clipboard_state.last_copy_time = Some(web_time::SystemTime::now());
    let _ = app.update(Message::ClipboardCopyFinished { hash: 46, result: Err("denied".into()) });
    assert_eq!(app.clipboard_state.last_copied_code_hash, None);
    assert!(app.clipboard_state.last_copy_time.is_none());
    assert_eq!(app.toast_state.active_toast.as_ref().unwrap().kind, ToastKind::Error);
}
