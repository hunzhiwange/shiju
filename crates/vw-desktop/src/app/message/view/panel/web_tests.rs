//! Web 书签面板交互测试，覆盖新增、编辑、取消和输入解析流程。
//! 同时验证无效索引及缺失可选字段不会破坏用户尚未提交的草稿。

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
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("web_tests"));
}

#[test]
fn toggle_web_links_menu_flips_current_visibility() {
    let mut app = app();

    let _ = update(&mut app, ViewMessage::ToggleWebLinksMenu);
    assert!(app.show_web_links_menu);

    let _ = update(&mut app, ViewMessage::ToggleWebLinksMenu);
    assert!(!app.show_web_links_menu);
}

#[test]
fn bookmark_add_inputs_preserve_user_text_verbatim() {
    let mut app = app();

    let _ = update(&mut app, ViewMessage::WebBookmarkTitleChanged("  Example 标题  ".to_string()));
    let _ = update(
        &mut app,
        ViewMessage::WebBookmarkUrlChanged(" `https://example.test` ".to_string()),
    );
    let _ = update(&mut app, ViewMessage::WebBookmarkWidthChanged(" 1280 ".to_string()));
    let _ = update(&mut app, ViewMessage::WebBookmarkHeightChanged("720px".to_string()));

    assert_eq!(app.web_bookmark_title_input, "  Example 标题  ");
    assert_eq!(app.web_bookmark_url_input, " `https://example.test` ");
    assert_eq!(app.web_bookmark_width_input, " 1280 ");
    assert_eq!(app.web_bookmark_height_input, "720px");
}

#[test]
fn bookmark_add_save_rejects_blank_title_without_clearing_draft() {
    let mut app = app();
    app.web_bookmark_title_input = " \n ".to_string();
    app.web_bookmark_url_input = "https://example.test".to_string();
    app.web_bookmark_width_input = "1280".to_string();
    app.web_bookmark_height_input = "720".to_string();

    let _ = update(&mut app, ViewMessage::WebBookmarkAddSave);

    assert!(app.web_bookmarks.is_empty());
    assert_eq!(app.web_bookmark_title_input, " \n ");
    assert_eq!(app.web_bookmark_url_input, "https://example.test");
    assert_eq!(app.web_bookmark_width_input, "1280");
    assert_eq!(app.web_bookmark_height_input, "720");
}

#[test]
fn bookmark_add_save_rejects_blank_url_without_clearing_draft() {
    let mut app = app();
    app.web_bookmark_title_input = "Example".to_string();
    app.web_bookmark_url_input = "\t ".to_string();

    let _ = update(&mut app, ViewMessage::WebBookmarkAddSave);

    assert!(app.web_bookmarks.is_empty());
    assert_eq!(app.web_bookmark_title_input, "Example");
    assert_eq!(app.web_bookmark_url_input, "\t ");
}

#[test]
fn bookmark_add_save_trims_fields_parses_dimensions_and_clears_draft() {
    let mut app = app();
    app.web_bookmark_title_input = "  Console  ".to_string();
    app.web_bookmark_url_input = " https://example.test/app ".to_string();
    app.web_bookmark_width_input = "1280".to_string();
    app.web_bookmark_height_input = "bad height".to_string();

    let _ = update(&mut app, ViewMessage::WebBookmarkAddSave);

    assert_eq!(app.web_bookmarks.len(), 1);
    assert_eq!(app.web_bookmarks[0].title, "Console");
    assert_eq!(app.web_bookmarks[0].url, "https://example.test/app");
    assert_eq!(app.web_bookmarks[0].width, Some(1280));
    assert_eq!(app.web_bookmarks[0].height, None);
    assert!(app.web_bookmarks[0].cookie_configs.is_none());
    assert!(app.web_bookmark_title_input.is_empty());
    assert!(app.web_bookmark_url_input.is_empty());
    assert!(app.web_bookmark_width_input.is_empty());
    assert!(app.web_bookmark_height_input.is_empty());
}

#[test]
fn bookmark_add_cancel_closes_menu_and_clears_every_draft_field() {
    let mut app = app();
    app.show_web_links_menu = true;
    app.web_bookmark_title_input = "title".to_string();
    app.web_bookmark_url_input = "url".to_string();
    app.web_bookmark_width_input = "800".to_string();
    app.web_bookmark_height_input = "600".to_string();

    let _ = update(&mut app, ViewMessage::WebBookmarkAddCancel);

    assert!(!app.show_web_links_menu);
    assert!(app.web_bookmark_title_input.is_empty());
    assert!(app.web_bookmark_url_input.is_empty());
    assert!(app.web_bookmark_width_input.is_empty());
    assert!(app.web_bookmark_height_input.is_empty());
}

