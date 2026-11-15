use super::McpAppNetworkPolicy;
use serde_json::json;

#[test]
fn mcp_app_network_rejects_invalid_declarations() {
    for origin in [
        "http://host.example.test",
        "https://*.example.test",
        "https://host.example.test:0",
        "https://host.example.test:65536",
        "https://host.example.test/+",
        "https://host.example.test?token=test",
        "https://host.example.test; frame-src https:",
        "https://user@host.example.test",
        "https://host..example.test",
        "https://host.example.test:+443",
        "https://127.1",
    ] {
        assert!(
            McpAppNetworkPolicy::parse(Some(&json!({"connectDomains":[origin]}))).is_err(),
            "{origin}"
        );
    }
    assert!(McpAppNetworkPolicy::parse(Some(&json!({"unsupported":[]}))).is_err());
    assert!(McpAppNetworkPolicy::parse(Some(&json!({"resourceDomains":"https:"}))).is_err());
}
