use super::{ExternalTarget, Navigation, is_page_link_url, is_safe_navigation_url, navigation};

#[test]
fn page_link_allows_generated_urls_only_inside_the_webview() {
    for url in ["blob:https://example.com/id", "data:text/plain,download%20content"] {
        assert!(is_page_link_url(url));
        assert!(!is_safe_navigation_url(url));
    }
    assert!(is_page_link_url("https://example.com/download"));
    for url in ["file:///tmp/example", "javascript:alert(1)", "data:", "blob:https://a/\nx"] {
        assert!(!is_page_link_url(url));
    }
}

#[test]
fn external_protocol_rejects_unsafe_and_accepts_page_links() {
    for url in [
        "javascript:alert(1)",
        "file:///tmp/private",
        "ms-settings:privacy",
        "search-ms:query=secret",
        "shell:startup",
        "x-apple.systempreferences:com.apple.settings",
        "chrome://settings",
        "view-source:https://example.test",
        "vibe://localhost/private",
        "mailto:a@example.test\r\nBcc:other@example.test",
        "mailto:a@example.test?subject=%0d%0aBcc:other@example.test",
        "tel:",
        "1bad:payload",
        "C:private.exe",
        "custom://",
    ] {
        assert!(matches!(navigation(url, false), Navigation::Blocked), "accepted {url:?}");
    }
    for url in [
        "https://example.test",
        "http://127.0.0.1:5000",
        "blob:https://example.test/id",
        "data:text/plain,download",
        "about:blank",
    ] {
        assert!(matches!(navigation(url, false), Navigation::InWebView));
        assert!(ExternalTarget::parse(url).is_none());
    }
    assert!(matches!(navigation("vibe://localhost/index.html", true), Navigation::InWebView));
}
