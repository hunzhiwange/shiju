//! 本地代码节点执行。

use super::code_runner_http::HttpBridge;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::Write;
use std::process::Stdio;
use std::time::Duration;
use tempfile::Builder;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout;

pub(super) const CODE_TIMEOUT_SECS: u64 = 60;
const MAX_ERROR_CHARS: usize = 4_000;
const SENSITIVE_KEY_MARKERS: [&str; 7] =
    ["token", "secret", "password", "api_key", "authorization", "auth", "skey"];
const PYTHON_REQUESTS_SHIM: &str = r#"
import http.client as __vw_http_client
import json as __vw_json
import os as __vw_os
import sys as __vw_sys
import types as __vw_types
import urllib.parse as __vw_urlparse

class __VwRequestException(Exception):
    pass

class __VwTimeout(__VwRequestException):
    pass

class __VwHttpError(__VwRequestException):
    def __init__(self, message, response=None):
        super().__init__(message)
        self.response = response

_vw_json = __vw_json
_vw_http_error = __VwHttpError

class __VwResponse:
    def __init__(self, status_code, text, headers, url):
        self.status_code = status_code
        self.text = text
        self.headers = headers
        self.url = url
        self.content = text.encode("utf-8")

    def json(self):
        return _vw_json.loads(self.text)

    def raise_for_status(self):
        if 400 <= self.status_code:
            raise _vw_http_error(f"HTTP {self.status_code} for {self.url}", self)

def __vw_timeout_seconds(timeout):
    if timeout is None:
        return None
    if isinstance(timeout, (int, float)):
        return float(timeout)
    if isinstance(timeout, (list, tuple)) and timeout:
        return float(max(timeout))
    raise __VwRequestException("unsupported timeout value")

def __vw_bridge_request(method, url, headers=None, params=None, timeout=None, data=None, json=None, **kwargs):
    if kwargs:
        unsupported = ", ".join(sorted(kwargs.keys()))
        raise __VwRequestException(f"unsupported requests arguments: {unsupported}")
    endpoint = __vw_os.environ.get("VW_WORKFLOW_HTTP_BRIDGE")
    if not endpoint:
        raise __VwRequestException("workflow HTTP bridge is unavailable")
    bridge_token = __vw_os.environ.get("VW_WORKFLOW_HTTP_BRIDGE_TOKEN")
    if not bridge_token:
        raise __VwRequestException("workflow HTTP bridge token is unavailable")
    parsed = __vw_urlparse.urlparse(endpoint)
    timeout_seconds = __vw_timeout_seconds(timeout)
    body = __vw_json.dumps({
        "bridge_token": bridge_token,
        "method": method,
        "url": url,
        "headers": headers or {},
        "params": params,
        "timeout_secs": timeout_seconds,
        "body": data,
        "json": json,
    }, ensure_ascii=False, default=str).encode("utf-8")
    connection = __vw_http_client.HTTPConnection(
        parsed.hostname,
        parsed.port,
        timeout=(timeout_seconds + 5.0) if timeout_seconds else None,
    )
    try:
        connection.request(
            "POST",
            parsed.path or "/",
            body=body,
            headers={"Content-Type": "application/json", "Content-Length": str(len(body))},
        )
        response = connection.getresponse()
        payload = __vw_json.loads(response.read().decode("utf-8"))
    finally:
        connection.close()
    if not payload.get("ok"):
        message = payload.get("error") or "workflow HTTP request failed"
        if payload.get("timeout"):
            raise __VwTimeout(message)
        raise __VwRequestException(message)
    return __VwResponse(
        payload.get("status_code") or 0,
        payload.get("text") or "",
        payload.get("headers") or {},
        payload.get("url") or url,
    )

def __vw_requests_get(url, **kwargs):
    return __vw_bridge_request("GET", url, **kwargs)

def __vw_requests_post(url, **kwargs):
    return __vw_bridge_request("POST", url, **kwargs)

