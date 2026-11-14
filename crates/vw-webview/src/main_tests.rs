//! WebView 启动入口、Cookie 与数据目录的行为测试。

use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(name: &str) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir()
            .join(format!("vw-webview-main-tests-{}-{name}-{id}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn shiju_app_set_cookie_mode_rejects_untrusted_targets() {
    assert!(is_trusted_shiju_app_target("https://app.shiju.cloud/app/ABC123/"));
    for target in [
        "http://app.shiju.cloud/app/ABC123/",
        "https://evil.example/app/ABC123/",
        "https://app.shiju.cloud.evil.example/app/ABC123/",
        "https://app.shiju.cloud/app/abc123/",
        "https://app.shiju.cloud/app/ABC-123/",
        "https://app.shiju.cloud/app/ABC123",
        "https://app.shiju.cloud/app/ABC123/?next=evil",
    ] {
        assert!(!is_trusted_shiju_app_target(target), "accepted untrusted target: {target}");
        let error = read_shiju_app_set_cookie_for_target(target, &mut "do-not-read\n".as_bytes())
            .unwrap_err();
        assert!(!error.to_string().contains("do-not-read"));
    }
}

#[test]
fn shiju_app_set_cookie_input_is_bounded_and_redacted() {
    let set_cookie =
        "app_runtime_session=shiju-secret-token; Path=/; HttpOnly; Secure; SameSite=None";
    assert_eq!(
        read_shiju_app_set_cookie_for_target(
            "https://app.shiju.cloud/app/ABC123/",
            &mut format!("{set_cookie}\n").as_bytes(),
        )
        .unwrap(),
        set_cookie
    );

    for input in [
        format!("{}\n", "x".repeat(MAX_SHIJU_APP_SET_COOKIE_BYTES + 1)),
        "secret\u{0}token\n".to_string(),
        "secret\ttoken\n".to_string(),
        "\n".to_string(),
    ] {
        let error = read_shiju_app_set_cookie_for_target(
            "https://app.shiju.cloud/app/ABC123/",
            &mut input.as_bytes(),
        )
        .unwrap_err();
        assert!(!error.to_string().contains(&input));
        assert!(!error.to_string().contains("secret"));
    }
}

#[test]
fn shiju_app_runtime_cookie_preserves_response_security_attributes() {
    let cookie = runtime_cookie(
        "app_runtime_session=fixture-session-secret; Path=/; Max-Age=3600; HttpOnly; Secure; SameSite=None",
    )
    .unwrap();
    let headers = runtime_cookie_headers(&cookie).unwrap();

    assert_eq!(cookie.name(), "app_runtime_session");
    assert_eq!(cookie.value(), "fixture-session-secret");
    assert_eq!(cookie.domain(), Some("app.shiju.cloud"));
    assert_eq!(cookie.path(), Some("/"));
    assert_eq!(cookie.http_only(), Some(true));
    assert_eq!(cookie.secure(), Some(true));
    assert_eq!(cookie.same_site(), Some(wry::cookie::SameSite::None));
    assert_eq!(headers["Cookie"], "app_runtime_session=fixture-session-secret");
}

#[test]
fn append_log_creates_parent_and_appends_lines() {
    let dir = TestDir::new("log");
    let log_path = dir.path().join("logs/vw-webview.log");

    append_log(&log_path, "one");
    append_log(&log_path, "two");

    assert_eq!(fs::read_to_string(log_path).unwrap(), "one\ntwo\n");
}

#[test]
fn build_init_script_escapes_json_values_and_embeds_cookie_config() {
    let script = build_init_script(
        Path::new("/tmp/path with \"quote\""),
        r#"[{"name":"sid","domain":"example.com"}]"#,
        "Agent \"A\"",
    )
    .unwrap();

    assert!(script.contains(r#"const dataPath = "/tmp/path with \"quote\"";"#));
    assert!(script.contains(r#"const userAgent = "Agent \"A\"";"#));
    assert!(script.contains(r#"const cookieConfigs = [{"name":"sid","domain":"example.com"}];"#));
    assert!(script.contains("function initSessionSync()"));
    assert!(script.contains("history.pushState = function"));
    assert!(script.contains("history.replaceState = function"));
    assert!(script.contains("window.addEventListener('popstate', reportNavigation)"));
    assert!(script.contains("window.ipc.postMessage"));
}

#[test]
fn constants_keep_expected_defaults() {
    assert!(SAFARI_USER_AGENT.contains("Safari/605.1.15"));
}

#[test]
fn user_event_open_url_keeps_url_payload() {
    assert_eq!(
        UserEvent::OpenUrl("https://example.com".to_string()),
        UserEvent::OpenUrl("https://example.com".to_string())
    );
}

#[test]
fn webview_data_dir_uses_webview_data_leaf() {
    assert_eq!(webview_data_dir().file_name().and_then(|s| s.to_str()), Some("webview_data"));
}

#[test]
fn fallback_webview_data_dir_uses_webview_data_leaf() {
    assert_eq!(
        fallback_webview_data_dir().file_name().and_then(|s| s.to_str()),
        Some("webview_data")
    );
}

#[test]
fn fallback_webview_data_dir_from_uses_base_or_current_dir() {
    assert_eq!(
        fallback_webview_data_dir_from(Some(Path::new("/tmp/data"))),
        PathBuf::from("/tmp/data/vibe-window/webview_data")
    );
    assert_eq!(
        fallback_webview_data_dir_from(None),
        std::env::current_dir().unwrap().join("webview_data")
    );
}

#[cfg(coverage)]
#[test]
fn coverage_main_is_empty() {
    main();
}

#[test]
fn show_error_dialog_accepts_message() {
    show_error_dialog("VibeWindow WebView", "test error");
}
