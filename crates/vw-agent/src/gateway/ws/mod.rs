//! WebSocket 代理聊天处理器。
//!
//! 该模块提供了基于 WebSocket 的实时代理聊天功能，支持流式响应和工具调用。
//!
//! ## 协议格式
//!
//! 通信协议使用 JSON 格式的消息，具体如下：
//!
//! ```text
//! 客户端 -> 服务端: {"type":"message","content":"Hello"}
//! 服务端 -> 客户端: {"type":"chunk","content":"Hi! "}
//! 服务端 -> 客户端: {"type":"tool_call","name":"shell","args":{...}}
//! 服务端 -> 客户端: {"type":"tool_result","name":"shell","output":"...","result":{...}}
//! 服务端 -> 客户端: {"type":"done","full_response":"..."}
//! ```
//!
//! ## 消息类型说明
//!
//! - `message`: 客户端发送的用户消息
//! - `chunk`: 服务端发送的内容片段（流式响应）
//! - `tool_call`: 服务端发送的工具调用开始事件
//! - `tool_result`: 服务端发送的工具执行结果事件
//! - `done`: 服务端发送的完整响应（会话结束标记）
//! - `error`: 错误消息

use super::AppState;
use crate::app::agent::agent::loop_::{
    DRAFT_CLEAR_SENTINEL, DRAFT_PROGRESS_SENTINEL, DRAFT_WS_EVENT_SENTINEL, run_tool_call_loop,
};
use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::providers::ChatMessage;
use crate::app::agent::security::SecurityPolicy;
use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, header},
    response::IntoResponse,
};
use serde_json::{Value, json};
use std::sync::Arc;
use vw_api_types::tools::ToolResultDto;

/// WebSocket 空响应时的后备文本。
///
/// 当工具执行完成但模型未返回任何文本响应时，使用此消息提示用户。
const EMPTY_WS_RESPONSE_FALLBACK: &str = "Tool execution completed, but the model returned no final text response. Please ask me to summarize the result.";

/// WebSocket 聊天子协议标识符。
///
/// 用于在 WebSocket 握手时指定 VibeWindow 的协议版本。
const WS_CHAT_SUBPROTOCOL: &str = "vibewindow.v1";

/// WebSocket 增量事件类型。
///
/// 表示在代理执行过程中发生的各种事件，用于向客户端实时推送状态更新。
#[derive(Debug, Clone, PartialEq)]
enum WsDeltaEvent {
    /// 内容片段，包含模型生成的文本片段。
    ContentChunk(String),

    /// 工具调用事件，表示代理正在调用某个工具。
    ToolCall {
        /// 工具名称。
        name: String,
        /// 可选的工具提示信息，用于向用户展示工具正在执行的操作。
        hint: Option<String>,
    },

    /// 工具执行结果事件，表示工具调用已完成。
    ///
    /// 当前对外仍保留 `name / success / duration_secs / output` 兼容字段，
    /// 同时优先附带共享的 `ToolResultDto`，供 WebSocket 客户端读取结构化结果。
    ToolResult {
        /// 工具名称。
        name: String,
        /// 执行是否成功。
        success: bool,
        /// 执行耗时（秒）。
        duration_secs: Option<u64>,
        /// 对应的工具调用 ID。
        tool_call_id: Option<String>,
        /// 结构化工具结果 DTO。
        result: Option<ToolResultDto>,
    },
}

fn parse_ws_private_event(progress: &str) -> Option<WsDeltaEvent> {
    let payload = progress.strip_prefix(DRAFT_WS_EVENT_SENTINEL)?.trim();
    let value = serde_json::from_str::<Value>(payload).ok()?;
    let event = value.get("event").and_then(Value::as_str)?;
    if event != "tool_result" {
        return None;
    }

    let result = value
        .get("result")
        .cloned()
        .and_then(|value| serde_json::from_value::<ToolResultDto>(value).ok());
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| {
            result
                .as_ref()
                .and_then(|dto| dto.tool_id.as_ref().map(|tool_id| tool_id.as_ref().to_string()))
        })
        .unwrap_or_else(|| "tool".to_string());
    let success = result
        .as_ref()
        .and_then(|dto| dto.success)
        .or_else(|| value.get("success").and_then(Value::as_bool))
        .unwrap_or(false);
    let duration_secs = value.get("duration_secs").and_then(Value::as_u64);
    let tool_call_id = result
        .as_ref()
        .and_then(|dto| dto.tool_use_id.clone())
        .or_else(|| value.get("tool_call_id").and_then(Value::as_str).map(ToOwned::to_owned));

    Some(WsDeltaEvent::ToolResult { name, success, duration_secs, tool_call_id, result })
}

