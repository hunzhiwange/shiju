use super::gateway_health::{check_servers, server_health_key};
use crate::app::state::GatewayClientServerDraft;
use std::io::{Read, Write};
use std::net::TcpListener;

fn server(host: &str, port: u16) -> GatewayClientServerDraft {
    GatewayClientServerDraft {
        id: "local".to_string(),
        name: "Local".to_string(),
        host: host.to_string(),
        port,
        platform_id: "10000".to_string(),
        company_id: "100000000".to_string(),
        skey: String::new(),
    }
}

#[test]
fn server_health_key_defaults_blank_host_to_loopback() {
    let key = server_health_key(&server("  ", 42617));

    assert_eq!(key.as_deref(), Some("http://127.0.0.1:42617/v1/health"));
}

#[test]
fn server_health_key_adds_scheme_port_and_health_path() {
    let key = server_health_key(&server("gateway.local", 8080));

    assert_eq!(key.as_deref(), Some("http://gateway.local:8080/v1/health"));
}

#[test]
fn server_health_key_preserves_explicit_scheme_and_replaces_path_query() {
    let key = server_health_key(&server("https://gateway.example.test/base?debug=true", 443));

    assert_eq!(key.as_deref(), Some("https://gateway.example.test:443/v1/health"));
}

#[test]
fn server_health_key_clamps_zero_port_to_one() {
    let key = server_health_key(&server("127.0.0.1", 0));

    assert_eq!(key.as_deref(), Some("http://127.0.0.1:1/v1/health"));
}

#[test]
fn server_health_key_returns_none_for_unparseable_host() {
    let key = server_health_key(&server("http://[", 8080));

    assert_eq!(key, None);
}

#[tokio::test(flavor = "multi_thread")]
async fn check_servers_skips_unparseable_hosts_without_network_access() {
    let results = check_servers(vec![server("http://[", 8080)]).await;

    assert!(results.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn check_servers_marks_unreachable_loopback_server_unhealthy() {
    let results = check_servers(vec![server("127.0.0.1", 1)]).await;

    assert_eq!(results, vec![("http://127.0.0.1:1/v1/health".to_string(), false)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn check_servers_omits_tenant_headers() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind health fixture");
    let port = listener.local_addr().expect("read fixture address").port();
    let received_request = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept health request");
        let mut request = Vec::new();
        let mut buffer = [0_u8; 512];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let read = stream.read(&mut buffer).expect("read health request");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
        }
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"status\":\"ok\"}",
            )
            .expect("write health response");
        String::from_utf8(request).expect("health request is utf8")
    });
    let mut gateway = server("127.0.0.1", port);
    gateway.platform_id = "20000".to_string();
    gateway.company_id = "200000001".to_string();

    let results = check_servers(vec![gateway]).await;

    assert_eq!(results, vec![(format!("http://127.0.0.1:{port}/v1/health"), true)]);
    let request = received_request.join().expect("health fixture joins");
    let request = request.to_ascii_lowercase();
    assert!(!request.contains("x-platform-id:"));
    assert!(!request.contains("x-company-id:"));
}
