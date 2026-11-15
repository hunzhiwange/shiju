// 验证网关客户端服务器与租户配置的增删选改行为。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
type App = GatewayClientSettingsState;

fn app() -> App { App::default() }

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

    app.name_input = " primary ".to_string();
    app.host_input = "".to_string();
    app.platform_id_input = " 20000 ".to_string();
    app.company_id_input = " 200000001 ".to_string();
    sync_selected_server_from_inputs(&mut app).unwrap();
    assert_eq!(app.servers[0].name, "primary");
    assert_eq!(app.servers[0].host, "127.0.0.1");
    assert_eq!(app.servers[0].platform_id, "20000");
    assert_eq!(app.servers[0].company_id, "200000001");

    app.servers[0].host = "0.0.0.0".to_string();
    load_selected_server_into_inputs(&mut app);
    assert_eq!(app.host_input, "0.0.0.0");
}

#[test]
fn gateway_client_helpers_ignore_missing_selection_and_reject_invalid_tenant() {
    let mut app = app();
    let original = app.servers[0].clone();

    app.selected_server_id = "missing".to_string();
    app.name_input = "unchanged input".to_string();
    app.platform_id_input = " ".to_string();
    assert_eq!(selected_index(&app), None);
    assert_eq!(sync_selected_server_from_inputs(&mut app), Ok(()));
    load_selected_server_into_inputs(&mut app);
    assert_eq!(app.name_input, "unchanged input");

    app.selected_server_id = original.id.clone();
    let error = sync_selected_server_from_inputs(&mut app).unwrap_err();
    assert!(error.contains("租户配置无效"));
    assert_eq!(app.servers[0].name, original.name);
    assert_eq!(app.servers[0].platform_id, original.platform_id);
}

#[test]
fn gateway_client_helpers_clamp_loaded_port_and_choose_next_available_id() {
    let mut app = app();
    app.servers[0].host = " ".to_string();
    app.servers[0].port = 0;
    load_selected_server_into_inputs(&mut app);
    assert_eq!(app.host_input, "127.0.0.1");
    assert_eq!(app.port, 1);

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
