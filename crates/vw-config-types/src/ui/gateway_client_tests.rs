use super::{GatewayClientServerConfig, GatewayClientSystemSettingsConfig};

// 服务器归一化负责修复空字段和无效活动项，选择逻辑始终产生可用回退。
#[test]
fn normalized_servers_falls_back_repairs_blanks_and_selects_active() {
    let gateway = GatewayClientSystemSettingsConfig {
        host: "gateway.local".to_string(),
        port: 19999,
        platform_id: "10000".to_string(),
        company_id: "100000000".to_string(),
        bearer_token: "token".to_string(),
        username: "user".to_string(),
        password: "pass".to_string(),
        skey: "skey".to_string(),
        active_server_id: String::new(),
        servers: Vec::new(),
    };
    let servers = gateway.normalized_servers();
    assert_eq!(servers[0].id, "local");
    assert_eq!(servers[0].name, "本地网关");
    assert_eq!(servers[0].host, "gateway.local");
    assert_eq!(servers[0].port, 19999);
    assert_eq!(servers[0].bearer_token, "token");

    let gateway = GatewayClientSystemSettingsConfig {
        active_server_id: "remote".to_string(),
        servers: vec![
            GatewayClientServerConfig {
                id: " ".to_string(),
                name: " ".to_string(),
                host: "one".to_string(),
                ..GatewayClientServerConfig::default()
            },
            GatewayClientServerConfig {
                id: "remote".to_string(),
                name: "".to_string(),
                host: "two".to_string(),
                ..GatewayClientServerConfig::default()
            },
        ],
        ..GatewayClientSystemSettingsConfig::default()
    };
    let normalized = gateway.normalized_servers();
    assert_eq!(normalized[0].id, "local");
    assert_eq!(normalized[0].name, "本地网关");
    assert_eq!(normalized[1].name, "网关 2");
    assert_eq!(gateway.active_server().id, "remote");
}

// 设置服务器列表时同步旧版单连接字段，保持新旧消费者读取一致。
#[test]
fn set_servers_syncs_active_connection_fields_and_falls_back() {
    let mut gateway = GatewayClientSystemSettingsConfig::default();
    let local = GatewayClientServerConfig {
        id: "local".to_string(),
        host: "127.0.0.1".to_string(),
        port: 42617,
        ..GatewayClientServerConfig::default()
    };
    let remote = GatewayClientServerConfig {
        id: "remote".to_string(),
        name: "Remote".to_string(),
        host: "remote.example".to_string(),
        port: 443,
        platform_id: "20000".to_string(),
        company_id: "200000001".to_string(),
        bearer_token: "token".to_string(),
        username: "u".to_string(),
        password: "p".to_string(),
        skey: "s".to_string(),
    };

    gateway.set_servers(vec![local.clone(), remote.clone()], "remote".to_string());

    assert_eq!(gateway.active_server_id, "remote");
    assert_eq!(gateway.host, "remote.example");
    assert_eq!(gateway.port, 443);
    assert_eq!(gateway.platform_id, "20000");
    assert_eq!(gateway.company_id, "200000001");
    assert_eq!(gateway.bearer_token, "token");
    assert_eq!(gateway.username, "u");
    assert_eq!(gateway.password, "p");
    assert_eq!(gateway.skey, "s");
    assert_eq!(gateway.servers, vec![local, remote]);

    gateway.set_servers(Vec::new(), "missing".to_string());
    assert_eq!(gateway.active_server_id, "local");
    assert_eq!(gateway.servers, vec![GatewayClientServerConfig::default()]);
}

#[test]
fn gateway_client_server_config_default() {
    let server = GatewayClientServerConfig::default();
    assert_eq!(server.id, "local");
    assert_eq!(server.name, "本地网关");
    assert_eq!(server.host, "127.0.0.1");
    assert_eq!(server.port, config_gateway::DEFAULT_GATEWAY_PORT);
    assert_eq!(server.platform_id, "10000");
    assert_eq!(server.company_id, "100000000");
    assert!(server.bearer_token.is_empty());
    assert!(server.username.is_empty());
    assert!(server.password.is_empty());
    assert!(server.skey.is_empty());
}

