//! ACP 输出消息到 VibeWindow 流式事件的转换工具。
//!
//! ACP 代理会以 JSON-RPC session/update 消息上报文本、推理、工具和 token 使用量。
//! 本模块只做结构解析和事件映射，不执行本地工具调用；远端工具更新会作为 transcript
//! 文本呈现，避免突破本地工具权限边界。

use serde_json::{Map, Value, json};
use vw_acp::AcpJsonRpcMessage;
use vw_api_types::tools::{StructuredPatchHunkDto, ToolResultContentDto, ToolResultDto};

use crate::app::agent::session::ui_types;

use super::{StreamEvent, ToolCall};

/// 从 ACP 内容块中提取可展示文本。
///
/// 仅接受文本或资源 URI，其他结构化内容保持未处理，避免把未知载荷错误展示为模型文本。
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

/// 将 JSON 数字宽松转换为 `i64`。
fn json_number_to_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().map(|raw| raw as i64))
        .or_else(|| value.as_f64().map(|raw| raw as i64))
}

/// 按候选键读取 token 使用量字段。
fn token_usage_value(source: &serde_json::Map<String, Value>, keys: &[&str]) -> i64 {
    keys.iter().find_map(|key| source.get(*key).and_then(json_number_to_i64)).unwrap_or_default()
}

/// 从 ACP JSON-RPC 消息中取出 `params.update` 负载。
fn session_update_payload_from_message(
    message: &AcpJsonRpcMessage,
) -> Option<serde_json::Map<String, Value>> {
    serde_json::to_value(message).ok()?.get("params")?.get("update")?.as_object().cloned()
}

/// 读取工具更新的机器类型。
fn tool_kind_from_update(update: &Map<String, Value>) -> Option<&str> {
    update.get("kind").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty())
}

/// 读取工具更新的人类可读标题。
fn tool_title_from_update(update: &Map<String, Value>) -> Option<&str> {
    update.get("title").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty())
}

/// 生成 transcript 中使用的工具名。
fn transcript_tool_name(update: &Map<String, Value>) -> String {
    tool_kind_from_update(update)
        .or_else(|| tool_title_from_update(update))
        .unwrap_or("tool_call")
        .to_string()
}

/// 生成 transcript 中使用的工具标题。
fn transcript_tool_title(update: &Map<String, Value>, tool_name: &str) -> String {
    tool_title_from_update(update).unwrap_or(tool_name).to_string()
}

/// 将工具输入规范化成字符串。
fn tool_input_string(update: &Map<String, Value>) -> String {
    match update.get("rawInput").or_else(|| update.get("input")) {
        Some(Value::String(value)) => value.clone(),
        Some(Value::Null) | None => "{}".to_string(),
        Some(value) => serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string()),
    }
}

/// 从多种工具结果形态中提取最适合展示的文本。
fn extract_tool_result_text_value(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(boolean) => Some(boolean.to_string()),
        Value::Array(values) => values.iter().find_map(extract_tool_result_text_value),
        Value::Object(object) => {
            for key in [
                "model_result",
                "output",
                "content",
                "text",
                "stdout",
                "stderr",
                "message",
                "data",
                "value",
            ] {
                if let Some(extracted) = object.get(key).and_then(extract_tool_result_text_value)
                    && !extracted.trim().is_empty()
                {
                    return Some(extracted);
                }
            }
            None
        }
    }
}

/// 将结构化 patch hunk 渲染为近似 unified diff 的文本。
fn structured_patch_diff_text(hunks: &[StructuredPatchHunkDto]) -> Option<String> {
    if hunks.is_empty() {
        return None;
    }

    let mut diff = String::new();
    let mut current_path = String::new();
    for hunk in hunks {
        let Some(path) = hunk.path.as_deref().map(str::trim).filter(|path| !path.is_empty()) else {
            continue;
        };
        if path != current_path {
            if !diff.is_empty() && !diff.ends_with('\n') {
                diff.push('\n');
            }
            diff.push_str("--- a/");
            diff.push_str(path);
            diff.push('\n');
            diff.push_str("+++ b/");
            diff.push_str(path);
            diff.push('\n');
            current_path = path.to_string();
        }
        if !hunk.header.trim().is_empty() {
            diff.push_str(&hunk.header);
            diff.push('\n');
        }
        for line in &hunk.lines {
            diff.push_str(line);
            diff.push('\n');
        }
    }

    (!diff.trim().is_empty()).then_some(diff)
}

