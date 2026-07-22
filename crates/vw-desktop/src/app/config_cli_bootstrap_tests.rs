use super::*;

#[cfg(unix)]
fn write_test_cli(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    fs::write(
        path,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  printf '%s\n' 'vibewindow test'
  exit 0
fi
case "$*" in
  *fail-stderr*) printf '%s\n' 'stderr failure' >&2; exit 7 ;;
  *fail-stdout*) printf '%s\n' 'stdout failure'; exit 8 ;;
  *) printf '%s\n' "ok:$*"; exit 0 ;;
esac
"#,
    )
    .expect("write test cli");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("chmod test cli");
}

#[test]
fn gateway_runtime_endpoint_normalizes_blank_host_and_zero_port() {
    let endpoint = GatewayRuntimeEndpoint::new("  ", 0);

    assert_eq!(endpoint.host, "127.0.0.1");
    assert_eq!(endpoint.port, 1);
    assert_eq!(endpoint.connect_host(), "127.0.0.1");
    assert_eq!(endpoint.describe(), "127.0.0.1:1");
}

#[test]
fn gateway_runtime_endpoint_maps_unspecified_hosts_for_local_connections() {
    for (host, expected) in [
        ("0.0.0.0", "127.0.0.1"),
        ("::", "::1"),
        ("[::]", "::1"),
        ("[::1]", "::1"),
        ("localhost", "localhost"),
    ] {
        let endpoint = GatewayRuntimeEndpoint::new(host, 42617);
        assert_eq!(endpoint.connect_host(), expected, "host={host}");
    }
}

#[test]
fn local_gateway_host_recognizes_normalized_loopback_forms_only() {
    for host in ["", " 127.0.0.1 ", "LOCALHOST", "[::1]", "0.0.0.0", "[::]"] {
        assert!(is_local_gateway_host(host), "host={host}");
    }
    for host in ["127.0.0.2", "gateway.internal", "localhost.example.com"] {
        assert!(!is_local_gateway_host(host), "host={host}");
    }
}

#[test]
fn format_cli_command_keeps_argument_order_and_empty_argument_list() {
    let cli = Path::new("/opt/Vibe Window/vibewindow");

    assert_eq!(format_cli_command(cli, &[]), "/opt/Vibe Window/vibewindow");
    assert_eq!(
        format_cli_command(cli, &["service", "install"]),
        "/opt/Vibe Window/vibewindow service install",
    );
}

#[test]
fn same_file_contents_distinguishes_equal_length_and_different_length_files() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let left = tmp.path().join("left");
    let right = tmp.path().join("right");
    fs::write(&left, b"same").expect("write left");
    fs::write(&right, b"same").expect("write right");

    assert!(same_file_contents(&left, &right).expect("compare equal files"));

    fs::write(&right, b"diff").expect("rewrite right with equal length");
    assert!(!same_file_contents(&left, &right).expect("compare unequal bytes"));

    fs::write(&right, b"longer").expect("rewrite right with different length");
    assert!(!same_file_contents(&left, &right).expect("compare unequal lengths"));
}

#[test]
fn same_file_contents_reports_missing_metadata_path() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let missing = tmp.path().join("missing");
    let present = tmp.path().join("present");
    fs::write(&present, b"content").expect("write present file");

    let error = same_file_contents(&missing, &present).expect_err("missing file should fail");

    assert!(error.contains("读取文件元数据失败"));
    assert!(error.contains(missing.to_string_lossy().as_ref()));
}

#[test]
fn ensure_profile_contains_path_creates_parent_and_is_idempotent() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let profile = tmp.path().join("nested").join("profile");
    let install_dir = tmp.path().join("bin");

    assert!(ensure_profile_contains_path(&profile, &install_dir).expect("first profile update"));
    assert!(profile.exists());
    assert!(!ensure_profile_contains_path(&profile, &install_dir).expect("second profile update"));

    let content = fs::read_to_string(&profile).expect("read profile");
    assert_eq!(content.matches(install_dir.to_string_lossy().as_ref()).count(), 1);
    assert!(content.ends_with('\n'));
}

#[test]
fn ensure_profile_contains_path_adds_separator_after_existing_content() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let profile = tmp.path().join("profile");
    let install_dir = tmp.path().join("bin");
    fs::write(&profile, "existing setting").expect("write existing profile");

    assert!(ensure_profile_contains_path(&profile, &install_dir).expect("profile update"));

    let content = fs::read_to_string(&profile).expect("read profile");
    assert!(content.starts_with("existing setting\n"));
    assert!(content.ends_with('\n'));
}

#[test]
fn ensure_profile_contains_path_accepts_existing_install_path_reference() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let profile = tmp.path().join("profile");
    let install_dir = tmp.path().join("bin");
    let existing = format!("tool_path={}\n", install_dir.display());
    fs::write(&profile, &existing).expect("write existing profile");

    assert!(!ensure_profile_contains_path(&profile, &install_dir).expect("existing path check"));
    assert_eq!(fs::read_to_string(&profile).expect("read profile"), existing);
}

