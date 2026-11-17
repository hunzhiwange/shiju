use super::{Local, map};
use crate::app::{Message, message::{ChatMessage, ViewMessage}};

#[test]
fn settings_help_routes_links_and_text_to_host_handlers() {
    let url = "https://example.com/help".to_owned();
    assert!(matches!(map(Local::OpenUrlExternal(url.clone())),
        Message::View(ViewMessage::OpenUrlExternal(value)) if value == url));
    let text = "帮助内容".to_owned();
    assert!(matches!(map(Local::AppendText(text.clone())),
        Message::Chat(ChatMessage::AppendText(value)) if value == text));
    assert!(matches!(map(Local::CopyCode("code".into())),
        Message::CopyCode(value) if value == "code"));
}

#[test]
fn desktop_linking_preserves_panic_unwinding() {
    assert!(std::panic::catch_unwind(|| panic!("unwind probe")).is_err());
}
