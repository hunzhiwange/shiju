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

fn request(path: &str) -> Request<Vec<u8>> {
    Request::builder().uri(format!("vibe://localhost{path}")).body(Vec::new()).unwrap()
}

fn body_text(response: Response<Cow<'static, [u8]>>) -> String {
    String::from_utf8(response.body().to_vec()).unwrap()
}

#[test]
fn normalize_url_trims_nested_quotes() {
    assert_eq!(normalize_url("  `\"'https://example.com'\"`  "), "https://example.com");
    assert_eq!(normalize_url("  https://example.com  "), "https://example.com");
}

#[test]
fn normalize_url_keeps_unbalanced_quotes() {
    assert_eq!(normalize_url("'https://example.com"), "'https://example.com");
}

#[test]
fn percent_encode_url_path_keeps_unreserved_and_slash() {
    assert_eq!(percent_encode_url_path("/a b/中文/file.js"), "/a%20b/%E4%B8%AD%E6%96%87/file.js");
}

#[test]
fn percent_decode_decodes_hex_and_preserves_invalid_sequences() {
    assert_eq!(percent_decode("a%20b/%E4%B8%AD%E6%96%87"), "a b/中文");
    assert_eq!(percent_decode("%af"), "\u{FFFD}");
    assert_eq!(percent_decode("bad%2Gtail%"), "bad%2Gtail%");
}

#[test]
fn percent_decode_replaces_invalid_utf8() {
    assert_eq!(percent_decode("%FF"), "\u{FFFD}");
}

#[test]
fn is_safe_relative_path_rejects_escape_paths() {
    assert!(is_safe_relative_path(Path::new("assets/app.js")));
    assert!(!is_safe_relative_path(Path::new("../secret")));
    assert!(!is_safe_relative_path(Path::new("/absolute")));
}

#[test]
fn guess_mime_returns_known_types_and_default() {
    assert_eq!(guess_mime(Path::new("index.html")), "text/html; charset=utf-8");
    assert_eq!(guess_mime(Path::new("style.CSS")), "text/css; charset=utf-8");
    assert_eq!(guess_mime(Path::new("app.wasm")), "application/wasm");
    assert_eq!(guess_mime(Path::new("archive.bin")), "application/octet-stream");
}

#[test]
fn local_protocol_response_serves_index_for_root() {
    let dir = TestDir::new("root");
    fs::write(dir.path().join("index.html"), "<main>home</main>").unwrap();

    let response = local_protocol_response(dir.path(), request("/"));

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["Content-Type"], "text/html; charset=utf-8");
    assert_eq!(body_text(response), "<main>home</main>");
}

#[test]
fn local_protocol_response_serves_nested_file_with_mime() {
    let dir = TestDir::new("nested");
    fs::create_dir_all(dir.path().join("assets")).unwrap();
    fs::write(dir.path().join("assets/app.js"), "console.log(1);").unwrap();

    let response = local_protocol_response(dir.path(), request("/assets/app.js"));

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["Content-Type"], "text/javascript; charset=utf-8");
    assert_eq!(body_text(response), "console.log(1);");
}

#[test]
fn local_protocol_response_decodes_percent_encoded_path() {
    let dir = TestDir::new("encoded");
    fs::write(dir.path().join("hello world.txt"), "hello").unwrap();

    let response = local_protocol_response(dir.path(), request("/hello%20world.txt"));

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_text(response), "hello");
}

#[test]
fn local_protocol_response_serves_directory_index() {
    let dir = TestDir::new("directory");
    fs::create_dir_all(dir.path().join("docs")).unwrap();
    fs::write(dir.path().join("docs/index.html"), "docs").unwrap();

    let response = local_protocol_response(dir.path(), request("/docs"));

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_text(response), "docs");
}

#[test]
fn local_protocol_response_falls_back_to_index_for_spa_route() {
    let dir = TestDir::new("spa");
    fs::write(dir.path().join("index.html"), "spa").unwrap();

    let response = local_protocol_response(dir.path(), request("/settings/profile"));

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_text(response), "spa");
}

#[test]
fn local_protocol_response_returns_not_found_for_missing_asset() {
    let dir = TestDir::new("missing");
    fs::write(dir.path().join("index.html"), "spa").unwrap();

    let response = local_protocol_response(dir.path(), request("/missing.css"));

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(response.body().is_empty());
}

