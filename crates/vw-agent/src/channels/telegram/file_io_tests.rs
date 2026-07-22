use super::TelegramChannel;
use axum::{
    Router,
    body::Body,
    extract::{OriginalUri, State},
    http::{StatusCode, header::CONTENT_TYPE},
    response::Response,
    routing::get,
};
use std::{collections::VecDeque, sync::Arc};
use tokio::sync::{Mutex, oneshot};

#[derive(Clone, Debug)]
struct ResponseSpec {
    status: StatusCode,
    body: Vec<u8>,
    content_type: &'static str,
}

struct TestServerState {
    paths: Mutex<Vec<String>>,
    responses: Mutex<VecDeque<ResponseSpec>>,
}

struct TestServer {
    base_url: String,
    state: Arc<TestServerState>,
    shutdown: Option<oneshot::Sender<()>>,
}

impl TestServer {
    async fn spawn(responses: Vec<ResponseSpec>) -> Self {
        let state = Arc::new(TestServerState {
            paths: Mutex::new(Vec::new()),
            responses: Mutex::new(VecDeque::from(responses)),
        });
        let app = Router::new().route("/{*path}", get(record_request)).with_state(state.clone());
        let listener =
            tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.expect("bind test server");
        let addr = listener.local_addr().expect("server addr");
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .expect("serve test server");
        });

        Self { base_url: format!("http://{addr}"), state, shutdown: Some(shutdown_tx) }
    }

    async fn paths(&self) -> Vec<String> {
        self.state.paths.lock().await.clone()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}

async fn record_request(
    State(state): State<Arc<TestServerState>>,
    uri: OriginalUri,
) -> Response<Body> {
    state
        .paths
        .lock()
        .await
        .push(uri.path_and_query().map(ToString::to_string).unwrap_or_default());
    let response = state.responses.lock().await.pop_front().expect("test should provide response");

    Response::builder()
        .status(response.status)
        .header(CONTENT_TYPE, response.content_type)
        .body(Body::from(response.body))
        .expect("response should build")
}

fn test_channel(base_url: &str) -> TelegramChannel {
    TelegramChannel::new("123:ABC".to_string(), vec![], false).with_api_base(base_url.to_string())
}

#[test]
fn file_api_url_keeps_method_path() {
    let channel = TelegramChannel::new("123:ABC".to_string(), vec![], false);

    assert_eq!(channel.api_url("getFile"), "https://api.telegram.org/bot123:ABC/getFile");
}

#[tokio::test]
async fn get_file_path_calls_get_file_with_query() {
    let server = TestServer::spawn(vec![ResponseSpec {
        status: StatusCode::OK,
        body: br#"{"ok":true,"result":{"file_path":"docs/report.pdf"}}"#.to_vec(),
        content_type: "application/json",
    }])
    .await;
    let channel = test_channel(&server.base_url);

    let file_path = channel.get_file_path("file-123").await.expect("file path");

    assert_eq!(file_path, "docs/report.pdf");
    assert_eq!(server.paths().await, vec!["/bot123:ABC/getFile?file_id=file-123"]);
}

#[tokio::test]
async fn get_file_path_reports_missing_result_path() {
    let server = TestServer::spawn(vec![ResponseSpec {
        status: StatusCode::OK,
        body: br#"{"ok":true,"result":{}}"#.to_vec(),
        content_type: "application/json",
    }])
    .await;
    let channel = test_channel(&server.base_url);

    let error = channel.get_file_path("file-123").await.expect_err("missing path should fail");

    assert!(error.to_string().contains("missing file_path"));
}

#[tokio::test]
async fn get_file_path_rejects_non_string_result_path() {
    let server = TestServer::spawn(vec![ResponseSpec {
        status: StatusCode::OK,
        body: br#"{"ok":true,"result":{"file_path":42}}"#.to_vec(),
        content_type: "application/json",
    }])
    .await;
    let channel = test_channel(&server.base_url);

    let error = channel.get_file_path("file-123").await.expect_err("numeric path should fail");

    assert!(error.to_string().contains("missing file_path"));
}

#[tokio::test]
async fn download_file_reads_bytes_from_file_endpoint() {
    let server = TestServer::spawn(vec![ResponseSpec {
        status: StatusCode::OK,
        body: b"file-bytes".to_vec(),
        content_type: "application/octet-stream",
    }])
    .await;
    let channel = test_channel(&server.base_url);

    let bytes = channel.download_file("docs/report.pdf").await.expect("download bytes");

    assert_eq!(bytes, b"file-bytes");
    assert_eq!(server.paths().await, vec!["/file/bot123:ABC/docs/report.pdf"]);
}

#[tokio::test]
async fn download_file_reports_status_without_leaking_url() {
    let server = TestServer::spawn(vec![ResponseSpec {
        status: StatusCode::NOT_FOUND,
        body: b"missing token 123:ABC".to_vec(),
        content_type: "text/plain",
    }])
    .await;
    let channel = test_channel(&server.base_url);

    let error =
        channel.download_file("docs/report.pdf").await.expect_err("status should fail").to_string();

    assert!(error.contains("404 Not Found"));
    assert!(!error.contains("123:ABC"));
}
