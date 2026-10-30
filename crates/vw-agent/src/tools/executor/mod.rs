use super::context::{ToolUseContext, scope_tool_use_context};
use super::decision::finalize_tool_input;
use super::hooks::{PostToolHook, PreToolHook};
use super::{Tool, ToolCallResult, ToolRenderHint, ToolSpec, is_web_fetch_tool_id};
use crate::app::agent::providers::{ChatMessage, ToolCall};
use crate::app::agent::tools::FileSnapshot;
use serde_json::Value;
use std::fmt;
use std::fmt::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use vw_api_types::tools::PermissionRequestDto;

/// Claude Tools V2 运行时错误。
#[derive(Debug, Clone, PartialEq)]
pub enum ToolCallError {
    /// 调用被权限或安全策略拒绝。
    Denied { message: String, permission_request: Option<PermissionRequestDto> },
    /// 调用失败。
    Failed(String),
}

impl ToolCallError {
    pub fn denied(message: impl Into<String>) -> Self {
        Self::Denied { message: message.into(), permission_request: None }
    }

    pub fn denied_with_permission_request(
        message: impl Into<String>,
        permission_request: PermissionRequestDto,
    ) -> Self {
        Self::Denied { message: message.into(), permission_request: Some(permission_request) }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::Denied { message, .. } | Self::Failed(message) => message,
        }
    }

    pub fn permission_request(&self) -> Option<&PermissionRequestDto> {
        match self {
            Self::Denied { permission_request, .. } => permission_request.as_ref(),
            Self::Failed(_) => None,
        }
    }
}

impl fmt::Display for ToolCallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Denied { message, .. } | Self::Failed(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ToolCallError {}

/// 已执行工具调用的结构化结果。
#[derive(Debug, Clone)]
pub struct ExecutedToolCall {
    /// 规范化后的工具 ID。
    pub tool_name: String,
    /// 最终执行所使用的输入。
    pub input: Value,
    /// 工具的结构化返回值。
    pub result: ToolCallResult,
    /// 工具执行耗时。
    pub duration: Duration,
}

/// 写回对话历史的工具结果条目。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResultHistoryEntry {
    /// 实际执行的工具名称。
    pub tool_name: String,
    /// 原生 tool call id。
    pub tool_call_id: Option<String>,
    /// 面向模型的文本输出。
    pub output: String,
}

/// 执行单个工具调用。
///
/// 该函数是 Claude Tools V2 运行时唯一合法的单工具执行入口，负责串联：
///
/// 1. 输入归一化与校验
/// 2. 权限决策流水线
/// 3. before-tool hook
/// 4. 实际工具调用
/// 5. after-tool hook
/// 6. 模型结果映射与渲染提示补全
/// 7. read_state 回写
pub async fn execute_tool_from_registry(
    tools: &[Box<dyn Tool>],
    requested_name: &str,
    input: Value,
    tool_use_context: Arc<ToolUseContext>,
) -> Result<ExecutedToolCall, ToolCallError> {
    let hook_runner = tool_use_context.hook_runner();
    let started = Instant::now();
    let execution_context = tool_use_context.clone();
    let (tool_name, final_input, mut result) =
        scope_tool_use_context(tool_use_context.clone(), async move {
            let (approved_name, approved_input) =
                prepare_tool_input(tools, requested_name, input, execution_context.as_ref())
                    .await?;
            let (hooked_name, hooked_input) =
                PreToolHook::run(hook_runner, approved_name.clone(), approved_input.clone())
                    .await?;
            let (final_name, final_input) =
                if hooked_name == approved_name && hooked_input == approved_input {
                    (approved_name, approved_input)
                } else {
                    prepare_tool_input(
                        tools,
                        hooked_name.as_str(),
                        hooked_input,
                        execution_context.as_ref(),
                    )
                    .await?
                };

            let Some(tool) = find_tool_by_name(tools, final_name.as_str()) else {
                return Err(ToolCallError::Failed(format!("Unknown tool: {final_name}")));
            };
            let result = tool.call(final_input.clone()).await.map_err(classify_anyhow_error)?;
            Ok::<_, ToolCallError>((final_name, final_input, result))
        })
        .await?;

    PostToolHook::run(hook_runner, tool_name.as_str(), &result, started.elapsed()).await;

    let Some(tool) = find_tool_by_name(tools, tool_name.as_str()) else {
        return Err(ToolCallError::Failed(format!("Unknown tool: {tool_name}")));
    };
    let spec = tool.spec();

    result.model_result = tool.map_result_for_model(&result);
    if result.render_hint.is_none() {
        result.render_hint = tool.render_hint(&result);
    }
    ensure_default_render_hint(&mut result, &spec);
    update_read_state(tool_use_context.as_ref(), spec.id.as_str(), &final_input, &result);

    Ok(ExecutedToolCall {
        tool_name: spec.id,
        input: final_input,
        result,
        duration: started.elapsed(),
    })
}

