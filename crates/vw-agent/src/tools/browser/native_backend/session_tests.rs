use super::*;
use axum::{
    Json, Router,
    body::Bytes,
    extract::State,
    http::{Method, StatusCode, Uri},
};
use serde_json::json;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
struct CapturedRequest {
    method: Method,
    path: String,
    body: Value,
}

#[derive(Clone)]
struct WebDriverStubState {
    requests: Arc<Mutex<Vec<CapturedRequest>>>,
    close_status: StatusCode,
}

struct WebDriverStub {
    base_url: String,
    requests: Arc<Mutex<Vec<CapturedRequest>>>,
    handle: tokio::task::JoinHandle<std::io::Result<()>>,
}

impl WebDriverStub {
    async fn spawn() -> Self {
        Self::spawn_with_close_status(StatusCode::OK).await
    }

    async fn spawn_with_close_status(close_status: StatusCode) -> Self {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let app = Router::new()
            .fallback(handle_webdriver_request)
            .with_state(WebDriverStubState { requests: requests.clone(), close_status });
        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind webdriver stub");
        let base_url = format!("http://{}", listener.local_addr().expect("webdriver stub addr"));
        let handle = tokio::spawn(axum::serve(listener, app).into_future());

        Self { base_url, requests, handle }
    }

    fn requests(&self) -> Vec<CapturedRequest> {
        self.requests.lock().expect("webdriver requests mutex").clone()
    }
}

impl Drop for WebDriverStub {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

async fn handle_webdriver_request(
    State(state): State<WebDriverStubState>,
    method: Method,
    uri: Uri,
    body: Bytes,
) -> (StatusCode, Json<Value>) {
    state.requests.lock().expect("webdriver requests mutex").push(CapturedRequest {
        method: method.clone(),
        path: uri.path().to_string(),
        body: serde_json::from_slice(&body).unwrap_or(Value::Null),
    });

    let (status, value) = if method == Method::POST && uri.path() == "/session" {
        (
            StatusCode::OK,
            json!({
                "value": {
                    "sessionId": "native-session",
                    "capabilities": {}
                }
            }),
        )
    } else if state.close_status.is_success() {
        (state.close_status, json!({ "value": null }))
    } else {
        (
            state.close_status,
            json!({
                "value": {
                    "error": "unknown error",
                    "message": "stubbed close failure",
                    "stacktrace": ""
                }
            }),
        )
    };
    (status, Json(value))
}

#[test]
fn active_client_reports_missing_session() {
    let state = NativeBrowserState::default();

    let err = state.active_client().expect_err("session should be absent");

    assert!(err.to_string().contains("No active native browser session"));
}

#[tokio::test]
async fn reset_session_is_idempotent_without_client() {
    let mut state = NativeBrowserState::default();

    state.reset_session().await;
    state.reset_session().await;

    assert!(state.active_client().is_err());
}

#[tokio::test]
async fn ensure_session_reports_invalid_webdriver_url_with_context() {
    let mut state = NativeBrowserState::default();

    let err = state
        .ensure_session(true, "not a webdriver url", Some("  "))
        .await
        .expect_err("invalid webdriver URL should fail");

    assert!(err.to_string().contains("Failed to connect to WebDriver"));
    assert!(err.to_string().contains("not a webdriver url"));
    assert!(err.to_string().contains("Start chromedriver/geckodriver first"));
    assert!(state.active_client().is_err());
}

#[tokio::test]
async fn ensure_session_sends_headless_arguments_and_trimmed_binary() {
    let stub = WebDriverStub::spawn().await;
    let mut state = NativeBrowserState::default();

    state
        .ensure_session(true, &stub.base_url, Some("  /opt/chrome-for-test  "))
        .await
        .expect("webdriver stub should create a session");

    assert!(state.active_client().is_ok());
    let requests = stub.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, Method::POST);
    assert_eq!(requests[0].path, "/session");
    let chrome_options = &requests[0].body["capabilities"]["alwaysMatch"]["goog:chromeOptions"];
    assert_eq!(
        chrome_options["args"],
        json!(["--disable-blink-features=AutomationControlled", "--headless=new", "--disable-gpu"])
    );
    assert_eq!(chrome_options["binary"], "/opt/chrome-for-test");

    state.reset_session().await;
}

#[tokio::test]
async fn ensure_session_always_disables_automation_controlled() {
    let stub = WebDriverStub::spawn().await;
    let mut state = NativeBrowserState::default();

    state
        .ensure_session(false, &stub.base_url, None)
        .await
        .expect("webdriver stub should create a session");

    let requests = stub.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, Method::POST);
    assert_eq!(requests[0].path, "/session");
    let chrome_options = &requests[0].body["capabilities"]["alwaysMatch"]["goog:chromeOptions"];
    assert_eq!(chrome_options["args"], json!(["--disable-blink-features=AutomationControlled"]));

    state.reset_session().await;
}

#[tokio::test]
async fn ensure_session_omits_headless_arguments_and_blank_binary() {
    let stub = WebDriverStub::spawn().await;
    let mut state = NativeBrowserState::default();

    state
        .ensure_session(false, &stub.base_url, Some(" \t\n "))
        .await
        .expect("webdriver stub should create a session");

    let requests = stub.requests();
    let chrome_options = &requests[0].body["capabilities"]["alwaysMatch"]["goog:chromeOptions"];
    assert_eq!(chrome_options["args"], json!(["--disable-blink-features=AutomationControlled"]));
    assert!(chrome_options.get("binary").is_none());

    state.reset_session().await;
}

#[tokio::test]
async fn ensure_session_reuses_existing_client_without_second_handshake() {
    let stub = WebDriverStub::spawn().await;
    let mut state = NativeBrowserState::default();

    state
        .ensure_session(false, &stub.base_url, None)
        .await
        .expect("first session should be created");
    state
        .ensure_session(true, "not a webdriver url", Some("/ignored/chrome"))
        .await
        .expect("existing session should be reused");

    let requests = stub.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, Method::POST);
    assert_eq!(requests[0].path, "/session");

    state.reset_session().await;
}

#[tokio::test]
async fn reset_session_closes_active_client_and_clears_state() {
    let stub = WebDriverStub::spawn().await;
    let mut state = NativeBrowserState::default();
    state
        .ensure_session(false, &stub.base_url, None)
        .await
        .expect("webdriver stub should create a session");

    state.reset_session().await;

    assert!(state.active_client().is_err());
    let requests = stub.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1].method, Method::DELETE);
    assert_eq!(requests[1].path, "/session/native-session");
}

#[tokio::test]
async fn reset_session_clears_state_when_webdriver_close_fails() {
    let stub = WebDriverStub::spawn_with_close_status(StatusCode::INTERNAL_SERVER_ERROR).await;
    let mut state = NativeBrowserState::default();
    state
        .ensure_session(false, &stub.base_url, None)
        .await
        .expect("webdriver stub should create a session");

    state.reset_session().await;

    assert!(state.active_client().is_err());
    let requests = stub.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1].method, Method::DELETE);
}
