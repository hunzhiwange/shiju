use super::show_error_dialog;

#[test]
fn show_error_dialog_accepts_message() {
    show_error_dialog("VibeWindow WebView", "test error");
}
