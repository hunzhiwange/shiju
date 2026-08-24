use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
use vw_tui::V3Action;

use super::app::{ActiveRunner, V3GatewayApp};
use super::runtime::gateway::{GatewaySessionSeed, GatewayUiRuntime};
use super::state::TuiState;

#[test]
fn active_runner_executes_local_actions_and_drains_queue() {
    let (endpoint, requests, server) = two_turn_gateway_fixture();
    let mut app = test_app(endpoint);
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|draft| {
        Ok(Some(format!("{draft} edited").trim().to_string()))
    }));

    app.widget_mut().replace_draft("draft");
    runner.execute_action(&mut app, V3Action::OpenExternalEditor);
    drain_until(&mut runner, &mut app, |runner, _| !runner.editor_in_flight());
    assert_eq!(app.widget().draft(), "draft edited");

    runner.execute_action(&mut app, V3Action::ShowTranscript);
    assert!(runner.has_transcript_pager());

    runner.execute_action(
        &mut app,
        V3Action::Submit { text: "first".to_string(), attachments: Vec::new() },
    );
    runner.execute_action(
        &mut app,
        V3Action::Queue { text: "second".to_string(), attachments: Vec::new() },
    );
    assert_eq!(app.widget().queued_input_count(), 1);

    drain_until(&mut runner, &mut app, |runner, app| {
        !runner.has_active_stream() && app.widget().queued_input_count() == 0
    });

    let paths = requests.try_iter().collect::<Vec<_>>();
    assert_eq!(
        paths.iter().filter(|path| path.as_str() == "/v1/chat/stream").count(),
        2,
        "the queued prompt must be submitted exactly once after the first terminal event"
    );
    server.join().expect("two-turn gateway fixture");
}

#[test]
fn interrupt_stops_the_matching_resumable_stream() {
    let (endpoint, requests, server) = interrupt_gateway_fixture();
    let mut app = test_app(endpoint);
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));

    runner.execute_action(
        &mut app,
        V3Action::Submit { text: "long running".to_string(), attachments: Vec::new() },
    );
    drain_until(&mut runner, &mut app, |_, app| {
        app.transcript_lines(80, true).iter().any(|line| line.to_string().contains("working"))
    });
    let request_id =
        runner.active_request_id().expect("active stream has a stable request id").to_string();
    runner.execute_action(&mut app, V3Action::Interrupt);

    drain_until(&mut runner, &mut app, |runner, _| !runner.has_active_stream());
    let recorded = requests.recv_timeout(Duration::from_secs(2)).expect("record stream and stop");
    assert_eq!(recorded.stream_request_id, request_id);
    assert_eq!(recorded.stop_path, format!("/v1/chat/stream/{request_id}/stop"));
    server.join().expect("interrupt gateway fixture");
}