/// 净化 WebSocket 响应内容。
///
/// 对模型返回的响应进行安全净化，移除可能包含恶意内容的工具调用输出。
/// 如果原始响应非空但净化后为空，则返回错误提示消息。
///
/// # 参数
///
/// * `response` - 原始响应字符串。
/// * `tools` - 可用工具列表，用于净化逻辑。
///
/// # 返回值
///
/// 净化后的安全响应字符串。
fn sanitize_ws_response(
    response: &str,
    tools: &[Box<dyn crate::app::agent::tools::Tool>],
) -> String {
    let sanitized = crate::app::agent::channels::sanitize_channel_response(response, tools);
    if sanitized.is_empty() && !response.trim().is_empty() {
        "I encountered malformed tool-call output and could not produce a safe reply. Please try again."
            .to_string()
    } else {
        sanitized
    }
}

/// 最终化 WebSocket 响应。
///
/// 处理模型响应，确保返回给客户端的内容是安全且有意义的。
/// 如果模型响应为空，则直接回退到静态提示，避免再从历史文本协议中恢复工具结果。
///
/// # 参数
///
/// * `response` - 模型的原始响应。
/// * `history` - 对话历史，用于提取工具输出。
/// * `tools` - 可用工具列表，用于净化逻辑。
///
/// # 返回值
///
/// 最终的安全响应字符串。
fn finalize_ws_response(
    response: &str,
    _history: &[ChatMessage],
    tools: &[Box<dyn crate::app::agent::tools::Tool>],
) -> String {
    // 首先尝试净化原始响应
    let sanitized = sanitize_ws_response(response, tools);
    if !sanitized.trim().is_empty() {
        return sanitized;
    }

    EMPTY_WS_RESPONSE_FALLBACK.to_string()
}

/// 解析工具完成负载。
///
/// 从格式化的字符串中解析工具名称和执行耗时。
/// 预期格式：`<tool_name> (<duration>s)`
///
/// # 参数
///
/// * `raw` - 原始负载字符串。
///
/// # 返回值
///
/// 如果解析成功，返回 `Some((tool_name, duration_secs))`；否则返回 `None`。
///
/// # 示例
///
/// ```
/// let result = parse_tool_completion_payload("shell (5s)");
/// assert_eq!(result, Some(("shell".to_string(), Some(5))));
/// ```
fn parse_tool_completion_payload(raw: &str) -> Option<(String, Option<u64>)> {
    let trimmed = raw.trim();
    // 从右边分割，获取名称和括号部分
    let (name_part, duration_part) = trimmed.rsplit_once(" (")?;
    let duration_part = duration_part.strip_suffix(')')?;
    // 解析秒数
    let secs = duration_part.strip_suffix('s').and_then(|value| value.parse::<u64>().ok());
    Some((name_part.trim().to_string(), secs))
}

/// 解析 WebSocket 增量事件。
///
/// 将代理循环产生的增量字符串解析为结构化的事件类型。
/// 支持识别以下格式：
/// - `DRAFT_CLEAR_SENTINEL`: 清空标记，返回 `None`
/// - `DRAFT_PROGRESS_SENTINEL + "⏳ <name>: <hint>"`: 工具调用开始
/// - `DRAFT_PROGRESS_SENTINEL + "✅ <name> (<duration>s)"`: 工具调用成功
/// - `DRAFT_PROGRESS_SENTINEL + "❌ <name> (<duration>s)"`: 工具调用失败
/// - `DRAFT_PROGRESS_SENTINEL + DRAFT_WS_EVENT_SENTINEL + <json>`: ws 私有结构化工具结果
/// - 其他非空字符串：内容片段
///
/// # 参数
///
/// * `delta` - 增量字符串。
///
/// # 返回值
///
/// 如果解析成功，返回 `Some(event)`；否则返回 `None`。
fn parse_ws_delta_event(delta: &str) -> Option<WsDeltaEvent> {
    // 清空标记，不产生事件
    if delta == DRAFT_CLEAR_SENTINEL {
        return None;
    }

    // 处理进度标记
    if let Some(progress) = delta.strip_prefix(DRAFT_PROGRESS_SENTINEL) {
        let progress = progress.trim();

        if let Some(event) = parse_ws_private_event(progress) {
            return Some(event);
        }

        // 解析工具调用开始：⏳ <name>: <hint>
        if let Some(rest) = progress.strip_prefix("⏳ ") {
            let rest = rest.trim();
            if rest.is_empty() {
                return None;
            }
            let (name, hint) = match rest.split_once(':') {
                Some((name, hint)) => {
                    let hint = hint.trim();
                    (
                        name.trim().to_string(),
                        if hint.is_empty() { None } else { Some(hint.to_string()) },
                    )
                }
                None => (rest.to_string(), None),
            };
            return Some(WsDeltaEvent::ToolCall { name, hint });
        }

        // 解析工具调用成功：✅ <name> (<duration>s)
        if let Some(rest) = progress.strip_prefix("✅ ") {
            if let Some((name, duration_secs)) = parse_tool_completion_payload(rest) {
                return Some(WsDeltaEvent::ToolResult {
                    name,
                    success: true,
                    duration_secs,
                    tool_call_id: None,
                    result: None,
                });
            }
        }

        // 解析工具调用失败：❌ <name> (<duration>s)
        if let Some(rest) = progress.strip_prefix("❌ ") {
            if let Some((name, duration_secs)) = parse_tool_completion_payload(rest) {
                return Some(WsDeltaEvent::ToolResult {
                    name,
                    success: false,
                    duration_secs,
                    tool_call_id: None,
                    result: None,
                });
            }
        }

        return None;
    }

    // 其他非空字符串作为内容片段
    if delta.is_empty() { None } else { Some(WsDeltaEvent::ContentChunk(delta.to_string())) }
}

