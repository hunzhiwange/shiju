//! Registry adaptation and tool call protocol helpers.
use crate::{Tool, ToolCallResult, ToolRenderHint, ToolResult, ToolSpec, ExecutedToolCall, ToolCallError, is_web_fetch_tool_id};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone)]
struct ArcDelegatingTool {
    inner: Arc<dyn Tool>,
}

impl ArcDelegatingTool {
    fn boxed(inner: Arc<dyn Tool>) -> Box<dyn Tool> {
        Box::new(Self { inner })
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for ArcDelegatingTool {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.inner.parameters_schema()
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        self.inner.execute(args).await
    }

    fn validate_input(&self, input: Value) -> anyhow::Result<Value> {
        self.inner.validate_input(input)
    }

    async fn check_permissions(&self, input: &Value) -> anyhow::Result<()> {
        self.inner.check_permissions(input).await
    }

    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        self.inner.call(input).await
    }

    fn map_result_for_model(&self, result: &ToolCallResult) -> Value {
        self.inner.map_result_for_model(result)
    }

    fn render_hint(&self, result: &ToolCallResult) -> Option<ToolRenderHint> {
        self.inner.render_hint(result)
    }

    fn is_concurrency_safe(&self) -> bool {
        self.inner.is_concurrency_safe()
    }

    fn is_read_only(&self) -> bool {
        self.inner.is_read_only()
    }

    fn to_audit_input(&self, input: &Value) -> Value {
        self.inner.to_audit_input(input)
    }

    fn spec(&self) -> ToolSpec {
        self.inner.spec()
    }
}

pub fn boxed_registry_from_arcs(tools: Vec<Arc<dyn Tool>>) -> Vec<Box<dyn Tool>> {
    tools.into_iter().map(ArcDelegatingTool::boxed).collect()
}

pub fn finish_tool_call(executed: ExecutedToolCall) -> Result<ToolCallResult, ToolCallError> {
    // A managed process terminal snapshot is protocol data even when the command
    // failed or was cancelled. Flattening it would discard its handle and stderr.
    let shell_snapshot =
        matches!(executed.tool_name.as_str(), "bash" | "bash_wait" | "bash_cancel")
            && executed.result.data.get("process_id").and_then(Value::as_str).is_some()
            && matches!(
                executed.result.data.get("state").and_then(Value::as_str),
                Some("running" | "exited" | "cancelled" | "timed_out")
            );
    if executed.result.is_success() || shell_snapshot {
        return Ok(executed.result);
    }

    Err(classify_message(
        executed.result.error_text().unwrap_or_else(|| "tool execution failed".to_string()),
    ))
}

/// 检测文件是否为二进制内容。
pub fn is_binary(path: &std::path::Path) -> bool {
    let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "zip" | "tar" | "gz" | "7z" | "exe" | "dll" | "so" | "class" | "jar" | "war" | "doc"
        | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "odp" | "bin" | "dat"
        | "obj" | "o" | "a" | "lib" | "wasm" | "pyc" | "pyo" => {
            return true;
        }
        _ => {}
    }

    use std::io::Read;

    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };

    let mut bytes = [0u8; 4096];
    let Ok(read_len) = file.read(&mut bytes) else {
        return false;
    };
    if read_len == 0 {
        return false;
    }

    let mut bad = 0usize;
    for byte in &bytes[..read_len] {
        if *byte == 0 {
            return true;
        }
        if *byte < 9 || (*byte > 13 && *byte < 32) {
            bad += 1;
        }
    }

    (bad as f32) / (read_len as f32) > 0.3
}

pub fn parse_tool_input(tool_id: &str, raw_input: &str) -> Result<Value, ToolCallError> {
    let raw = raw_input.trim();
    if raw.is_empty() {
        return Ok(serde_json::json!({}));
    }

    if raw.starts_with('{') {
        return serde_json::from_str::<Value>(raw)
            .map_err(|error| ToolCallError::Failed(format!("invalid JSON arguments: {error}")));
    }

    if let Some(default_key) = default_scalar_input_key(tool_id) {
        return Ok(serde_json::json!({ default_key: raw }));
    }

    Err(ToolCallError::Failed("invalid arguments: expected JSON object".to_string()))
}

fn default_scalar_input_key(tool_id: &str) -> Option<&'static str> {
    match tool_id {
        "file_read" | "file_write" => Some("path"),
        id if is_web_fetch_tool_id(id) => Some("url"),
        _ => None,
    }
}

pub fn classify_message(message: String) -> ToolCallError {
    if is_denied_error(&message) {
        ToolCallError::denied(message)
    } else {
        ToolCallError::Failed(message)
    }
}

fn is_denied_error(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("denied")
        || lower.contains("not allowed")
        || lower.contains("forbidden")
        || lower.contains("blocked")
}
