// 验证网关客户端服务器与租户配置的增删选改行为。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;

fn app() -> App {
    let mut app = App::new().0;
    app.gateway_client_settings = crate::app::state::GatewayClientSettingsState::default();
    app
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn gateway_client_helpers_normalize_and_select() {
    let mut app = app();

    assert_eq!(normalize_host(""), "127.0.0.1");
    assert_eq!(normalize_host(" gateway.example "), "gateway.example");
    assert_eq!(normalize_name(" ", 1), "网关 2");
    assert_eq!(normalize_name(" primary ", 0), "primary");
    assert!(selected_index(&app).is_some());

    app.gateway_client_settings.name_input = " primary ".to_string();
    app.gateway_client_settings.host_input = "".to_string();
    app.gateway_client_settings.platform_id_input = " 20000 ".to_string();
    app.gateway_client_settings.company_id_input = " 200000001 ".to_string();
    sync_selected_server_from_inputs(&mut app).unwrap();
    assert_eq!(app.gateway_client_settings.servers[0].name, "primary");
    assert_eq!(app.gateway_client_settings.servers[0].host, "127.0.0.1");
    assert_eq!(app.gateway_client_settings.servers[0].platform_id, "20000");
    assert_eq!(app.gateway_client_settings.servers[0].company_id, "200000001");

    app.gateway_client_settings.servers[0].host = "0.0.0.0".to_string();
    load_selected_server_into_inputs(&mut app);
    assert_eq!(app.gateway_client_settings.host_input, "0.0.0.0");
}

#[test]
fn gateway_client_helpers_ignore_missing_selection_and_reject_invalid_tenant() {
    let mut app = app();
    let original = app.gateway_client_settings.servers[0].clone();

    app.gateway_client_settings.selected_server_id = "missing".to_string();
    app.gateway_client_settings.name_input = "unchanged input".to_string();
    app.gateway_client_settings.platform_id_input = " ".to_string();
    assert_eq!(selected_index(&app), None);
    assert_eq!(sync_selected_server_from_inputs(&mut app), Ok(()));
    load_selected_server_into_inputs(&mut app);
    assert_eq!(app.gateway_client_settings.name_input, "unchanged input");

    app.gateway_client_settings.selected_server_id = original.id.clone();
    let error = sync_selected_server_from_inputs(&mut app).unwrap_err();
    assert!(error.contains("租户配置无效"));
    assert_eq!(app.gateway_client_settings.servers[0].name, original.name);
    assert_eq!(app.gateway_client_settings.servers[0].platform_id, original.platform_id);
}

#[test]
fn gateway_client_helpers_clamp_loaded_port_and_choose_next_available_id() {
    let mut app = app();
    app.gateway_client_settings.servers[0].host = " ".to_string();
    app.gateway_client_settings.servers[0].port = 0;
    load_selected_server_into_inputs(&mut app);
    assert_eq!(app.gateway_client_settings.host_input, "127.0.0.1");
    assert_eq!(app.gateway_client_settings.port, 1);

    let mut config = GatewayClientSystemSettingsConfig::default();
    assert_eq!(next_server_id(&config), "gateway-2");
    let mut second = config.servers[0].clone();
    second.id = "gateway-2".to_string();
    config.servers.push(second);
    assert_eq!(next_server_id(&config), "gateway-3");
}

// 行为契约：使用代表性场景固定该分支的可观察结果。
// 断言聚焦输入、状态与返回任务，不复述内部控制流程。
#[test]
fn next_server_id_uses_fallback_after_all_numbered_ids_are_taken() {
    let mut config = GatewayClientSystemSettingsConfig::default();
    let template = config.servers[0].clone();
    let mut servers = Vec::with_capacity(10_000);
    for index in 1..=10_000 {
        let mut server = template.clone();
        server.id = if index == 1 { "local".to_string() } else { format!("gateway-{index}") };
        servers.push(server);
    }
    config.set_servers(servers, "local".to_string());

    assert_eq!(next_server_id(&config), "gateway-new");
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
    assert!(app.active_toast.is_some());
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

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);
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
    let health_key = crate::app::message::gateway_health::server_health_key(&removed).unwrap();
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