#[test]
fn file_search_discards_stale_results_and_applies_latest_query() {
    let (endpoint, requests, server) = out_of_order_file_search_fixture();
    let mut app = test_app(endpoint);
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));

    app.widget_mut().replace_draft("@old");
    runner.execute_action(
        &mut app,
        V3Action::FileSearchQueryChanged { query: Some("old".to_string()) },
    );
    app.widget_mut().replace_draft("@new");
    runner.execute_action(
        &mut app,
        V3Action::FileSearchQueryChanged { query: Some("new".to_string()) },
    );

    let recorded = requests.recv_timeout(Duration::from_secs(3)).expect("record file searches");
    assert_eq!(recorded.len(), 2);
    assert!(recorded.iter().any(|path| {
        path == "/v1/find/file?directory=%2Fworkspace&query=old&dirs=false&type=file&limit=20"
    }));
    assert!(recorded.iter().any(|path| {
        path == "/v1/find/file?directory=%2Fworkspace&query=new&dirs=false&type=file&limit=20"
    }));
    server.join().expect("out-of-order file search fixture");

    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        runner.drain(&mut app);
        let rendered = render_widget(&app);
        if rendered.contains("src/new.rs") {
            thread::sleep(Duration::from_millis(50));
            runner.drain(&mut app);
            let settled = render_widget(&app);
            assert!(settled.contains("src/new.rs"));
            assert!(!settled.contains("src/old.rs"));
            break;
        }
        assert!(Instant::now() < deadline, "latest file search result was not rendered");
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn file_search_error_ends_loading_without_changing_the_draft() {
    let (endpoint, request, server) = failing_file_search_fixture();
    let mut app = test_app(endpoint);
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));

    app.widget_mut().replace_draft("@broken");
    runner.execute_action(
        &mut app,
        V3Action::FileSearchQueryChanged { query: Some("broken".to_string()) },
    );

    assert_eq!(
        request.recv_timeout(Duration::from_secs(3)).expect("record failed file search"),
        "/v1/find/file?directory=%2Fworkspace&query=broken&dirs=false&type=file&limit=20"
    );
    server.join().expect("failing file search fixture");
    drain_until(&mut runner, &mut app, |_, app| {
        render_widget(app).contains("file search unavailable")
    });
    assert_eq!(app.widget().draft(), "@broken");
}

#[test]
fn runner_renders_gateway_session_diff() {
    let (endpoint, request, server) = session_diff_fixture();
    let mut app = test_app(endpoint);
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));

    runner.execute_action(
        &mut app,
        V3Action::Submit { text: "/diff".to_string(), attachments: Vec::new() },
    );

    assert!(!runner.has_active_stream(), "/diff must not start a chat stream");
    assert_eq!(
        request.recv_timeout(Duration::from_secs(3)).expect("record session diff request"),
        "/v1/session/runner-session/diff?directory=%2Fworkspace"
    );
    server.join().expect("session diff gateway fixture");

    drain_until(&mut runner, &mut app, |_, app| {
        let rendered = render_widget(app);
        rendered.contains("src/lib.rs") && rendered.contains("new value")
    });
}

#[test]
fn runner_reports_empty_and_failed_session_diff() {
    let (endpoint, request, server) = session_diff_response_fixture("200 OK", "[]", Duration::ZERO);
    let mut app = test_app(endpoint);
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));
    runner.execute_action(
        &mut app,
        V3Action::Submit { text: "/diff".to_string(), attachments: Vec::new() },
    );
    request.recv_timeout(Duration::from_secs(3)).expect("record empty diff request");
    server.join().expect("empty diff gateway fixture");
    drain_until(&mut runner, &mut app, |_, app| {
        render_widget(app).contains("当前会话没有文件变化")
    });

    let (endpoint, request, server) = session_diff_response_fixture(
        "503 Service Unavailable",
        r#"{"message":"diff unavailable"}"#,
        Duration::ZERO,
    );
    let mut app = test_app(endpoint);
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));
    runner.execute_action(
        &mut app,
        V3Action::Submit { text: "/diff".to_string(), attachments: Vec::new() },
    );
    request.recv_timeout(Duration::from_secs(3)).expect("record failed diff request");
    server.join().expect("failed diff gateway fixture");
    drain_until(&mut runner, &mut app, |_, app| {
        render_widget(app).contains("failed to load session diff")
    });
}

#[test]
fn runner_reports_missing_session_id_for_diff() {
    let client = GatewayClient::new(GatewayEndpoint::new("127.0.0.1", 9))
        .expect("test gateway client");
    let runtime = GatewayUiRuntime::new(client, GatewaySessionSeed::new(PathBuf::from("/workspace")));
    let mut state = TuiState::default();
    state.project.workspace_root = Some(PathBuf::from("/workspace"));
    let mut app = V3GatewayApp::new(runtime, state);
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));

    runner.execute_action(
        &mut app,
        V3Action::Select {
            surface: "command_palette".to_string(),
            item_id: "diff".to_string(),
        },
    );

    drain_until(&mut runner, &mut app, |_, app| {
        render_widget(app).contains("gateway runtime session id is required")
    });
}

