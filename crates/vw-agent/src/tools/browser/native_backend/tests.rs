//! 验证原生浏览器后端的导航、元素操作、截图与安全边界。

use super::*;
use axum::body::to_bytes;
use axum::extract::{Request, State};
use axum::{Json, Router};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const MOCK_SESSION_ID: &str = "mock-session";
const MOCK_ELEMENT_ID: &str = "mock-element";
const SCREENSHOT_BYTES: &[u8] = &[0, 1, 2, 3];
const SCREENSHOT_BASE64: &str = "AAECAw==";

#[derive(Clone, Debug)]
struct MockRequest {
    method: String,
    path: String,
    body: Value,
}

#[derive(Clone, Default)]
struct MockWebDriverState {
    requests: Arc<Mutex<Vec<MockRequest>>>,
    active_element_missing: Arc<AtomicBool>,
    selected_reads: Arc<AtomicUsize>,
}

struct MockWebDriver {
    endpoint: String,
    state: MockWebDriverState,
    task: tokio::task::JoinHandle<()>,
}

impl MockWebDriver {
    async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("mock webdriver should bind to loopback");
        let address = listener.local_addr().expect("mock webdriver should have an address");
        let state = MockWebDriverState::default();
        let app = Router::new().fallback(mock_webdriver_handler).with_state(state.clone());
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("mock webdriver should serve requests");
        });

        Self { endpoint: format!("http://{address}"), state, task }
    }

    fn requests(&self) -> Vec<MockRequest> {
        self.state.requests.lock().expect("request log should not be poisoned").clone()
    }
}

