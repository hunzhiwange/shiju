use super::super::messages::GatewayClientMessage;
use super::*;
use crate::app::App;
use crate::app::state::GatewayClientServerDraft;
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

        app.navigation_tabs.project_path = Some("/gateway-test".into());
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
            config_ui::AppSystemSettingsConfig {
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

#[test]
fn gateway_client_invalid_tenant_effect_shows_host_toast() {
    let _isolation = crate::app::config::isolated_settings();
    let mut app = App::new().0;
    app.gateway_client_settings = Default::default();
    let _ = update(&mut app, SettingsMessage::GatewayClient(GatewayClientMessage::PlatformIdChanged(" ".into())));
    assert!(app.toast_state.active_toast.is_some());
}
