//! 文本、JSON 与静默模式的输出格式化实现。
//!
//! 本模块负责把 ACP 运行时产生的消息流转换为适合终端用户消费的输出。
//! 它同时支持人类可读文本、结构化 JSON 以及静默模式三种呈现方式。
//!
//! # 主要职责
//!
//! - 跟踪会话过程中的 JSON-RPC 消息和错误事件
//! - 把消息映射为稳定的终端输出格式
//! - 在需要时抑制重复、冗余或读类工具输出
//! - 为 CLI 层提供统一的格式化器抽象

use std::collections::HashMap;
use std::io::Write;

use serde_json::{Map, Value};

use crate::read_output_suppression::{ReadLikeToolDescriptor, is_read_like_tool};
use crate::types::{
    AcpJsonRpcMessage, OutputErrorParams, OutputFormat, OutputFormatter, OutputFormatterContext,
};
use crate::{
    JsonOutputFormatter, SUPPRESSED_READ_OUTPUT, parse_json_rpc_error_message,
    parse_prompt_stop_reason,
};

const MAX_OUTPUT_LENGTH: usize = 2_000;
const INDENT: &str = "  ";

fn json_rpc_id_key(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Number(n)) => Some(n.to_string()),
        _ => None,
    }
}

#[derive(Debug, Clone, Default)]
pub struct OutputFormatterOptions {
    pub context: Option<OutputFormatterContext>,
    pub suppress_reads: bool,
    pub is_tty: bool,
}

pub enum AnyOutputFormatter<W: Write> {
    Text(TextOutputFormatter<W>),
    Json(JsonOutputFormatter<W>),
    Quiet(QuietOutputFormatter<W>),
}

impl<W: Write> AnyOutputFormatter<W> {
    pub fn into_inner(self) -> W {
        match self {
            Self::Text(formatter) => formatter.into_inner(),
            Self::Json(formatter) => formatter.into_inner(),
            Self::Quiet(formatter) => formatter.into_inner(),
        }
    }
}

impl<W: Write> OutputFormatter for AnyOutputFormatter<W> {
    fn set_context(&mut self, context: OutputFormatterContext) {
        match self {
            Self::Text(formatter) => formatter.set_context(context),
            Self::Json(formatter) => formatter.set_context(context),
            Self::Quiet(formatter) => formatter.set_context(context),
        }
    }

    fn on_acp_message(&mut self, message: AcpJsonRpcMessage) {
        match self {
            Self::Text(formatter) => formatter.on_acp_message(message),
            Self::Json(formatter) => formatter.on_acp_message(message),
            Self::Quiet(formatter) => formatter.on_acp_message(message),
        }
    }

    fn on_error(&mut self, params: OutputErrorParams) {
        match self {
            Self::Text(formatter) => formatter.on_error(params),
            Self::Json(formatter) => formatter.on_error(params),
            Self::Quiet(formatter) => formatter.on_error(params),
        }
    }

    fn flush(&mut self) {
        match self {
            Self::Text(formatter) => formatter.flush(),
            Self::Json(formatter) => formatter.flush(),
            Self::Quiet(formatter) => formatter.flush(),
        }
    }
}

