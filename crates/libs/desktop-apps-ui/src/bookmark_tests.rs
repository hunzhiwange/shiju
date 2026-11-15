use super::{CookieConfig, WebBookmark};

#[test]
fn web_bookmark_round_trips_nested_cookie_configuration() {
    let bookmark = WebBookmark {
        title: "Console".to_string(),
        url: "https://example.test/app".to_string(),
        width: Some(1280),
        height: Some(720),
        cookie_configs: Some(vec![CookieConfig {
            name: "session".to_string(),
            domain: Some("example.test".to_string()),
            days: Some(30),
            url_filter: Some("/app".to_string()),
        }]),
    };

    let encoded = serde_json::to_value(&bookmark).expect("bookmark should serialize");
    let decoded: WebBookmark =
        serde_json::from_value(encoded).expect("bookmark should deserialize");

    assert_eq!(decoded.title, "Console");
    assert_eq!(decoded.url, "https://example.test/app");
    assert_eq!(decoded.width, Some(1280));
    assert_eq!(decoded.height, Some(720));
    let cookies = decoded.cookie_configs.expect("cookie configuration should remain present");
    assert_eq!(cookies.len(), 1);
    assert_eq!(cookies[0].name, "session");
    assert_eq!(cookies[0].domain.as_deref(), Some("example.test"));
    assert_eq!(cookies[0].days, Some(30));
    assert_eq!(cookies[0].url_filter.as_deref(), Some("/app"));
}

#[test]
fn web_bookmark_preserves_absent_optional_values() {
    let bookmark: WebBookmark = serde_json::from_value(serde_json::json!({
        "title": "Minimal",
        "url": "about:blank",
        "width": null,
        "height": null,
        "cookie_configs": null
    }))
    .expect("minimal bookmark should deserialize");

    assert!(bookmark.width.is_none());
    assert!(bookmark.height.is_none());
    assert!(bookmark.cookie_configs.is_none());
}