/// 从标准工具结果 DTO 中提取 transcript 文本。
fn extract_tool_result_dto_text(result: &ToolResultDto) -> Option<String> {
    for block in &result.content {
        match block {
            ToolResultContentDto::Text { text } if !text.trim().is_empty() => {
                return Some(text.clone());
            }
            ToolResultContentDto::Json { value } => {
                if let Some(text) = extract_tool_result_text_value(value)
                    && !text.trim().is_empty()
                {
                    return Some(text);
                }
            }
            ToolResultContentDto::StructuredPatch { hunks } if !hunks.is_empty() => {
                if let Some(text) = structured_patch_diff_text(hunks) {
                    return Some(text);
                }
            }
            _ => {}
        }
    }

    extract_tool_result_text_value(&result.model_result)
        .or_else(|| extract_tool_result_text_value(&result.data))
}

/// 尝试把 ACP rawOutput 解析为 VibeWindow 工具结果 DTO。
fn parse_tool_result_dto(update: &Map<String, Value>) -> Option<ToolResultDto> {
    update
        .get("rawOutput")
        .or_else(|| update.get("result"))
        .cloned()
        .and_then(|value| serde_json::from_value::<ToolResultDto>(value).ok())
}

/// 将 ACP 工具状态压缩为 UI 期望的少量状态值。
fn normalize_tool_status(update: &Map<String, Value>, result: Option<&ToolResultDto>) -> String {
    if let Some(status) = update.get("status").and_then(Value::as_str) {
        let normalized = status.trim().to_ascii_lowercase();
        if normalized.contains("running") || normalized.contains("pending") {
            return "running".to_string();
        }
        if normalized.contains("error")
            || normalized.contains("fail")
            || normalized.contains("deny")
        {
            return "error".to_string();
        }
        if normalized.contains("complete")
            || normalized.contains("done")
            || normalized.contains("success")
        {
            return "completed".to_string();
        }
        if !normalized.is_empty() {
            return normalized;
        }
    }

    match result.and_then(|dto| dto.success) {
        Some(true) => "completed".to_string(),
        Some(false) => "error".to_string(),
        None if update.contains_key("rawOutput") => "completed".to_string(),
        None => "running".to_string(),
    }
}

/// 提取工具渲染元数据。
fn metadata_from_tool_result(result: Option<&ToolResultDto>) -> Value {
    result
        .and_then(|dto| dto.render_hint.as_ref())
        .map(|hint| hint.metadata.clone())
        .filter(|value| !value.is_null())
        .unwrap_or_else(|| json!({}))
}

/// 解析工具输出文本。
fn output_from_tool_update(update: &Map<String, Value>, result: Option<&ToolResultDto>) -> String {
    if let Some(result) = result
        && let Some(text) = extract_tool_result_dto_text(result)
    {
        return text;
    }

    update
        .get("rawOutput")
        .or_else(|| update.get("output"))
        .and_then(extract_tool_result_text_value)
        .or_else(|| {
            update.get("rawOutput").or_else(|| update.get("output")).and_then(|value| {
                serde_json::to_string(value).ok().filter(|text| !text.trim().is_empty())
            })
        })
        .unwrap_or_default()
}