fn test_app(endpoint: GatewayEndpoint) -> V3GatewayApp {
    let client = GatewayClient::new(endpoint).expect("test gateway client");
    let runtime = GatewayUiRuntime::new(
        client,
        GatewaySessionSeed::new(PathBuf::from("/workspace"))
            .with_id(Some("runner-session".to_string())),
    );
    let mut state = TuiState::default();
    state.session.session_id = Some("runner-session".to_string());
    state.project.workspace_root = Some(PathBuf::from("/workspace"));
    V3GatewayApp::new(runtime, state)
}

fn drain_until(
    runner: &mut ActiveRunner,
    app: &mut V3GatewayApp,
    complete: impl Fn(&ActiveRunner, &V3GatewayApp) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !complete(runner, app) {
        runner.drain(app);
        assert!(Instant::now() < deadline, "active runner did not settle before deadline");
        thread::sleep(Duration::from_millis(10));
    }
}

fn two_turn_gateway_fixture() -> (GatewayEndpoint, mpsc::Receiver<String>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind two-turn gateway fixture");
    let port = listener.local_addr().expect("gateway fixture address").port();
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        let mut stream_count = 0;
        let mut save_count = 0;
        while stream_count < 2 || save_count < 2 {
            let (mut stream, _) = listener.accept().expect("accept gateway request");
            let request = read_request(&mut stream);
            sender.send(request.path.clone()).expect("record gateway request");
            if request.path == "/v1/chat/stream" {
                stream_count += 1;
                let body = format!(
                    "id: {stream_count}\ndata: {{\"type\":\"chat.done\",\"finish_reason\":\"stop\"}}\n\n"
                );
                write_response(&mut stream, "text/event-stream", body.as_str());
            } else if request.path.starts_with("/v1/session/runner-session/ui") {
                save_count += 1;
                write_response(&mut stream, "application/json", "true");
            } else {
                panic!("unexpected gateway request: {}", request.path);
            }
        }
    });
    (GatewayEndpoint::new("127.0.0.1", port), receiver, handle)
}

#[derive(Debug)]
struct InterruptRequests {
    stream_request_id: String,
    stop_path: String,
}

fn interrupt_gateway_fixture()
-> (GatewayEndpoint, mpsc::Receiver<InterruptRequests>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind interrupt gateway fixture");
    let port = listener.local_addr().expect("gateway fixture address").port();
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept stream request");
        let request = read_request(&mut stream);
        assert_eq!(request.path, "/v1/chat/stream");
        let stream_request_id = request
            .header("x-vw-stream-request-id")
            .expect("resumable stream request id")
            .to_string();
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\nid: 1\ndata: {\"type\":\"chat.delta\",\"delta\":\"working\"}\n\n",
            )
            .expect("write streaming response");
        stream.flush().expect("flush streaming response");

        let (mut stop_stream, _) = listener.accept().expect("accept stop request");
        let stop = read_request(&mut stop_stream);
        write_response(&mut stop_stream, "application/json", r#"{"cancelled":true}"#);
        sender
            .send(InterruptRequests { stream_request_id, stop_path: stop.path })
            .expect("record stop request");
        drop(stream);

        let (mut save_stream, _) = listener.accept().expect("accept snapshot request");
        let save = read_request(&mut save_stream);
        assert!(save.path.starts_with("/v1/session/runner-session/ui"));
        write_response(&mut save_stream, "application/json", "true");
    });
    (GatewayEndpoint::new("127.0.0.1", port), receiver, handle)
}