#[test]
fn resolve_gateway_socket_address_accepts_numeric_loopback() {
    let endpoint = GatewayRuntimeEndpoint::new("127.0.0.1", 42617);
    let address = resolve_gateway_socket_address(&endpoint).expect("numeric address");

    assert_eq!(address.ip().to_string(), "127.0.0.1");
    assert_eq!(address.port(), 42617);
}

#[test]
fn resolve_gateway_socket_address_rejects_malformed_host_text() {
    let endpoint = GatewayRuntimeEndpoint::new("invalid\0host", 42617);

    assert_eq!(resolve_gateway_socket_address(&endpoint), None);
}

#[test]
fn cli_binary_name_matches_current_platform() {
    let expected = if cfg!(windows) { "vibewindow.exe" } else { "vibewindow" };
    assert_eq!(cli_binary_name(), expected);
}

#[cfg(unix)]
#[test]
fn cli_command_helpers_return_trimmed_stdout_and_prefer_stderr_failures() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let cli = tmp.path().join("vibewindow");
    let config_dir = tmp.path().join("config");
    fs::create_dir_all(&config_dir).expect("config dir");
    write_test_cli(&cli);

    assert_eq!(cli_version(&cli).as_deref(), Ok("vibewindow test"));
    assert_eq!(
        run_cli_command(&cli, &config_dir, &["service", "install"]).expect("success"),
        format!("ok:--config-dir {} service install", config_dir.display())
    );

    let stderr_error =
        run_cli_command(&cli, &config_dir, &["fail-stderr"]).expect_err("stderr failure");
    assert!(stderr_error.contains("fail-stderr"));
    assert!(stderr_error.contains("stderr failure"));

    let stdout_error =
        run_cli_command(&cli, &config_dir, &["fail-stdout"]).expect_err("stdout failure");
    assert!(stdout_error.contains("stdout failure"));
}

#[test]
fn load_gateway_runtime_endpoint_prefers_local_server_and_defaults_missing_port() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let config_dir = vw_config_types::paths::home_config_dir(tmp.path());
    fs::create_dir_all(&config_dir).expect("config dir");
    let config = GatewayClientSystemSettingsConfig {
        host: "gateway.internal".to_string(),
        port: 5000,
        servers: vec![
            GatewayClientServerConfig {
                id: "remote".to_string(),
                host: "gateway.internal".to_string(),
                port: 5000,
                ..GatewayClientServerConfig::default()
            },
            GatewayClientServerConfig {
                id: "local".to_string(),
                host: "::".to_string(),
                port: 0,
                platform_id: "20000".to_string(),
                company_id: "200000001".to_string(),
                ..GatewayClientServerConfig::default()
            },
        ],
        ..GatewayClientSystemSettingsConfig::default()
    };
    fs::write(
        config_dir.join("gateway-client-bootstrap.json"),
        serde_json::to_string(&config).expect("serialize config"),
    )
    .expect("write config");

    let endpoint = load_gateway_runtime_endpoint(tmp.path()).expect("endpoint");

    assert_eq!(endpoint.host, "::");
    assert_eq!(endpoint.connect_host(), "::1");
    assert_eq!(endpoint.port, DEFAULT_GATEWAY_PORT);
    assert_eq!(endpoint.tenant_scope.platform_id().as_ref(), "20000");
    assert_eq!(endpoint.tenant_scope.company_id().as_ref(), "200000001");
}

#[test]
fn load_gateway_runtime_endpoint_uses_defaults_for_missing_or_invalid_cache() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let endpoint = load_gateway_runtime_endpoint(tmp.path()).expect("default endpoint");
    assert_eq!(endpoint.host, "127.0.0.1");
    assert_eq!(endpoint.port, DEFAULT_GATEWAY_PORT);

    let config_dir = vw_config_types::paths::home_config_dir(tmp.path());
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::write(config_dir.join("gateway-client-bootstrap.json"), "{bad json").expect("bad config");
    let invalid_endpoint = load_gateway_runtime_endpoint(tmp.path()).expect("invalid endpoint");
    assert_eq!(invalid_endpoint.host, "127.0.0.1");
    assert_eq!(invalid_endpoint.port, DEFAULT_GATEWAY_PORT);
}

#[test]
fn gateway_health_ready_rejects_non_ok_http_responses() {
    use std::net::TcpListener;

    for response in [
        b"HTTP/1.1 500 ERROR\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"status\":\"ok\"}"
            .as_slice(),
        b"HTTP/1.1 200 OK\r\nContent-Length: 18\r\nConnection: close\r\n\r\n{\"status\":\"bad\"}"
            .as_slice(),
        b"\r\n".as_slice(),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind health stub");
        let port = listener.local_addr().expect("address").port();
        let response = response.to_vec();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept health probe");
            let _ = stream.write_all(&response);
        });
        let endpoint = GatewayRuntimeEndpoint::new("127.0.0.1", port);

        assert!(!gateway_health_ready(&endpoint));
        server.join().expect("health stub should finish");
    }
}

#[test]
fn wait_for_gateway_health_respects_zero_timeout() {
    let endpoint = GatewayRuntimeEndpoint::new("127.0.0.1", 1);

    assert!(!wait_for_gateway_health(&endpoint, Duration::ZERO));
}
