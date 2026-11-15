//! Workflow code 节点的 Rust HTTP bridge。

use axum::{Json, Router, extract::State, routing::post};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use uuid::Uuid;

use super::code_runner::CODE_TIMEOUT_SECS;

const HTTP_BRIDGE_DEBUG_MAX_CHARS: usize = 4_000;
const SENSITIVE_KEY_MARKERS: [&str; 9] = [
    "token",
    "secret",
    "password",
    "api_key",
    "authorization",
    "auth",
    "skey",
    "cookie",
    "session",
];

#[derive(Clone)]
struct HttpBridgeState {
    client: reqwest::Client,
    token: String,
}

#[derive(Clone, Debug, Deserialize)]
struct HttpBridgeRequest {
    bridge_token: String,
    method: String,
    url: String,
    #[serde(default)]
    headers: BTreeMap<String, String>,
    #[serde(default)]
    params: Option<Value>,
    #[serde(default)]
    timeout_secs: Option<f64>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default, rename = "json")]
    json_body: Option<Value>,
}

#[derive(Debug, Serialize)]
struct HttpBridgeResponse {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    status_code: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    headers: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    timeout: bool,
}

#[derive(Debug)]
struct HttpBridgeError {
    message: String,
    timeout: bool,
}

pub(super) struct HttpBridge {
    endpoint: String,
    token: String,
    shutdown: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

impl HttpBridge {
    pub(super) async fn start() -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .user_agent("VibeWindow Workflow CodeRunner")
            .timeout(Duration::from_secs(CODE_TIMEOUT_SECS))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| format!("初始化 workflow HTTP bridge 失败: {error}"))?;
        let token = Uuid::new_v4().to_string();
        let state = HttpBridgeState { client, token: token.clone() };
        let app = Router::new().route("/", post(handle_http_bridge_request)).with_state(state);
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .map_err(|error| format!("启动 workflow HTTP bridge 失败: {error}"))?;
        let addr = listener
            .local_addr()
            .map_err(|error| format!("读取 workflow HTTP bridge 地址失败: {error}"))?;
        let endpoint = format!("http://{addr}/");
        let (shutdown, shutdown_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            let server = axum::serve(listener, app).with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            });
            if let Err(error) = server.await {
                tracing::debug!("workflow HTTP bridge stopped with error: {error}");
            }
        });
        Ok(Self { endpoint, token, shutdown: Some(shutdown), task })
    }

    pub(super) fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub(super) fn token(&self) -> &str {
        &self.token
    }

    pub(super) async fn stop(mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        let _ = self.task.await;
    }
}

async fn handle_http_bridge_request(
    State(state): State<HttpBridgeState>,
    Json(request): Json<HttpBridgeRequest>,
) -> Json<HttpBridgeResponse> {
    if request.bridge_token != state.token {
        return Json(HttpBridgeResponse {
            ok: false,
            status_code: None,
            text: None,
            headers: BTreeMap::new(),
            url: None,
            error: Some("workflow HTTP bridge token mismatch".to_string()),
            timeout: false,
        });
    }
    match execute_http_bridge_request(&state.client, request).await {
        Ok(response) => Json(response),
        Err(error) => Json(HttpBridgeResponse {
            ok: false,
            status_code: None,
            text: None,
            headers: BTreeMap::new(),
            url: None,
            error: Some(error.message),
            timeout: error.timeout,
        }),
    }
}

