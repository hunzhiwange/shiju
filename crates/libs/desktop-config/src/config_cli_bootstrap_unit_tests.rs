
    use super::*;

    #[cfg(unix)]
    fn write_test_cli(path: &Path, version: &str, marker: &str) {
        use std::os::unix::fs::PermissionsExt;

        fs::write(path, format!("#!/bin/sh\n# {marker}\nprintf '%s\\n' '{version}'\n")).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn installed_cli_matches_source_requires_same_version() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("target-cli");
        let source = tmp.path().join("source-cli");
        write_test_cli(&target, "vibewindow 1.0.0", "same-build");
        write_test_cli(&source, "vibewindow 1.0.0", "same-build");

        assert!(installed_cli_matches_source(&target, &source));

        write_test_cli(&target, "vibewindow 1.0.0", "old-build");

        assert!(!installed_cli_matches_source(&target, &source));

        write_test_cli(&target, "vibewindow 0.9.0", "same-build");

        assert!(!installed_cli_matches_source(&target, &source));
    }

    #[test]
    fn load_gateway_runtime_endpoint_reads_bootstrap_cache() {
        let tmp = tempfile::tempdir().unwrap();
        let config_dir = config_runtime_paths::home_config_dir(tmp.path());
        fs::create_dir_all(&config_dir).unwrap();
        let config = GatewayClientSystemSettingsConfig {
            host: "0.0.0.0".to_string(),
            port: 14682,
            servers: Vec::new(),
            ..GatewayClientSystemSettingsConfig::default()
        };
        fs::write(
            config_dir.join("gateway-client-bootstrap.json"),
            serde_json::to_string(&config).unwrap(),
        )
        .unwrap();

        let endpoint = load_gateway_runtime_endpoint(tmp.path()).unwrap();

        assert_eq!(endpoint.host, "0.0.0.0");
        assert_eq!(endpoint.connect_host(), "127.0.0.1");
        assert_eq!(endpoint.port, 14682);
    }

    #[test]
    fn refresh_bootstrap_keeps_remote_gateway_client_server() {
        let active = GatewayClientServerConfig {
            host: "gateway.internal".to_string(),
            port: 53607,
            ..GatewayClientServerConfig::default()
        };
        let endpoint = GatewayRuntimeEndpoint::new("127.0.0.1", 42617);

        assert!(!should_refresh_gateway_client_bootstrap(&active, &endpoint));
    }

    #[test]
    fn refresh_bootstrap_replaces_unhealthy_loopback_server() {
        let active = GatewayClientServerConfig {
            host: "127.0.0.1".to_string(),
            port: 1,
            ..GatewayClientServerConfig::default()
        };
        let endpoint = GatewayRuntimeEndpoint::new("127.0.0.1", 42617);

        assert!(should_refresh_gateway_client_bootstrap(&active, &endpoint));
    }

    #[test]
    fn gateway_health_probe_omits_tenant_headers() {
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut buffer = [0_u8; 512];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let read = stream.read(&mut buffer).unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
            }
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"status\":\"ok\"}",
                )
                .unwrap();
            String::from_utf8(request).unwrap()
        });
        let endpoint = GatewayRuntimeEndpoint::new("127.0.0.1", port).with_tenant_scope(
            vw_api_transport::TenantScope::new("20000", "200000001").unwrap(),
        );

        assert!(gateway_health_ready(&endpoint));
        let request = server.join().unwrap();
        assert!(!request.contains("X-Platform-ID:"));
        assert!(!request.contains("X-Company-ID:"));
    }
