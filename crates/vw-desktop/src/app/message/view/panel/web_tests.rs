use crate::app::{App, CookieConfig, WebBookmark};
use iced::widget::text_editor;
use super::{ViewMessage, update};

fn app() -> App {
    let mut app = App::new().0;
    app.show_web_links_menu = false;
    app.web_bookmarks.clear();
    app.web_bookmark_title_input.clear();
    app.web_bookmark_url_input.clear();
    app.web_bookmark_width_input.clear();
    app.web_bookmark_height_input.clear();
    app.editing_web_bookmark = None;
    app.edit_web_bookmark_title_input.clear();
    app.edit_web_bookmark_url_input.clear();
    app.edit_web_bookmark_width_input.clear();
    app.edit_web_bookmark_height_input.clear();
    app.edit_web_bookmark_cookie_configs_editor = text_editor::Content::new();
    app
}

fn bookmark() -> WebBookmark {
    WebBookmark {
        title: "Console".to_string(),
        url: "https://example.test/app".to_string(),
        width: Some(1280),
        height: Some(720),
        cookie_configs: Some(vec![CookieConfig {
            name: "session".to_string(),
            domain: Some("example.test".to_string()),
            days: Some(30),
            url_filter: Some("https://example.test/app".to_string()),
        }]),
    }
}

#[test]
fn unrelated_view_message_preserves_web_state() {
    let mut app = app();
    app.show_web_links_menu = true;
    app.web_bookmarks.push(bookmark());
    app.web_bookmark_title_input = "draft".to_string();

    let _ = update(&mut app, ViewMessage::ActivityAnimationTick);

    assert!(app.show_web_links_menu);
    assert_eq!(app.web_bookmarks.len(), 1);
    assert_eq!(app.web_bookmarks[0].title, "Console");
    assert_eq!(app.web_bookmark_title_input, "draft");
}
