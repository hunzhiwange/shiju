use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::thread;

use vw_gateway_client::{GatewayClient, GatewayEndpoint};

use super::gateway::{GatewaySessionSeed, GatewayUiRuntime};
use super::hooks::{load_hook_audit_blocking, load_hook_catalog_blocking};

#[test]
fn hook_catalog_and_audit_render_safe_fields() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind hooks fixture");
    let port = listener.local_addr().expect("hooks fixture address").port();
    let server = thread::spawn(move || {
        let (mut catalog_stream, _) = listener.accept().expect("accept hook catalog request");
        let catalog_path = read_request_path(&mut catalog_stream);
        write_json_response(
            &mut catalog_stream,
            r#"[
                {
                    "plugin_id":"safe-plugin",
                    "event":"session.start",
                    "command":"/bin/sh token=raw-command-secret",
                    "args":["--authorization=raw-argument-secret"],
                    "working_dir":"/private/raw-working-directory",
                    "mode":"blocking",
                    "hook_hash":"abc123",
                    "trusted":false
                },
                {
                    "plugin_id":"",
                    "event":"session.end",
                    "command":"ignored",
                    "args":[],
                    "working_dir":"ignored",
                    "mode":"blocking",
                    "hook_hash":"",
                    "trusted":true
                }
            ]"#,
        );

        let (mut audit_stream, _) = listener.accept().expect("accept hook audit request");
        let audit_path = read_request_path(&mut audit_stream);
        write_json_response(
            &mut audit_stream,
            r#"[
                {
                    "timestamp":"2026-08-24T10:00:00Z",
                    "tool_name":"shell",
                    "duration_ms":42,
                    "success":true,
                    "environment":"TOKEN=raw-environment-secret",
                    "command":"raw-audit-command",
                    "authorization":"Bearer raw-auth-secret"
                },
                {"timestamp":"", "tool_name":"", "duration_ms":0, "success":false}
            ]"#,
        );
        (catalog_path, audit_path)
    });
    let runtime = runtime(GatewayEndpoint::new("127.0.0.1", port));

    let catalog = load_hook_catalog_blocking(&runtime).expect("hook catalog should load");
    let catalog_items = catalog.into_selection_items();
    let catalog_rendered = catalog_items
        .iter()
        .flat_map(|item| {
            [
                item.label.as_str(),
                item.description.as_deref().unwrap_or_default(),
                item.search_value.as_deref().unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(catalog_rendered.contains("safe-plugin"));
    assert!(catalog_rendered.contains("session.start"));
    assert!(catalog_rendered.contains("abc123"));
    assert!(catalog_rendered.contains("1 malformed"));
    assert!(!catalog_rendered.contains("raw-command-secret"));
    assert!(!catalog_rendered.contains("raw-argument-secret"));
    assert!(!catalog_rendered.contains("raw-working-directory"));

    let audit = load_hook_audit_blocking(&runtime).expect("hook audit should load");
    let audit_rendered = audit.render_text();
    assert!(audit_rendered.contains("2026-08-24T10:00:00Z"));
    assert!(audit_rendered.contains("shell"));
    assert!(audit_rendered.contains("42 ms"));
    assert!(audit_rendered.contains("1 malformed"));
    for secret in [
        "raw-environment-secret",
        "raw-audit-command",
        "raw-auth-secret",
    ] {
        assert!(!audit_rendered.contains(secret), "audit leaked {secret}");
    }

    assert_eq!(
        server.join().expect("join hooks fixture"),
        ("/v1/hooks".to_string(), "/v1/hooks/audit".to_string())
    );
}

fn runtime(endpoint: GatewayEndpoint) -> GatewayUiRuntime {
    let client = GatewayClient::new(endpoint).expect("test gateway client");
    GatewayUiRuntime::new(client, GatewaySessionSeed::new(PathBuf::from("/workspace")))
}

fn read_request_path(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let read = stream.read(&mut buffer).expect("read hooks request");
        assert!(read > 0, "hooks request closed before headers");
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8_lossy(&bytes)
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .expect("hooks request path")
        .to_string()
}

fn write_json_response(stream: &mut TcpStream, body: &str) {
    write!(
        stream,
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    )
    .expect("write hooks response");
}
