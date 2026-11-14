//! 启动选项与日志脱敏的行为测试。

use super::*;

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
fn constants_keep_expected_defaults() {
    assert_eq!(DEFAULT_TARGET, "about:blank");
    assert_eq!(DEFAULT_TITLE, "Vibe Window WebView");
}
