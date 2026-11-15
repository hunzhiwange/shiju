//! 验证原生浏览器元素动作、重试恢复与错误上下文。
//! 模拟 WebDriver 按请求队列返回结果，恢复重试只针对可恢复查找失败且保留操作上下文。

use std::collections::VecDeque;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{Method, StatusCode, Uri};
use axum::response::IntoResponse;
use axum::{Json, Router, routing::any};
use fantoccini::ClientBuilder;
use serde_json::{Value, json};
use tokio::sync::Mutex;

use super::*;

const ELEMENT_KEY: &str = "element-6066-11e4-a52e-4f735466cecf";
const ELEMENT_ID: &str = "element-1";
const SESSION_ID: &str = "test-session";

#[derive(Clone, Debug, PartialEq)]
struct RecordedRequest {
    method: Method,
    path: String,
    body: Value,
}

// 队列式 WebDriver 夹具让每一步查找、清空和输入请求都能按顺序断言。
#[derive(Debug)]
struct MockResponse {
    status: StatusCode,
    value: Value,
}

impl MockResponse {
    fn success(value: Value) -> Self {
        Self { status: StatusCode::OK, value }
    }

    fn error(error: &str, message: &str) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            value: json!({
                "error": error,
                "message": message,
                "stacktrace": "",
            }),
        }
    }
}

#[derive(Debug)]
struct MockState {
    responses: Mutex<VecDeque<MockResponse>>,
    requests: Mutex<Vec<RecordedRequest>>,
}

struct MockWebDriver {
    client: Client,
    state: Arc<MockState>,
    server: tokio::task::JoinHandle<()>,
}

impl MockWebDriver {
    async fn start(responses: Vec<MockResponse>) -> Self {
        let state = Arc::new(MockState {
            responses: Mutex::new(responses.into()),
            requests: Mutex::new(Vec::new()),
        });
        let app = Router::new().fallback(any(handle_webdriver_request)).with_state(state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("mock WebDriver should bind to loopback");
        let address = listener.local_addr().expect("mock WebDriver should have an address");
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("mock WebDriver should serve requests");
        });
        let client = ClientBuilder::rustls()
            .expect("rustls client builder should initialize")
            .connect(&format!("http://{address}"))
            .await
            .expect("mock WebDriver should create a session");

        Self { client, state, server }
    }

    async fn command_requests(&self) -> Vec<RecordedRequest> {
        self.state
            .requests
            .lock()
            .await
            .iter()
            .filter(|request| {
                request.path != "/session" && request.path != format!("/session/{SESSION_ID}")
            })
            .cloned()
            .collect()
    }
}

impl Drop for MockWebDriver {
    fn drop(&mut self) {
        self.server.abort();
    }
}

async fn handle_webdriver_request(
    State(state): State<Arc<MockState>>,
    method: Method,
    uri: Uri,
    body: Bytes,
) -> impl IntoResponse {
    let body = if body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&body).expect("fantoccini should send JSON request bodies")
    };
    let path = uri.path().to_owned();
    state.requests.lock().await.push(RecordedRequest {
        method: method.clone(),
        path: path.clone(),
        body,
    });

    let response =
        if method == Method::POST && path == "/session" {
            MockResponse::success(json!({
                "sessionId": SESSION_ID,
                "capabilities": {},
            }))
        } else if method == Method::DELETE && path == format!("/session/{SESSION_ID}") {
            MockResponse::success(Value::Null)
        } else {
            state.responses.lock().await.pop_front().unwrap_or_else(|| {
                MockResponse::error("unknown error", "unexpected WebDriver request")
            })
        };

    (response.status, Json(json!({ "value": response.value })))
}

fn element_response() -> MockResponse {
    MockResponse::success(json!({ ELEMENT_KEY: ELEMENT_ID }))
}

fn null_response() -> MockResponse {
    MockResponse::success(Value::Null)
}

#[tokio::test]
async fn find_element_returns_match_and_preserves_css_selector() {
    let webdriver = MockWebDriver::start(vec![element_response()]).await;

    let element = find_element(&webdriver.client, "button[data-action='save']")
        .await
        .expect("element should be found");

    assert_eq!(String::from(element.element_id()), ELEMENT_ID);
    assert_eq!(
        webdriver.command_requests().await,
        vec![RecordedRequest {
            method: Method::POST,
            path: format!("/session/{SESSION_ID}/element"),
            body: json!({
                "using": "css selector",
                "value": "button[data-action='save']",
            }),
        }]
    );
}

