use super::gateway_health::{check_servers, server_health_key};
use crate::app::state::GatewayClientServerDraft;

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