/// 根据执行结果构造回填历史消息。
///
/// 当前 prompt-mode 仍使用 `[Tool results]` + `<tool_result>` 作为过渡协议，
/// 但该细节只保留在工具运行时内部，loop 层只消费已经构造好的消息对象。
pub fn build_tool_result_history_messages(
    native_tool_calls: &[ToolCall],
    results: &[ToolResultHistoryEntry],
    use_native_tools: bool,
) -> Vec<ChatMessage> {
    if results.is_empty() {
        return Vec::new();
    }

    if native_tool_calls.is_empty() {
        let all_results_have_ids =
            use_native_tools && results.iter().all(|entry| entry.tool_call_id.is_some());

        if all_results_have_ids {
            return results
                .iter()
                .map(|entry| {
                    let tool_msg = serde_json::json!({
                        "tool_call_id": entry.tool_call_id,
                        "content": entry.output,
                    });
                    ChatMessage::tool(tool_msg.to_string())
                })
                .collect();
        }

        let mut content = String::new();
        for entry in results {
            let _ = writeln!(
                content,
                "<tool_result name=\"{}\">\n{}\n</tool_result>",
                entry.tool_name, entry.output,
            );
        }

        return vec![ChatMessage::user(format!("[Tool results]\n{content}"))];
    }

    native_tool_calls
        .iter()
        .zip(results.iter())
        .map(|(native_call, entry)| {
            let tool_msg = serde_json::json!({
                "tool_call_id": native_call.id,
                "content": entry.output,
            });
            ChatMessage::tool(tool_msg.to_string())
        })
        .collect()
}

async fn prepare_tool_input(
    tools: &[Box<dyn Tool>],
    requested_name: &str,
    input: Value,
    context: &ToolUseContext,
) -> Result<(String, Value), ToolCallError> {
    let Some(tool) = find_tool_by_name(tools, requested_name) else {
        return Err(ToolCallError::Failed(format!("Unknown tool: {requested_name}")));
    };

    let spec = tool.spec();
    if !context.tool_is_exposed(&spec.id) {
        return Err(ToolCallError::denied(format!(
            "tool not exposed in this model request: {}",
            spec.id
        )));
    }
    let normalized = normalize_tool_args(spec.id.as_str(), input);
    let validated = tool.validate_input(normalized).map_err(classify_anyhow_error)?;
    let approved_input = finalize_tool_input(tool, validated, context).await?;
    Ok((spec.id, approved_input))
}

fn find_tool_by_name<'a>(tools: &'a [Box<dyn Tool>], requested_name: &str) -> Option<&'a dyn Tool> {
    tools
        .iter()
        .find(|tool| {
            let spec = tool.spec();
            spec.id == requested_name || spec.aliases.iter().any(|alias| alias == requested_name)
        })
        .map(|tool| tool.as_ref())
}

fn normalize_tool_args(tool_id: &str, args: Value) -> Value {
    if !args.is_object() {
        return args;
    }

    let mut obj = args.as_object().cloned().unwrap_or_default();
    match tool_id {
        "file_read" | "file_write" => {
            if !obj.contains_key("path")
                && let Some(value) =
                    obj.get("filePath").cloned().or_else(|| obj.get("file_path").cloned())
            {
                obj.entry("path".to_string()).or_insert(value);
            }
        }
        "file_edit" => {
            if !obj.contains_key("file_path")
                && let Some(value) = obj
                    .get("filePath")
                    .cloned()
                    .or_else(|| obj.get("file_path").cloned())
                    .or_else(|| obj.get("path").cloned())
            {
                obj.entry("file_path".to_string()).or_insert(value);
            }
            if !obj.contains_key("old_string")
                && let Some(value) = obj.get("oldString").cloned()
            {
                obj.entry("old_string".to_string()).or_insert(value);
            }
            if !obj.contains_key("new_string")
                && let Some(value) = obj.get("newString").cloned()
            {
                obj.entry("new_string".to_string()).or_insert(value);
            }
            if !obj.contains_key("replace_all")
                && let Some(value) = obj.get("replaceAll").cloned()
            {
                obj.entry("replace_all".to_string()).or_insert(value);
            }
        }
        id if is_web_fetch_tool_id(id) => {
            if !obj.contains_key("url")
                && let Some(value) = obj.get("href").cloned()
            {
                obj.entry("url".to_string()).or_insert(value);
            }
        }
        _ => {}
    }

    Value::Object(obj)
}

