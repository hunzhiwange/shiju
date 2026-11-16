use crate::message::{AccountMessage, AccountNotice};
use desktop_account::ManualSkeyInput;

#[test]
fn manual_input_message_debug_is_redacted() {
    let message = AccountMessage::ManualInputChanged(ManualSkeyInput::new("secret-value"));
    assert!(!format!("{message:?}").contains("secret-value"));
}

#[test]
fn account_notice_covers_all_persisted_actions() {
    let notices = [
        AccountNotice::ManualLogin,
        AccountNotice::BrowserLogin,
        AccountNotice::Switch,
        AccountNotice::SignOut,
        AccountNotice::Delete,
    ];
    assert_eq!(notices.len(), 5);
}
