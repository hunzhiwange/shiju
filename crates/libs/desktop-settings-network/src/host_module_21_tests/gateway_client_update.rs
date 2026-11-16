use super::*;
fn app() -> NetworkFixture {
    NetworkFixture::default()
}
fn update(app: &mut NetworkFixture, message: Message) {
    if let crate::message::gateway_client::Effect::Save { .. } =
        crate::message::gateway_client::update(&mut app.gateway_client_settings, message)
    {
        crate::message::gateway_client::save_completed(&mut app.gateway_client_settings);
    }
}
#[test]
fn gateway_client_tenant_messages_persist_switch_and_reject_blank_inputs() {
    let mut app = app();

    let _ = update(
        &mut app,
        SettingsMessage::GatewayClient(GatewayClientMessage::PlatformIdChanged(
            "20000".to_string(),
        )),
    );
    let _ = update(
        &mut app,
        SettingsMessage::GatewayClient(GatewayClientMessage::CompanyIdChanged(
            "200000001".to_string(),
        )),
    );
    assert_eq!(app.gateway_client_settings.servers[0].platform_id, "20000");
    assert_eq!(app.gateway_client_settings.servers[0].company_id, "200000001");

    let _ = update(&mut app, SettingsMessage::GatewayClient(GatewayClientMessage::AddServer));
    assert_eq!(app.gateway_client_settings.platform_id_input, "10000");
    assert_eq!(app.gateway_client_settings.company_id_input, "100000000");

    let original_server_id = app.gateway_client_settings.servers[0].id.clone();
    let _ = update(
        &mut app,
        SettingsMessage::GatewayClient(GatewayClientMessage::SelectServer(original_server_id)),
    );
    assert_eq!(app.gateway_client_settings.platform_id_input, "20000");
    assert_eq!(app.gateway_client_settings.company_id_input, "200000001");

    let _ = update(
        &mut app,
        SettingsMessage::GatewayClient(GatewayClientMessage::PlatformIdChanged(" ".to_string())),
    );
    assert_eq!(app.gateway_client_settings.servers[0].platform_id, "20000");
    assert!(app.gateway_client_settings.save_error.is_some());
    assert!(app.gateway_client_settings.save_error.is_some());
}

#[test]
fn gateway_client_update_adds_selects_and_removes_servers() {
    let mut app = app();
    let initial_len = app.gateway_client_settings.servers.len();
    let original = app.gateway_client_settings.selected_server_id.clone();

    let _ = update(&mut app, SettingsMessage::GatewayClient(GatewayClientMessage::AddServer));
    assert_eq!(app.gateway_client_settings.servers.len(), initial_len + 1);
    assert_ne!(app.gateway_client_settings.selected_server_id, original);

    let new_id = app.gateway_client_settings.selected_server_id.clone();
    let _ = update(
        &mut app,
        SettingsMessage::GatewayClient(GatewayClientMessage::RemoveServerRequested(new_id.clone())),
    );
    assert_eq!(
        app.gateway_client_settings.pending_remove_server_id.as_deref(),
        Some(new_id.as_str())
    );
    let _ = update(
        &mut app,
        SettingsMessage::GatewayClient(GatewayClientMessage::RemoveServerCanceled),
    );
    assert!(app.gateway_client_settings.pending_remove_server_id.is_none());

    let _ = update(
        &mut app,
        SettingsMessage::GatewayClient(GatewayClientMessage::RemoveServerRequested(new_id.clone())),
    );
    let _ = update(
        &mut app,
        SettingsMessage::GatewayClient(GatewayClientMessage::RemoveServerConfirmed(new_id)),
    );
    assert_eq!(app.gateway_client_settings.servers.len(), initial_len);

    let _ = update(&mut app, SettingsMessage::GatewayClient(GatewayClientMessage::HelpOpen));
    assert!(app.gateway_client_settings.show_help_modal);
    let _ = update(&mut app, SettingsMessage::GatewayClient(GatewayClientMessage::HelpClose));
    assert!(!app.gateway_client_settings.show_help_modal);
}