#[test]
fn local_protocol_response_returns_not_found_without_fallback_index() {
    let dir = TestDir::new("no-index");

    let response = local_protocol_response(dir.path(), request("/dashboard"));

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(response.body().is_empty());
}

#[test]
fn local_protocol_response_rejects_path_traversal_after_decode() {
    let dir = TestDir::new("forbidden");

    let response = local_protocol_response(dir.path(), request("/%2E%2E/secret.txt"));

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(response.body().is_empty());
}

#[test]
fn file_url_to_path_decodes_localhost_url() {
    let path = file_url_to_path("file://localhost/tmp/a%20b/index.html").unwrap();

    #[cfg(not(target_os = "windows"))]
    assert_eq!(path, PathBuf::from("/tmp/a b/index.html"));
    #[cfg(target_os = "windows")]
    assert_eq!(path, PathBuf::from("tmp\\a b\\index.html"));
}

#[test]
fn resolve_target_keeps_http_urls() {
    assert_eq!(
        resolve_target("HTTPS://example.com"),
        StartTarget::Url("HTTPS://example.com".into())
    );
}

#[test]
fn resolve_target_uses_directory_index() {
    let dir = TestDir::new("resolve-dir");
    fs::write(dir.path().join("index.html"), "home").unwrap();

    let target = resolve_target(dir.path().to_str().unwrap());

    match target {
        StartTarget::LocalDir { root, entry } => {
            assert_eq!(root, fs::canonicalize(dir.path()).unwrap());
            assert_eq!(entry, "/index.html");
        }
        StartTarget::Url(_) => panic!("expected local directory"),
    }
}

#[test]
fn resolve_target_uses_file_parent_as_root() {
    let dir = TestDir::new("resolve-file");
    let file = dir.path().join("app.html");
    fs::write(&file, "app").unwrap();

    let target = resolve_target(file.to_str().unwrap());

    match target {
        StartTarget::LocalDir { root, entry } => {
            assert_eq!(root, fs::canonicalize(dir.path()).unwrap());
            assert_eq!(entry, "/app.html");
        }
        StartTarget::Url(_) => panic!("expected local file"),
    }
}

#[test]
fn resolve_target_uses_url_for_directory_without_index() {
    let dir = TestDir::new("resolve-missing-index");

    assert_eq!(
        resolve_target(dir.path().to_str().unwrap()),
        StartTarget::Url(dir.path().to_string_lossy().to_string())
    );
}

#[test]
fn resolve_target_uses_url_for_missing_file_url_and_missing_plain_path() {
    assert_eq!(
        resolve_target("FILE:///definitely/missing/index.html"),
        StartTarget::Url("FILE:///definitely/missing/index.html".to_string())
    );
    assert_eq!(
        resolve_target("/definitely/missing/index.html"),
        StartTarget::Url("/definitely/missing/index.html".to_string())
    );
}

#[test]
fn parse_launch_options_reads_known_flags() {
    let args = vec![
        "--mode=embedded".to_string(),
        "--cookies=[{\"name\":\"sid\"}]".to_string(),
        "--shiju-app-set-cookie-stdin".to_string(),
        "--width=1024".to_string(),
        "--height=768".to_string(),
        "--x=-10".to_string(),
        "--y=25".to_string(),
        "--title=Custom".to_string(),
    ];

    assert_eq!(
        parse_launch_options(&args),
        LaunchOptions {
            mode: "embedded".to_string(),
            window_title: "Custom".to_string(),
            width: Some(1024),
            height: Some(768),
            pos_x: Some(-10),
            pos_y: Some(25),
            cookies_json: "[{\"name\":\"sid\"}]".to_string(),
            shiju_app_set_cookie_stdin: true,
        }
    );
}

#[test]
fn parse_launch_options_ignores_invalid_numbers() {
    let args = vec!["--width=wide".to_string(), "--height=".to_string(), "--unknown=x".to_string()];

    assert_eq!(parse_launch_options(&args), LaunchOptions::default());
}

#[test]
fn parse_stdin_event_accepts_resize_and_move() {
    assert_eq!(parse_stdin_event(" resize 640 480 "), Some(UserEvent::Resize(640, 480)));
    assert_eq!(parse_stdin_event("move -5 20 trailing"), Some(UserEvent::Move(-5, 20)));
}