impl Drop for MockWebDriver {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn mock_webdriver_handler(
    State(state): State<MockWebDriverState>,
    request: Request,
) -> Json<Value> {
    let method = request.method().as_str().to_string();
    let path = request.uri().path().to_string();
    let body = to_bytes(request.into_body(), usize::MAX)
        .await
        .ok()
        .filter(|bytes| !bytes.is_empty())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or(Value::Null);

    state.requests.lock().expect("request log should not be poisoned").push(MockRequest {
        method: method.clone(),
        path: path.clone(),
        body: body.clone(),
    });

    if method == "POST" && path == "/session" {
        return Json(json!({
            "value": {
                "sessionId": MOCK_SESSION_ID,
                "capabilities": {},
            }
        }));
    }

    let value = if method == "GET" && path.ends_with("/title") {
        json!("Mock page title")
    } else if method == "GET" && path.ends_with("/url") {
        json!("https://example.test/landing")
    } else if method == "GET" && path.ends_with("/screenshot") {
        json!(SCREENSHOT_BASE64)
    } else if method == "GET"
        && path.ends_with("/element/active")
        && state.active_element_missing.load(Ordering::SeqCst)
    {
        Value::Null
    } else if method == "GET" && path.ends_with("/element/active") {
        json!({"element-6066-11e4-a52e-4f735466cecf": MOCK_ELEMENT_ID})
    } else if method == "POST" && path.ends_with("/element") {
        json!({"element-6066-11e4-a52e-4f735466cecf": MOCK_ELEMENT_ID})
    } else if method == "GET" && path.ends_with("/text") {
        json!("Mock element text")
    } else if method == "GET" && path.ends_with("/displayed") {
        json!(true)
    } else if method == "GET" && path.ends_with("/selected") {
        json!(state.selected_reads.fetch_add(1, Ordering::SeqCst) > 0)
    } else if method == "POST" && path.ends_with("/execute/sync") {
        let script = body.get("script").and_then(Value::as_str).unwrap_or_default();
        if script.contains("window.scrollBy") {
            json!({"x": 17, "y": 29})
        } else if script.contains("MouseEvent") {
            Value::Null
        } else {
            json!({"tree": "mock snapshot"})
        }
    } else {
        Value::Null
    };

    Json(json!({"value": value}))
}

async fn opened_state(server: &MockWebDriver) -> NativeBrowserState {
    let mut state = NativeBrowserState::default();
    state
        .execute_action(
            BrowserAction::Open { url: "https://example.test/start".into() },
            true,
            &server.endpoint,
            None,
        )
        .await
        .expect("mock webdriver should open a page");
    state
}

#[test]
fn invalid_webdriver_endpoint_is_not_available() {
    assert!(!NativeBrowserState::is_available(true, "not a url", None));
}

#[tokio::test]
async fn reachable_webdriver_endpoint_is_available() {
    let server = MockWebDriver::start().await;

    assert!(NativeBrowserState::is_available(false, &server.endpoint, Some("ignored")));
}

#[tokio::test]
async fn open_action_reports_webdriver_connection_context() {
    let mut state = NativeBrowserState::default();

    let err = state
        .execute_action(
            BrowserAction::Open { url: "https://example.test/start".into() },
            true,
            "not a webdriver url",
            None,
        )
        .await
        .expect_err("invalid webdriver endpoint should prevent open");

    assert!(err.to_string().contains("Failed to connect to WebDriver"));
    assert!(err.to_string().contains("not a webdriver url"));
}

#[tokio::test]
async fn open_action_creates_session_and_reports_redirected_url() {
    let server = MockWebDriver::start().await;
    let mut state = NativeBrowserState::default();

    let value = state
        .execute_action(
            BrowserAction::Open { url: "https://example.test/start".into() },
            true,
            &server.endpoint,
            None,
        )
        .await
        .expect("open should use the mock webdriver");

    assert_eq!(value["backend"], "rust_native");
    assert_eq!(value["action"], "open");
    assert_eq!(value["url"], "https://example.test/landing");

    let navigation = server
        .requests()
        .into_iter()
        .find(|request| request.method == "POST" && request.path.ends_with("/url"))
        .expect("open should send a navigation request");
    assert_eq!(navigation.body["url"], "https://example.test/start");
}

#[tokio::test]
async fn page_read_actions_preserve_webdriver_values() {
    let server = MockWebDriver::start().await;
    let mut state = opened_state(&server).await;

    let title = state
        .execute_action(BrowserAction::GetTitle, true, &server.endpoint, None)
        .await
        .expect("title should be returned");
    let url = state
        .execute_action(BrowserAction::GetUrl, true, &server.endpoint, None)
        .await
        .expect("url should be returned");
    let snapshot = state
        .execute_action(
            BrowserAction::Snapshot { interactive_only: true, compact: false, depth: Some(3) },
            true,
            &server.endpoint,
            None,
        )
        .await
        .expect("snapshot should be returned");

    assert_eq!(
        title,
        json!({
            "backend": "rust_native",
            "action": "get_title",
            "title": "Mock page title",
        })
    );
    assert_eq!(
        url,
        json!({
            "backend": "rust_native",
            "action": "get_url",
            "url": "https://example.test/landing",
        })
    );
    assert_eq!(snapshot["backend"], "rust_native");
    assert_eq!(snapshot["action"], "snapshot");
    assert_eq!(snapshot["data"], json!({"tree": "mock snapshot"}));
}

#[tokio::test]
async fn element_actions_return_behavior_focused_payloads() {
    let server = MockWebDriver::start().await;
    let mut state = opened_state(&server).await;

    let click = state
        .execute_action(
            BrowserAction::Click { selector: "#submit".into() },
            true,
            &server.endpoint,
            None,
        )
        .await
        .expect("click should succeed");
    let fill = state
        .execute_action(
            BrowserAction::Fill { selector: "#name".into(), value: "Ada".into() },
            true,
            &server.endpoint,
            None,
        )
        .await
        .expect("fill should succeed");
    let typed = state
        .execute_action(
            BrowserAction::Type { selector: "#name".into(), text: "Rust".into() },
            true,
            &server.endpoint,
            None,
        )
        .await
        .expect("type should succeed");
    let text = state
        .execute_action(
            BrowserAction::GetText { selector: "main".into() },
            true,
            &server.endpoint,
            None,
        )
        .await
        .expect("text should be returned");
    let visible = state
        .execute_action(
            BrowserAction::IsVisible { selector: "#dialog".into() },
            true,
            &server.endpoint,
            None,
        )
        .await
        .expect("visibility should be returned");
    let hover = state
        .execute_action(
            BrowserAction::Hover { selector: "#menu".into() },
            true,
            &server.endpoint,
            None,
        )
        .await
        .expect("hover should succeed");

    assert_eq!(click["selector"], "#submit");
    assert_eq!(fill["action"], "fill");
    assert_eq!(fill["selector"], "#name");
    assert!(fill.get("value").is_none());
    assert_eq!(typed["typed"], 4);
    assert_eq!(text["text"], "Mock element text");
    assert_eq!(visible["visible"], true);
    assert_eq!(hover["action"], "hover");
    assert_eq!(hover["selector"], "#menu");
}

#[tokio::test]
async fn type_action_reports_utf8_byte_count_without_echoing_text() {
    let server = MockWebDriver::start().await;
    let mut state = opened_state(&server).await;
    let text = "é🙂";

    let typed = state
        .execute_action(
            BrowserAction::Type { selector: "#message".into(), text: text.into() },
            true,
            &server.endpoint,
            None,
        )
        .await
        .expect("unicode text should be sent through the mock webdriver");

    assert_eq!(typed["backend"], "rust_native");
    assert_eq!(typed["action"], "type");
    assert_eq!(typed["selector"], "#message");
    assert_eq!(typed["typed"], text.len());
    assert!(typed.get("text").is_none());
}

#[tokio::test]
async fn screenshot_without_path_returns_base64_and_metadata() {
    let server = MockWebDriver::start().await;
    let mut state = opened_state(&server).await;

    let value = state
        .execute_action(
            BrowserAction::Screenshot { path: None, full_page: true },
            true,
            &server.endpoint,
            None,
        )
        .await
        .expect("screenshot should return inline data");

    assert_eq!(value["action"], "screenshot");
    assert_eq!(value["full_page"], true);
    assert_eq!(value["bytes"], SCREENSHOT_BYTES.len());
    assert_eq!(value["png_base64"], SCREENSHOT_BASE64);
    assert!(value.get("path").is_none());
}

#[tokio::test]
async fn screenshot_with_path_writes_exact_png_bytes() {
    let server = MockWebDriver::start().await;
    let mut state = opened_state(&server).await;
    let directory = tempfile::tempdir().expect("temporary screenshot directory should exist");
    let path = directory.path().join("capture.png");
    let path_string = path.to_string_lossy().into_owned();

    let value = state
        .execute_action(
            BrowserAction::Screenshot { path: Some(path_string.clone()), full_page: false },
            true,
            &server.endpoint,
            None,
        )
        .await
        .expect("screenshot should be written to the temporary directory");

    let written = tokio::fs::read(&path).await.expect("written screenshot should be readable");
    assert_eq!(written, SCREENSHOT_BYTES);
    assert_eq!(value["path"], path_string);
    assert_eq!(value["bytes"], SCREENSHOT_BYTES.len());
    assert!(value.get("png_base64").is_none());
}

#[tokio::test]
async fn screenshot_with_directory_path_reports_write_context() {
    let server = MockWebDriver::start().await;
    let mut state = opened_state(&server).await;
    let directory = tempfile::tempdir().expect("temporary screenshot directory should exist");
    let directory_path = directory.path().to_string_lossy().into_owned();

    let err = state
        .execute_action(
            BrowserAction::Screenshot { path: Some(directory_path.clone()), full_page: false },
            true,
            &server.endpoint,
            None,
        )
        .await
        .expect_err("writing screenshot bytes to a directory should fail");

    assert!(err.to_string().contains("Failed to write screenshot to"));
    assert!(err.to_string().contains(&directory_path));
}

#[tokio::test(start_paused = true)]
async fn wait_action_covers_selector_delay_text_and_default_branches() {
    let server = MockWebDriver::start().await;
    let mut state = opened_state(&server).await;

    let selector = state
        .execute_action(
            BrowserAction::Wait {
                selector: Some("#ready".into()),
                ms: Some(999),
                text: Some("ignored".into()),
            },
            true,
            &server.endpoint,
            None,
        )
        .await
        .expect("selector should take precedence");
    let delay = state
        .execute_action(
            BrowserAction::Wait { selector: None, ms: Some(0), text: Some("ignored".into()) },
            true,
            &server.endpoint,
            None,
        )
        .await
        .expect("explicit delay should take precedence over text");
    let text = state
        .execute_action(
            BrowserAction::Wait { selector: None, ms: None, text: Some("Ready".into()) },
            true,
            &server.endpoint,
            None,
        )
        .await
        .expect("text wait should succeed");
    let default = state
        .execute_action(
            BrowserAction::Wait { selector: None, ms: None, text: None },
            true,
            &server.endpoint,
            None,
        )
        .await
        .expect("empty wait should use the default delay");

    assert_eq!(selector["selector"], "#ready");
    assert_eq!(delay["ms"], 0);
    assert_eq!(text["text"], "Ready");
    assert_eq!(default["ms"], 250);
}

#[tokio::test]
async fn press_action_uses_active_element_and_reports_key() {
    let server = MockWebDriver::start().await;
    let mut state = opened_state(&server).await;

    let value = state
        .execute_action(BrowserAction::Press { key: "Enter".into() }, true, &server.endpoint, None)
        .await
        .expect("press should send keys to the active element");

    assert_eq!(value["action"], "press");
    assert_eq!(value["key"], "Enter");
    assert!(
        server
            .requests()
            .iter()
            .any(|request| request.method == "GET" && request.path.ends_with("/element/active"))
    );
}

#[tokio::test]
async fn press_action_falls_back_to_body_when_active_element_is_unavailable() {
    let server = MockWebDriver::start().await;
    let mut state = opened_state(&server).await;
    server.state.active_element_missing.store(true, Ordering::SeqCst);

    let value = state
        .execute_action(BrowserAction::Press { key: "Escape".into() }, true, &server.endpoint, None)
        .await
        .expect("press should fall back to the body element");

    assert_eq!(value["key"], "Escape");
    assert!(server.requests().iter().any(|request| {
        request.method == "POST"
            && request.path.ends_with("/element")
            && request.body["value"] == "body"
    }));
}

#[tokio::test]
async fn scroll_action_maps_directions_defaults_and_rejects_unknown_values() {
    let server = MockWebDriver::start().await;
    let mut state = opened_state(&server).await;

    for (direction, pixels) in
        [("up", Some(25)), ("down", None), ("left", Some(10)), ("right", Some(20))]
    {
        let value = state
            .execute_action(
                BrowserAction::Scroll { direction: direction.into(), pixels },
                true,
                &server.endpoint,
                None,
            )
            .await
            .expect("supported scroll direction should succeed");
        assert_eq!(value["position"], json!({"x": 17, "y": 29}));
    }

    let scroll_arguments: Vec<Value> = server
        .requests()
        .into_iter()
        .filter(|request| {
            request.method == "POST"
                && request.path.ends_with("/execute/sync")
                && request.body["script"]
                    .as_str()
                    .is_some_and(|script| script.contains("window.scrollBy"))
        })
        .map(|request| request.body["args"].clone())
        .collect();
    assert_eq!(
        scroll_arguments,
        vec![json!([0, -25]), json!([0, 600]), json!([-10, 0]), json!([20, 0]),]
    );

    let err = state
        .execute_action(
            BrowserAction::Scroll { direction: "diagonal".into(), pixels: Some(1) },
            true,
            &server.endpoint,
            None,
        )
        .await
        .expect_err("unknown scroll direction should be rejected");
    assert!(err.to_string().contains("Unsupported scroll direction 'diagonal'"));
}

#[tokio::test]
async fn find_action_covers_supported_results_and_validation_errors() {
    let server = MockWebDriver::start().await;
    let mut state = opened_state(&server).await;

    let click = state
        .execute_action(
            BrowserAction::Find {
                by: "role".into(),
                value: "button".into(),
                action: "click".into(),
                fill_value: None,
            },
            true,
            &server.endpoint,
            None,
        )
        .await
        .expect("find click should succeed");
    let fill = state
        .execute_action(
            BrowserAction::Find {
                by: "label".into(),
                value: "Name".into(),
                action: "fill".into(),
                fill_value: Some("Ada".into()),
            },
            true,
            &server.endpoint,
            None,
        )
        .await
        .expect("find fill should succeed");
    let text = state
        .execute_action(
            BrowserAction::Find {
                by: "text".into(),
                value: "Welcome".into(),
                action: "text".into(),
                fill_value: None,
            },
            true,
            &server.endpoint,
            None,
        )
        .await
        .expect("find text should succeed");
    let hover = state
        .execute_action(
            BrowserAction::Find {
                by: "testid".into(),
                value: "menu".into(),
                action: "hover".into(),
                fill_value: None,
            },
            true,
            &server.endpoint,
            None,
        )
        .await
        .expect("find hover should succeed");
    let check = state
        .execute_action(
            BrowserAction::Find {
                by: "placeholder".into(),
                value: "Accept".into(),
                action: "check".into(),
                fill_value: None,
            },
            true,
            &server.endpoint,
            None,
        )
        .await
        .expect("find check should click an unchecked element");

    assert_eq!(click["data"], json!({"result": "clicked"}));
    assert_eq!(fill["data"], json!({"result": "filled", "typed": 3}));
    assert_eq!(text["data"], json!({"result": "text", "text": "Mock element text"}));
    assert_eq!(hover["data"], json!({"result": "hovered"}));
    assert_eq!(
        check["data"],
        json!({
            "result": "checked",
            "checked_before": false,
            "checked_after": true,
        })
    );

    let missing_fill = state
        .execute_action(
            BrowserAction::Find {
                by: "label".into(),
                value: "Name".into(),
                action: "fill".into(),
                fill_value: None,
            },
            true,
            &server.endpoint,
            None,
        )
        .await
        .expect_err("find fill without a value should be rejected");
    assert!(missing_fill.to_string().contains("requires fill_value"));

    let unsupported = state
        .execute_action(
            BrowserAction::Find {
                by: "role".into(),
                value: "button".into(),
                action: "drag".into(),
                fill_value: None,
            },
            true,
            &server.endpoint,
            None,
        )
        .await
        .expect_err("unknown find action should be rejected");
    assert!(unsupported.to_string().contains("Unsupported find_action 'drag'"));
}

#[tokio::test]
async fn find_check_skips_click_when_element_is_already_selected() {
    let server = MockWebDriver::start().await;
    let mut state = opened_state(&server).await;
    server.state.selected_reads.store(1, Ordering::SeqCst);

    let value = state
        .execute_action(
            BrowserAction::Find {
                by: "testid".into(),
                value: "terms".into(),
                action: "check".into(),
                fill_value: None,
            },
            true,
            &server.endpoint,
            None,
        )
        .await
        .expect("already selected element should remain selected");

    assert_eq!(
        value["data"],
        json!({
            "result": "checked",
            "checked_before": true,
            "checked_after": true,
        })
    );
    assert!(
        !server
            .requests()
            .iter()
            .any(|request| request.method == "POST" && request.path.ends_with("/click"))
    );
}

#[tokio::test]
async fn close_action_closes_existing_session_and_remains_idempotent() {
    let server = MockWebDriver::start().await;
    let mut state = opened_state(&server).await;

    let first = state
        .execute_action(BrowserAction::Close, true, &server.endpoint, None)
        .await
        .expect("existing session should close");
    let second = state
        .execute_action(BrowserAction::Close, true, &server.endpoint, None)
        .await
        .expect("closing an absent session should stay idempotent");
    let err = state
        .execute_action(BrowserAction::GetTitle, true, &server.endpoint, None)
        .await
        .expect_err("closed state should no longer expose a client");

    assert_eq!(first["closed"], true);
    assert_eq!(second["closed"], true);
    assert!(err.to_string().contains("Run browser action='open' first"));
    assert!(
        server
            .requests()
            .iter()
            .any(|request| request.method == "DELETE" && request.path.ends_with(MOCK_SESSION_ID))
    );
}

#[tokio::test]
async fn actions_requiring_session_report_missing_open_hint() {
    let mut state = NativeBrowserState::default();

    for action in [
        BrowserAction::GetTitle,
        BrowserAction::GetUrl,
        BrowserAction::Snapshot { interactive_only: true, compact: true, depth: None },
        BrowserAction::Click { selector: "#go".into() },
        BrowserAction::Fill { selector: "#name".into(), value: "Ada".into() },
        BrowserAction::Type { selector: "#name".into(), text: "Ada".into() },
        BrowserAction::GetText { selector: "main".into() },
        BrowserAction::Screenshot { path: None, full_page: false },
        BrowserAction::Wait { selector: Some("#ready".into()), ms: None, text: None },
        BrowserAction::Press { key: "Enter".into() },
        BrowserAction::Hover { selector: "#menu".into() },
        BrowserAction::Scroll { direction: "down".into(), pixels: Some(10) },
        BrowserAction::IsVisible { selector: "#modal".into() },
        BrowserAction::Find {
            by: "text".into(),
            value: "Save".into(),
            action: "click".into(),
            fill_value: None,
        },
    ] {
        let err = state
            .execute_action(action, true, "http://127.0.0.1:9", None)
            .await
            .expect_err("action should require an open session");
        assert!(err.to_string().contains("Run browser action='open' first"));
    }
}
