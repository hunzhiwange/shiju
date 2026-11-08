// 验证网关客户端服务器与租户配置的增删选改行为。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;

fn app() -> App {
    #[cfg(not(target_arch = "wasm32"))]
    crate::app::config::desktop_test_gateway::set_gateway_port(1);
    let mut app = App::new().0;
    app.gateway_client_settings = crate::app::state::GatewayClientSettingsState::default();
    app
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn gateway_client_helpers_normalize_and_select() {
    #[cfg(not(target_arch = "wasm32"))]
    let _isolation = crate::app::config::isolated_settings();
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
    #[cfg(not(target_arch = "wasm32"))]
    let _isolation = crate::app::config::isolated_settings();
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
    #[cfg(not(target_arch = "wasm32"))]
    let _isolation = crate::app::config::isolated_settings();
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
    #[cfg(not(target_arch = "wasm32"))]
    let _isolation = crate::app::config::isolated_settings();
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
    #[cfg(not(target_arch = "wasm32"))]
    let _isolation = crate::app::config::isolated_settings();
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
    #[cfg(not(target_arch = "wasm32"))]
    let _isolation = crate::app::config::isolated_settings();
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
    #[cfg(not(target_arch = "wasm32"))]
    let _isolation = crate::app::config::isolated_settings();
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
    #[cfg(not(target_arch = "wasm32"))]
    let _isolation = crate::app::config::isolated_settings();
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
    #[cfg(not(target_arch = "wasm32"))]
    let _isolation = crate::app::config::isolated_settings();
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
    #[cfg(not(target_arch = "wasm32"))]
    let _isolation = crate::app::config::isolated_settings();
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

#[cfg(not(target_arch = "wasm32"))]
mod remote_save {
    use super::*;
    use crate::app::config::{
        SystemSettingsSection, isolated_settings, read_http_request, request_json_body,
    };
    use iced_runtime::{Action, futures::futures::StreamExt, task::into_stream};
    use std::io::Write;
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc;
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    const TIMEOUT: Duration = Duration::from_secs(5);

    struct Gateway {
        port: u16,
        patches: mpsc::Receiver<(String, mpsc::Sender<u16>)>,
        preferences: mpsc::Receiver<String>,
        thread: Option<JoinHandle<()>>,
    }

    impl Gateway {
        fn start() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            let (send_patch, patches) = mpsc::channel();
            let (send_preferences, preferences) = mpsc::channel();
            let thread = thread::spawn(move || {
                thread::scope(|scope| {
                    for stream in listener.incoming() {
                        let mut stream = stream.unwrap();
                        stream.set_read_timeout(Some(TIMEOUT)).unwrap();
                        let request = read_http_request(&mut stream);
                        if request.is_empty() {
                            break;
                        }
                        if request.starts_with("PATCH ") {
                            let (reply, status) = mpsc::channel();
                            send_patch.send((request, reply)).unwrap();
                            scope.spawn(move || {
                                let status = status.recv_timeout(TIMEOUT).unwrap_or(500);
                                respond(&mut stream, status);
                            });
                        } else {
                            if request.starts_with("GET /v1/desktop/project-preferences?") {
                                send_preferences.send(request).unwrap();
                            }
                            respond(&mut stream, 200);
                        }
                    }
                });
            });
            Self { port, patches, preferences, thread: Some(thread) }
        }
    }

    impl Drop for Gateway {
        fn drop(&mut self) {
            drop(TcpStream::connect(("127.0.0.1", self.port)));
            self.thread.take().unwrap().join().unwrap();
        }
    }

    fn respond(stream: &mut TcpStream, status: u16) {
        let body = if status == 500 { "test-token" } else { "{}" };
        let _ = write!(
            stream,
            "HTTP/1.1 {status} Test\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
    }

    fn edit(app: &mut App, message: GatewayClientMessage) -> Task<Message> {
        app.update(Message::Settings(SettingsMessage::GatewayClient(message)))
    }

    fn drive(runtime: &tokio::runtime::Runtime, task: Task<Message>) -> mpsc::Receiver<Message> {
        let (send, output) = mpsc::channel();
        if let Some(mut stream) = into_stream(task) {
            runtime.spawn(async move {
                while let Some(action) = stream.next().await {
                    if let Action::Output(message) = action
                        && send.send(message).is_err()
                    {
                        break;
                    }
                }
            });
        }
        output
    }

    fn receive<T>(output: &mpsc::Receiver<T>) -> T {
        output.recv_timeout(TIMEOUT).expect("expected Gateway request or Iced message")
    }

    fn matching(output: &mpsc::Receiver<Message>, predicate: impl Fn(&Message) -> bool) -> Message {
        loop {
            let message = receive(output);
            if predicate(&message) {
                return message;
            }
        }
    }

    fn save_result(output: &mpsc::Receiver<Message>) -> Message {
        matching(output, |message| {
            matches!(
                message,
                Message::SystemSettingsSaved(_, SystemSettingsSection::GatewayClient, _)
            )
        })
    }

    fn health_results(
        runtime: &tokio::runtime::Runtime,
        app: &mut App,
        tick: Message,
    ) -> Vec<(String, bool)> {
        let output = drive(runtime, app.update(tick));
        let Message::GatewayHealthChecked(results) =
            matching(&output, |message| matches!(message, Message::GatewayHealthChecked(_)))
        else {
            unreachable!("filtered to health check results");
        };
        results
    }

    fn assert_context(request: &str, platform: &str, company: &str, skey: Option<&str>) {
        let (headers, _) = request.split_once("\r\n\r\n").unwrap();
        let header = |key: &str| {
            headers
                .lines()
                .filter_map(|line| line.split_once(':'))
                .find_map(|(name, value)| name.eq_ignore_ascii_case(key).then_some(value.trim()))
        };
        let authorization = skey.map(|skey| format!("Bearer {skey}"));
        assert_eq!(header("authorization"), authorization.as_deref());
        assert_eq!(header("x-platform-id"), Some(platform));
        assert_eq!(header("x-company-id"), Some(company));
    }

    fn assert_patch(request: &str, active: &str, name: &str, port: u16) -> serde_json::Value {
        assert!(request.starts_with("PATCH /v1/global/config "));
        let body = request_json_body(request);
        let settings = body["app_ui"]["system_settings"].as_object().unwrap();
        assert_eq!(settings.len(), 1);
        let config = &settings["gateway_client"];
        assert_eq!(config["active_server_id"], active);
        assert_eq!(config["port"], port);
        let server = config["servers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|server| server["id"] == active)
            .unwrap();
        assert_eq!(server["name"], name);
        assert_eq!(server["port"], port);
        config.clone()
    }

    #[test]
    fn gateway_client_edits_do_not_wait_for_remote_save() {
        let _isolation = isolated_settings();
        let first = Gateway::start();
        let second = Gateway::start();
        crate::app::config::desktop_test_gateway::set_gateway_port(first.port);
        let (mut app, _) = App::new();
        let stale_bootstrap = crate::app::load_gateway_client_config();
        let first_id = app.gateway_client_settings.selected_server_id.clone();
        app.gateway_client_settings.servers.push(GatewayClientServerDraft {
            id: "second".into(),
            name: "Second".into(),
            host: "127.0.0.1".into(),
            port: second.port,
            platform_id: "20000".into(),
            company_id: "200000001".into(),
            skey: "second-token".into(),
        });

        let save = edit(&mut app, GatewayClientMessage::NameChanged("First".into()));
        assert!(
            matches!(first.patches.try_recv(), Err(mpsc::TryRecvError::Empty)),
            "the UI handler sent a PATCH before Iced polled the returned task"
        );
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let first_result = drive(&runtime, save);
        let (request, release_first) = receive(&first.patches);
        assert_patch(&request, &first_id, "First", first.port);
        assert_context(&request, "10000", "100000000", Some("test-token"));
        let idle = app.desktop_preferences_persistence.wait_for_idle().unwrap();

        let interim = edit(&mut app, GatewayClientMessage::NameChanged("Interim".into()));
        assert!(into_stream(interim).is_none());
        let latest = edit(&mut app, GatewayClientMessage::NameChanged("Latest".into()));
        assert!(into_stream(latest).is_none());
        let cleared_key = edit(&mut app, GatewayClientMessage::SkeyChanged(String::new()));
        assert!(matches!(receive(&drive(&runtime, cleared_key)), Message::GatewayHealthTick));

        app.project_path = Some("/gateway-test".into());
        let switch = edit(&mut app, GatewayClientMessage::SelectServer("second".into()));
        assert_eq!(crate::app::load_gateway_client_config().active_server_id, "second");
        let switch_result = drive(&runtime, switch);
        let (request, release_second) = receive(&second.patches);
        let config = assert_patch(&request, "second", "Second", second.port);
        assert_eq!(config["platform_id"], "20000");
        assert_eq!(config["company_id"], "200000001");
        assert_eq!(config["servers"][0]["name"], "Latest");
        assert_context(&request, "20000", "200000001", Some("second-token"));
        assert_context(&receive(&second.preferences), "20000", "200000001", Some("second-token"));

        let tick =
            matching(&switch_result, |message| matches!(message, Message::GatewayHealthTick));
        let health = health_results(&runtime, &mut app, tick);
        assert!(health.contains(&(format!("http://127.0.0.1:{}/v1/health", second.port), true)));
        assert!(!*idle.borrow());
        release_second.send(200).expect("second PATCH must still be pending");
        let saved = save_result(&switch_result);
        assert!(matches!(saved, Message::SystemSettingsSaved(_, _, Ok(()))));
        let _ = app.update(saved);

        let _ = app.update(Message::BootstrapSystemSettings(Ok(
            vw_config_types::ui::AppSystemSettingsConfig {
                gateway_client: stale_bootstrap,
                ..Default::default()
            },
        )));
        assert_eq!(app.gateway_client_settings.selected_server_id, "second");

        release_first.send(500).expect("first PATCH must still be pending");
        let rejected = save_result(&first_result);
        assert!(matches!(&rejected, Message::SystemSettingsSaved(_, _, Err(error))
            if error == "gateway connection sync failed: 500 Internal Server Error"));
        let next = app.update(rejected);
        let next_result = drive(&runtime, next);
        let (request, release_latest) = receive(&first.patches);
        let config = assert_patch(&request, &first_id, "Latest", first.port);
        assert!(config.get("skey").is_some_and(serde_json::Value::is_null));
        assert_context(&request, "10000", "100000000", None);
        assert!(!*idle.borrow());
        release_latest.send(500).unwrap();
        let _ = app.update(save_result(&next_result));
        assert!(app.gateway_client_settings.save_error.is_none(), "old Gateway failure is stale");
        assert!(*idle.borrow(), "quit can proceed after both destinations settle");
        assert!(matches!(first.patches.try_recv(), Err(mpsc::TryRecvError::Empty)));

        let (restarted, _) = App::new();
        assert_eq!(restarted.gateway_client_settings.selected_server_id, "second");
        assert_eq!(restarted.gateway_client_settings.port, second.port);
        assert_eq!(restarted.gateway_client_settings.skey_input, "second-token");
        assert_eq!(restarted.gateway_client_settings.company_id_input, "200000001");
    }

    #[test]
    fn gateway_client_edits_and_switches_when_selected_gateway_is_unreachable() {
        let _isolation = isolated_settings();
        let reachable = Gateway::start();
        crate::app::config::desktop_test_gateway::set_gateway_port(reachable.port);
        let (mut app, _) = App::new();
        let first_id = app.gateway_client_settings.selected_server_id.clone();
        let runtime = tokio::runtime::Runtime::new().unwrap();

        // Reserve and release an ephemeral address instead of relying on a specific unused port.
        let socket = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = socket.local_addr().unwrap().port();
        drop(socket);
        let offline_id = "offline".to_string();
        let mut server = app.gateway_client_settings.servers[0].clone();
        server.id.clone_from(&offline_id);
        server.port = port;
        app.gateway_client_settings.servers.push(server);
        let selected = edit(&mut app, GatewayClientMessage::SelectServer(offline_id.clone()));
        let result = drive(&runtime, selected);
        let first = receive(&result);
        let second = receive(&result);
        let (tick, failure) = if matches!(first, Message::GatewayHealthTick) {
            (first, second)
        } else {
            (second, first)
        };
        assert!(matches!(tick, Message::GatewayHealthTick));
        assert!(matches!(
            failure,
            Message::SystemSettingsSaved(_, SystemSettingsSection::GatewayClient, Err(_))
        ));
        let health = health_results(&runtime, &mut app, tick);
        assert!(health.contains(&(format!("http://127.0.0.1:{port}/v1/health"), false)));
        let _ = app.update(failure);
        assert!(app.gateway_client_settings.save_error.as_ref().unwrap().contains("远端同步失败"));

        let _ = edit(&mut app, GatewayClientMessage::PlatformIdChanged(" ".into()));
        let saved = crate::app::load_gateway_client_config();
        assert_eq!(saved.active_server_id, offline_id);
        assert_eq!(saved.port, port);
        assert_eq!(saved.platform_id, "10000");
        assert!(app.gateway_client_settings.save_error.as_ref().unwrap().contains("租户配置无效"));
        let valid = edit(&mut app, GatewayClientMessage::PlatformIdChanged("30000".into()));
        assert_eq!(crate::app::load_gateway_client_config().platform_id, "30000");
        let pending = drive(&runtime, valid);
        let switch = edit(&mut app, GatewayClientMessage::SelectServer(first_id.clone()));
        let switched = drive(&runtime, switch);
        let (request, release) = receive(&reachable.patches);
        assert_patch(&request, &first_id, "Test", reachable.port);
        release.send(200).unwrap();
        assert_eq!(crate::app::load_gateway_client_config().active_server_id, first_id);
        let _ = app.update(save_result(&pending));
        let _ = app.update(save_result(&switched));
        assert!(app.gateway_client_settings.save_error.is_none());
    }
}
