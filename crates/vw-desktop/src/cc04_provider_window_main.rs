//! Explicit isolated provider list acceptance; never installs production bootstrap.
#![allow(dead_code, unused_imports, clippy::all, clippy::pedantic)]
mod app;
mod apps;
mod desktop_update_helper;
mod fonts;
use app::message::app_providers::list::{ListMessage, message};
use app::{App, Message};
use iced::{Element, Task, Theme};
fn main() {
    let case = std::env::var("SHIJU_CC04_WINDOW_CASE").expect("window case");
    assert!(matches!(case.as_str(), "list" | "edit" | "claude"), "unknown window case");
    let edit = case == "edit";
    let claude = case == "claude";
    let theme = match std::env::var("SHIJU_CC04_WINDOW_THEME").as_deref() {
        Ok("dark") => Theme::Dark,
        Ok("light") => Theme::Light,
        _ => panic!("unknown theme"),
    };
    let (port, stop, server) = fixture(claude);
    app::config::install_window_test_bootstrap(port);
    iced::application(
        move || {
            let (mut state, _) = App::new();
            state.apps_catalog.host = "fixture-host".into();
            state.apps_catalog.token = "fake-host-token".into();
            let task = app::message::app_providers::list::update(
                &mut state,
                ListMessage::Open(vw_gateway_client::vw_api_types::agent_apps::AppId::Claude),
            );
            if edit {
                use vw_gateway_client::vw_api_types::app_providers::{ProviderProtocol, create::*};
                state.app_providers.new_draft = Some(AppProviderCreate {
                    name: "fixture".into(),
                    protocol: ProviderProtocol::Anthropic,
                    api_key: ProviderSecretWrite::Keep,
                    base_url: "https://fixture-api.example.test".into(),
                    model: "fixture-model".into(),
                    request_options: None,
                    add_to_live: false,
                });
            }
            (state, task)
        },
        |state: &mut App, event: Message| match event {
            Message::Settings(value) => app::message::settings::update(state, value),
            _ => Task::none(),
        },
        view,
    )
    .title(if claude {
        "Claude Code 供应商隔离验收"
    } else {
        "供应商基础编辑隔离验收"
    })
    .theme(move |_: &App| theme.clone())
    .window_size((1000., 820.))
    .settings(iced::Settings {
        fonts: fonts::load_all(),
        default_font: iced::Font::with_name("PingFang SC"),
        ..Default::default()
    })
    .run()
    .unwrap();
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    server.join().unwrap();
}
fn view(state: &App) -> Element<'_, Message> {
    iced::widget::stack![
        app::components::app_providers::list::overlay(
            &state.app_providers,
            iced::widget::container(iced::widget::text("隔离 HTTP SDK 验收"))
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .into()
        ),
        app::components::toast::view(state)
    ]
    .into()
}
fn fixture(
    claude: bool,
) -> (u16, std::sync::Arc<std::sync::atomic::AtomicBool>, std::thread::JoinHandle<()>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let done = stop.clone();
    let server = std::thread::spawn(move || {
        let mut presentation = serde_json::json!({"sortIndex":1,"category":"工作","notes":"中文动态备注：实际字体整形验收。"});
        let mut basics: std::collections::BTreeMap<String, serde_json::Value> = (0..16).map(|i| (format!("provider-{i}"), serde_json::json!({"name":format!("中文供应商 {i}"),"category":"工作","notes":"中文动态备注","websiteUrl":"https://website.example.test"}))).collect();
        let mut claude_profiles = std::collections::BTreeMap::from([
            (
                "provider-0".to_owned(),
                (
                    serde_json::json!({
                        "kind":"third_party","baseUrl":"https://fixture-api.example.test","apiKeyHeader":false
                    }),
                    serde_json::json!("现有中文模型"),
                ),
            ),
            (
                "provider-1".to_owned(),
                (serde_json::json!({"kind":"official"}), serde_json::Value::Null),
            ),
        ]);
        while !done.load(std::sync::atomic::Ordering::Relaxed) {
            let (mut stream, _) = match listener.accept() {
                Ok(v) => v,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                    continue;
                }
                Err(e) => panic!("{e}"),
            };
            stream.set_read_timeout(Some(std::time::Duration::from_secs(3))).unwrap();
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                let mut b = [0];
                if stream.read_exact(&mut b).is_err() {
                    break;
                }
                head.push(b[0]);
            }
            let head = String::from_utf8(head).unwrap();
            let line = head.lines().next().unwrap_or("");
            let len: usize = head
                .lines()
                .find_map(|l| {
                    l.to_lowercase()
                        .strip_prefix("content-length:")
                        .map(|v| v.trim().parse().unwrap())
                })
                .unwrap_or(0);
            let mut payload = vec![0; len];
            stream.read_exact(&mut payload).unwrap();
            let mut status = "200 OK";
            let path = line.split_whitespace().nth(1).unwrap_or("");
            let id = path
                .strip_prefix("/v1/app-providers/claude/")
                .and_then(|v| v.split('/').next())
                .unwrap_or("");
            let body = if claude && path.ends_with("/form/read") && line.starts_with("POST ") {
                assert!(head.to_lowercase().contains("x-agent-apps-token: fake-host-token"));
                let target: serde_json::Value = serde_json::from_slice(&payload).unwrap();
                assert_eq!(target["targetHostId"], "fixture-host");
                let (config, model) = claude_profiles.get(id).cloned().unwrap_or((
                    serde_json::json!({"kind":"unsupported"}),
                    serde_json::Value::Null,
                ));
                serde_json::json!({"fields":basics[id],"config":config,"model":model})
            } else if claude
                && path.ends_with("/form")
                && (line.starts_with("POST ") || line.starts_with("PATCH "))
            {
                assert!(head.to_lowercase().contains("x-agent-apps-token: fake-host-token"));
                let mut value: serde_json::Value = serde_json::from_slice(&payload).unwrap();
                assert_eq!(value["targetHostId"], "fixture-host");
                let mut config = value["config"].take();
                if config["kind"] == "third_party"
                    && !config["baseUrl"].as_str().is_some_and(|url| url.starts_with("https://"))
                {
                    status = "422 Unprocessable Entity";
                    serde_json::Value::Null
                } else {
                    config.as_object_mut().unwrap().remove("apiKey");
                    claude_profiles.insert(id.into(), (config, value["model"].take()));
                    basics.insert(id.into(), value["fields"].take());
                    if line.starts_with("POST ") {
                        status = "201 Created";
                    }
                    serde_json::json!({"catalogSaved":true,"live":"not_attempted"})
                }
            } else if path.ends_with("/basic") && line.starts_with("GET ") {
                basics.get(id).cloned().unwrap_or_else(|| {
                    status = "404 Not Found";
                    serde_json::json!({"message":"missing"})
                })
            } else if path.ends_with("/basic") && line.starts_with("PUT ") {
                let value: serde_json::Value = serde_json::from_slice(&payload).unwrap();
                if value["fields"]["name"] == "fail" {
                    status = "403 Forbidden";
                    serde_json::json!({"message":"denied"})
                } else {
                    if value["apiKey"]["action"] == "keep" {
                        assert!(value["apiKey"].get("value").is_none());
                    }
                    basics.insert(id.into(), value["fields"].clone());
                    value["fields"].clone()
                }
            } else if line.starts_with("DELETE ") {
                assert!(head.to_lowercase().contains("x-agent-apps-token: fake-host-token"));
                if id == "provider-0" {
                    status = "409 Conflict";
                    serde_json::json!({"message":"referenced"})
                } else {
                    basics.remove(id);
                    serde_json::json!({"deleted":true})
                }
            } else if line.starts_with("GET /v1/app-providers/claude/catalog/state?") {
                assert!(head.to_lowercase().contains("x-agent-apps-token: fake-host-token"));
                serde_json::json!({"appId":"claude","rows":basics.iter().map(|(id, fields)|serde_json::json!({"provider":{"id":id,"appId":"claude","name":fields["name"],"configured":true},"nativeMember":if id=="provider-0"{"present"}else{"absent"}})).collect::<Vec<_>>(),"direct":{"status":"selected","id":"provider-0"},"inUse":{"status":"selected","id":"provider-0"},"nativeRead":null})
            } else if line.starts_with("GET /v1/app-providers/claude/presentation/entries ") {
                serde_json::json!([{"id":"provider-0","presentation":presentation}])
            } else if line.starts_with("PUT /v1/app-providers/claude/")
                && line.contains("/presentation ")
            {
                let value: serde_json::Value = serde_json::from_slice(&payload).unwrap();
                if value["notes"] == "fail" {
                    status = "403 Forbidden";
                    serde_json::json!({"error":"denied"})
                } else {
                    presentation = value;
                    presentation.clone()
                }
            } else {
                status = "404 Not Found";
                serde_json::json!({"error":"unknown fixture path"})
            };
            let body = body.to_string();
            write!(stream,"HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
        }
    });
    (port, stop, server)
}
