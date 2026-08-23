//! 验证计算机操作端点、网址、坐标及输出路径的安全校验。
//! 端点、目标网址与输出路径分别校验；网络探测和建目录只能发生在静态验证之后。

use super::*;
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::tools::browser::computer_use::ComputerUseConfig;
use std::sync::Arc;
use tempfile::TempDir;

fn client(config: ComputerUseConfig, allowed_domains: Vec<String>) -> ComputerUseClient {
    ComputerUseClient::new(Arc::new(SecurityPolicy::default()), allowed_domains, None, config)
}

// 策略夹具允许分别控制网络白名单与文件路径规则，避免把两类拒绝混为一谈。
fn client_with_policy(
    security: SecurityPolicy,
    config: ComputerUseConfig,
    allowed_domains: Vec<String>,
) -> ComputerUseClient {
    ComputerUseClient::new(Arc::new(security), allowed_domains, None, config)
}

#[test]
fn endpoint_url_keeps_remote_public_hosts_https_only() {
    let local = client(ComputerUseConfig::default(), vec!["example.com".into()]);
    assert!(local.endpoint_url().is_ok());

    let remote_http = client(
        ComputerUseConfig {
            endpoint: "http://example.com/actions".into(),
            allow_remote_endpoint: true,
            ..Default::default()
        },
        vec!["example.com".into()],
    );
    assert!(remote_http.endpoint_url().is_err());

    let remote_https = client(
        ComputerUseConfig {
            endpoint: "https://example.com/actions".into(),
            allow_remote_endpoint: true,
            ..Default::default()
        },
        vec!["example.com".into()],
    );
    assert!(remote_https.endpoint_url().is_ok());
}

#[test]
fn endpoint_url_trims_endpoint_and_allows_private_http_in_remote_mode() {
    let c = client(
        ComputerUseConfig {
            endpoint: "  http://127.0.0.1:8787/v1/actions  ".into(),
            allow_remote_endpoint: true,
            ..Default::default()
        },
        vec![],
    );

    let endpoint = c.endpoint_url().expect("private HTTP endpoint should be accepted");
    assert_eq!(endpoint.as_str(), "http://127.0.0.1:8787/v1/actions");
}

#[test]
fn endpoint_url_rejects_empty_bad_scheme_public_and_zero_timeout() {
    for (config, expected) in [
        (
            ComputerUseConfig { endpoint: "   ".into(), ..Default::default() },
            "endpoint cannot be empty",
        ),
        (
            ComputerUseConfig { endpoint: "ftp://127.0.0.1/actions".into(), ..Default::default() },
            "must use http:// or https://",
        ),
        (
            ComputerUseConfig { endpoint: "not a url".into(), ..Default::default() },
            "Invalid browser.computer_use.endpoint",
        ),
        (
            ComputerUseConfig {
                endpoint: "https://example.com/actions".into(),
                allow_remote_endpoint: false,
                ..Default::default()
            },
            "host 'example.com' is public",
        ),
        (ComputerUseConfig { timeout_ms: 0, ..Default::default() }, "timeout_ms must be > 0"),
    ] {
        let c = client(config, vec!["example.com".into()]);
        let err = c.endpoint_url().expect_err("endpoint should be rejected");
        assert!(err.to_string().contains(expected), "{err}");
    }
}

#[test]
fn available_propagates_endpoint_validation_before_network_probe() {
    let c = client(ComputerUseConfig { timeout_ms: 0, ..Default::default() }, vec![]);

    let err = c.available().expect_err("invalid endpoint config should fail before probing");
    assert!(err.to_string().contains("timeout_ms must be > 0"));
}

#[test]
fn available_reports_unreachable_valid_loopback_endpoint() {
    let c = client(
        ComputerUseConfig { endpoint: "http://127.0.0.1:0/actions".into(), ..Default::default() },
        vec![],
    );

    assert!(!c.available().expect("valid endpoint should produce availability state"));
}

#[test]
fn validate_url_rejects_unsafe_shapes_and_accepts_allowlisted_hosts() {
    let c = client(ComputerUseConfig::default(), vec!["example.com".into()]);

    assert!(c.validate_url("https://example.com/path").is_ok());
    assert!(c.validate_url("http://www.example.com/path").is_ok());

    for (url, expected) in [
        ("   ", "URL cannot be empty"),
        ("file:///tmp/a.txt", "file:// URLs are not allowed"),
        ("ssh://example.com", "Only http:// and https:// URLs are allowed"),
        ("http://127.0.0.1:3000", "Blocked local/private host"),
        ("https://example.net", "not in browser.allowed_domains"),
    ] {
        let err = c.validate_url(url).expect_err("URL should be rejected");
        assert!(err.to_string().contains(expected), "{url}: {err}");
    }

    let no_domains = client(ComputerUseConfig::default(), vec![]);
    let err = no_domains.validate_url("https://example.com").unwrap_err();
    assert!(err.to_string().contains("no allowed_domains configured"));
}