#[tokio::test]
async fn find_element_adds_lookup_context_to_webdriver_error() {
    let webdriver = MockWebDriver::start(vec![MockResponse::error(
        "no such element",
        "selector did not match",
    )])
    .await;

    let error =
        find_element(&webdriver.client, ".missing").await.expect_err("missing element should fail");

    assert_eq!(error.to_string(), "Failed to find element");
    assert!(format!("{error:#}").contains("selector did not match"));
}

#[tokio::test]
async fn wait_for_selector_returns_first_available_match() {
    let webdriver = MockWebDriver::start(vec![element_response()]).await;

    let element = wait_for_selector(&webdriver.client, "#ready")
        .await
        .expect("available element should be returned");

    assert_eq!(String::from(element.element_id()), ELEMENT_ID);
    assert_eq!(webdriver.command_requests().await[0].body["value"], "#ready");
}

#[tokio::test]
async fn wait_for_selector_adds_context_to_non_retryable_error() {
    let webdriver =
        MockWebDriver::start(vec![MockResponse::error("unknown error", "driver stopped waiting")])
            .await;

    let error = wait_for_selector(&webdriver.client, "#ready")
        .await
        .expect_err("non-retryable driver error should fail");

    assert_eq!(error.to_string(), "Failed to wait for selector");
    assert!(format!("{error:#}").contains("driver stopped waiting"));
}

#[tokio::test]
async fn click_with_recovery_finds_then_clicks_element() {
    let webdriver = MockWebDriver::start(vec![element_response(), null_response()]).await;

    click_with_recovery(&webdriver.client, "#submit").await.expect("click should succeed");

    let requests = webdriver.command_requests().await;
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].path, format!("/session/{SESSION_ID}/element"));
    assert_eq!(
        requests[1],
        RecordedRequest {
            method: Method::POST,
            path: format!("/session/{SESSION_ID}/element/{ELEMENT_ID}/click"),
            body: json!({}),
        }
    );
}

#[tokio::test]
async fn click_with_recovery_adds_click_context_to_webdriver_error() {
    let webdriver = MockWebDriver::start(vec![
        element_response(),
        MockResponse::error("element click intercepted", "button is covered"),
    ])
    .await;

    let error = click_with_recovery(&webdriver.client, "#submit")
        .await
        .expect_err("intercepted click should fail");

    assert_eq!(error.to_string(), "Failed to click element");
    assert!(format!("{error:#}").contains("button is covered"));
}

#[tokio::test]
// fill 与 type 的区别在于前者必须先清空；清空失败时禁止继续发送敏感文本。
async fn fill_with_recovery_clears_before_sending_unicode_text() {
    let webdriver =
        MockWebDriver::start(vec![element_response(), null_response(), null_response()]).await;

    fill_with_recovery(&webdriver.client, "#name", "你好").await.expect("fill should succeed");

    let requests = webdriver.command_requests().await;
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[1].path, format!("/session/{SESSION_ID}/element/{ELEMENT_ID}/clear"));
    assert_eq!(
        requests[2],
        RecordedRequest {
            method: Method::POST,
            path: format!("/session/{SESSION_ID}/element/{ELEMENT_ID}/value"),
            body: json!({ "text": "你好" }),
        }
    );
}

#[tokio::test]
async fn fill_with_recovery_stops_when_clear_fails() {
    let webdriver = MockWebDriver::start(vec![
        element_response(),
        MockResponse::error("invalid element state", "input cannot be cleared"),
    ])
    .await;

    let error = fill_with_recovery(&webdriver.client, "#name", "replacement")
        .await
        .expect_err("clear failure should stop fill");

    assert!(format!("{error:#}").contains("input cannot be cleared"));
    let requests = webdriver.command_requests().await;
    assert_eq!(requests.len(), 2);
    assert!(requests.iter().all(|request| !request.path.ends_with("/value")));
}

#[tokio::test]
async fn type_with_recovery_sends_empty_text_without_clearing() {
    let webdriver = MockWebDriver::start(vec![element_response(), null_response()]).await;

    type_with_recovery(&webdriver.client, "#query", "")
        .await
        .expect("empty text remains a valid send-keys request");

    let requests = webdriver.command_requests().await;
    assert_eq!(requests.len(), 2);
    assert!(requests.iter().all(|request| !request.path.ends_with("/clear")));
    assert_eq!(requests[1].body, json!({ "text": "" }));
}