/// 发送 WebSocket 增量事件。
///
/// 将事件转换为 JSON 格式并通过 WebSocket 发送给客户端。
///
/// # 参数
///
/// * `socket` - WebSocket 连接。
/// * `event` - 要发送的事件。
async fn emit_ws_delta_event(socket: &mut WebSocket, event: WsDeltaEvent) {
    let payload = match event {
        WsDeltaEvent::ContentChunk(content) => json!({
            "type": "chunk",
            "content": content,
        }),
        WsDeltaEvent::ToolCall { name, hint } => json!({
            "type": "tool_call",
            "name": name,
            "args": {
                "hint": hint,
            },
        }),
        WsDeltaEvent::ToolResult { name, success, duration_secs, tool_call_id, result } => {
            let status = if success { "ok" } else { "error" };
            let output = match duration_secs {
                Some(secs) => format!("{status} ({secs}s)"),
                None => status.to_string(),
            };
            let mut payload = json!({
                "type": "tool_result",
                "name": name,
                "success": success,
                "duration_secs": duration_secs,
                "output": output,
            });
            if let Some(tool_call_id) = tool_call_id {
                payload["tool_call_id"] = Value::String(tool_call_id);
            }
            if let Some(result) = result
                && let Ok(value) = serde_json::to_value(result)
            {
                payload["result"] = value;
            }
            payload
        }
    };

    let _ = socket.send(Message::Text(payload.to_string().into())).await;
}

/// 处理 WebSocket 聊天升级请求。
///
/// 该函数处理 `GET /ws/chat` 端点，负责：
/// 1. 验证客户端身份（通过 Authorization 头或 WebSocket 协议令牌）
/// 2. 协商 WebSocket 子协议
/// 3. 升级 HTTP 连接到 WebSocket
///
/// # 身份验证
///
/// 支持两种身份验证方式：
/// - `Authorization: Bearer <skey>` HTTP 头
/// - `Sec-WebSocket-Protocol: vibewindow.v1, bearer.<skey>` WebSocket 协议头
///
/// # 参数
///
/// * `State(state)` - 应用状态，包含配置、配对管理器等。
/// * `headers` - HTTP 请求头，用于提取身份验证令牌。
/// * `ws` - WebSocket 升级器。
///
/// # 返回值
///
/// 返回 WebSocket 升级响应或未授权错误（401）。
///
/// # 示例
///
/// ```javascript
/// // 客户端连接示例
/// const ws = new WebSocket('ws://localhost:8080/ws/chat', [
///     'vibewindow.v1',
///     'bearer.your-skey-here'
/// ]);
/// ```
pub async fn handle_ws_chat(
    State(state): State<AppState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    // 通过 Authorization Bearer 或 WebSocket 协议 skey 进行身份验证
    if state.pairing.auth_enabled() {
        let skey = extract_ws_auth_skey(&headers).unwrap_or_default();
        if !state.pairing.is_authenticated(&skey) {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                "Unauthorized — provide Authorization: Bearer <skey> or Sec-WebSocket-Protocol: vibewindow.v1, bearer.<skey>",
            )
                .into_response();
        }
    }

    // 升级到 WebSocket 并指定子协议
    ws.protocols([WS_CHAT_SUBPROTOCOL])
        .on_upgrade(move |socket| handle_socket(socket, state))
        .into_response()
}