#[test]
fn validate_url_trims_input_and_rejects_missing_host() {
    let c = client(ComputerUseConfig::default(), vec!["example.com".into()]);

    assert!(c.validate_url("  https://example.com/path  ").is_ok());
    let err = c.validate_url("https://").expect_err("URL without a host should be rejected");
    assert!(err.to_string().contains("Invalid URL: no host"));
}

#[test]
fn validate_url_rejects_private_ipv6_hosts() {
    let c = client(ComputerUseConfig::default(), vec!["*".into()]);

    for url in ["https://[::1]/", "https://[::ffff:127.0.0.1]/"] {
        let err = c.validate_url(url).expect_err("private IPv6 host should be rejected");
        assert!(err.to_string().contains("Blocked local/private host"), "{url}: {err}");
    }
}

#[test]
fn validate_coordinate_enforces_lower_and_upper_bounds() {
    let c = client(ComputerUseConfig::default(), vec![]);
    assert!(c.validate_coordinate("x", 0, Some(10)).is_ok());
    assert!(c.validate_coordinate("x", -1, Some(10)).is_err());
    assert!(c.validate_coordinate("x", 11, Some(10)).is_err());
    assert!(c.validate_coordinate("x", 1, Some(-1)).is_err());
}

#[test]
fn validate_coordinate_without_limit_accepts_large_nonnegative_value() {
    let c = client(ComputerUseConfig::default(), vec![]);

    assert!(c.validate_coordinate("x", i64::MAX, None).is_ok());
}

#[test]
fn read_required_i64_rejects_missing_or_wrong_type() {
    let params = serde_json::json!({"x": 3, "y": "4"}).as_object().unwrap().clone();
    assert_eq!(read_required_i64(&params, "x").unwrap(), 3);
    assert!(read_required_i64(&params, "y").is_err());
    assert!(read_required_i64(&params, "z").is_err());
}

#[test]
fn validate_output_path_rejects_escape_empty_and_null_paths() {
    let c = client(ComputerUseConfig::default(), vec![]);

    assert!(c.validate_output_path("path", "screens/out.png").is_ok());

    for (path, expected) in [
        (" ", "path cannot be empty"),
        ("bad\0path", "invalid null byte"),
        ("../escape.png", "must stay within the workspace"),
        ("/tmp/escape.png", "must stay within the workspace"),
    ] {
        let err = c.validate_output_path("path", path).expect_err("path should be rejected");
        assert!(err.to_string().contains(expected), "{path:?}: {err}");
    }
}

#[test]
fn validate_output_path_applies_security_policy() {
    let mut security = SecurityPolicy::default();
    security.forbidden_paths = vec!["blocked".into()];
    let c = client_with_policy(security, ComputerUseConfig::default(), vec![]);

    let err = c
        .validate_output_path("path", "blocked/out.png")
        .expect_err("policy-blocked relative path should be rejected");
    assert!(err.to_string().contains("path blocked by security policy"));
}

#[tokio::test]
// 路径先做词法与安全策略校验，再异步创建父目录并拒绝链接或非普通文件。
async fn resolve_output_path_creates_parent_and_rejects_non_files() {
    let tmp = TempDir::new().expect("tempdir should create");
    let mut security = SecurityPolicy::default();
    security.workspace_dir = tmp.path().to_path_buf();
    let c = client_with_policy(security, ComputerUseConfig::default(), vec![]);

    let resolved = c
        .resolve_output_path_for_write("path", "screens/out.png")
        .await
        .expect("relative output should resolve");
    assert!(resolved.ends_with("screens/out.png"));
    assert!(resolved.parent().unwrap().is_dir());

    tokio::fs::create_dir_all(tmp.path().join("dir-output")).await.unwrap();
    let err = c
        .resolve_output_path_for_write("path", "dir-output")
        .await
        .expect_err("directory output should fail");
    assert!(err.to_string().contains("not a regular file"));
}