async fn execute_http_bridge_request(
    client: &reqwest::Client,
    request: HttpBridgeRequest,
) -> Result<HttpBridgeResponse, HttpBridgeError> {
    let method_text = request.method.to_ascii_uppercase();
    let method = http_method(&request.method)?;
    let url = append_query_params(&request.url, request.params.as_ref())?;
    let debug_headers = debug_json_value(
        &serde_json::to_value(redact_string_map(&request.headers)).unwrap_or(Value::Null),
    );
    let debug_params =
        request.params.as_ref().map(debug_redacted_value).unwrap_or_else(|| "null".to_string());
    let debug_body = request.body.as_ref().map(|body| debug_body_text(body));
    let debug_json =
        request.json_body.as_ref().map(debug_redacted_value).unwrap_or_else(|| "null".to_string());
    tracing::debug!(
        target: "vw_agent::workflow::http_bridge",
        method = %method_text,
        url = %redact_url_for_log(&url),
        headers = %debug_headers,
        params = %debug_params,
        timeout_secs = ?request.timeout_secs,
        body = ?debug_body,
        json = %debug_json,
        "workflow HTTP bridge request"
    );
    let mut builder = client.request(method, url.clone());
    if let Some(timeout_secs) = request.timeout_secs {
        if !timeout_secs.is_finite() || timeout_secs <= 0.0 {
            return Err(http_bridge_error("HTTP timeout must be a positive number"));
        }
        builder = builder.timeout(Duration::from_secs_f64(timeout_secs));
    }
    for (key, value) in request.headers {
        builder = builder.header(&key, value);
    }
    if let Some(json_body) = request.json_body {
        builder = builder.json(&json_body);
    } else if let Some(body) = request.body {
        builder = builder.body(body);
    }
    let response = match builder.send().await {
        Ok(response) => response,
        Err(error) => {
            let error = reqwest_bridge_error(error);
            tracing::debug!(
                target: "vw_agent::workflow::http_bridge",
                method = %method_text,
                url = %redact_url_for_log(&url),
                error = %error.message,
                timeout = error.timeout,
                "workflow HTTP bridge request failed"
            );
            return Err(error);
        }
    };
    let status_code = response.status().as_u16();
    let response_url = response.url().to_string();
    let headers = response_headers(response.headers());
    let text = response.text().await.map_err(reqwest_bridge_error)?;
    let debug_response_headers =
        debug_json_value(&serde_json::to_value(redact_string_map(&headers)).unwrap_or(Value::Null));
    let debug_response_body = debug_body_text(&text);
    tracing::debug!(
        target: "vw_agent::workflow::http_bridge",
        method = %method_text,
        url = %redact_url_for_log(&response_url),
        status_code,
        headers = %debug_response_headers,
        body = %debug_response_body,
        "workflow HTTP bridge response"
    );
    Ok(HttpBridgeResponse {
        ok: true,
        status_code: Some(status_code),
        text: Some(text),
        headers,
        url: Some(response_url),
        error: None,
        timeout: false,
    })
}

fn http_method(method: &str) -> Result<reqwest::Method, HttpBridgeError> {
    match method.to_ascii_uppercase().as_str() {
        "GET" => Ok(reqwest::Method::GET),
        "POST" => Ok(reqwest::Method::POST),
        "PUT" => Ok(reqwest::Method::PUT),
        "DELETE" => Ok(reqwest::Method::DELETE),
        "PATCH" => Ok(reqwest::Method::PATCH),
        "HEAD" => Ok(reqwest::Method::HEAD),
        "OPTIONS" => Ok(reqwest::Method::OPTIONS),
        _ => Err(http_bridge_error(format!("Unsupported HTTP method: {method}"))),
    }
}

fn append_query_params(raw_url: &str, params: Option<&Value>) -> Result<String, HttpBridgeError> {
    let Some(params) = params else {
        return Ok(raw_url.to_string());
    };
    let mut url = reqwest::Url::parse(raw_url)
        .map_err(|error| http_bridge_error(format!("Invalid HTTP URL: {error}")))?;
    let query_params = query_param_pairs(params)?;
    {
        let mut pairs = url.query_pairs_mut();
        for (key, value) in query_params {
            pairs.append_pair(&key, &value);
        }
    }
    Ok(url.to_string())
}

fn query_param_pairs(params: &Value) -> Result<Vec<(String, String)>, HttpBridgeError> {
    let mut pairs = Vec::new();
    match params {
        Value::Object(object) => {
            for (key, value) in object {
                push_query_pair_values(&mut pairs, key, value);
            }
        }
        Value::Array(items) => {
            for item in items {
                let Some(pair) = item.as_array().filter(|pair| pair.len() == 2) else {
                    return Err(http_bridge_error("params array items must be [key, value]"));
                };
                let Some(key) = pair[0].as_str() else {
                    return Err(http_bridge_error("params array keys must be strings"));
                };
                push_query_pair_values(&mut pairs, key, &pair[1]);
            }
        }
        _ => return Err(http_bridge_error("params must be an object or array of pairs")),
    }
    Ok(pairs)
}

