use super::icons::{Operation, execute, finish};
use crate::app::state::app_providers::icons::{Draft, Form};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::atomic::{AtomicBool, Ordering},
};
use vw_gateway_client::{
    GatewayClient, GatewayEndpoint,
    vw_api_types::{
        agent_apps::{AddCustomIconRequest, AppId, CustomIconEntry},
        app_providers::icons::{AppProviderIcon, AppProviderIconChange},
    },
};

const IMAGE: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";
const INVALID: &str = "data:image/png;base64,bm90IGEgcG5n";

struct Server {
    stop: std::sync::Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let result = self.thread.take().unwrap().join();
        if !std::thread::panicking() {
            result.unwrap();
        }
    }
}

fn fixture() -> (GatewayClient, Server) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let client = GatewayClient::new(GatewayEndpoint::new(
        "127.0.0.1",
        listener.local_addr().unwrap().port(),
    ))
    .unwrap();
    let stop = std::sync::Arc::new(AtomicBool::new(false));
    let signal = stop.clone();
    let thread = std::thread::spawn(move || {
        let mut deleted = false;
        while !signal.load(Ordering::Relaxed) {
            let (mut stream, _) = match listener.accept() {
                Ok(connection) => connection,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                    continue;
                }
                Err(e) => panic!("{e}"),
            };
            stream.set_nonblocking(false).unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(2))).unwrap();
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                stream.read_exact(&mut byte).unwrap();
                head.push(byte[0]);
            }
            let head = String::from_utf8(head).unwrap();
            let len = head
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse().unwrap())
                })
                .unwrap_or(0);
            let mut payload = vec![0; len];
            stream.read_exact(&mut payload).unwrap();
            let mut route = head.split_whitespace();
            let (status, body) = match (route.next(), route.next()) {
                (Some("POST"), Some("/v1/agent-apps/custom-icons")) => {
                    let request: AddCustomIconRequest = serde_json::from_slice(&payload).unwrap();
                    assert_eq!(request.name, "中文草稿");
                    assert_eq!(request.data, INVALID);
                    (
                        "400 Bad Request",
                        serde_json::json!({"code":"invalid_input","message":"Invalid icon input"})
                            .to_string(),
                    )
                }
                (Some("DELETE"), Some("/v1/agent-apps/custom-icons/remove")) => {
                    deleted = true;
                    ("204 No Content", String::new())
                }
                (Some("GET"), Some("/v1/agent-apps/custom-icons")) => {
                    let mut rows = vec![icon("remove", "待删除"), icon("remote", "服务端新条目")];
                    if deleted {
                        rows.remove(0);
                    }
                    ("200 OK", serde_json::to_string(&rows).unwrap())
                }
                (Some("GET"), Some("/v1/app-providers/claude/fixture/icon")) => {
                    ("200 OK", serde_json::json!({"data":IMAGE}).to_string())
                }
                _ => panic!("unexpected fixture route: {}", head.lines().next().unwrap()),
            };
            write!(stream,"HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
        }
    });
    (client, Server { stop, thread: Some(thread) })
}

fn icon(id: &str, name: &str) -> CustomIconEntry {
    CustomIconEntry { id: id.into(), name: name.into(), data: IMAGE.into(), created_at: 1 }
}

#[tokio::test]
async fn cc04_ui_icons_failed_upload_keeps_form() {
    let (client, _server) = fixture();
    let mut form = Form {
        busy: true,
        draft: Some(Draft::Upload { name: "中文草稿".into(), data: INVALID.into() }),
        current: Some(AppProviderIcon { data: Some(IMAGE.into()) }),
        ..Form::default()
    };
    let request = AddCustomIconRequest { name: "中文草稿".into(), data: INVALID.into() };
    let result = execute(client, AppId::Claude, "fixture".into(), Operation::Upload(request)).await;
    finish(&mut form, result);
    let Some(Draft::Upload { name, data }) = &form.draft else {
        panic!("上传失败应保留原草稿");
    };
    assert_eq!(name, "中文草稿");
    assert_eq!(data, INVALID);
    assert!(form.notice.as_deref().unwrap().contains("Invalid icon input"));
    assert_eq!(form.current.unwrap().data.as_deref(), Some(IMAGE));
    assert!(!form.busy);
}

#[tokio::test]
async fn cc04_ui_icons_delete_refreshes_references() {
    let (client, _server) = fixture();
    let mut form = Form {
        busy: true,
        rows: Some(vec![icon("remove", "待删除")]),
        selection: Some(AppProviderIconChange::Select { icon_id: "remove".into() }),
        ..Form::default()
    };
    let result =
        execute(client, AppId::Claude, "fixture".into(), Operation::Delete("remove".into())).await;
    finish(&mut form, result);
    let ids = form.rows.unwrap().into_iter().map(|row| row.id).collect::<Vec<_>>();
    assert_eq!(ids, ["remote"]);
    assert_eq!(form.current.unwrap().data.as_deref(), Some(IMAGE));
    assert!(form.selection.is_none());
    assert!(form.notice.is_none());
    assert!(!form.busy);
}