#[test]
fn gateway_client_system_settings_config_default() {
    let settings = GatewayClientSystemSettingsConfig::default();
    assert_eq!(settings.host, "127.0.0.1");
    assert_eq!(settings.port, config_gateway::DEFAULT_GATEWAY_PORT);
    assert_eq!(settings.platform_id, "10000");
    assert_eq!(settings.company_id, "100000000");
    assert!(settings.bearer_token.is_empty());
    assert_eq!(settings.active_server_id, "local");
    assert_eq!(settings.servers, vec![GatewayClientServerConfig::default()]);
}

// 租户字段空白会破坏网关路由，因此反序列化边界直接拒绝。
#[test]
fn gateway_client_tenant_fields_default_serialize_and_reject_blanks() {
    let server: GatewayClientServerConfig = serde_json::from_value(serde_json::json!({
        "id": "legacy"
    }))
    .unwrap();
    assert_eq!(server.platform_id, "10000");
    assert_eq!(server.company_id, "100000000");

    let json = serde_json::to_value(&server).unwrap();
    assert_eq!(json["platform_id"], "10000");
    assert_eq!(json["company_id"], "100000000");

    assert!(
        serde_json::from_value::<GatewayClientServerConfig>(serde_json::json!({
            "platform_id": " ",
            "company_id": "100000000"
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<GatewayClientSystemSettingsConfig>(serde_json::json!({
            "platform_id": "10000",
            "company_id": "\t"
        }))
        .is_err()
    );
}

#[test]
fn gateway_client_tenant_fields_preserve_explicit_non_empty_values() {
    let server: GatewayClientServerConfig = serde_json::from_value(serde_json::json!({
        "platform_id": " 20000 ",
        "company_id": "200000001"
    }))
    .expect("non-empty tenant ids deserialize");

    assert_eq!(server.platform_id, " 20000 ");
    assert_eq!(server.company_id, "200000001");
}

// 活动服务器查找优先显式标识，失配时回退首项且不制造虚假连接。
#[test]
fn active_server_finds_by_id() {
    let gateway = GatewayClientSystemSettingsConfig {
        active_server_id: "remote".to_string(),
        servers: vec![
            GatewayClientServerConfig {
                id: "local".to_string(),
                host: "127.0.0.1".to_string(),
                ..GatewayClientServerConfig::default()
            },
            GatewayClientServerConfig {
                id: "remote".to_string(),
                host: "remote.example".to_string(),
                ..GatewayClientServerConfig::default()
            },
        ],
        ..GatewayClientSystemSettingsConfig::default()
    };
    let active = gateway.active_server();
    assert_eq!(active.id, "remote");
    assert_eq!(active.host, "remote.example");
}

#[test]
fn active_server_falls_back_to_first_when_id_not_found() {
    let gateway = GatewayClientSystemSettingsConfig {
        active_server_id: "nonexistent".to_string(),
        servers: vec![GatewayClientServerConfig {
            id: "default".to_string(),
            host: "10.0.0.1".to_string(),
            ..GatewayClientServerConfig::default()
        }],
        ..GatewayClientSystemSettingsConfig::default()
    };
    let active = gateway.active_server();
    assert_eq!(active.id, "default");
    assert_eq!(active.host, "10.0.0.1");
}

#[test]
fn set_servers_preserves_existing_when_no_match() {
    let mut gateway = GatewayClientSystemSettingsConfig::default();
    let server = GatewayClientServerConfig {
        id: "alpha".to_string(),
        host: "alpha.local".to_string(),
        ..GatewayClientServerConfig::default()
    };
    gateway.set_servers(vec![server.clone()], "beta".to_string());
    assert_eq!(gateway.active_server_id, "alpha");
    assert_eq!(gateway.host, "alpha.local");
    assert_eq!(gateway.servers, vec![server]);
}
