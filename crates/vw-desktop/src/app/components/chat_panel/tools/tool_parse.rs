//! 工具解析模块
//!
//! 本模块提供用于解析和处理工具调用相关的辅助函数集合。
//! 主要功能包括：
//! - 从原始工具调用文本中提取工具名称和标识符
//! - 判断工具类型（如探索类工具）
//! - 处理工具输入参数的解析
//! - 解析和规范化文件路径
//!
//! 这些函数主要用于聊天面板中对工具调用的解析、显示和处理。

use serde_json::{Map, Value};

use crate::app::components::chat_panel::utils::normalize_file_reference_to_path;

use super::super::tool_names::canonical_tool_name;
use super::types::{ChangeFile, ChangeFileSummary};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExploreToolKind {
    Read,
    Search,
    List,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AgentToolSessionStatus {
    Running,
    Waiting,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AgentToolSessionPayload {
    pub parent_session_id: Option<String>,
    pub child_session_id: String,
    pub agent: String,
    pub task: String,
    pub status: AgentToolSessionStatus,
}

fn normalized_object_string(object: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .filter_map(|key| object.get(*key))
        .filter_map(Value::as_str)
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(str::to_string)
}

fn embedded_json_object(value: Option<&Value>) -> Option<Value> {
    match value? {
        Value::Object(_) => value.cloned(),
        Value::String(raw) => {
            serde_json::from_str::<Value>(raw.trim()).ok().filter(|parsed| parsed.is_object())
        }
        _ => None,
    }
}

fn agent_tool_payload_from_value(value: &Value) -> Option<AgentToolSessionPayload> {
    let status = match tool_status(value).trim().to_ascii_lowercase().as_str() {
        "running" => AgentToolSessionStatus::Running,
        "waiting" => AgentToolSessionStatus::Waiting,
        "completed" => AgentToolSessionStatus::Completed,
        "error" | "failed" | "denied" => AgentToolSessionStatus::Failed,
        "cancelled" => AgentToolSessionStatus::Cancelled,
        _ => return None,
    };

    let output_payload = embedded_json_object(value.get("output"));
    let result_payload = tool_result_data(value).and_then(|data| {
        if data.is_object()
            && (data.get("child_session_id").is_some() || data.get("session_id").is_some())
        {
            Some(data.clone())
        } else {
            embedded_json_object(data.get("output"))
        }
    });
    let input_payload = embedded_json_object(value.get("input").or_else(|| value.get("rawInput")));

    let mut objects = Vec::new();
    if let Some(metadata) = value.get("metadata").and_then(Value::as_object) {
        objects.push(metadata);
    }
    if let Some(metadata) = tool_render_hint_metadata(value) {
        objects.push(metadata);
    }
    if let Some(object) = output_payload.as_ref().and_then(Value::as_object) {
        objects.push(object);
    }
    if let Some(object) = result_payload.as_ref().and_then(Value::as_object) {
        objects.push(object);
    }

    let child_session_id = objects
        .iter()
        .find_map(|object| normalized_object_string(object, &["child_session_id", "session_id"]))?;
    let parent_session_id =
        objects.iter().find_map(|object| normalized_object_string(object, &["parent_session_id"]));
    let agent = objects
        .iter()
        .find_map(|object| normalized_object_string(object, &["agent"]))
        .or_else(|| {
            input_payload
                .as_ref()
                .and_then(Value::as_object)
                .and_then(|object| normalized_object_string(object, &["agent"]))
        })
        .unwrap_or_default();
    let task = objects
        .iter()
        .find_map(|object| normalized_object_string(object, &["task"]))
        .or_else(|| {
            input_payload
                .as_ref()
                .and_then(Value::as_object)
                .and_then(|object| normalized_object_string(object, &["task", "prompt"]))
        })
        .unwrap_or_default();

    Some(AgentToolSessionPayload { parent_session_id, child_session_id, agent, task, status })
}

fn agent_tool_status_from_task(
    task: &Value,
    action: Option<&str>,
) -> Option<AgentToolSessionStatus> {
    match task
        .get("state")
        .or_else(|| task.get("status"))
        .and_then(Value::as_str)?
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "queued" | "running" if action == Some("status") => Some(AgentToolSessionStatus::Running),
        "queued" | "running" => Some(AgentToolSessionStatus::Waiting),
        "succeeded" | "completed" => Some(AgentToolSessionStatus::Completed),
        "error" | "failed" | "denied" => Some(AgentToolSessionStatus::Failed),
        "cancelled" => Some(AgentToolSessionStatus::Cancelled),
        _ => None,
    }
}

fn agent_tool_payload_from_task(
    task: &Value,
    action: Option<&str>,
) -> Option<AgentToolSessionPayload> {
    let object = task.as_object()?;
    let child_session_id = normalized_object_string(object, &["child_session_id", "session_id"])?;
    Some(AgentToolSessionPayload {
        parent_session_id: normalized_object_string(object, &["parent_session_id"]),
        child_session_id,
        agent: normalized_object_string(object, &["agent"]).unwrap_or_default(),
        task: normalized_object_string(object, &["task", "prompt"]).unwrap_or_default(),
        status: agent_tool_status_from_task(task, action)?,
    })
}

fn agent_tool_structured_bodies(value: &Value) -> Vec<Value> {
    let mut bodies = Vec::new();
    if value.is_object() {
        bodies.push(value.clone());
    }
    if let Some(output) = embedded_json_object(value.get("output")) {
        bodies.push(output);
    }
    if let Some(data) = tool_result_data(value) {
        if data.is_object() {
            bodies.push(data.clone());
        }
        if let Some(output) = embedded_json_object(data.get("output")) {
            bodies.push(output);
        }
    }
    bodies
}

fn agent_tool_payloads_from_value(value: &Value) -> Vec<AgentToolSessionPayload> {
    let mut payloads = Vec::new();
    let mut positions = std::collections::HashMap::<String, usize>::new();
    for body in agent_tool_structured_bodies(value) {
        let action =
            body.get("action").and_then(Value::as_str).map(str::trim).map(str::to_ascii_lowercase);
        let Some(tasks) = body.get("tasks").and_then(Value::as_array) else {
            continue;
        };
        for task in tasks {
            let Some(payload) = agent_tool_payload_from_task(task, action.as_deref()) else {
                continue;
            };
            if let Some(position) = positions.get(&payload.child_session_id).copied() {
                payloads[position] = payload;
            } else {
                positions.insert(payload.child_session_id.clone(), payloads.len());
                payloads.push(payload);
            }
        }
    }
    if payloads.is_empty() {
        agent_tool_payload_from_value(value).into_iter().collect()
    } else {
        payloads
    }
}

fn tool_block_value_at(content: &str, start: usize) -> Option<(usize, &str, Value)> {
    let tail = content.get(start..)?;
    let line_end = tail.find('\n')?;
    let tool_name = tail[..line_end].trim().strip_prefix("tool ")?.trim();
    if tool_name.is_empty() {
        return None;
    }

    let json_start = start + line_end + 1;
    let mut values =
        serde_json::Deserializer::from_str(content.get(json_start..)?).into_iter::<Value>();
    let value = values.next()?.ok()?;
    let end = json_start + values.byte_offset();
    Some((end, tool_name, value))
}

fn is_tool_block_boundary(content: &str, start: usize) -> bool {
    if start == 0 || content[..start].ends_with('\n') {
        return true;
    }

    let prefix = content[..start].trim_end_matches([' ', '\t']);
    prefix.ends_with(':') || prefix.ends_with('：')
}

pub(crate) fn agent_tool_session_payloads_from_content(
    content: &str,
) -> Vec<AgentToolSessionPayload> {
    let mut payloads = Vec::new();
    let mut cursor = 0;

    while let Some(relative_start) = content[cursor..].find("tool ") {
        let start = cursor + relative_start;
        if !is_tool_block_boundary(content, start) {
            cursor = start + "tool ".len();
            continue;
        }
        let Some((end, tool_name, value)) = tool_block_value_at(content, start) else {
            cursor = start + "tool ".len();
            continue;
        };
        cursor = end.max(start + "tool ".len());

        if canonical_tool_name(tool_name) != "AgentTool" {
            continue;
        }
        payloads.extend(agent_tool_payloads_from_value(&value));
    }

    payloads
}

fn canonical_tool_name_from_value(value: &Value) -> Option<String> {
    render_hint_object(value)
        .and_then(|hint| hint.get("metadata"))
        .and_then(Value::as_object)
        .and_then(|metadata| metadata.get("canonical_tool_id"))
        .and_then(Value::as_str)
        .map(canonical_tool_name)
        .map(ToOwned::to_owned)
}

/// 从原始工具调用文本中提取工具名称
///
/// 原始文本格式通常为 "tool <tool_name>\n<json_params>"。
/// 此函数从第一行提取工具名称部分。
///
/// # 参数
///
/// * `raw` - 原始工具调用文本字符串的引用
///
/// # 返回值
///
/// * `Some(String)` - 成功提取的工具名称
/// * `None` - 如果格式不正确或工具名称为空
///
/// # 示例
///
/// ```ignore
/// let raw = "tool read\n{\"file\": \"test.txt\"}";
/// assert_eq!(tool_name_from_raw(raw), Some("read"));
///
/// let invalid = "invalid format";
/// assert_eq!(tool_name_from_raw(invalid), None);
/// ```
pub fn tool_name_from_raw(raw: &str) -> Option<String> {
    // 按换行符分割，获取第一行（包含工具声明）
    let (first, rest) = raw.split_once('\n')?;

    // 去除 "tool " 前缀并清理空白字符
    let tool_name = first.trim().strip_prefix("tool ")?.trim();

    // 确保工具名称非空
    if tool_name.is_empty() {
        return None;
    }

    if let Ok(value) = serde_json::from_str::<Value>(rest.trim()) {
        if let Some(canonical) = canonical_tool_name_from_value(&value) {
            return Some(canonical);
        }
    }

    Some(canonical_tool_name(tool_name).to_string())
}

fn call_id_from_object(object: &Map<String, Value>) -> Option<String> {
    ["tool_call_id", "toolCallId", "call_id", "callId", "tool_use_id", "toolUseId"]
        .into_iter()
        .filter_map(|key| object.get(key))
        .filter_map(Value::as_str)
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn call_id_from_value(value: &Value) -> Option<String> {
    value
        .as_object()
        .and_then(call_id_from_object)
        .or_else(|| value.get("metadata").and_then(Value::as_object).and_then(call_id_from_object))
        .or_else(|| render_hint_object(value).and_then(call_id_from_object))
        .or_else(|| {
            render_hint_object(value)
                .and_then(|hint| hint.get("metadata"))
                .and_then(Value::as_object)
                .and_then(call_id_from_object)
        })
        .or_else(|| result_object(value).and_then(call_id_from_object))
        .or_else(|| {
            result_object(value)
                .and_then(|result| result.get("metadata"))
                .and_then(Value::as_object)
                .and_then(call_id_from_object)
        })
}

pub fn tool_call_id_from_raw(raw: &str) -> Option<String> {
    let (_, rest) = raw.split_once('\n')?;
    let value = serde_json::from_str::<Value>(rest.trim()).ok()?;
    call_id_from_value(&value)
}

pub(crate) fn tool_status_from_raw(raw: &str) -> Option<String> {
    let (_, rest) = raw.split_once('\n')?;
    let value = serde_json::from_str::<Value>(rest.trim()).ok()?;
    Some(tool_status(&value).to_string())
}

pub(crate) fn explore_item_dedupe_key(raw: &str) -> Option<String> {
    tool_call_id_from_raw(raw)
        .map(|call_id| format!("call:{call_id}"))
        .or_else(|| tool_identity_from_raw(raw).map(|identity| format!("identity:{identity}")))
}

/// 判断指定工具是否为探索类工具
///
/// 探索类工具主要用于代码库探索、文件搜索和内容检索，
/// 当前主表面以 read / glob / grep / lsp 为主，并兼容历史消息中的旧搜索别名。
///
/// # 参数
///
/// * `tool_name` - 工具名称字符串
///
/// # 返回值
///
/// * `true` - 如果工具是探索类工具
/// * `false` - 否则
///
/// # 示例
///
/// ```ignore
/// assert!(is_explore_tool("read"));
/// assert!(is_explore_tool("grep"));
/// assert!(!is_explore_tool("bash"));
/// ```
pub fn explore_tool_kind(tool_name: &str) -> Option<ExploreToolKind> {
    let normalized = canonical_tool_name(tool_name).to_ascii_lowercase();
    match normalized.as_str() {
        "read" | "file_read" | "pdf_read" | "read_file" => Some(ExploreToolKind::Read),
        "grep"
        | "glob"
        | "lsp"
        | "glob_search"
        | "codesearch"
        | "content_search"
        | "searchcodebase"
        | "grep_search"
        | "file_search"
        | "semantic_search"
        | "fetch_webpage"
        | "github_repo"
        | "copilot_getnotebooksummary"
        | "vscode_listcodeusages" => Some(ExploreToolKind::Search),
        "ls" | "list_dir" => Some(ExploreToolKind::List),
        _ => None,
    }
}

pub fn is_explore_tool(tool_name: &str) -> bool {
    explore_tool_kind(tool_name).is_some()
}

fn result_object(value: &Value) -> Option<&Map<String, Value>> {
    value.get("result").and_then(Value::as_object)
}

pub(crate) fn tool_result_data(value: &Value) -> Option<&Value> {
    result_object(value).and_then(|result| result.get("data")).or_else(|| value.get("data"))
}

fn render_hint_object(value: &Value) -> Option<&Map<String, Value>> {
    value
        .get("renderHint")
        .and_then(Value::as_object)
        .or_else(|| value.get("render_hint").and_then(Value::as_object))
        .or_else(|| {
            result_object(value).and_then(|result| {
                result
                    .get("render_hint")
                    .and_then(Value::as_object)
                    .or_else(|| result.get("renderHint").and_then(Value::as_object))
            })
        })
}

pub(crate) fn tool_render_hint_metadata(value: &Value) -> Option<&Map<String, Value>> {
    render_hint_object(value).and_then(|hint| hint.get("metadata")).and_then(Value::as_object)
}

fn extract_text_value(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(boolean) => Some(boolean.to_string()),
        Value::Array(values) => values.iter().find_map(extract_text_value),
        Value::Object(object) => {
            for key in [
                "text",
                "message",
                "output",
                "stdout",
                "stderr",
                "model_result",
                "content",
                "data",
                "value",
            ] {
                if let Some(text) = object.get(key).and_then(extract_text_value)
                    && !text.trim().is_empty()
                {
                    return Some(text);
                }
            }
            None
        }
    }
}

fn result_content(value: &Value) -> Option<&Vec<Value>> {
    result_object(value)?.get("content")?.as_array()
}

fn tool_result_content_text(value: &Value) -> Option<String> {
    for block in result_content(value)? {
        let block_type = block.get("type").and_then(Value::as_str).unwrap_or_default();
        match block_type {
            "text" => {
                if let Some(text) = block.get("text").and_then(Value::as_str)
                    && !text.trim().is_empty()
                {
                    return Some(text.to_string());
                }
            }
            "json" => {
                if let Some(text) = block.get("value").and_then(extract_text_value)
                    && !text.trim().is_empty()
                {
                    return Some(text);
                }
            }
            _ => {}
        }
    }
    None
}

#[derive(Clone)]
struct StructuredPatchHunkView {
    path: String,
    header: String,
    lines: Vec<String>,
}

fn structured_patch_hunks(value: &Value) -> Vec<StructuredPatchHunkView> {
    let mut hunks = Vec::new();
    let Some(content) = result_content(value) else {
        return hunks;
    };

    for block in content {
        if block.get("type").and_then(Value::as_str) != Some("structured_patch") {
            continue;
        }
        let Some(raw_hunks) = block.get("hunks").and_then(Value::as_array) else {
            continue;
        };
        for raw_hunk in raw_hunks {
            let Some(path) = raw_hunk.get("path").and_then(Value::as_str).map(str::trim) else {
                continue;
            };
            if path.is_empty() {
                continue;
            }
            let header =
                raw_hunk.get("header").and_then(Value::as_str).unwrap_or_default().to_string();
            let lines = raw_hunk
                .get("lines")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            hunks.push(StructuredPatchHunkView { path: path.to_string(), header, lines });
        }
    }

    hunks
}

pub fn tool_status(value: &Value) -> &str {
    if let Some(status) = value.get("status").and_then(Value::as_str) {
        return status;
    }
    match result_object(value).and_then(|result| result.get("success")).and_then(Value::as_bool) {
        Some(true) => "completed",
        Some(false) => "error",
        None => "",
    }
}

pub fn tool_input(value: &Value) -> &str {
    value.get("input").or_else(|| value.get("rawInput")).and_then(Value::as_str).unwrap_or("")
}

pub fn tool_output_text(value: &Value) -> Option<String> {
    value
        .get("output")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
        .or_else(|| tool_result_content_text(value))
        .or_else(|| {
            result_object(value)
                .and_then(|result| result.get("model_result"))
                .and_then(extract_text_value)
        })
        .or_else(|| {
            result_object(value).and_then(|result| result.get("data")).and_then(extract_text_value)
        })
        .or_else(|| tool_structured_diff_text(value))
}

pub fn tool_error_text(value: &Value) -> Option<String> {
    value
        .get("error")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
        .or_else(|| if tool_status(value) == "error" { tool_output_text(value) } else { None })
}

pub fn tool_summary_text(value: &Value) -> Option<String> {
    value
        .get("summary")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            render_hint_object(value)
                .and_then(|hint| hint.get("summary"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .map(ToString::to_string)
        })
}

pub fn tool_output_path(value: &Value) -> Option<String> {
    value
        .pointer("/metadata/outputPath")
        .and_then(Value::as_str)
        .and_then(normalize_file_reference_to_path)
        .or_else(|| {
            render_hint_object(value)
                .and_then(|hint| hint.get("metadata"))
                .and_then(|metadata| metadata.get("outputPath"))
                .and_then(Value::as_str)
                .and_then(normalize_file_reference_to_path)
        })
}

pub fn tool_change_files(value: &Value) -> Vec<ChangeFile> {
    let mut files: Vec<ChangeFile> = Vec::new();

    for hunk in structured_patch_hunks(value) {
        let Some(existing) = files.iter_mut().find(|file| file.path == hunk.path) else {
            let mut file = ChangeFile {
                path: hunk.path.clone(),
                additions: 0,
                deletions: 0,
                before: String::new(),
                after: String::new(),
            };
            for line in &hunk.lines {
                match line.chars().next() {
                    Some('+') => {
                        file.additions += 1;
                        file.after.push_str(&line[1..]);
                        file.after.push('\n');
                    }
                    Some('-') => {
                        file.deletions += 1;
                        file.before.push_str(&line[1..]);
                        file.before.push('\n');
                    }
                    _ => {
                        let content = line.strip_prefix(' ').unwrap_or(line.as_str());
                        file.before.push_str(content);
                        file.before.push('\n');
                        file.after.push_str(content);
                        file.after.push('\n');
                    }
                }
            }
            files.push(file);
            continue;
        };

        for line in &hunk.lines {
            match line.chars().next() {
                Some('+') => {
                    existing.additions += 1;
                    existing.after.push_str(&line[1..]);
                    existing.after.push('\n');
                }
                Some('-') => {
                    existing.deletions += 1;
                    existing.before.push_str(&line[1..]);
                    existing.before.push('\n');
                }
                _ => {
                    let content = line.strip_prefix(' ').unwrap_or(line.as_str());
                    existing.before.push_str(content);
                    existing.before.push('\n');
                    existing.after.push_str(content);
                    existing.after.push('\n');
                }
            }
        }
    }

    files
}

pub fn tool_change_file_summaries(value: &Value) -> Vec<ChangeFileSummary> {
    tool_change_files(value)
        .into_iter()
        .map(|file| ChangeFileSummary {
            kind: if file.before.is_empty() {
                'A'
            } else if file.after.is_empty() {
                'D'
            } else {
                'M'
            },
            path: file.path,
            additions: file.additions,
            deletions: file.deletions,
        })
        .collect()
}

pub fn tool_structured_diff_text(value: &Value) -> Option<String> {
    let hunks = structured_patch_hunks(value);
    if hunks.is_empty() {
        return None;
    }

    let mut diff = String::new();
    let mut current_path = String::new();
    for hunk in hunks {
        if hunk.path != current_path {
            if !diff.is_empty() && !diff.ends_with('\n') {
                diff.push('\n');
            }
            diff.push_str("--- a/");
            diff.push_str(&hunk.path);
            diff.push('\n');
            diff.push_str("+++ b/");
            diff.push_str(&hunk.path);
            diff.push('\n');
            current_path = hunk.path.clone();
        }
        if !hunk.header.is_empty() {
            diff.push_str(&hunk.header);
            diff.push('\n');
        }
        for line in hunk.lines {
            diff.push_str(&line);
            diff.push('\n');
        }
    }

    Some(diff)
}

/// 从原始工具调用文本中提取完整的工具标识符
///
/// 此函数不仅提取工具名称，还解析参数并生成一个唯一的标识符字符串。
/// 对于某些特殊工具（如 bash、grep），会根据其参数内容生成更具体的标识符。
///
/// # 参数
///
/// * `raw` - 原始工具调用文本
///
/// # 返回值
///
/// * `Some(String)` - 工具标识符字符串，格式为 "<tool_name>:<identity>"
/// * `None` - 如果格式不正确或解析失败
///
/// # 特殊处理
///
/// - **bash 工具**：标识符格式为 "bash:<command>"
/// - **grep 工具**：标识符格式为 "grep:<pattern>|<include>|<path>"
/// - **其他工具**：标识符格式为 "<tool_name>:<input>"
///
/// # 示例
///
/// ```ignore
/// let raw = "tool bash\n{\"input\": \"ls -la\"}";
/// assert_eq!(tool_identity_from_raw(raw), Some("bash:ls -la".to_string()));
/// ```
pub fn tool_identity_from_raw(raw: &str) -> Option<String> {
    // 分割第一行和剩余内容
    let (first, rest) = raw.split_once('\n')?;

    // 提取工具名称
    let tool_name = canonical_tool_name(first.trim().strip_prefix("tool ")?.trim());
    if tool_name.is_empty() {
        return None;
    }

    // 解析 JSON 参数部分
    let v = serde_json::from_str::<Value>(rest.trim()).ok()?;
    let input = v.get("input").and_then(|vv| vv.as_str()).unwrap_or("").trim();

    // 处理 bash 工具的特殊标识符逻辑
    if tool_name == "bash" {
        // 判断输入是否为 JSON 格式
        let cmd = if input.trim_start().starts_with('{') {
            // 从 JSON 中提取 command 字段
            serde_json::from_str::<Value>(input)
                .ok()
                .and_then(|vv| vv.get("command").and_then(|x| x.as_str()).map(|s| s.to_string()))
                .unwrap_or_default()
        } else {
            // 直接使用输入作为命令
            input.to_string()
        };
        return Some(format!("bash:{}", cmd.trim()));
    }

    // 处理 grep 工具的特殊标识符逻辑
    if tool_name == "grep" {
        let ident = if input.trim_start().starts_with('{') {
            // 从 JSON 中提取 grep 的各个参数
            let vv = serde_json::from_str::<Value>(input).ok();
            let pattern = vv
                .as_ref()
                .and_then(|x| x.get("pattern").and_then(|v| v.as_str()))
                .unwrap_or("")
                .trim();
            let include = vv
                .as_ref()
                .and_then(|x| x.get("include").and_then(|v| v.as_str()))
                .unwrap_or("")
                .trim();
            let path = vv
                .as_ref()
                .and_then(|x| x.get("path").and_then(|v| v.as_str()))
                .unwrap_or("")
                .trim();
            // 组合成 "pattern|include|path" 格式
            format!("{}|{}|{}", pattern, include, path)
        } else {
            input.to_string()
        };
        return Some(format!("grep:{}", ident));
    }

    if let Some(path) = edit_tool_identity_path(tool_name, &v, input) {
        let identity_tool = match tool_name {
            "write" | "file_write" => "file_write",
            _ => tool_name,
        };
        return Some(format!("{}:{}", identity_tool, path.trim()));
    }

    // 默认标识符格式：工具名:输入内容
    Some(format!("{}:{}", tool_name, input))
}

fn edit_tool_identity_path(tool_name: &str, value: &Value, input: &str) -> Option<String> {
    if !matches!(tool_name, "write" | "file_write" | "file_edit" | "notebook_edit") {
        return None;
    }

    tool_input_path(input).or_else(|| {
        value
            .pointer("/metadata/path")
            .and_then(Value::as_str)
            .and_then(normalize_file_reference_to_path)
    })
}

/// 判断是否应该隐藏工具块
///
/// 某些工具调用（如已完成的 todo 操作）在界面上应该被隐藏，
/// 以减少视觉噪音。此函数根据工具类型和状态判断是否隐藏。
///
/// # 参数
///
/// * `raw` - 原始工具调用文本
///
/// # 返回值
///
/// * `true` - 应该隐藏该工具块
/// * `false` - 应该显示该工具块
///
/// # 隐藏条件
///
/// 工具名称为 "todoread" 或 "todowrite"，且状态为 "completed"
///
/// # 示例
///
/// ```ignore
/// let raw = "tool todowrite\n{\"status\": \"completed\"}";
/// assert!(should_hide_tool_block(raw));
/// ```
pub fn should_hide_tool_block(raw: &str) -> bool {
    // 尝试分割第一行和剩余内容
    let Some((first, rest)) = raw.split_once('\n') else {
        return false;
    };

    // 尝试提取工具名称
    let Some(tool_name) = first.trim().strip_prefix("tool ") else {
        return false;
    };
    let tool_name = canonical_tool_name(tool_name);

    // 只对 todo 相关工具进行判断
    if !matches!(tool_name, "todoread" | "todowrite") {
        return false;
    }

    // 解析 JSON 并检查状态字段
    let Ok(v) = serde_json::from_str::<Value>(rest.trim()) else {
        return false;
    };
    let status = v.get("status").and_then(|v| v.as_str()).unwrap_or("");

    // 只有完成状态才隐藏
    status == "completed"
}

/// 从工具输入中提取文件路径
///
/// 支持两种输入格式：
/// 1. JSON 格式：提取 "filePath"、"file_path" 或 "path" 字段
/// 2. 纯文本格式：直接使用输入内容作为路径
///
/// # 参数
///
/// * `input` - 工具输入字符串
///
/// # 返回值
///
/// * `Some(String)` - 提取到的文件路径
/// * `None` - 如果输入为空或无法提取路径
///
/// # 示例
///
/// ```ignore
/// let json_input = "{\"filePath\": \"/path/to/file.txt\"}";
/// assert_eq!(tool_input_path(json_input), Some("/path/to/file.txt".to_string()));
///
/// let text_input = "/another/path.txt";
/// assert_eq!(tool_input_path(text_input), Some("/another/path.txt".to_string()));
/// ```
pub fn tool_input_path(input: &str) -> Option<String> {
    // 判断输入是否为 JSON 格式
    if input.trim_start().starts_with('{') {
        // 尝试从 JSON 中提取 filePath、file_path 或 path 字段
        serde_json::from_str::<serde_json::Value>(input.trim()).ok().and_then(|v| {
            v.get("filePath")
                .or_else(|| v.get("file_path"))
                .or_else(|| v.get("path"))
                .and_then(|vv| vv.as_str())
                .and_then(normalize_file_reference_to_path)
        })
    } else {
        // 非JSON格式，直接使用输入作为路径
        normalize_file_reference_to_path(input)
    }
}

/// 解析并规范化输出路径
///
/// 将给定的输出路径转换为绝对路径。如果路径已经是绝对路径，
/// 直接返回；如果是相对路径，则相对于项目根目录进行解析。
///
/// # 参数
///
/// * `app` - 应用实例引用，用于获取项目根路径
/// * `output_path` - 待解析的输出路径字符串
///
/// # 返回值
///
/// 返回规范化后的绝对路径字符串
///
/// # 路径解析规则
///
/// 1. 首先规范化文件 URL（如 file://）为普通路径
/// 2. 如果路径已经是绝对路径，直接返回
/// 3. 如果存在项目根路径，将相对路径与根路径拼接
/// 4. 否则返回原始规范化路径
///
/// # 示例
///
/// ```ignore
/// // 假设项目根路径为 "/home/user/project"
/// let result = resolve_output_path(&app, "output/result.txt");
/// assert_eq!(result, "/home/user/project/output/result.txt");
///
/// let absolute = resolve_output_path(&app, "/tmp/file.txt");
/// assert_eq!(absolute, "/tmp/file.txt");
/// ```
pub fn resolve_output_path(app: &crate::app::App, output_path: &str) -> String {
    // 首先规范化文件 URL 为普通路径
    let p = crate::app::components::chat_panel::utils::normalize_file_url_to_path(output_path)
        .to_string();

    if output_path.trim_start().starts_with("file://") {
        return p;
    }

    // 如果已经是绝对路径，直接返回
    if std::path::Path::new(&p).is_absolute() {
        return p;
    }

    // 如果存在项目根路径，拼接成完整路径
    if let Some(root) = app.project_path.as_deref() {
        return std::path::PathBuf::from(root).join(p).to_string_lossy().to_string();
    }

    // 没有项目根路径时，返回原始规范化路径
    p
}
