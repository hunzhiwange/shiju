use super::mcp_oauth_discovery::bind;
use serde_json::json;
fn resource() -> serde_json::Value {
    json!({"resource":"https://mcp.example/mcp", "authorization_servers":["https://auth.example/tenant"]})
}
fn authority() -> serde_json::Value {
    json!({"issuer":"https://auth.example/tenant", "authorization_endpoint":"https://auth.example/authorize", "token_endpoint":"https://auth.example/token", "scopes_supported":["read"]})
}
#[test]
fn codex_plugin_compat_t26_binds_discovery_to_server_issuer() {
    let result = bind("https://mcp.example/mcp", resource(), authority()).unwrap();
    assert_eq!(result.issuer, "https://auth.example/tenant");
    assert_eq!(result.scopes_supported.unwrap(), ["read"]);
}
#[test]
fn codex_plugin_compat_t26_rejects_invalid_input() {
    let mut metadata = authority();
    metadata["issuer"] = json!("https://evil.example");
    assert!(bind("https://mcp.example/mcp", resource(), metadata).is_err());
    let mut metadata = authority();
    metadata["token_endpoint"] = json!("https://127.0.0.1/token");
    assert!(bind("https://mcp.example/mcp", resource(), metadata).is_err());
}
#[test]
fn codex_plugin_compat_t26_preserves_isolation() {
    let original = resource();
    assert!(bind("https://other.example/mcp", original.clone(), authority()).is_err());
    assert_eq!(original, resource());
    assert!(bind("https://mcp.example/mcp", original, authority()).is_ok());
}

#[tokio::test]
async fn codex_plugin_compat_t26_rejects_private_discovery_before_network() {
    for url in ["https://127.0.0.1/mcp", "http://public.example/mcp", "https://[::1]/mcp"] {
        assert!(super::mcp_oauth_discovery::discover(url).await.is_err());
    }
}

#[tokio::test]
async fn codex_plugin_compat_t26_fetches_resource_then_exact_issuer_metadata() {
    let mut requests = Vec::new();
    let result = super::mcp_oauth_discovery::discover_with("https://mcp.example/mcp", |url| {
        requests.push(url.to_string());
        std::future::ready(Ok(if requests.len() == 1 { resource() } else { authority() }))
    })
    .await
    .unwrap();
    assert_eq!(
        requests,
        [
            "https://mcp.example/.well-known/oauth-protected-resource/mcp",
            "https://auth.example/.well-known/oauth-authorization-server/tenant"
        ]
    );
    assert_eq!(result.resource, "https://mcp.example/mcp");
    assert!(result.client_id.is_none());
}

#[tokio::test]
async fn codex_plugin_compat_t26_resource_mismatch_stops_before_issuer_request() {
    let mut requests = Vec::new();
    let result = super::mcp_oauth_discovery::discover_with("https://other.example/mcp", |url| {
        requests.push(url);
        std::future::ready(Ok(resource()))
    })
    .await;
    assert!(result.unwrap_err().to_string().contains("resource mismatch"));
    assert_eq!(requests.len(), 1);
}

#[tokio::test]
async fn codex_plugin_compat_t26_host_entry_revalidates_package_identity() {
    use super::t17_tests::{options, plugin};
    use agent_client_protocol::McpServer;
    let temp = tempfile::tempdir().unwrap();
    let options = options(temp.path());
    let root = plugin(
        &options,
        "discovery",
        json!({"url":"https://mcp.example/mcp", "oauth":{"client_id":"public-fixture"}}),
    );
    let original = std::fs::read(root.join(".codex-plugin/plugin.json")).unwrap();
    let mut servers = Vec::new();
    crate::plugins::mcp::merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    let McpServer::Http(server) = &mut servers[0] else { panic!("HTTP fixture") };
    server.url = "https://other.example/mcp".into();
    let error = crate::plugins::discover_plugin_mcp_oauth(server).await.unwrap_err();
    assert!(error.to_string().contains("endpoint or identity changed"));
    assert_eq!(std::fs::read(root.join(".codex-plugin/plugin.json")).unwrap(), original);
}