pub fn create_output_formatter<W: Write>(
    format: OutputFormat,
    stdout: W,
    options: OutputFormatterOptions,
) -> AnyOutputFormatter<W> {
    match format {
        OutputFormat::Text => AnyOutputFormatter::Text(TextOutputFormatter::new(stdout, options)),
        OutputFormat::Json => AnyOutputFormatter::Json(JsonOutputFormatter::new(
            stdout,
            options.suppress_reads,
            options.context,
        )),
        OutputFormat::Quiet => {
            AnyOutputFormatter::Quiet(QuietOutputFormatter::new(stdout, options.context))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolStatus {
    Running,
    Completed,
    Failed,
}

impl ToolStatus {
    fn from_value(value: Option<&str>) -> Self {
        match value {
            Some("completed") => Self::Completed,
            Some("failed") => Self::Failed,
            _ => Self::Running,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Default)]
struct ToolRenderState {
    title: Option<String>,
    kind: Option<String>,
    status: Option<String>,
    locations: Vec<Value>,
    raw_input: Option<Value>,
    raw_output: Option<Value>,
    content: Vec<Value>,
}

pub struct TextOutputFormatter<W: Write> {
    stdout: W,
    context: Option<OutputFormatterContext>,
    suppress_reads: bool,
    is_tty: bool,
    chunk_buffer: String,
    thought_buffer: String,
    tool_states: HashMap<String, ToolRenderState>,
    last_load_request_id: Option<String>,
    last_client_op: Option<String>,
}

impl<W: Write> TextOutputFormatter<W> {
    pub fn new(stdout: W, options: OutputFormatterOptions) -> Self {
        Self {
            stdout,
            context: options.context,
            suppress_reads: options.suppress_reads,
            is_tty: options.is_tty,
            chunk_buffer: String::new(),
            thought_buffer: String::new(),
            tool_states: HashMap::new(),
            last_load_request_id: None,
            last_client_op: None,
        }
    }

    pub fn into_inner(self) -> W {
        self.stdout
    }

    fn write_line(&mut self, line: impl AsRef<str>) {
        if !self.chunk_buffer.is_empty() && !self.chunk_buffer.ends_with('\n') {
            let _ = writeln!(self.stdout);
            self.chunk_buffer.push('\n');
        }
        let _ = writeln!(self.stdout, "{}", line.as_ref());
        let _ = self.stdout.flush();
    }

    fn write_block(&mut self, block: impl AsRef<str>) {
        let _ = self.stdout.write_all(block.as_ref().as_bytes());
        let _ = self.stdout.flush();
    }

    fn flush_chunk_buffer(&mut self) {
        if self.chunk_buffer.is_empty() {
            return;
        }
        if !self.chunk_buffer.ends_with('\n') {
            let _ = writeln!(self.stdout);
            let _ = self.stdout.flush();
        }
        self.chunk_buffer.clear();
    }

    fn flush_thought_buffer(&mut self) {
        if self.thought_buffer.is_empty() {
            return;
        }
        let content = std::mem::take(&mut self.thought_buffer);
        self.write_line(format!("\x1b[90m[thought]\n{content}\x1b[0m"));
    }

    fn colorize(&self, code: &str, text: &str) -> String {
        if self.is_tty { format!("\x1b[{code}m{text}\x1b[0m") } else { text.to_string() }
    }

    fn render_tool_update(&mut self, update: &Map<String, Value>) {
        let Some(tool_call_id) = update.get("toolCallId").and_then(Value::as_str) else {
            return;
        };

        let snapshot = {
            let state = self.tool_states.entry(tool_call_id.to_string()).or_default();
            if let Some(title) = update.get("title").and_then(Value::as_str) {
                state.title = Some(title.to_string());
            }
            if let Some(kind) = update.get("kind").and_then(Value::as_str) {
                state.kind = Some(kind.to_string());
            }
            if let Some(status) = update.get("status").and_then(Value::as_str) {
                state.status = Some(status.to_string());
            }
            if let Some(locations) = update.get("locations").and_then(Value::as_array) {
                state.locations = locations.clone();
            }
            if let Some(raw_input) = update.get("rawInput") {
                state.raw_input = Some(raw_input.clone());
            }
            if let Some(raw_output) = update.get("rawOutput") {
                state.raw_output = Some(raw_output.clone());
            }
            if let Some(content) = update.get("content").and_then(Value::as_array) {
                state.content = content.clone();
            }
            state.clone()
        };

        let title = snapshot.title.clone().unwrap_or_else(|| "Tool".to_string());
        let status = ToolStatus::from_value(snapshot.status.as_deref());
        let status_text = match status {
            ToolStatus::Running => self.colorize("33", status.label()),
            ToolStatus::Completed => self.colorize("32", status.label()),
            ToolStatus::Failed => self.colorize("31", status.label()),
        };
        self.write_line(format!("{} {} ({status_text})", self.colorize("1", "[tool]"), title));

        if let Some(input_summary) = summarize_tool_input(snapshot.raw_input.as_ref()) {
            self.write_line(format!("{INDENT}{input_summary}"));
        }

        let locations = format_locations(&snapshot.locations);
        if !locations.is_empty() {
            self.write_line(format!("{INDENT}locations: {locations}"));
        }

        if matches!(status, ToolStatus::Completed | ToolStatus::Failed) {
            if let Some(output) = render_tool_output(
                self.suppress_reads,
                ToolDescriptorView {
                    title: snapshot.title.as_deref(),
                    kind: snapshot.kind.as_deref(),
                },
                snapshot.raw_output.as_ref(),
                &snapshot.content,
            ) {
                self.write_line(format!("{INDENT}{output}"));
            }
            self.tool_states.remove(tool_call_id);
        }
    }

    fn render_plan_update(&mut self, update: &Map<String, Value>) {
        let Some(entries) = update.get("entries").and_then(Value::as_array) else {
            return;
        };
        self.write_line(format!("{} plan updated", self.colorize("1", "[plan]")));
        for entry in entries {
            let status = entry.get("status").and_then(Value::as_str).unwrap_or("pending");
            let content = entry.get("content").and_then(Value::as_str).unwrap_or_default();
            if !content.is_empty() {
                self.write_line(format!("{INDENT}- [{status}] {content}"));
            }
        }
    }

    fn render_done(&mut self, stop_reason: &str) {
        self.write_line(self.colorize("2", &format!("[done] {stop_reason}")));
    }

    fn render_error_message(&mut self, message: &str) {
        self.write_line(format!("{} {}", self.colorize("31", "[error]"), message.trim()));
    }

    fn render_client_operation(&mut self, method: &str) {
        self.write_line(format!(
            "{} {}",
            self.colorize("1", "[client]"),
            self.colorize("33", &format!("{method} (running)"))
        ));
    }
}

impl<W: Write> OutputFormatter for TextOutputFormatter<W> {
    fn set_context(&mut self, context: OutputFormatterContext) {
        self.context = Some(context);
    }

    fn on_acp_message(&mut self, message: AcpJsonRpcMessage) {
        let value = serde_json::to_value(message).unwrap_or(Value::Null);

        if let Some(update) = session_update(&value) {
            let update_type =
                update.get("sessionUpdate").and_then(Value::as_str).unwrap_or_default();

            if update_type != "agent_thought_chunk" && !self.thought_buffer.is_empty() {
                self.flush_thought_buffer();
            }
            if update_type != "agent_message_chunk" && !self.chunk_buffer.is_empty() {
                self.flush_chunk_buffer();
            }

            match update_type {
                "agent_message_chunk" => {
                    if let Some(content) = update.get("content").and_then(extract_text_content) {
                        self.write_block(content);
                        self.chunk_buffer.push_str(content);
                    }
                }
                "agent_thought_chunk" => {
                    if let Some(content) = update.get("content").and_then(extract_text_content) {
                        self.thought_buffer.push_str(content);
                    }
                }
                "tool_call" | "tool_call_update" => {
                    self.flush_chunk_buffer();
                    self.render_tool_update(update);
                }
                "plan" => {
                    self.flush_chunk_buffer();
                    self.render_plan_update(update);
                }
                _ => {}
            }
            return;
        }

        if let Some(stop_reason) = parse_prompt_stop_reason(&value) {
            self.flush_thought_buffer();
            self.flush_chunk_buffer();
            self.render_done(&stop_reason);
            return;
        }

        if let Some(error_message) = parse_json_rpc_error_message(&value) {
            let id = json_rpc_id_key(value.get("id"));
            let is_load_error = id.is_some() && id == self.last_load_request_id;

            if !is_load_error && !error_message.to_lowercase().contains("internal error") {
                self.flush_thought_buffer();
                self.flush_chunk_buffer();
                self.render_error_message(&error_message);
            }
            return;
        }

        if let Some(method) = value.get("method").and_then(Value::as_str) {
            if method != "session/prompt"
                && method != "session/cancel"
                && method != "session/update"
            {
                self.flush_thought_buffer();
                self.flush_chunk_buffer();

                // TS behavior: it logs all client operations EXCEPT those handled completely silently
                // But it DOES log `initialize`, `session/new`, `session/load` etc. initially
                // The spam issue comes from `prompt_runner.rs` emitting multiple callbacks or retries.
                // Here we just render what we get, but hide repeated identical methods
                if self.last_client_op.as_deref() == Some(method) {
                    // Skip consecutive identical operations (e.g. initialize, initialize)
                } else if self.last_load_request_id.is_some()
                    && (method == "initialize"
                        || method == "session/new"
                        || method == "session/load"
                        || method == "session/resume")
                {
                    // Also hide silent background reconnects if a load/resume already failed
                } else {
                    self.last_client_op = Some(method.to_string());
                    self.render_client_operation(method);
                }
            } else {
                self.last_client_op = None; // Reset on normal prompt traffic
            }
            if method == "session/load" || method == "session/resume" {
                self.last_load_request_id = json_rpc_id_key(value.get("id"));
            }
        }
    }

    fn on_error(&mut self, params: OutputErrorParams) {
        self.flush_thought_buffer();
        self.flush_chunk_buffer();
        let label = self.colorize("31", "[error]");
        if let Some(detail_code) = params.detail_code {
            self.write_line(format!("{label} {} ({detail_code})", params.message));
        } else {
            self.write_line(format!("{label} {}", params.message));
        }
    }

    fn flush(&mut self) {
        self.flush_thought_buffer();
        self.flush_chunk_buffer();
        let _ = self.stdout.flush();
    }
}

pub struct QuietOutputFormatter<W: Write> {
    stdout: W,
    context: Option<OutputFormatterContext>,
    output: String,
}

impl<W: Write> QuietOutputFormatter<W> {
    pub fn new(stdout: W, context: Option<OutputFormatterContext>) -> Self {
        Self { stdout, context, output: String::new() }
    }

    pub fn into_inner(self) -> W {
        self.stdout
    }
}

impl<W: Write> OutputFormatter for QuietOutputFormatter<W> {
    fn set_context(&mut self, context: OutputFormatterContext) {
        self.context = Some(context);
    }

    fn on_acp_message(&mut self, message: AcpJsonRpcMessage) {
        let value = serde_json::to_value(message).unwrap_or(Value::Null);

        if let Some(update) = session_update(&value) {
            let update_type =
                update.get("sessionUpdate").and_then(Value::as_str).unwrap_or_default();
            if update_type == "agent_message_chunk" {
                if let Some(content) = update.get("content").and_then(extract_text_content) {
                    self.output.push_str(content);
                    let _ = self.stdout.write_all(content.as_bytes());
                }
            } else if update_type == "agent_thought_chunk"
                && let Some(content) = update.get("content").and_then(Value::as_str)
            {
                self.output.push_str(content);
                let _ = self.stdout.write_all(content.as_bytes());
            }
        }

        if parse_prompt_stop_reason(&value).is_some() && !self.output.trim().is_empty() {
            if !self.output.ends_with('\n') {
                let _ = writeln!(self.stdout);
                let _ = self.stdout.flush();
            }
            self.output.clear();
        }
    }

    fn on_error(&mut self, params: OutputErrorParams) {
        let _ = params;
    }

    fn flush(&mut self) {
        let _ = self.stdout.flush();
    }
}

struct ToolDescriptorView<'a> {
    title: Option<&'a str>,
    kind: Option<&'a str>,
}

fn session_update(value: &Value) -> Option<&Map<String, Value>> {
    let params = value.get("params")?.as_object()?;
    let update = params.get("update")?;
    update.as_object()
}

fn extract_text_content(value: &Value) -> Option<&str> {
    let object = value.as_object()?;
    let content_type = object.get("type").and_then(Value::as_str)?;
    match content_type {
        "text" => object.get("text").and_then(Value::as_str),
        "resource_link" => object.get("uri").and_then(Value::as_str),
        "resource" => object
            .get("resource")
            .and_then(Value::as_object)
            .and_then(|resource| resource.get("uri"))
            .and_then(Value::as_str),
        _ => None,
    }
}

fn format_locations(locations: &[Value]) -> String {
    locations
        .iter()
        .filter_map(|location| {
            let path = location.get("path").and_then(Value::as_str)?;
            let line = location.get("line").and_then(Value::as_u64)?;
            Some(format!("{path}:{line}"))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn summarize_tool_input(raw_input: Option<&Value>) -> Option<String> {
    let raw_input = raw_input?;
    let Some(object) = raw_input.as_object() else {
        let rendered = truncate_json(raw_input);
        return (!rendered.is_empty()).then(|| format!("input: {rendered}"));
    };

    let parts = ["command", "pattern", "query", "prompt", "path", "url"]
        .into_iter()
        .filter_map(|key| {
            object.get(key).map(|value| {
                let rendered = match value {
                    Value::String(text) => text.clone(),
                    _ => truncate_json(value),
                };
                format!("{key}={rendered}")
            })
        })
        .collect::<Vec<_>>();

    if !parts.is_empty() {
        return Some(format!("input: {}", parts.join(", ")));
    }

    let rendered = truncate_json(raw_input);
    (!rendered.is_empty()).then(|| format!("input: {rendered}"))
}

fn render_tool_output(
    suppress_reads: bool,
    descriptor: ToolDescriptorView<'_>,
    raw_output: Option<&Value>,
    content: &[Value],
) -> Option<String> {
    if suppress_reads
        && is_read_like_tool(&ReadLikeToolDescriptor {
            title: descriptor.title.map(str::to_string),
            kind: descriptor.kind.map(str::to_string),
        })
    {
        return Some(SUPPRESSED_READ_OUTPUT.to_string());
    }

    let output =
        raw_output.and_then(extract_output_text).or_else(|| summarize_tool_content(content))?;
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(truncate_text(trimmed))
}

fn summarize_tool_content(content: &[Value]) -> Option<String> {
    for item in content {
        let item_type = item.get("type").and_then(Value::as_str).unwrap_or_default();
        match item_type {
            "content" => {
                if let Some(content) = item.get("content").and_then(extract_text_content)
                    && !content.trim().is_empty()
                {
                    return Some(content.to_string());
                }
            }
            "diff" => {
                if let Some(diff) = item.get("diff").and_then(Value::as_str)
                    && !diff.trim().is_empty()
                {
                    return Some(diff.to_string());
                }
            }
            "terminal" => {
                if let Some(output) = item.get("output").and_then(Value::as_str)
                    && !output.trim().is_empty()
                {
                    return Some(output.to_string());
                }
            }
            _ => {}
        }
    }
    None
}

fn extract_output_text(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(boolean) => Some(boolean.to_string()),
        Value::Array(values) => values.iter().find_map(extract_output_text),
        Value::Object(object) => {
            for key in
                ["model_result", "output", "content", "text", "stdout", "stderr", "message", "data"]
            {
                if let Some(extracted) = object.get(key).and_then(extract_output_text)
                    && !extracted.trim().is_empty()
                {
                    return Some(extracted);
                }
            }
            let rendered = truncate_json(value);
            (!rendered.is_empty()).then_some(rendered)
        }
    }
}

fn truncate_json(value: &Value) -> String {
    let rendered = serde_json::to_string(value).unwrap_or_default();
    truncate_text(&rendered)
}

fn truncate_text(text: &str) -> String {
    if text.len() <= MAX_OUTPUT_LENGTH {
        return text.to_string();
    }

    let mut end = MAX_OUTPUT_LENGTH.min(text.len());
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }

    format!("{}...", &text[..end])
}

#[cfg(test)]
#[path = "output_coverage_tests.rs"]
mod output_coverage_tests;
#[cfg(test)]
#[path = "output_tests.rs"]
mod output_tests;