fn out_of_order_file_search_fixture()
-> (GatewayEndpoint, mpsc::Receiver<Vec<String>>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind file search gateway fixture");
    let port = listener.local_addr().expect("gateway fixture address").port();
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        let mut requests = Vec::new();
        let mut old_stream = None;
        let mut new_stream = None;
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept file search request");
            let request = read_request(&mut stream);
            if request.path.contains("query=old") {
                old_stream = Some(stream);
            } else if request.path.contains("query=new") {
                new_stream = Some(stream);
            } else {
                panic!("unexpected file search request: {}", request.path);
            }
            requests.push(request.path);
        }
        sender.send(requests).expect("record file search requests");

        write_response(
            &mut new_stream.expect("new file search request"),
            "application/json",
            r#"["src/new.rs"]"#,
        );
        thread::sleep(Duration::from_millis(100));
        write_response(
            &mut old_stream.expect("old file search request"),
            "application/json",
            r#"["src/old.rs"]"#,
        );
    });
    (GatewayEndpoint::new("127.0.0.1", port), receiver, handle)
}

fn failing_file_search_fixture()
-> (GatewayEndpoint, mpsc::Receiver<String>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind failing file search fixture");
    let port = listener.local_addr().expect("gateway fixture address").port();
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept failing file search request");
        let request = read_request(&mut stream);
        sender.send(request.path).expect("record failing file search request");
        write_status_response(
            &mut stream,
            "503 Service Unavailable",
            "application/json",
            r#"{"message":"index unavailable"}"#,
        );
    });
    (GatewayEndpoint::new("127.0.0.1", port), receiver, handle)
}

fn session_diff_fixture()
-> (GatewayEndpoint, mpsc::Receiver<String>, thread::JoinHandle<()>) {
    session_diff_response_fixture(
        "200 OK",
        r#"[{"file":"src/lib.rs","before":"old value\n","after":"new value\n","additions":1,"deletions":1,"status":"modified"}]"#,
        Duration::from_millis(100),
    )
}

fn session_diff_response_fixture(
    status: &'static str,
    body: &'static str,
    delay: Duration,
) -> (GatewayEndpoint, mpsc::Receiver<String>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind session diff gateway fixture");
    let port = listener.local_addr().expect("gateway fixture address").port();
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept session diff request");
        let request = read_request(&mut stream);
        sender.send(request.path).expect("record session diff request");
        thread::sleep(delay);
        write_status_response(&mut stream, status, "application/json", body);
    });
    (GatewayEndpoint::new("127.0.0.1", port), receiver, handle)
}

fn render_widget(app: &V3GatewayApp) -> String {
    let area = Rect::new(0, 0, 80, 18);
    let mut buffer = Buffer::empty(area);
    app.widget().render(area, &mut buffer);
    buffer.content.iter().map(|cell| cell.symbol()).collect()
}

struct RecordedRequest {
    path: String,
    headers: Vec<(String, String)>,
}

impl RecordedRequest {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

fn read_request(stream: &mut TcpStream) -> RecordedRequest {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    let header_end = loop {
        let read = stream.read(&mut buffer).expect("read gateway request");
        assert!(read > 0, "gateway request closed before headers");
        bytes.extend_from_slice(&buffer[..read]);
        if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break index;
        }
    };
    let headers = String::from_utf8_lossy(&bytes[..header_end]).into_owned();
    let mut lines = headers.lines();
    let path = lines
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .expect("gateway request path")
        .to_string();
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.to_string(), value.trim().to_string()))
        .collect::<Vec<_>>();
    let content_length = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.parse::<usize>().ok())
        .unwrap_or_default();
    let body_start = header_end.saturating_add(4);
    while bytes.len().saturating_sub(body_start) < content_length {
        let read = stream.read(&mut buffer).expect("read gateway request body");
        assert!(read > 0, "gateway request closed before body");
        bytes.extend_from_slice(&buffer[..read]);
    }
    RecordedRequest { path, headers }
}

fn write_response(stream: &mut TcpStream, content_type: &str, body: &str) {
    write_status_response(stream, "200 OK", content_type, body);
}

fn write_status_response(stream: &mut TcpStream, status: &str, content_type: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).expect("write gateway response");
}