#[tokio::test]
async fn type_with_recovery_adds_send_keys_context_to_webdriver_error() {
    let webdriver = MockWebDriver::start(vec![
        element_response(),
        MockResponse::error("element not interactable", "input is disabled"),
    ])
    .await;

    let error = type_with_recovery(&webdriver.client, "#query", "text")
        .await
        .expect_err("disabled input should reject typing");

    assert_eq!(error.to_string(), "Failed to send keys");
    assert!(format!("{error:#}").contains("input is disabled"));
}

#[tokio::test]
async fn element_checked_returns_both_selection_states() {
    let webdriver = MockWebDriver::start(vec![
        element_response(),
        MockResponse::success(Value::Bool(true)),
        MockResponse::success(Value::Bool(false)),
    ])
    .await;
    let element =
        find_element(&webdriver.client, "#choice").await.expect("checkbox should be found");

    assert!(element_checked(&element).await.expect("selected state should be read"));
    assert!(!element_checked(&element).await.expect("unselected state should be read"));

    let requests = webdriver.command_requests().await;
    assert_eq!(requests[1].path, format!("/session/{SESSION_ID}/element/{ELEMENT_ID}/selected"));
    assert_eq!(requests[1].method, Method::GET);
}

#[tokio::test]
async fn element_checked_adds_selection_context_to_webdriver_error() {
    let webdriver = MockWebDriver::start(vec![
        element_response(),
        MockResponse::error("stale element reference", "checkbox was detached"),
    ])
    .await;
    let element =
        find_element(&webdriver.client, "#choice").await.expect("checkbox should be found");

    let error = element_checked(&element).await.expect_err("detached checkbox should fail");

    assert_eq!(error.to_string(), "Failed to check if element is selected");
    assert!(format!("{error:#}").contains("checkbox was detached"));
}

#[tokio::test]
async fn prepare_interactable_element_returns_lookup_result() {
    let webdriver = MockWebDriver::start(vec![element_response()]).await;

    let element = prepare_interactable_element(&webdriver.client, "[role='button']")
        .await
        .expect("interactable element should be prepared");

    assert_eq!(String::from(element.element_id()), ELEMENT_ID);
    assert_eq!(
        webdriver.command_requests().await[0].body,
        json!({ "using": "css selector", "value": "[role='button']" })
    );
}

#[tokio::test]
async fn prepare_interactable_element_propagates_lookup_context() {
    let webdriver = MockWebDriver::start(vec![MockResponse::error(
        "no such element",
        "interactable target is missing",
    )])
    .await;

    let error = prepare_interactable_element(&webdriver.client, "#missing")
        .await
        .expect_err("missing interactable target should fail");

    assert_eq!(error.to_string(), "Failed to find element");
    assert!(format!("{error:#}").contains("interactable target is missing"));
}

#[tokio::test]
async fn hover_element_dispatches_mouseover_with_serialized_element() {
    let webdriver = MockWebDriver::start(vec![element_response(), null_response()]).await;
    let element =
        find_element(&webdriver.client, "#menu").await.expect("hover target should be found");

    hover_element(&webdriver.client, &element).await.expect("hover script should execute");

    let requests = webdriver.command_requests().await;
    assert_eq!(requests[1].path, format!("/session/{SESSION_ID}/execute/sync"));
    assert!(requests[1].body["script"].as_str().unwrap().contains("MouseEvent('mouseover'"));
    assert!(requests[1].body["script"].as_str().unwrap().contains("dispatchEvent"));
    assert_eq!(requests[1].body["args"], json!([{ ELEMENT_KEY: ELEMENT_ID }]));
}

#[tokio::test]
async fn hover_element_adds_script_context_to_webdriver_error() {
    let webdriver = MockWebDriver::start(vec![
        element_response(),
        MockResponse::error("javascript error", "MouseEvent is unavailable"),
    ])
    .await;
    let element =
        find_element(&webdriver.client, "#menu").await.expect("hover target should be found");

    let error = hover_element(&webdriver.client, &element)
        .await
        .expect_err("script error should fail hover");

    assert_eq!(error.to_string(), "Failed to hover element via JS");
    assert!(format!("{error:#}").contains("MouseEvent is unavailable"));
}
