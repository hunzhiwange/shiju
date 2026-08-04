#[test]
fn gateway_defaults_cover_auth_and_rate_limits() {
    let config = super::GatewayConfig::default();
    assert_eq!(config.port, super::DEFAULT_GATEWAY_PORT);
    assert_eq!(config.host, "127.0.0.1");
    assert!(!config.auth_enabled);
    assert!(config.skeys.is_empty());
    assert_eq!(config.webhook_rate_limit_per_minute, 60);
    assert_eq!(config.idempotency_ttl_secs, 300);
    assert!(!config.node_control.enabled);
}

#[test]
fn tunnel_defaults_and_node_control_deserialize() {
    let tunnel = super::TunnelConfig::default();
    assert_eq!(tunnel.provider, "none");

    let parsed: super::GatewayConfig = serde_json::from_value(serde_json::json!({
        "node_control": {
            "enabled": true,
            "auth_token": "secret",
            "allowed_node_ids": ["node-1"]
        }
    }))
    .unwrap();

    assert!(parsed.node_control.enabled);
    assert_eq!(parsed.node_control.auth_token.as_deref(), Some("secret"));
    assert_eq!(parsed.node_control.allowed_node_ids, vec!["node-1"]);
}

#[test]
fn gateway_skey_defaults_to_enabled_for_legacy_configs() {
    let skey: super::GatewaySkey = serde_json::from_value(serde_json::json!({
        "skey_hash": "a".repeat(64),
        "name": "legacy"
    }))
    .unwrap();

    assert!(skey.enabled);
    assert!(skey.masked_skey.is_empty());
}