#[tokio::test]
async fn resolve_output_path_accepts_existing_regular_file() {
    let tmp = TempDir::new().expect("tempdir should create");
    let mut security = SecurityPolicy::default();
    security.workspace_dir = tmp.path().to_path_buf();
    let c = client_with_policy(security, ComputerUseConfig::default(), vec![]);

    tokio::fs::write(tmp.path().join("existing.png"), b"existing")
        .await
        .expect("fixture file should create");
    let resolved = c
        .resolve_output_path_for_write("path", "existing.png")
        .await
        .expect("existing regular file should be accepted");

    assert!(resolved.is_file());
}

#[cfg(unix)]
#[tokio::test]
async fn resolve_output_path_rejects_existing_symlink() {
    let tmp = TempDir::new().expect("tempdir should create");
    let mut security = SecurityPolicy::default();
    security.workspace_dir = tmp.path().to_path_buf();
    let c = client_with_policy(security, ComputerUseConfig::default(), vec![]);

    let target = tmp.path().join("target.png");
    tokio::fs::write(&target, b"target").await.expect("target fixture should create");
    std::os::unix::fs::symlink(&target, tmp.path().join("output.png"))
        .expect("symlink fixture should create");

    let err = c
        .resolve_output_path_for_write("path", "output.png")
        .await
        .expect_err("symlink output should be rejected");
    assert!(err.to_string().contains("Refusing to write browser output through symlink"));
}

#[cfg(unix)]
#[tokio::test]
async fn resolve_output_path_rejects_symlinked_parent_escape() {
    let tmp = TempDir::new().expect("tempdir should create");
    let workspace = tmp.path().join("workspace");
    let outside = tmp.path().join("outside");
    tokio::fs::create_dir_all(&workspace).await.expect("workspace should create");
    tokio::fs::create_dir_all(&outside).await.expect("outside fixture should create");
    std::os::unix::fs::symlink(&outside, workspace.join("escape"))
        .expect("directory symlink fixture should create");

    let mut security = SecurityPolicy::default();
    security.workspace_dir = workspace;
    let c = client_with_policy(security, ComputerUseConfig::default(), vec![]);

    let err = c
        .resolve_output_path_for_write("path", "escape/out.png")
        .await
        .expect_err("resolved parent outside workspace should be rejected");
    assert!(err.to_string().contains("Resolved path escapes workspace allowlist"));
}

#[test]
fn validate_action_covers_computer_use_specific_rules() {
    let c = client(
        ComputerUseConfig {
            max_coordinate_x: Some(10),
            max_coordinate_y: Some(20),
            ..Default::default()
        },
        vec!["example.com".into()],
    );

    let open = serde_json::json!({"url": "https://example.com"}).as_object().unwrap().clone();
    assert!(c.validate_action("open", &open).is_ok());

    let mouse = serde_json::json!({"x": 10, "y": 20}).as_object().unwrap().clone();
    assert!(c.validate_action("mouse_move", &mouse).is_ok());
    assert!(c.validate_action("mouse_click", &mouse).is_ok());

    let drag = serde_json::json!({"from_x": 0, "from_y": 1, "to_x": 10, "to_y": 20})
        .as_object()
        .unwrap()
        .clone();
    assert!(c.validate_action("mouse_drag", &drag).is_ok());

    let key_type = serde_json::json!({"text": "hello"}).as_object().unwrap().clone();
    assert!(c.validate_action("key_type", &key_type).is_ok());

    let key_press = serde_json::json!({"key": "Control+C"}).as_object().unwrap().clone();
    assert!(c.validate_action("key_press", &key_press).is_ok());

    let capture = serde_json::json!({"path": "screens/out.png"}).as_object().unwrap().clone();
    assert!(c.validate_action("screen_capture", &capture).is_ok());
    assert!(c.validate_action("unknown", &serde_json::Map::new()).is_ok());
}