/// 从错误状态的工具输出中生成错误文本。
fn error_from_tool_update(status: &str, output: &str) -> Option<String> {
    if status == "error" {
        let trimmed = output.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

/// 把远端工具更新规整为只读生命周期事件。
///
/// ACP 工具已经在远端代理侧运行，因此这里保留展示所需的结构化信息，但绝不把它转换成
/// 本地 `ToolCall` 执行请求或普通答复文本。
fn remote_tool_update_from_update(update: &Map<String, Value>) -> Option<StreamEvent> {
    let update_type = update.get("sessionUpdate").and_then(Value::as_str)?;
    if update_type != "tool_call" && update_type != "tool_call_update" {
        return None;
    }

    let tool_call_id = update.get("toolCallId").and_then(Value::as_str)?.trim();
    if tool_call_id.is_empty() {
        return None;
    }

    let tool_name = transcript_tool_name(update);
    let tool_title = transcript_tool_title(update, &tool_name);
    let result = parse_tool_result_dto(update);
    let status = normalize_tool_status(update, result.as_ref());
    let input = tool_input_string(update);
    let output = output_from_tool_update(update, result.as_ref());
    let error = error_from_tool_update(&status, &output);
    let summary = result
        .as_ref()
        .and_then(|dto| dto.render_hint.as_ref())
        .and_then(|hint| hint.summary.clone());
    let render_hint = result.as_ref().and_then(|dto| dto.render_hint.clone());

    Some(StreamEvent::RemoteToolUpdate {
        call_id: tool_call_id.to_string(),
        name: tool_name,
        is_initial: update_type == "tool_call",
        payload: json!({
            "status": status,
            "toolCallId": tool_call_id,
            "input": input,
            "title": tool_title,
            "metadata": metadata_from_tool_result(result.as_ref()),
            "output": output,
            "error": error,
            "summary": summary,
            "renderHint": render_hint,
            "result": result,
        }),
    })
}

/// 提取 ACP agent message chunk 的文本增量。
///
/// 非文本更新或空内容返回 `None`；函数不产生错误，调用方可以安全地对每条消息尝试解析。
pub(crate) fn extract_delta_from_acp_message(message: &AcpJsonRpcMessage) -> Option<String> {
    let update = session_update_payload_from_message(message)?;
    if update.get("sessionUpdate").and_then(Value::as_str) != Some("agent_message_chunk") {
        return None;
    }
    let content = update.get("content")?;
    let text = extract_text_content(content).or_else(|| content.as_str())?;
    (!text.is_empty()).then(|| text.to_string())
}

/// 提取 ACP agent thought chunk 的推理增量。
///
/// 返回值只包含可展示文本；未知内容块会被忽略。
pub(crate) fn extract_reasoning_delta_from_acp_message(
    message: &AcpJsonRpcMessage,
) -> Option<String> {
    let update = session_update_payload_from_message(message)?;
    if update.get("sessionUpdate").and_then(Value::as_str) != Some("agent_thought_chunk") {
        return None;
    }
    let content = update.get("content")?;
    let text = extract_text_content(content).or_else(|| content.as_str())?;
    (!text.is_empty()).then(|| text.to_string())
}

/// 提取 ACP 工具更新的概要信息。
///
/// 返回的 `ToolCall` 只用于日志和兼容检查，不应被当作本地工具执行请求。
pub(crate) fn extract_tool_call_from_acp_message(message: &AcpJsonRpcMessage) -> Option<ToolCall> {
    let update = session_update_payload_from_message(message)?;
    let update_type = update.get("sessionUpdate").and_then(Value::as_str)?;
    if update_type != "tool_call" && update_type != "tool_call_update" {
        return None;
    }

    let id = update.get("toolCallId").and_then(Value::as_str)?.trim();
    if id.is_empty() {
        return None;
    }

    let name = transcript_tool_title(&update, &transcript_tool_name(&update));

    let arguments = update
        .get("rawInput")
        .and_then(|value| serde_json::to_string(value).ok())
        .unwrap_or_else(|| "{}".to_string());

    Some(ToolCall { id: id.to_string(), name, arguments })
}

/// 从 ACP usage update 中提取 token 用量。
fn extract_usage_from_acp_message(message: &AcpJsonRpcMessage) -> Option<ui_types::TokenUsage> {
    let update = session_update_payload_from_message(message)?;
    if update.get("sessionUpdate").and_then(Value::as_str) != Some("usage_update") {
        return None;
    }
    let source = update
        .get("_meta")
        .and_then(Value::as_object)
        .and_then(|meta| meta.get("usage"))
        .and_then(Value::as_object)
        .unwrap_or(&update);
    Some(ui_types::TokenUsage {
        input_tokens: token_usage_value(source, &["input_tokens", "inputTokens"]),
        output_tokens: token_usage_value(source, &["output_tokens", "outputTokens"]),
        cached_tokens: token_usage_value(
            source,
            &["cache_read_input_tokens", "cacheReadInputTokens", "cached_tokens", "cachedTokens"],
        ),
        reasoning_tokens: token_usage_value(source, &["thought_tokens", "reasoning_tokens"]),
    })
}

/// 将单条 ACP JSON-RPC 消息转发为上层流式事件。
///
/// 文本和工具 transcript 会增加 `delta_count`；usage 更新会覆盖 `latest_usage`。
/// 函数不会返回错误，无法识别的 ACP 消息会被忽略。
pub(crate) fn forward_acp_message(
    message: &AcpJsonRpcMessage,
    on_event: &mut impl FnMut(StreamEvent),
    latest_usage: &mut ui_types::TokenUsage,
    delta_count: &mut usize,
) {
    if let Some(delta) = extract_delta_from_acp_message(message)
        && !delta.is_empty()
    {
        *delta_count = delta_count.saturating_add(1);
        on_event(StreamEvent::Delta(delta));
    }
    if let Some(reasoning) = extract_reasoning_delta_from_acp_message(message)
        && !reasoning.is_empty()
    {
        on_event(StreamEvent::ReasoningDelta(reasoning));
    }
    if let Some(tool_update) = session_update_payload_from_message(message)
        .and_then(|update| remote_tool_update_from_update(&update))
    {
        *delta_count = delta_count.saturating_add(1);
        on_event(tool_update);
    }
    if let Some(tool_call) = extract_tool_call_from_acp_message(message) {
        tracing::info!(
            target: "vw_agent",
            tool_call_id = %tool_call.id,
            tool_name = %tool_call.name,
            tool_input = %crate::app::agent::util::truncate_with_ellipsis(
                &crate::agent::loop_::scrub_credentials(&tool_call.arguments),
                200,
            ),
            "received ACP remote tool update; not forwarding as local tool call"
        );
    }
    if let Some(usage) = extract_usage_from_acp_message(message) {
        *latest_usage = usage;
    }
}
#[cfg(test)]
#[path = "updates_tests.rs"]
mod updates_tests;