#[test]
fn gateway_client_update_ignores_other_settings_messages_and_unknown_server() {
    let mut app = app();
    let selected = app.gateway_client_settings.selected_server_id.clone();
    let original_name = app.gateway_client_settings.name_input.clone();

    let _ = update(&mut app, SettingsMessage::None);
    assert_eq!(app.gateway_client_settings.name_input, original_name);

    let _ = update(
        &mut app,
        SettingsMessage::GatewayClient(GatewayClientMessage::SelectServer("missing".to_string())),
    );
    assert_eq!(app.gateway_client_settings.selected_server_id, selected);
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn gateway_client_update_normalizes_and_persists_editable_fields() {
    let mut app = app();

    let _ = update(
        &mut app,
        SettingsMessage::GatewayClient(GatewayClientMessage::NameChanged(
            "  primary  ".to_string(),
        )),
    );
    let _ = update(
        &mut app,
        SettingsMessage::GatewayClient(GatewayClientMessage::HostChanged("  ".to_string())),
    );
    let _ = update(&mut app, SettingsMessage::GatewayClient(GatewayClientMessage::PortChanged(0)));
    let _ = update(
        &mut app,
        SettingsMessage::GatewayClient(GatewayClientMessage::SkeyChanged(
            "  test-skey  ".to_string(),
        )),
    );

    let server = &app.gateway_client_settings.servers[0];
    assert_eq!(server.name, "primary");
    assert_eq!(server.host, "127.0.0.1");
    assert_eq!(server.port, 1);
    assert_eq!(server.skey, "test-skey");
    assert_eq!(app.gateway_client_settings.name_input, "primary");
    assert_eq!(app.gateway_client_settings.host_input, "127.0.0.1");
    assert_eq!(app.gateway_client_settings.port, 1);
    assert_eq!(app.gateway_client_settings.skey_input, "test-skey");
    assert!(app.gateway_client_settings.save_error.is_none());
}

#[test]
fn gateway_client_update_rejects_add_and_switch_when_current_tenant_is_invalid() {
    let mut app = app();
    let original_id = app.gateway_client_settings.selected_server_id.clone();
    let original_len = app.gateway_client_settings.servers.len();
    app.gateway_client_settings.company_id_input = " ".to_string();

    let _ = update(&mut app, SettingsMessage::GatewayClient(GatewayClientMessage::AddServer));
    assert_eq!(app.gateway_client_settings.servers.len(), original_len);
    assert_eq!(app.gateway_client_settings.selected_server_id, original_id);
    assert!(app.gateway_client_settings.save_error.is_some());

    app.gateway_client_settings.servers.push(GatewayClientServerDraft {
        id: "remote".to_string(),
        name: "Remote".to_string(),
        host: "gateway.example".to_string(),
        port: 443,
        platform_id: "20000".to_string(),
        company_id: "200000001".to_string(),
        skey: String::new(),
    });
    let _ = update(
        &mut app,
        SettingsMessage::GatewayClient(GatewayClientMessage::SelectServer("remote".to_string())),
    );
    assert_eq!(app.gateway_client_settings.selected_server_id, original_id);
}

#[test]
fn gateway_client_update_requires_valid_remove_request_and_confirmation() {
    let mut app = app();
    app.gateway_client_settings.servers.truncate(1);
    let only_id = app.gateway_client_settings.selected_server_id.clone();

    let _ = update(
        &mut app,
        SettingsMessage::GatewayClient(GatewayClientMessage::RemoveServerRequested(only_id)),
    );
    assert!(app.gateway_client_settings.pending_remove_server_id.is_none());

    let _ = update(&mut app, SettingsMessage::GatewayClient(GatewayClientMessage::AddServer));
    let new_id = app.gateway_client_settings.selected_server_id.clone();
    let _ = update(
        &mut app,
        SettingsMessage::GatewayClient(GatewayClientMessage::RemoveServerRequested(new_id)),
    );
    let _ = update(
        &mut app,
        SettingsMessage::GatewayClient(GatewayClientMessage::RemoveServerConfirmed(
            "different".to_string(),
        )),
    );
    assert_eq!(app.gateway_client_settings.servers.len(), 2);
    assert!(app.gateway_client_settings.pending_remove_server_id.is_none());
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
#[test]
fn gateway_client_update_removes_selected_server_health_and_selects_first() {
    let mut app = app();
    let initial_len = app.gateway_client_settings.servers.len();
    let first_id = app.gateway_client_settings.selected_server_id.clone();
    let _ = update(&mut app, SettingsMessage::GatewayClient(GatewayClientMessage::AddServer));
    let removed_id = app.gateway_client_settings.selected_server_id.clone();
    let removed = app.gateway_client_settings.servers.last().unwrap().clone();
    let health_key = crate::server_health_key(&removed).unwrap();
    app.gateway_client_settings.health.insert(health_key.clone(), true);

    let _ = update(
        &mut app,
        SettingsMessage::GatewayClient(GatewayClientMessage::RemoveServerRequested(
            removed_id.clone(),
        )),
    );
    let _ = update(
        &mut app,
        SettingsMessage::GatewayClient(GatewayClientMessage::RemoveServerConfirmed(removed_id)),
    );

    assert_eq!(app.gateway_client_settings.servers.len(), initial_len);
    assert_eq!(app.gateway_client_settings.selected_server_id, first_id);
    assert!(!app.gateway_client_settings.health.contains_key(&health_key));
    assert_eq!(app.gateway_client_settings.name_input, app.gateway_client_settings.servers[0].name);
}