/// 处理 WebSocket 连接会话。
///
/// 该函数管理整个 WebSocket 会话的生命周期，包括：
/// - 维护会话的对话历史
/// - 构建系统提示
/// - 接收和处理客户端消息
/// - 调用代理循环并实时推送增量事件
/// - 发送完成或错误响应
///
/// # 参数
///
/// * `socket` - WebSocket 连接。
/// * `state` - 应用状态。
///
/// # 消息处理流程
///
/// 1. 初始化会话历史和系统提示
/// 2. 循环接收客户端消息
/// 3. 解析 JSON 格式的消息，提取 `type` 和 `content` 字段
/// 4. 将用户消息添加到历史
/// 5. 运行代理循环，实时推送增量事件（内容片段、工具调用、工具结果）
/// 6. 发送完成响应或错误消息
async fn handle_socket(mut socket: WebSocket, state: AppState) {
    // 为此 WebSocket 会话维护对话历史
    let mut history: Vec<ChatMessage> = Vec::new();

    let timezone_name = state.config.lock().app_ui.system_settings.timezone.clone();
    let timezone = match crate::app::agent::util::locale::parse_timezone(&timezone_name) {
        Ok(timezone) => timezone,
        Err(error) => {
            let message = serde_json::json!({"type": "error", "message": error.to_string()});
            let _ = socket.send(Message::Text(message.to_string().into())).await;
            return;
        }
    };

    // 为会话构建一次系统提示
    let (system_prompt, protocol_messages) = {
        let config_guard = state.config.lock();
        let system_prompt = crate::app::agent::channels::build_system_prompt(
            &config_guard.workspace_dir,
            &state.model,
            &[],
            &[],
            Some(&config_guard.identity),
            None,
            timezone,
        );
        let skills = crate::app::agent::skills::load_skills_with_config(
            &config_guard.workspace_dir,
            &config_guard,
        );
        let skill_tool_enabled =
            state.tools_registry_exec.iter().any(|tool| tool.name().eq_ignore_ascii_case("skill"));
        let protocol_messages = crate::app::agent::channels::build_workspace_protocol_messages(
            &config_guard.workspace_dir,
            &skills,
            skill_tool_enabled,
            8_192,
        );
        (system_prompt, protocol_messages)
    };

    // 临时协议消息不在 WebSocket 输出中可见；它们排在 system/history 之前，
    // 与 session processor 的 AGENTS、技能目录、真实历史顺序一致。
    history.extend(protocol_messages);
    history.push(ChatMessage::system(&system_prompt));

    // 初始化审批管理器
    let (approval_manager, security) = {
        let config_guard = state.config.lock();
        (
            Arc::new(ApprovalManager::from_config(&config_guard.autonomy)),
            Arc::new(SecurityPolicy::from_configs(
                &config_guard.autonomy,
                &config_guard.security,
                &config_guard.workspace_dir,
            )),
        )
    };

    // 主消息循环
    while let Some(msg) = socket.recv().await {
        let msg = match msg {
            Ok(Message::Text(text)) => text,
            Ok(Message::Close(_)) | Err(_) => break,
            _ => continue,
        };

        // 解析传入的消息
        let parsed: serde_json::Value = match serde_json::from_str(&msg) {
            Ok(v) => v,
            Err(_) => {
                let err = serde_json::json!({"type": "error", "message": "Invalid JSON"});
                let _ = socket.send(Message::Text(err.to_string().into())).await;
                continue;
            }
        };

        // 只处理 "message" 类型的消息
        let msg_type = parsed["type"].as_str().unwrap_or("");
        if msg_type != "message" {
            continue;
        }

        // 提取消息内容
        let content = parsed["content"].as_str().unwrap_or("").to_string();
        if content.is_empty() {
            continue;
        }

        // 将用户消息添加到历史
        history.push(ChatMessage::user(&content));

        // 获取提供者信息
        let provider_label =
            state.config.lock().default_provider.clone().unwrap_or_else(|| "unknown".to_string());

        // 广播代理开始事件
        let _ = state.event_tx.send(serde_json::json!({
            "type": "agent_start",
            "provider": provider_label,
            "model": state.model,
        }));

        // 运行代理循环，支持实时增量流式传输给 Web 客户端
        let result = {
            // 创建增量事件的通道
            let (delta_tx, mut delta_rx) = tokio::sync::mpsc::channel::<String>(128);

            // 固定代理循环的 future，以便在 select! 中使用
            let mut loop_future = std::pin::pin!(run_tool_call_loop(
                state.provider.as_ref(),
                &mut history,
                state.tools_registry_exec.as_ref(),
                state.observer.as_ref(),
                &provider_label,
                &state.model,
                state.temperature,
                true, // silent - 不输出到控制台
                Some(approval_manager.clone()),
                "webchat",
                &state.multimodal,
                state.max_tool_iterations,
                None,           // cancellation token - 取消令牌
                Some(delta_tx), // delta streaming - 增量流式传输
                None,           // hooks - 钩子
                Some(security.clone()),
                &[], // excluded tools - 排除的工具
            ));

            // 同时运行代理循环和处理增量事件
            loop {
                tokio::select! {
                    // 接收增量事件并推送给客户端
                    maybe_delta = delta_rx.recv() => {
                        if let Some(delta) = maybe_delta {
                            if let Some(event) = parse_ws_delta_event(&delta) {
                                emit_ws_delta_event(&mut socket, event).await;
                            }
                        } else {
                            // 通道关闭，等待代理循环完成
                            break loop_future.await;
                        }
                    }
                    // 代理循环完成
                    response = &mut loop_future => {
                        // 处理剩余的增量事件
                        while let Ok(delta) = delta_rx.try_recv() {
                            if let Some(event) = parse_ws_delta_event(&delta) {
                                emit_ws_delta_event(&mut socket, event).await;
                            }
                        }
                        break response;
                    }
                }
            }
        };

        // 处理代理循环的结果
        match result {
            Ok(response) => {
                // 净化响应
                let safe_response =
                    finalize_ws_response(&response, &history, state.tools_registry_exec.as_ref());

                // 将助手响应添加到历史
                history.push(ChatMessage::assistant(&safe_response));

                // 发送完成消息，包含完整响应
                let done = serde_json::json!({
                    "type": "done",
                    "full_response": safe_response,
                });
                let _ = socket.send(Message::Text(done.to_string().into())).await;

                // 广播代理结束事件
                let _ = state.event_tx.send(serde_json::json!({
                    "type": "agent_end",
                    "provider": provider_label,
                    "model": state.model,
                }));
            }
            Err(e) => {
                // 净化错误消息
                let sanitized = crate::app::agent::providers::sanitize_api_error(&e.to_string());
                let err = serde_json::json!({
                    "type": "error",
                    "message": sanitized,
                });
                let _ = socket.send(Message::Text(err.to_string().into())).await;

                // 广播错误事件
                let _ = state.event_tx.send(serde_json::json!({
                    "type": "error",
                    "component": "ws_chat",
                    "message": sanitized,
                }));
            }
        }
    }
}