def __vw_requests_put(url, **kwargs):
    return __vw_bridge_request("PUT", url, **kwargs)

def __vw_requests_delete(url, **kwargs):
    return __vw_bridge_request("DELETE", url, **kwargs)

def __vw_requests_patch(url, **kwargs):
    return __vw_bridge_request("PATCH", url, **kwargs)

def __vw_requests_head(url, **kwargs):
    return __vw_bridge_request("HEAD", url, **kwargs)

def __vw_requests_options(url, **kwargs):
    return __vw_bridge_request("OPTIONS", url, **kwargs)

__vw_requests_module = __vw_types.ModuleType("requests")
__vw_requests_module.request = __vw_bridge_request
__vw_requests_module.get = __vw_requests_get
__vw_requests_module.post = __vw_requests_post
__vw_requests_module.put = __vw_requests_put
__vw_requests_module.delete = __vw_requests_delete
__vw_requests_module.patch = __vw_requests_patch
__vw_requests_module.head = __vw_requests_head
__vw_requests_module.options = __vw_requests_options
__vw_requests_module.exceptions = __vw_types.SimpleNamespace(
    RequestException=__VwRequestException,
    Timeout=__VwTimeout,
    HTTPError=__VwHttpError,
)
__vw_sys.modules["requests"] = __vw_requests_module
"#;

#[derive(Debug, Deserialize)]
struct CodeEnvelope {
    ok: bool,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<String>,
}

pub(crate) async fn run_code_node(
    language: &str,
    code: &str,
    inputs: &BTreeMap<String, Value>,
) -> Result<BTreeMap<String, Value>, String> {
    let language = language.trim().to_ascii_lowercase();
    if language.contains("python") {
        run_python_script(code, inputs).await
    } else if language.contains("javascript") || language.contains("node") || language == "js" {
        run_script("node", ".js", &javascript_script(code), inputs, &[]).await
    } else {
        Err(format!("不支持的 code_language: {language}"))
    }
}

async fn run_python_script(
    code: &str,
    inputs: &BTreeMap<String, Value>,
) -> Result<BTreeMap<String, Value>, String> {
    let bridge = HttpBridge::start().await?;
    let endpoint = bridge.endpoint().to_string();
    let token = bridge.token().to_string();
    let result = run_script(
        "python3",
        ".py",
        &python_script(code),
        inputs,
        &[
            ("VW_WORKFLOW_HTTP_BRIDGE", endpoint.as_str()),
            ("VW_WORKFLOW_HTTP_BRIDGE_TOKEN", token.as_str()),
        ],
    )
    .await;
    bridge.stop().await;
    result
}