#[test]
fn bookmark_edit_start_populates_dimensions_and_cookie_json() {
    let mut app = app();
    app.web_bookmarks.push(bookmark());

    let _ = update(&mut app, ViewMessage::WebBookmarkEditStart(0));

    assert_eq!(app.editing_web_bookmark, Some(0));
    assert_eq!(app.edit_web_bookmark_title_input, "Console");
    assert_eq!(app.edit_web_bookmark_url_input, "https://example.test/app");
    assert_eq!(app.edit_web_bookmark_width_input, "1280");
    assert_eq!(app.edit_web_bookmark_height_input, "720");
    let cookies: Vec<CookieConfig> =
        serde_json::from_str(&app.edit_web_bookmark_cookie_configs_editor.text())
            .expect("bookmark cookies should be rendered as valid JSON");
    assert_eq!(cookies.len(), 1);
    assert_eq!(cookies[0].name, "session");
    assert_eq!(cookies[0].domain.as_deref(), Some("example.test"));
    assert_eq!(cookies[0].days, Some(30));
    assert_eq!(cookies[0].url_filter.as_deref(), Some("https://example.test/app"));
}

#[test]
fn bookmark_edit_start_ignores_missing_index_and_preserves_draft() {
    let mut app = app();
    app.editing_web_bookmark = Some(4);
    app.edit_web_bookmark_title_input = "keep title".to_string();
    app.edit_web_bookmark_url_input = "keep url".to_string();
    app.edit_web_bookmark_width_input = "keep width".to_string();
    app.edit_web_bookmark_height_input = "keep height".to_string();
    app.edit_web_bookmark_cookie_configs_editor = text_editor::Content::with_text("keep cookies");

    let _ = update(&mut app, ViewMessage::WebBookmarkEditStart(99));

    assert_eq!(app.editing_web_bookmark, Some(4));
    assert_eq!(app.edit_web_bookmark_title_input, "keep title");
    assert_eq!(app.edit_web_bookmark_url_input, "keep url");
    assert_eq!(app.edit_web_bookmark_width_input, "keep width");
    assert_eq!(app.edit_web_bookmark_height_input, "keep height");
    assert_eq!(app.edit_web_bookmark_cookie_configs_editor.text(), "keep cookies");
}

#[test]
fn bookmark_edit_start_defaults_missing_optional_values() {
    let mut app = app();
    app.web_bookmarks.push(WebBookmark {
        title: "Docs".to_string(),
        url: "https://example.test/docs".to_string(),
        width: None,
        height: None,
        cookie_configs: None,
    });

    let _ = update(&mut app, ViewMessage::WebBookmarkEditStart(0));

    assert_eq!(app.editing_web_bookmark, Some(0));
    assert_eq!(app.edit_web_bookmark_title_input, "Docs");
    assert_eq!(app.edit_web_bookmark_url_input, "https://example.test/docs");
    assert!(app.edit_web_bookmark_width_input.is_empty());
    assert!(app.edit_web_bookmark_height_input.is_empty());
    let cookies: Vec<CookieConfig> =
        serde_json::from_str(&app.edit_web_bookmark_cookie_configs_editor.text())
            .expect("missing bookmark cookies should render as an empty JSON array");
    assert!(cookies.is_empty());
}

#[test]
fn bookmark_edit_inputs_and_editor_action_update_the_current_draft() {
    let mut app = app();

    let _ = update(&mut app, ViewMessage::WebBookmarkEditTitleChanged("  New title  ".to_string()));
    let _ = update(&mut app, ViewMessage::WebBookmarkEditUrlChanged(" new-url ".to_string()));
    let _ = update(&mut app, ViewMessage::WebBookmarkEditWidthChanged("1024".to_string()));
    let _ = update(&mut app, ViewMessage::WebBookmarkEditHeightChanged("invalid".to_string()));
    let _ = update(
        &mut app,
        ViewMessage::WebBookmarkEditCookieConfigsChanged(text_editor::Action::Edit(
            text_editor::Edit::Insert('x'),
        )),
    );

    assert_eq!(app.edit_web_bookmark_title_input, "  New title  ");
    assert_eq!(app.edit_web_bookmark_url_input, " new-url ");
    assert_eq!(app.edit_web_bookmark_width_input, "1024");
    assert_eq!(app.edit_web_bookmark_height_input, "invalid");
    assert_eq!(app.edit_web_bookmark_cookie_configs_editor.text(), "x");
}

