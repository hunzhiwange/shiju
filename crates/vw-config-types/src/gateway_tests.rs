//! 验证网关认证、限流、隧道与节点控制配置的默认及兼容行为。
//! 默认值测试关注安全关闭状态，兼容测试保护旧版密钥配置的启用语义。

// 默认监听本机且关闭可选控制面，避免空配置扩大网络或节点权限。
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

// 节点控制字段必须完整保留显式配置，同时隧道仍以禁用提供方为默认。
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

// 旧配置没有 `enabled` 字段，读取时仍应保留既有密钥的可用状态。
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