#[test]
fn parse_stdin_event_rejects_invalid_commands() {
    assert_eq!(parse_stdin_event(""), None);
    assert_eq!(parse_stdin_event("resize 10"), None);
    assert_eq!(parse_stdin_event("resize wide 10"), None);
    assert_eq!(parse_stdin_event("open https://example.com"), None);
}

#[test]
fn work_browser_protocol_parses_navigation_history_visibility_and_close() {
    assert_eq!(
        parse_stdin_event("navigate https://example.com/path?q=1"),
        Some(UserEvent::OpenUrl("https://example.com/path?q=1".to_string()))
    );
    assert_eq!(parse_stdin_event("back"), Some(UserEvent::Back));
    assert_eq!(parse_stdin_event("forward"), Some(UserEvent::Forward));
    assert_eq!(parse_stdin_event("reload"), Some(UserEvent::Reload));
    assert_eq!(parse_stdin_event("raise"), Some(UserEvent::SetAlwaysOnTop(true)));
    assert_eq!(parse_stdin_event("lower"), Some(UserEvent::SetAlwaysOnTop(false)));
    assert_eq!(parse_stdin_event("show"), Some(UserEvent::Show));
    assert_eq!(parse_stdin_event("hide"), Some(UserEvent::Hide));
    assert_eq!(parse_stdin_event("close"), Some(UserEvent::Close));
    assert_eq!(parse_stdin_event("back unexpected"), None);
    assert_eq!(parse_stdin_event("navigate"), None);
}

#[test]
fn work_browser_protocol_rejects_unsafe_navigation_scheme() {
    assert_eq!(parse_stdin_event("navigate javascript:alert(1)"), None);
    assert_eq!(parse_stdin_event("navigate file:///tmp/private"), None);
    assert_eq!(parse_stdin_event("navigate data:text/html,unsafe"), None);
    assert_eq!(parse_stdin_event("navigate https://"), None);
    assert_eq!(
        parse_stdin_event("navigate HTTP://example.com"),
        Some(UserEvent::OpenUrl("HTTP://example.com".to_string()))
    );
}

#[test]
fn work_browser_navigation_event_encodes_safe_url_for_parent() {
    assert_eq!(
        navigation_event_line("https://example.com/path?q=a%20b#part").as_deref(),
        Some(r#"navigation "https://example.com/path?q=a%20b#part""#)
    );
    assert_eq!(navigation_event_line("javascript:alert(1)"), None);
    assert_eq!(navigation_event_line("file:///tmp/private"), None);
}

#[test]
fn work_browser_navigation_ipc_accepts_only_safe_navigation_messages() {
    assert_eq!(
        parse_navigation_ipc_message(
            r#"{"type":"navigation","url":"https://example.com/client-route"}"#
        )
        .as_deref(),
        Some("https://example.com/client-route")
    );
    assert_eq!(
        parse_navigation_ipc_message(r#"{"type":"navigation","url":"javascript:alert(1)"}"#),
        None
    );
    assert_eq!(
        parse_navigation_ipc_message(r#"{"type":"other","url":"https://example.com"}"#),
        None
    );
    assert_eq!(parse_navigation_ipc_message("not-json"), None);
}

#[test]
fn work_browser_protocol_redacts_cookie_arguments() {
    let options = LaunchOptions {
        mode: "embedded".to_string(),
        window_title: "Sensitive title".to_string(),
        width: Some(1024),
        height: Some(768),
        pos_x: Some(10),
        pos_y: Some(20),
        cookies_json: r#"[{"name":"sid","value":"cookie-secret"}]"#.to_string(),
        shiju_app_set_cookie_stdin: true,
    };

    let summary =
        launch_log_summary("https://user:password@example.com/path?token=url-secret", &options);

    assert!(summary.contains("target=https://example.com"));
    assert!(summary.contains("width=1024"));
    assert!(summary.contains("height=768"));
    assert!(!summary.contains("cookie-secret"));
    assert!(!summary.contains("url-secret"));
    assert!(!summary.contains("password"));
    assert!(!summary.contains("Sensitive title"));
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
        let error =
            read_shiju_app_set_cookie_for_target(target, &mut "do-not-read\n".as_bytes())
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
    assert_eq!(
        headers["Cookie"],
        "app_runtime_session=fixture-session-secret"
    );
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
    assert_eq!(DEFAULT_TARGET, "about:blank");
    assert_eq!(DEFAULT_TITLE, "Vibe Window WebView");
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