#[test]
fn screen_capture_rejects_invalid_geometry() {
    let c = client(
        ComputerUseConfig {
            max_coordinate_x: Some(10),
            max_coordinate_y: Some(20),
            ..Default::default()
        },
        vec![],
    );

    for params in [
        serde_json::json!({"display_id": 0}),
        serde_json::json!({"display_id": -1}),
        serde_json::json!({"display_id": "1"}),
        serde_json::json!({"display_id": 1, "x": 0, "y": 0, "width": 1, "height": 1}),
        serde_json::json!({"x": -1, "y": 0, "width": 1, "height": 1}),
        serde_json::json!({"x": 0, "y": -1, "width": 1, "height": 1}),
        serde_json::json!({"x": 0, "y": 0, "width": 0, "height": 1}),
        serde_json::json!({"x": 0, "y": 0, "width": 1, "height": 0}),
        serde_json::json!({"x": 0, "y": 0, "width": 1}),
        serde_json::json!({"x": 11, "y": 0, "width": 1, "height": 1}),
        serde_json::json!({"x": 0, "y": 21, "width": 1, "height": 1}),
        serde_json::json!({"unexpected": true}),
    ] {
        let error = c
            .validate_action("screen_capture", params.as_object().unwrap())
            .expect_err("invalid screen capture geometry should fail");
        assert!(!error.to_string().is_empty());
    }

    for params in [
        serde_json::json!({}),
        serde_json::json!({"path": "screens/capture.png"}),
        serde_json::json!({"display_id": 1}),
        serde_json::json!({"x": 0, "y": 0, "width": 1, "height": 1}),
    ] {
        assert!(c.validate_action("screen_capture", params.as_object().unwrap()).is_ok());
    }
}

#[test]
fn validate_action_reports_missing_or_invalid_parameters() {
    let c = client(
        ComputerUseConfig {
            max_coordinate_x: Some(10),
            max_coordinate_y: Some(20),
            ..Default::default()
        },
        vec!["example.com".into()],
    );

    for (action, params, expected) in [
        ("open", serde_json::json!({}), "Missing 'url'"),
        ("mouse_move", serde_json::json!({"x": 1}), "Missing or invalid 'y'"),
        ("mouse_click", serde_json::json!({"x": 11, "y": 1}), "exceeds configured limit"),
        (
            "mouse_drag",
            serde_json::json!({"from_x": 0, "from_y": 0, "to_x": 0}),
            "Missing or invalid 'to_y'",
        ),
        ("key_type", serde_json::json!({"text": "   "}), "must not be empty"),
        ("key_type", serde_json::json!({"text": "x".repeat(4097)}), "exceeds maximum length"),
        ("key_press", serde_json::json!({"key": "bad key!"}), "must be 1-32 chars"),
        (
            "screen_capture",
            serde_json::json!({"path": "../out.png"}),
            "must stay within the workspace",
        ),
    ] {
        let params = params.as_object().unwrap().clone();
        let err = c.validate_action(action, &params).expect_err("action should fail");
        assert!(err.to_string().contains(expected), "{action}: {err}");
    }
}

#[test]
fn validate_action_accepts_keyboard_boundaries_and_optional_capture_path() {
    let c = client(ComputerUseConfig::default(), vec![]);

    let max_text = serde_json::json!({"text": "x".repeat(4096)});
    assert!(c.validate_action("key_type", max_text.as_object().unwrap()).is_ok());

    let max_key = serde_json::json!({"key": "A".repeat(32)});
    assert!(c.validate_action("key_press", max_key.as_object().unwrap()).is_ok());

    assert!(c.validate_action("screen_capture", &serde_json::Map::new()).is_ok());
}

#[test]
fn validate_action_applies_utf8_byte_limit_and_rejects_non_string_capture_path() {
    let c = client(ComputerUseConfig::default(), vec![]);
    let multibyte_text = serde_json::json!({"text": "界".repeat(1366)});
    let error = c
        .validate_action("key_type", multibyte_text.as_object().unwrap())
        .expect_err("UTF-8 input exceeding 4096 bytes should be rejected");
    assert!(error.to_string().contains("exceeds maximum length"));

    let non_string_path = serde_json::json!({"path": 7});
    let error = c
        .validate_action("screen_capture", non_string_path.as_object().unwrap())
        .expect_err("non-string capture path should be rejected");
    assert!(error.to_string().contains("must be a string"));
}

#[test]
fn validate_action_rejects_missing_or_wrong_typed_keyboard_parameters() {
    let c = client(ComputerUseConfig::default(), vec![]);

    for (action, params, expected) in [
        ("key_type", serde_json::json!({}), "Missing 'text'"),
        ("key_type", serde_json::json!({"text": 7}), "Missing 'text'"),
        ("key_press", serde_json::json!({}), "Missing 'key'"),
        ("key_press", serde_json::json!({"key": false}), "Missing 'key'"),
    ] {
        let err = c
            .validate_action(action, params.as_object().unwrap())
            .expect_err("missing or wrong-typed keyboard parameter should fail");
        assert!(err.to_string().contains(expected), "{action}: {err}");
    }
}