async fn run_script(
    program: &str,
    suffix: &str,
    script: &str,
    inputs: &BTreeMap<String, Value>,
    envs: &[(&str, &str)],
) -> Result<BTreeMap<String, Value>, String> {
    let mut file = Builder::new()
        .prefix("vw-workflow-code-")
        .suffix(suffix)
        .tempfile()
        .map_err(|error| error.to_string())?;
    file.write_all(script.as_bytes()).map_err(|error| error.to_string())?;
    file.flush().map_err(|error| error.to_string())?;
    let path = file.path().to_path_buf();
    let payload = serde_json::json!({ "inputs": inputs });
    let payload = serde_json::to_vec(&payload).map_err(|error| error.to_string())?;

    let mut command = Command::new(program);
    command
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    for (key, value) in envs {
        command.env(key, value);
    }
    let mut child = command.spawn().map_err(|error| format!("启动 {program} 失败: {error}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(&payload).await.map_err(|error| error.to_string())?;
    }

    let output = timeout(Duration::from_secs(CODE_TIMEOUT_SECS), child.wait_with_output())
        .await
        .map_err(|_| format!("code 节点执行超时: {CODE_TIMEOUT_SECS}s"))?
        .map_err(|error| error.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let envelope = parse_last_json_line(&stdout).ok_or_else(|| {
        format!(
            "code 节点没有返回合法 JSON: {}",
            sanitize_error_text(format!("stdout={stdout}; stderr={stderr}"), inputs)
        )
    })?;

    if !envelope.ok || !output.status.success() {
        let message = envelope.error.unwrap_or_else(|| "code 节点执行失败".to_string());
        return Err(sanitize_error_text(format!("{message}; stderr={stderr}"), inputs));
    }

    Ok(value_to_output_map(envelope.result.unwrap_or(Value::Null)))
}

fn parse_last_json_line(stdout: &str) -> Option<CodeEnvelope> {
    stdout
        .lines()
        .rev()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .find_map(|line| serde_json::from_str::<CodeEnvelope>(line).ok())
}

fn value_to_output_map(value: Value) -> BTreeMap<String, Value> {
    match value {
        Value::Object(object) => object.into_iter().collect(),
        other => BTreeMap::from([("result".to_string(), other)]),
    }
}

fn truncate_error(value: String) -> String {
    value.chars().take(MAX_ERROR_CHARS).collect()
}

fn sanitize_error_text(value: String, inputs: &BTreeMap<String, Value>) -> String {
    let mut redacted = value;
    for (key, value) in inputs {
        if !is_sensitive_key(key) {
            continue;
        }
        redact_value_text(value, &mut redacted);
    }
    truncate_error(redacted)
}

fn is_sensitive_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    SENSITIVE_KEY_MARKERS.iter().any(|marker| lower.contains(marker))
}

fn redact_value_text(value: &Value, text: &mut String) {
    match value {
        Value::String(secret) if !secret.is_empty() => {
            *text = text.replace(secret, "[REDACTED]");
        }
        Value::Array(items) => {
            for item in items {
                redact_value_text(item, text);
            }
        }
        Value::Object(object) => {
            for value in object.values() {
                redact_value_text(value, text);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn python_script(code: &str) -> String {
    let mut script = String::with_capacity(PYTHON_REQUESTS_SHIM.len() + code.len() + 512);
    script.push_str(PYTHON_REQUESTS_SHIM);
    script.push('\n');
    script.push_str(code);
    script.push_str(
        r#"

if __name__ == "__main__":
    import json
    import sys
    import traceback

    try:
        payload = json.load(sys.stdin)
        inputs = payload.get("inputs") or {}
        result = main(**inputs)
        print(json.dumps({"ok": True, "result": result}, ensure_ascii=False, default=str))
    except Exception as exc:
        traceback.print_exc(file=sys.stderr)
        print(json.dumps({"ok": False, "error": str(exc)}, ensure_ascii=False))
        sys.exit(1)
"#,
    );
    script
}

fn javascript_script(code: &str) -> String {
    format!(
        r#"{code}

const __vwReadStdin = () => new Promise((resolve) => {{
  let data = "";
  process.stdin.setEncoding("utf8");
  process.stdin.on("data", chunk => data += chunk);
  process.stdin.on("end", () => resolve(data));
}});

(async () => {{
  try {{
    const payloadText = await __vwReadStdin();
    const payload = payloadText.trim() ? JSON.parse(payloadText) : {{}};
    const fn = typeof main === "function" ? main : module.exports && module.exports.main;
    if (typeof fn !== "function") {{
      throw new Error("main function is required");
    }}
    const result = await fn(payload.inputs || {{}});
    console.log(JSON.stringify({{ ok: true, result }}));
  }} catch (error) {{
    console.error(error && error.stack ? error.stack : String(error));
    console.log(JSON.stringify({{ ok: false, error: error && error.message ? error.message : String(error) }}));
    process.exit(1);
  }}
}})();
"#
    )
}

#[cfg(test)]
#[path = "code_runner_tests.rs"]
mod code_runner_tests;
