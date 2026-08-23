//! 应用中心弹窗测试，覆盖书签新增编辑、激活优先级和提交条件。
//! 选择索引失效时仍需安全构建界面，且 Cookie 示例必须展示必要字段契约。

use crate::app::message::ViewMessage;
use crate::app::{App, Message};

fn new_app() -> App {
    App::new().0
}

#[test]
fn active_modal_returns_none_without_modal_state() {
    let app = new_app();
    assert!(super::active_modal(&app).is_none());
}

#[test]
fn active_modal_prefers_edit_modal_over_add_modal() {
    let mut app = new_app();
    app.show_web_links_menu = true;
    app.editing_web_bookmark = Some(2);

    let Some((_modal, close)) = super::active_modal(&app) else {
        panic!("expected edit modal");
    };
    assert!(matches!(close, Message::View(ViewMessage::WebBookmarkEditCancel)));
}

#[test]
fn active_modal_returns_add_bookmark_close_message() {
    let mut app = new_app();
    app.show_web_links_menu = true;

    let Some((_modal, close)) = super::active_modal(&app) else {
        panic!("expected add modal");
    };
    assert!(matches!(close, Message::View(ViewMessage::ToggleWebLinksMenu)));
}

#[test]
fn can_add_web_bookmark_requires_non_blank_title() {
    assert!(!super::can_add_web_bookmark("", "https://example.com"));
    assert!(!super::can_add_web_bookmark(" \n\t", "https://example.com"));
}

#[test]
fn can_add_web_bookmark_requires_non_blank_url() {
    assert!(!super::can_add_web_bookmark("Example", ""));
    assert!(!super::can_add_web_bookmark("Example", " \n\t"));
}

#[test]
fn can_add_web_bookmark_accepts_trimmed_non_empty_fields() {
    assert!(super::can_add_web_bookmark("  Example  ", "  https://example.com  "));
}

#[test]
fn add_web_bookmark_modal_builds_for_incomplete_input() {
    let app = new_app();

    let Some((_modal, close)) = super::add_web_bookmark_modal(&app) else {
        panic!("expected add modal");
    };
    assert!(matches!(close, Message::View(ViewMessage::ToggleWebLinksMenu)));
}

#[test]
fn add_web_bookmark_modal_builds_for_valid_input() {
    let mut app = new_app();
    app.web_bookmark_title_input = "Example".into();
    app.web_bookmark_url_input = "https://example.com".into();

    let Some((_modal, close)) = super::add_web_bookmark_modal(&app) else {
        panic!("expected add modal");
    };
    assert!(matches!(close, Message::View(ViewMessage::ToggleWebLinksMenu)));
}

#[test]
fn edit_web_bookmark_modal_builds_for_selected_bookmark() {
    let mut app = new_app();
    app.editing_web_bookmark = Some(7);

    let Some((_modal, close)) = super::edit_web_bookmark_modal(&app) else {
        panic!("expected edit modal");
    };
    assert!(matches!(close, Message::View(ViewMessage::WebBookmarkEditCancel)));
}

#[test]
fn edit_web_bookmark_modal_uses_safe_index_fallback_without_selection() {
    let app = new_app();

    let Some((_modal, close)) = super::edit_web_bookmark_modal(&app) else {
        panic!("expected edit modal");
    };
    assert!(matches!(close, Message::View(ViewMessage::WebBookmarkEditCancel)));
}

#[test]
fn cookie_config_example_documents_required_cookie_fields() {
    let example = super::WEB_BOOKMARK_COOKIE_CONFIG_EXAMPLE;

    assert!(example.starts_with('['));
    assert!(example.ends_with(']'));
    for field in ["\"name\"", "\"domain\"", "\"days\"", "\"url_filter\""] {
        assert!(example.contains(field));
    }
}

#[test]
fn modal_shell_wraps_any_content() {
    let content = iced::widget::text("body").into();
    let _ = super::modal_shell(content);
}