#[test]
fn bookmark_cookie_example_is_valid_and_contains_documented_defaults() {
    let mut app = app();
    app.edit_web_bookmark_cookie_configs_editor = text_editor::Content::with_text("stale");

    let _ = update(&mut app, ViewMessage::WebBookmarkEditCookieConfigsInsertExample);

    let cookies: Vec<CookieConfig> =
        serde_json::from_str(&app.edit_web_bookmark_cookie_configs_editor.text())
            .expect("inserted cookie example should be valid JSON");
    assert_eq!(cookies.len(), 1);
    assert_eq!(cookies[0].name, "session_id");
    assert_eq!(cookies[0].domain.as_deref(), Some("example.com"));
    assert_eq!(cookies[0].days, Some(365));
    assert_eq!(cookies[0].url_filter.as_deref(), Some("https://example.com/"));
}

#[test]
fn bookmark_edit_save_rejects_blank_url_and_resets_editor_state() {
    let mut app = app();
    app.web_bookmarks.push(bookmark());
    app.editing_web_bookmark = Some(0);
    app.edit_web_bookmark_title_input = "Replacement".to_string();
    app.edit_web_bookmark_url_input = " \n ".to_string();
    app.edit_web_bookmark_width_input = "640".to_string();
    app.edit_web_bookmark_height_input = "480".to_string();
    app.edit_web_bookmark_cookie_configs_editor = text_editor::Content::with_text("not json");

    let _ = update(&mut app, ViewMessage::WebBookmarkEditSave);

    assert_eq!(app.web_bookmarks.len(), 1);
    assert_eq!(app.web_bookmarks[0].title, "Console");
    assert_eq!(app.web_bookmarks[0].url, "https://example.test/app");
    assert_eq!(app.web_bookmarks[0].width, Some(1280));
    assert_eq!(app.web_bookmarks[0].height, Some(720));
    assert!(app.editing_web_bookmark.is_none());
    assert!(app.edit_web_bookmark_title_input.is_empty());
    assert!(app.edit_web_bookmark_url_input.is_empty());
    assert!(app.edit_web_bookmark_width_input.is_empty());
    assert!(app.edit_web_bookmark_height_input.is_empty());
    assert!(app.edit_web_bookmark_cookie_configs_editor.text().is_empty());
}

#[test]
fn bookmark_edit_save_without_selected_bookmark_resets_stale_draft() {
    let mut app = app();
    app.edit_web_bookmark_title_input = "stale title".to_string();
    app.edit_web_bookmark_url_input = "stale url".to_string();
    app.edit_web_bookmark_width_input = "800".to_string();
    app.edit_web_bookmark_height_input = "600".to_string();
    app.edit_web_bookmark_cookie_configs_editor = text_editor::Content::with_text("stale cookies");

    let _ = update(&mut app, ViewMessage::WebBookmarkEditSave);

    assert!(app.editing_web_bookmark.is_none());
    assert!(app.edit_web_bookmark_title_input.is_empty());
    assert!(app.edit_web_bookmark_url_input.is_empty());
    assert!(app.edit_web_bookmark_width_input.is_empty());
    assert!(app.edit_web_bookmark_height_input.is_empty());
    assert!(app.edit_web_bookmark_cookie_configs_editor.text().is_empty());
}