/// 从 HTTP 头中提取 WebSocket skey。
///
/// 支持两种 skey 提取方式：
/// 1. `Authorization: Bearer <skey>` HTTP 头
/// 2. `Sec-WebSocket-Protocol: bearer.<skey>` WebSocket 协议头
///
/// # 参数
///
/// * `headers` - HTTP 请求头。
///
/// # 返回值
///
/// 如果找到有效 skey，返回 `Some(skey)`；否则返回 `None`。
///
/// # 示例
///
/// ```
/// // 方式1：使用 Authorization 头
/// // Authorization: Bearer my-skey
///
/// // 方式2：使用 WebSocket 协议头
/// // Sec-WebSocket-Protocol: vibewindow.v1, bearer.my-skey
/// ```
fn extract_ws_auth_skey(headers: &HeaderMap) -> Option<String> {
    if let Some(skey) = super::api::auth::extract_auth_skey(headers) {
        return Some(skey.to_string());
    }

    // 尝试从 Sec-WebSocket-Protocol 头提取
    let offered =
        headers.get(header::SEC_WEBSOCKET_PROTOCOL).and_then(|value| value.to_str().ok())?;

    // 遍历所有协议，查找 bearer.<skey> 格式
    for protocol in offered.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(token) = protocol.strip_prefix("bearer.") {
            if !token.trim().is_empty() {
                return Some(token.trim().to_string());
            }
        }
    }

    None
}

#[cfg(test)]
fn extract_ws_bearer_token(headers: &HeaderMap) -> Option<String> {
    extract_ws_auth_skey(headers)
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
