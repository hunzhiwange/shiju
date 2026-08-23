//! `api system` 命令的路由、租户头与输出测试。

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;

use clap::Parser;

use super::{ApiCommands, run_with_writer};
use crate::cli::{Cli, Commands};

#[tokio::test]
async fn api_system_commands_use_gateway_discovery_routes() {
    let cases = [
        ("health", "/v1/health", false),
        ("metrics", "/v1/metrics", true),
        ("status", "/v1/status", false),
        ("tools", "/v1/tools", false),
        ("mcp-inspect", "/v1/mcp/inspect", false),
        ("api-health", "/v1/api-health", false),
        ("global-health", "/v1/global/health", false),
        ("models", "/v1/models", false),
        ("path", "/v1/path?directory=%2Ftmp%2Fworkspace", false),
        ("vcs", "/v1/vcs?directory=%2Ftmp%2Fworkspace", false),
        ("commands", "/v1/command?directory=%2Ftmp%2Fworkspace", false),
        ("agents", "/v1/agent?directory=%2Ftmp%2Fworkspace", false),
        ("skills", "/v1/skill?directory=%2Ftmp%2Fworkspace", false),
        ("cli-tools", "/v1/cli-tools", false),
    ];
    let (port, requests) = spawn_system_server(cases.len());

    for (command, expected_path, is_text) in cases {
        let cli = Cli::try_parse_from([
            "shiju",
            "api",
            "--host",
            "127.0.0.1",
            "--port",
            &port.to_string(),
            "--platform",
            "platform-a",
            "--company",
            "company-b",
            "--directory",
            "/tmp/workspace",
            "system",
            command,
        ])
        .unwrap_or_else(|error| panic!("system {command} should parse: {error}"));
        let Commands::Api { api_args } = cli.command else {
            panic!("expected api command");
        };
        assert!(matches!(api_args.command, ApiCommands::System { .. }));

        let mut output = Vec::new();
        run_with_writer(api_args, &mut output)
            .await
            .unwrap_or_else(|error| panic!("system {command} should succeed: {error}"));

        let request = requests.recv().expect("captured system request");
        assert!(request.starts_with(&format!("GET {expected_path} HTTP/1.1\r\n")));
        let request = request.to_ascii_lowercase();
        assert!(request.contains("\r\nx-platform-id: platform-a\r\n"));
        assert!(request.contains("\r\nx-company-id: company-b\r\n"));

        let output = String::from_utf8(output).expect("utf-8 output");
        if is_text {
            assert_eq!(output, "gateway_requests_total 7\n");
        } else {
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&output).unwrap(),
                serde_json::json!({"ok": true})
            );
        }
    }
}

fn spawn_system_server(request_count: usize) -> (u16, mpsc::Receiver<String>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind test server");
    let port = listener.local_addr().expect("read test server address").port();
    let (request_tx, request_rx) = mpsc::channel();

    std::thread::spawn(move || {
        for _ in 0..request_count {
            let (mut stream, _) = listener.accept().expect("accept system request");
            let mut request = Vec::new();
            let mut chunk = [0_u8; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let read = stream.read(&mut chunk).expect("read system request");
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..read]);
            }
            let request = String::from_utf8(request).expect("request should be UTF-8");
            let is_metrics = request.starts_with("GET /v1/metrics HTTP/1.1\r\n");
            request_tx.send(request).expect("send captured request");

            let (content_type, body): (&str, &[u8]) = if is_metrics {
                ("text/plain", b"gateway_requests_total 7\n")
            } else {
                ("application/json", br#"{"ok":true}"#)
            };
            write!(
                stream,
                "HTTP/1.1 200 OK\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                body.len()
            )
            .expect("write response headers");
            stream.write_all(body).expect("write response body");
        }
    });

    (port, request_rx)
}