#[test]
fn bookmark_edit_save_updates_selected_bookmark_and_uses_url_as_blank_title() {
    let mut app = app();
    app.web_bookmarks.push(bookmark());
    app.editing_web_bookmark = Some(0);
    app.edit_web_bookmark_title_input = " \n ".to_string();
    app.edit_web_bookmark_url_input = " https://example.test/new ".to_string();
    app.edit_web_bookmark_width_input = "wide".to_string();
    app.edit_web_bookmark_height_input = "480".to_string();
    app.edit_web_bookmark_cookie_configs_editor =
        text_editor::Content::with_text(r#"[{"name":"auth","days":7}]"#);

    let _ = update(&mut app, ViewMessage::WebBookmarkEditSave);

    assert_eq!(app.web_bookmarks[0].title, "https://example.test/new");
    assert_eq!(app.web_bookmarks[0].url, "https://example.test/new");
    assert_eq!(app.web_bookmarks[0].width, None);
    assert_eq!(app.web_bookmarks[0].height, Some(480));
    let cookies = app.web_bookmarks[0].cookie_configs.as_ref().unwrap();
    assert_eq!(cookies.len(), 1);
    assert_eq!(cookies[0].name, "auth");
    assert_eq!(cookies[0].days, Some(7));
    assert!(app.editing_web_bookmark.is_none());
    assert!(app.edit_web_bookmark_title_input.is_empty());
    assert!(app.edit_web_bookmark_cookie_configs_editor.text().is_empty());
}

#[test]
fn bookmark_edit_cancel_resets_selection_and_every_draft_field() {
    let mut app = app();
    app.editing_web_bookmark = Some(3);
    app.edit_web_bookmark_title_input = "title".to_string();
    app.edit_web_bookmark_url_input = "url".to_string();
    app.edit_web_bookmark_width_input = "800".to_string();
    app.edit_web_bookmark_height_input = "600".to_string();
    app.edit_web_bookmark_cookie_configs_editor = text_editor::Content::with_text("cookies");

    let _ = update(&mut app, ViewMessage::WebBookmarkEditCancel);

    assert!(app.editing_web_bookmark.is_none());
    assert!(app.edit_web_bookmark_title_input.is_empty());
    assert!(app.edit_web_bookmark_url_input.is_empty());
    assert!(app.edit_web_bookmark_width_input.is_empty());
    assert!(app.edit_web_bookmark_height_input.is_empty());
    assert!(app.edit_web_bookmark_cookie_configs_editor.text().is_empty());
}

#[test]
fn bookmark_remove_out_of_range_preserves_bookmarks_and_unrelated_edit() {
    let mut app = app();
    app.web_bookmarks.push(bookmark());
    app.editing_web_bookmark = Some(0);
    app.edit_web_bookmark_title_input = "keep".to_string();

    let _ = update(&mut app, ViewMessage::WebBookmarkRemove(9));

    assert_eq!(app.web_bookmarks.len(), 1);
    assert_eq!(app.web_bookmarks[0].title, "Console");
    assert_eq!(app.editing_web_bookmark, Some(0));
    assert_eq!(app.edit_web_bookmark_title_input, "keep");
}

#[test]
fn bookmark_remove_out_of_range_matching_edit_index_still_resets_editor() {
    let mut app = app();
    app.web_bookmarks.push(bookmark());
    app.editing_web_bookmark = Some(1);
    app.edit_web_bookmark_title_input = "stale title".to_string();
    app.edit_web_bookmark_url_input = "stale url".to_string();
    app.edit_web_bookmark_width_input = "800".to_string();
    app.edit_web_bookmark_height_input = "600".to_string();
    app.edit_web_bookmark_cookie_configs_editor = text_editor::Content::with_text("stale cookies");

    let _ = update(&mut app, ViewMessage::WebBookmarkRemove(1));

    assert_eq!(app.web_bookmarks.len(), 1);
    assert!(app.editing_web_bookmark.is_none());
    assert!(app.edit_web_bookmark_title_input.is_empty());
    assert!(app.edit_web_bookmark_url_input.is_empty());
    assert!(app.edit_web_bookmark_width_input.is_empty());
    assert!(app.edit_web_bookmark_height_input.is_empty());
    assert!(app.edit_web_bookmark_cookie_configs_editor.text().is_empty());
}

#[test]
fn bookmark_remove_existing_bookmark_persists_remaining_and_resets_matching_editor() {
    let mut app = app();
    app.web_bookmarks.push(bookmark());
    app.web_bookmarks.push(WebBookmark {
        title: "Docs".to_string(),
        url: "https://example.test/docs".to_string(),
        width: None,
        height: Some(600),
        cookie_configs: None,
    });
    app.editing_web_bookmark = Some(0);
    app.edit_web_bookmark_title_input = "stale title".to_string();
    app.edit_web_bookmark_url_input = "stale url".to_string();
    app.edit_web_bookmark_cookie_configs_editor = text_editor::Content::with_text("stale cookies");

    let _ = update(&mut app, ViewMessage::WebBookmarkRemove(0));

    assert_eq!(app.web_bookmarks.len(), 1);
    assert_eq!(app.web_bookmarks[0].title, "Docs");
    assert_eq!(app.web_bookmarks[0].height, Some(600));
    assert!(app.editing_web_bookmark.is_none());
    assert!(app.edit_web_bookmark_title_input.is_empty());
    assert!(app.edit_web_bookmark_url_input.is_empty());
    assert!(app.edit_web_bookmark_cookie_configs_editor.text().is_empty());
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
