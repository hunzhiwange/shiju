use super::*;
use serde_json::json;
#[test]
fn load_web_bookmarks_returns_default_when_config_key_is_missing() {
    let bookmarks = load_web_bookmarks(&json!({}));

    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0].title, "测试网站");
    assert_eq!(bookmarks[0].url, "https://example.com/");
    assert_eq!(bookmarks[0].width, None);
    assert_eq!(bookmarks[0].height, None);
    assert!(bookmarks[0].cookie_configs.is_none());
}

#[test]
fn load_web_bookmarks_keeps_explicit_empty_array_empty() {
    let bookmarks = load_web_bookmarks(&json!({
        "web_bookmarks": []
    }));

    assert!(bookmarks.is_empty());
}

#[test]
fn load_web_bookmarks_parses_valid_bookmarks_and_cookie_configs() {
    let bookmarks = load_web_bookmarks(&json!({
        "web_bookmarks": [
            {
                "title": "Console",
                "url": "https://console.example.com/",
                "width": 1280,
                "height": 720,
                "cookie_configs": [
                    {
                        "name": "session",
                        "domain": ".example.com",
                        "days": 7,
                        "url_filter": "console"
                    }
                ]
            }
        ]
    }));

    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0].title, "Console");
    assert_eq!(bookmarks[0].url, "https://console.example.com/");
    assert_eq!(bookmarks[0].width, Some(1280));
    assert_eq!(bookmarks[0].height, Some(720));

    let cookie_configs =
        bookmarks[0].cookie_configs.as_ref().expect("valid bookmark should keep cookie configs");
    assert_eq!(cookie_configs.len(), 1);
    assert_eq!(cookie_configs[0].name, "session");
    assert_eq!(cookie_configs[0].domain.as_deref(), Some(".example.com"));
    assert_eq!(cookie_configs[0].days, Some(7));
    assert_eq!(cookie_configs[0].url_filter.as_deref(), Some("console"));
}

#[test]
fn load_web_bookmarks_ignores_non_numeric_window_size_and_non_array_cookies() {
    let bookmarks = load_web_bookmarks(&json!({
        "web_bookmarks": [
            {
                "title": "Tool",
                "url": "https://tool.example.com/",
                "width": "wide",
                "height": false,
                "cookie_configs": "not an array"
            }
        ]
    }));

    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0].title, "Tool");
    assert_eq!(bookmarks[0].width, None);
    assert_eq!(bookmarks[0].height, None);
    assert!(bookmarks[0].cookie_configs.is_none());
}

#[test]
fn load_web_bookmarks_filters_invalid_bookmarks_and_cookie_configs() {
    let bookmarks = load_web_bookmarks(&json!({
        "web_bookmarks": [
            {
                "title": "Missing URL"
            },
            {
                "url": "https://missing-title.example.com/"
            },
            {
                "title": "Valid",
                "url": "https://valid.example.com/",
                "cookie_configs": [
                    {
                        "domain": ".example.com"
                    },
                    {
                        "name": "auth"
                    }
                ]
            }
        ]
    }));

    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0].title, "Valid");

    let cookie_configs = bookmarks[0]
        .cookie_configs
        .as_ref()
        .expect("valid bookmark should keep filtered cookie configs");
    assert_eq!(cookie_configs.len(), 1);
    assert_eq!(cookie_configs[0].name, "auth");
    assert_eq!(cookie_configs[0].domain, None);
    assert_eq!(cookie_configs[0].days, None);
    assert_eq!(cookie_configs[0].url_filter, None);
}

#[test]
fn load_web_bookmarks_keeps_valid_bookmark_without_cookie_configs() {
    let bookmarks = load_web_bookmarks(&json!({
        "web_bookmarks": [
            {
                "title": "Docs",
                "url": "https://docs.example.com/"
            }
        ]
    }));

    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0].title, "Docs");
    assert_eq!(bookmarks[0].url, "https://docs.example.com/");
    assert_eq!(bookmarks[0].width, None);
    assert_eq!(bookmarks[0].height, None);
    assert!(bookmarks[0].cookie_configs.is_none());
}

#[test]
fn load_web_bookmarks_keeps_empty_cookie_array_and_ignores_invalid_optional_fields() {
    let bookmarks = load_web_bookmarks(&json!({
        "web_bookmarks": [
            {
                "title": "Dashboard",
                "url": "https://dashboard.example.com/",
                "width": null,
                "height": "tall",
                "cookie_configs": []
            }
        ]
    }));

    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0].width, None);
    assert_eq!(bookmarks[0].height, None);
    let cookie_configs = bookmarks[0]
        .cookie_configs
        .as_ref()
        .expect("an explicit cookie array should remain present");
    assert!(cookie_configs.is_empty());
}

#[test]
fn load_web_bookmarks_returns_default_when_config_key_is_not_an_array() {
    let bookmarks = load_web_bookmarks(&json!({
        "web_bookmarks": "not an array"
    }));

    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0].title, "测试网站");
}