fn ensure_default_render_hint(result: &mut ToolCallResult, spec: &ToolSpec) {
    let hint =
        result.render_hint.get_or_insert_with(|| ToolRenderHint::titled(spec.display_name.clone()));
    if hint.title.is_none() {
        hint.title = Some(spec.display_name.clone());
    }
    if hint.metadata.is_null() {
        hint.metadata = Value::Object(Default::default());
    }
}

fn classify_anyhow_error(error: anyhow::Error) -> ToolCallError {
    classify_message(error.to_string())
}

fn classify_message(message: String) -> ToolCallError {
    if is_denied_error(&message) {
        ToolCallError::denied(message)
    } else {
        ToolCallError::Failed(message)
    }
}

fn update_read_state(
    tool_use_context: &ToolUseContext,
    tool_name: &str,
    input: &Value,
    result: &ToolCallResult,
) {
    if !result.is_success() {
        return;
    }

    let root = tool_use_context.workspace_root();
    let read_state = tool_use_context.read_state_handle();
    let mut read_state = read_state.lock().unwrap_or_else(|error| error.into_inner());

    match tool_name {
        "file_read" => {
            let Some(path) = primary_path(input) else {
                return;
            };
            let partial_view = file_read_partial_view(result);
            let snapshot = snapshot_for_input_path(root.as_deref(), input);
            read_state.note_read(
                root.as_deref(),
                path,
                result.model_text().len(),
                partial_view,
                input_value_usize(input, "offset"),
                input_value_usize(input, "limit"),
                snapshot,
            );
        }
        "file_edit" | "file_write" | "apply_patch" => {}
        _ => {}
    }
}

fn primary_path(input: &Value) -> Option<&str> {
    let Value::Object(map) = input else {
        return None;
    };

    map.get("path")
        .and_then(Value::as_str)
        .or_else(|| map.get("file_path").and_then(Value::as_str))
        .or_else(|| map.get("filePath").and_then(Value::as_str))
}

fn snapshot_for_input_path(root: Option<&Path>, input: &Value) -> Option<FileSnapshot> {
    let path = primary_path(input)?;
    let resolved = if Path::new(path).is_absolute() {
        Path::new(path).to_path_buf()
    } else if let Some(root) = root {
        root.join(path)
    } else {
        Path::new(path).to_path_buf()
    };
    let bytes = std::fs::read(&resolved).ok()?;
    Some(FileSnapshot::from_bytes(&bytes))
}

fn input_value_usize(input: &Value, key: &str) -> Option<usize> {
    input.get(key).and_then(Value::as_u64).and_then(|value| usize::try_from(value).ok())
}

fn file_read_partial_view(result: &ToolCallResult) -> bool {
    match result.data.get("kind").and_then(Value::as_str) {
        Some("text") => segmented_read_partial(
            result.data.get("total_lines").and_then(Value::as_u64),
            result.data.get("start_line").and_then(Value::as_u64),
            result.data.get("has_more").and_then(Value::as_bool),
            result.data.get("truncated_by_bytes").and_then(Value::as_bool),
        ),
        Some("pdf") => segmented_read_partial(
            result.data.get("total_pages").and_then(Value::as_u64),
            result.data.get("start_page").and_then(Value::as_u64),
            result.data.get("has_more").and_then(Value::as_bool),
            result.data.get("truncated_by_bytes").and_then(Value::as_bool),
        ),
        Some("notebook") => segmented_read_partial(
            result.data.get("total_cells").and_then(Value::as_u64),
            result.data.get("start_cell").and_then(Value::as_u64),
            result.data.get("has_more").and_then(Value::as_bool),
            result.data.get("truncated_by_bytes").and_then(Value::as_bool),
        ),
        Some("file_unchanged") => result
            .data
            .get("partial_view")
            .or_else(|| result.data.get("partialView"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        _ => false,
    }
}

fn segmented_read_partial(
    total: Option<u64>,
    start: Option<u64>,
    has_more: Option<bool>,
    truncated_by_bytes: Option<bool>,
) -> bool {
    let total = total.unwrap_or(0);
    if total == 0 {
        return false;
    }

    start.unwrap_or(0) != 1 || has_more.unwrap_or(false) || truncated_by_bytes.unwrap_or(false)
}

fn is_denied_error(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("denied")
        || lower.contains("not allowed")
        || lower.contains("forbidden")
        || lower.contains("blocked")
        || lower.contains("approval required")
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