fn push_query_pair_values(pairs: &mut Vec<(String, String)>, key: &str, value: &Value) {
    match value {
        Value::Array(items) => {
            for item in items {
                pairs.push((key.to_string(), query_value_text(item)));
            }
        }
        other => {
            pairs.push((key.to_string(), query_value_text(other)));
        }
    }
}

fn query_value_text(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => "None".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
    }
}

fn response_headers(headers: &reqwest::header::HeaderMap) -> BTreeMap<String, String> {
    headers
        .iter()
        .filter_map(|(key, value)| Some((key.to_string(), value.to_str().ok()?.to_string())))
        .collect()
}

fn reqwest_bridge_error(error: reqwest::Error) -> HttpBridgeError {
    HttpBridgeError { message: error.to_string(), timeout: error.is_timeout() }
}

fn http_bridge_error(message: impl Into<String>) -> HttpBridgeError {
    HttpBridgeError { message: message.into(), timeout: false }
}

fn debug_redacted_value(value: &Value) -> String {
    debug_json_value(&redact_value("value", value))
}

fn debug_body_text(text: &str) -> String {
    let value = serde_json::from_str::<Value>(text)
        .map(|value| redact_value("body", &value))
        .unwrap_or_else(|_| Value::String(redact_form_text(text)));
    debug_json_value(&value)
}

fn debug_json_value(value: &Value) -> String {
    let text =
        serde_json::to_string(value).unwrap_or_else(|_| "<json serialize failed>".to_string());
    truncate_debug_text(text)
}

fn truncate_debug_text(value: String) -> String {
    if value.chars().count() <= HTTP_BRIDGE_DEBUG_MAX_CHARS {
        return value;
    }
    let mut truncated: String = value.chars().take(HTTP_BRIDGE_DEBUG_MAX_CHARS).collect();
    truncated.push_str("...");
    truncated
}

fn redact_string_map(values: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    values
        .iter()
        .map(|(key, value)| {
            let value =
                if is_sensitive_key(key) { "[REDACTED]".to_string() } else { value.clone() };
            (key.clone(), value)
        })
        .collect()
}

fn redact_value(key: &str, value: &Value) -> Value {
    if is_sensitive_key(key) {
        return Value::String("[REDACTED]".to_string());
    }
    match value {
        Value::Object(object) => Value::Object(
            object.iter().map(|(key, value)| (key.clone(), redact_value(key, value))).collect(),
        ),
        Value::Array(items) => {
            Value::Array(items.iter().map(|item| redact_value(key, item)).collect())
        }
        other => other.clone(),
    }
}

fn redact_url_for_log(raw_url: &str) -> String {
    let Ok(mut url) = reqwest::Url::parse(raw_url) else {
        return truncate_debug_text(raw_url.to_string());
    };
    if !url.username().is_empty() {
        let _ = url.set_username("[REDACTED]");
    }
    if url.password().is_some() {
        let _ = url.set_password(Some("[REDACTED]"));
    }
    let query_pairs = url
        .query_pairs()
        .map(|(key, value)| {
            let value =
                if is_sensitive_key(&key) { "[REDACTED]".to_string() } else { value.into_owned() };
            (key.into_owned(), value)
        })
        .collect::<Vec<_>>();
    if !query_pairs.is_empty() {
        url.set_query(None);
        {
            let mut pairs = url.query_pairs_mut();
            for (key, value) in query_pairs {
                pairs.append_pair(&key, &value);
            }
        }
    }
    truncate_debug_text(url.to_string())
}

fn redact_form_text(text: &str) -> String {
    if !text.contains('=') {
        return text.to_string();
    }
    text.split('&')
        .map(|part| {
            let Some((key, _)) = part.split_once('=') else {
                return part.to_string();
            };
            if is_sensitive_key(key) { format!("{key}=[REDACTED]") } else { part.to_string() }
        })
        .collect::<Vec<_>>()
        .join("&")
}

fn is_sensitive_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    SENSITIVE_KEY_MARKERS.iter().any(|marker| lower.contains(marker))
}

#[cfg(test)]
#[path = "code_runner_http_tests.rs"]
mod code_runner_http_tests;
