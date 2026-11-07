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
    assert_eq!(
        std::env::var("SHIJU_CC04_WINDOW_CASE").as_deref(),
        Ok("list"),
        "unknown window case"
    );
    let theme = match std::env::var("SHIJU_CC04_WINDOW_THEME").as_deref() {
        Ok("dark") => Theme::Dark,
        Ok("light") => Theme::Light,
        _ => panic!("unknown theme"),
    };
    let (port, stop, server) = fixture();
    app::config::install_window_test_bootstrap(port);
    iced::application(
        || {
            let (mut state, _) = App::new();
            state.apps_catalog.host = "fixture-host".into();
            state.apps_catalog.token = "fake-host-token".into();
            let task = app::message::app_providers::list::update(
                &mut state,
                ListMessage::Open(vw_gateway_client::vw_api_types::agent_apps::AppId::Claude),
            );
            (state, task)
        },
        |state: &mut App, event: Message| match event {
            Message::Settings(value) => app::message::settings::update(state, value),
            _ => Task::none(),
        },
        view,
    )
    .title("供应商列表隔离验收")
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
fn fixture() -> (u16, std::sync::Arc<std::sync::atomic::AtomicBool>, std::thread::JoinHandle<()>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let done = stop.clone();
    let server = std::thread::spawn(move || {
        let mut presentation = serde_json::json!({"sortIndex":1,"category":"工作","notes":"中文动态备注：实际字体整形验收。"});
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
            let mut status = "200 OK";
            let body = if line.starts_with("GET /v1/app-providers/claude/catalog/state?") {
                assert!(head.to_lowercase().contains("x-agent-apps-token: fake-host-token"));
                serde_json::json!({"appId":"claude","rows":(0..16).map(|i|serde_json::json!({"provider":{"id":format!("provider-{i}"),"appId":"claude","name":format!("中文供应商 {i}"),"configured":true},"nativeMember":if i==0{"present"}else{"absent"}})).collect::<Vec<_>>(),"direct":{"status":"selected","id":"provider-0"},"inUse":{"status":"selected","id":"provider-0"},"nativeRead":null})
            } else if line.starts_with("GET /v1/app-providers/claude/presentation/entries ") {
                serde_json::json!([{"id":"provider-0","presentation":presentation}])
            } else if line.starts_with("PUT /v1/app-providers/claude/")
                && line.contains("/presentation ")
            {
                let len: usize = head
                    .lines()
                    .find_map(|l| {
                        l.to_lowercase()
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().parse().unwrap())
                    })
                    .unwrap();
                let mut buf = vec![0; len];
                stream.read_exact(&mut buf).unwrap();
                let value: serde_json::Value = serde_json::from_slice(&buf).unwrap();
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
