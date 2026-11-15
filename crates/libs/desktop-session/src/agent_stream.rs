//! 将桌面代理请求转换为网关流式消息。
//! 本模块隔离传输事件解析，让应用状态只接收明确的局部流消息。

use crate::message::chat::request::AgentRequest;
use super::agent_stream_events::StreamMessage;

mod reconnect;
mod workflow;
mod plan_run;
use reconnect::*;
use workflow::*;
use plan_run::*;

pub fn desktop_stream_request_id(req: &AgentRequest) -> String {
    use std::hash::{Hash, Hasher};
    use std::sync::OnceLock;

    static CLIENT_NONCE: OnceLock<u128> = OnceLock::new();

    let nonce = CLIENT_NONCE.get_or_init(rand::random);
    let mut session_hasher = std::collections::hash_map::DefaultHasher::new();
    req.session.hash(&mut session_hasher);
    format!("desktop-{nonce:032x}-{}-{:016x}", req.id, session_hasher.finish())
}

async fn gateway_stream_retry_sleep(duration: std::time::Duration) {
    #[cfg(not(target_arch = "wasm32"))]
    tokio::time::sleep(duration).await;

    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::{JsCast, JsValue, closure::Closure};

        let millis = duration.as_millis().min(i32::MAX as u128) as i32;
        let Some(window) = web_sys::window() else {
            return;
        };
        let promise = js_sys::Promise::new(&mut |resolve, _reject| {
            let Ok(resolve) = resolve.dyn_into::<js_sys::Function>() else {
                return;
            };
            let callback = Closure::once(move || {
                let _ = resolve.call0(&JsValue::NULL);
            });
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(),
                millis,
            );
            callback.forget();
        });
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    }
}

/// 使用宿主提供的配置入口创建会话事件流。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn agent_stream(req: &AgentRequest, config: desktop_config::ConfigAccess, endpoint: gateway_endpoint::GatewayEndpoint) -> AgentBoxStream<StreamMessage> {
    gateway_agent_stream(req.clone(), config, endpoint)
}

#[cfg(not(target_arch = "wasm32"))]
pub type AgentBoxStream<T> = iced::futures::stream::BoxStream<'static, T>;
#[cfg(target_arch = "wasm32")]
pub type AgentBoxStream<T> = iced::futures::stream::LocalBoxStream<'static, T>;

fn gateway_stream_stop_after_forward_err(
    err: iced::futures::channel::mpsc::TrySendError<StreamMessage>,
    stop_reason: &mut Option<GatewayStreamStopReason>,
) -> bool {
    *stop_reason = Some(if err.is_disconnected() {
        GatewayStreamStopReason::Cancelled
    } else {
        GatewayStreamStopReason::Backpressure
    });
    false
}

fn try_send_plan_run_delta_batch(
    output: &mut iced::futures::channel::mpsc::Sender<StreamMessage>,
    request_id: u64,
    batch: &PlanRunDeltaBatch,
) -> Result<(), iced::futures::channel::mpsc::TrySendError<StreamMessage>> {
    output.try_send(StreamMessage::PlanRunSessionDelta {
        session_id: batch.session_id.clone(),
        delta: batch.delta.clone(),
    })?;
    tracing::debug!(
        target: "vw_desktop",
        request_id,
        session_id = %batch.session_id,
        first_event_id = batch.first_event_id,
        last_event_id = batch.last_event_id,
        event_count = batch.event_count,
        char_count = batch.char_count,
        byte_count = batch.delta.len(),
        "desktop forwarded coalesced task plan-run session delta"
    );
    Ok(())
}

#[allow(dead_code)]
fn apply_workflow_auth(
    builder: reqwest::RequestBuilder,
    endpoint: &gateway_endpoint::GatewayEndpoint,
) -> reqwest::RequestBuilder {
    let Some(auth) = endpoint.auth.as_ref() else {
        return builder;
    };

    if let Some(skey) = auth.skey.as_deref().filter(|value| !value.trim().is_empty()) {
        builder.bearer_auth(skey)
    } else {
        builder
    }
}

#[allow(dead_code)]
fn take_next_workflow_stream_payload(buffer: &mut String) -> Option<String> {
    let separator = if let Some(idx) = buffer.find("\r\n\r\n") {
        Some((idx, 4))
    } else {
        buffer.find("\n\n").map(|idx| (idx, 2))
    }?;

    let (idx, sep_len) = separator;
    let frame = buffer[..idx].to_string();
    buffer.drain(..idx + sep_len);

    let payload = frame
        .lines()
        .filter_map(|line| line.trim_end_matches('\r').strip_prefix("data:"))
        .map(str::trim_start)
        .collect::<Vec<_>>()
        .join("\n");
    (!payload.is_empty()).then_some(payload)
}

#[allow(dead_code)]
fn parse_workflow_chat_messages_event(payload: serde_json::Value) -> WorkflowChatMessagesEvent {
    match payload.get("event").and_then(serde_json::Value::as_str).unwrap_or_default() {
        "message" => WorkflowChatMessagesEvent::Delta(
            payload
                .get("answer")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
        ),
        "message_end" => WorkflowChatMessagesEvent::Done {
            usage: payload.get("metadata").and_then(|metadata| metadata.get("usage")).cloned(),
            message_id: payload
                .get("message_id")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
        },
        "node_started" => {
            WorkflowChatMessagesEvent::NodeStarted(parse_workflow_node_event(&payload))
        }
        "text_chunk" => WorkflowChatMessagesEvent::Delta(
            payload
                .get("data")
                .and_then(|data| data.get("text"))
                .or_else(|| payload.get("answer"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
        ),
        "node_delta" => {
            WorkflowChatMessagesEvent::NodeDelta(parse_workflow_node_delta_event(&payload))
        }
        "node_finished" => {
            WorkflowChatMessagesEvent::NodeFinished(parse_workflow_node_event(&payload))
        }
        "workflow_finished" => match payload
            .get("data")
            .and_then(|data| data.get("status"))
            .and_then(serde_json::Value::as_str)
        {
            Some("failed") => WorkflowChatMessagesEvent::Error(
                payload
                    .get("data")
                    .and_then(|data| data.get("error"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("workflow failed")
                    .to_string(),
            ),
            _ => WorkflowChatMessagesEvent::Other,
        },
        "error" => WorkflowChatMessagesEvent::Error(
            payload
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("workflow stream failed")
                .to_string(),
        ),
        _ => WorkflowChatMessagesEvent::Other,
    }
}

#[allow(dead_code)]
async fn stream_workflow_chat_messages(
    endpoint: &gateway_endpoint::GatewayEndpoint,
    directory: Option<&str>,
    body: &serde_json::Value,
    mut on_event: impl FnMut(WorkflowChatMessagesEvent) -> bool,
) -> Result<(), String> {
    use iced::futures::StreamExt;
    use tracing::{debug, info};

    #[cfg(not(target_arch = "wasm32"))]
    let builder = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(WORKFLOW_STREAM_CONNECT_TIMEOUT_SECS))
        .timeout(std::time::Duration::from_secs(WORKFLOW_STREAM_REQUEST_TIMEOUT_SECS));
    #[cfg(target_arch = "wasm32")]
    let builder = reqwest::Client::builder();

    let client = builder.build().map_err(|error| error.to_string())?;
    let mut request =
        client.post(format!("{}{}", endpoint.base_url(), WORKFLOW_CHAT_MESSAGES_PATH)).json(body);
    if let Some(directory) = directory.filter(|value| !value.trim().is_empty()) {
        request = request.query(&[("directory", directory)]);
    }
    request = apply_workflow_auth(request, endpoint);

    let response = request.send().await.map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("workflow chat messages failed: {} {}", status, body.trim()));
    }

    info!(
        target: "vw_desktop",
        endpoint = %endpoint.describe(),
        path = WORKFLOW_CHAT_MESSAGES_PATH,
        "desktop connected workflow chat messages stream"
    );

    let mut bytes_stream = response.bytes_stream();
    let mut buffer = String::new();
    while let Some(chunk) = bytes_stream.next().await {
        let chunk = chunk.map_err(|error| error.to_string())?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(payload) = take_next_workflow_stream_payload(&mut buffer) {
            let payload: serde_json::Value =
                serde_json::from_str(&payload).map_err(|error| error.to_string())?;
            let event = parse_workflow_chat_messages_event(payload);
            debug!(
                target: "vw_desktop",
                path = WORKFLOW_CHAT_MESSAGES_PATH,
                "desktop received workflow chat messages stream event"
            );
            if !on_event(event) {
                return Ok(());
            }
        }
    }

    Ok(())
}

/// 决定本轮是否进入 `task_plan_run`。
///
/// 门控只认工作流开关：单独开启目标不会进入严格 plan-run。
/// 网关 plan-run 固定目标模式，不再区分 Goal/Workflow 参数。
pub fn task_plan_run_active(workflow_mode_enabled: bool, resume_history_only: bool) -> bool {
    workflow_mode_enabled && !resume_history_only
}

/// 无工作流时，目标模式开启便由独立评估器决定是否续轮。
fn lightweight_goal_active(workflow_mode_enabled: bool, resume_history_only: bool) -> bool {
    !workflow_mode_enabled && !resume_history_only
}

fn lightweight_goal_enabled_for_request(req: &AgentRequest) -> bool {
    req.goal_mode_enabled
        && lightweight_goal_active(req.workflow_mode_enabled, req.resume_history_only)
}

/// 按工作流门控构造 plan-run 请求；未进入 plan-run 时返回 `Ok(None)`。
/// 缺项目根目录错误仅在工作流已开启（即会进入 plan-run）时触发。
fn build_task_plan_run_request(
    req: &AgentRequest,
) -> Result<Option<vw_api_task::TaskPlanRunRequest>, String> {
    if !task_plan_run_active(req.workflow_mode_enabled, req.resume_history_only) {
        return Ok(None);
    }
    let Some(root) = req.root.as_deref().filter(|value| !value.trim().is_empty()) else {
        return Err("启用工作流执行需要先选择项目根目录".to_string());
    };
    Ok(Some(vw_api_task::TaskPlanRunRequest {
        query: req.query.clone(),
        root: root.to_string(),
        model: req.model.clone(),
        agent: req.agent.clone(),
        acp_agent: req.acp_agent.clone(),
        parent_session_id: {
            let trimmed = req.session.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        },
    }))
}

#[cfg(test)]
async fn stream_task_plan_run(
    endpoint: &gateway_endpoint::GatewayEndpoint,
    body: &vw_api_task::TaskPlanRunRequest,
    on_event: impl FnMut(vw_api_task::TaskPlanRunStreamEventDto) -> bool,
) -> Result<(), String> {
    tracing::info!(
        target: "vw_desktop",
        endpoint = %endpoint.describe(),
        path = TASK_PLAN_RUN_STREAM_PATH,
        "desktop connected task plan-run stream"
    );
    gateway_task_pool::GatewayTaskPoolClient::stream_task_plan_run(endpoint, body, on_event).await
}

async fn stream_task_plan_run_resumable(
    endpoint: &gateway_endpoint::GatewayEndpoint,
    body: &vw_api_task::TaskPlanRunRequest,
    resume: &gateway_chat_stream::GatewayStreamResume,
    on_event: impl FnMut(
        gateway_chat_stream::GatewayStreamEvent<vw_api_task::TaskPlanRunStreamEventDto>,
    ) -> bool,
) -> Result<gateway_chat_stream::GatewayStreamAttemptResult, String> {
    tracing::info!(
        target: "vw_desktop",
        endpoint = %endpoint.describe(),
        path = TASK_PLAN_RUN_STREAM_PATH,
        "desktop connected resumable task plan-run stream"
    );
    gateway_task_pool::GatewayTaskPoolClient::stream_task_plan_run_resumable(
        endpoint, body, resume, on_event,
    )
    .await
}

fn gateway_request_options(
    req: &AgentRequest,
    rpc_id: &str,
    client_time_zone: &str,
) -> Option<serde_json::Value> {
    use serde_json::{Value, json};

    let mut options = serde_json::Map::new();
    if let Some(full_access) = req.full_access_enabled {
        options.insert("full_access".to_string(), json!(full_access));
    }
    if let Some(agent) = &req.agent {
        options.insert("agent".to_string(), json!(agent));
    }
    if let Some(allowed_tools) = &req.allowed_tools {
        options.insert("allowed_tools".to_string(), json!(allowed_tools));
    }
    if req.require_agent_tool {
        options.insert("require_agent_tool".to_string(), json!(true));
    }
    if req.acp_test {
        options.insert("acp_test".to_string(), json!(true));
        options.insert("acp_agent".to_string(), json!(req.acp_agent));
        if let Some(acp_allowed_tools) = &req.acp_allowed_tools {
            options.insert("acp_allowed_tools".to_string(), json!(acp_allowed_tools));
        }
        options.insert("acp_force_new_session".to_string(), json!(req.acp_force_new_session));
        options.insert("acp_history_strategy".to_string(), json!(req.acp_history_mode.as_str()));
        options.insert("acp_history_recent_count".to_string(), json!(req.acp_recent_count));
        options.insert("cwd".to_string(), json!(req.root));
        if let Some(full_access) = req.full_access_enabled {
            options.insert(
                "acp_permission_mode".to_string(),
                json!(if full_access { "approve-all" } else { "approve-reads" }),
            );
        }
    }
    if req.resume_history_only {
        options.insert("desktop_resume_history_only".to_string(), json!(true));
    }
    if lightweight_goal_enabled_for_request(req) {
        // 目标模式由会话处理器依据 Todo 状态决定是否续轮。
        options.insert("goal_mode".to_string(), json!(true));
    }
    if !req.resume_history_only && !req.query.trim().is_empty() {
        options.insert(
            "_vw_input_source".to_string(),
            json!({
                "kind": "user",
                "rpcId": rpc_id,
                "clientTimeZone": client_time_zone,
            }),
        );
    }
    (!options.is_empty()).then_some(Value::Object(options))
}

fn chat_tool_message(request_id: u64, payload: &serde_json::Value) -> Option<StreamMessage> {
    gateway_chat_stream::normalize_chat_tool_event(payload)
        .map(|event| StreamMessage::AgentToolLifecycle(request_id, event))
}

fn gateway_agent_stream(req: AgentRequest, config: desktop_config::ConfigAccess, endpoint: gateway_endpoint::GatewayEndpoint) -> AgentBoxStream<StreamMessage> {
    use iced::futures::SinkExt;
    use iced::futures::StreamExt;
    use serde_json::{Value, json};
    use tracing::{debug, info};

    use session_ui_types as models;

    let s = iced::stream::channel(
        100,
        move |mut output: iced::futures::channel::mpsc::Sender<StreamMessage>| async move {
            info!(target: "vw_desktop", request_id = req.id, session_id = %req.session, endpoint = %endpoint.describe(), "desktop preparing gateway stream");
            let rpc_id = desktop_stream_request_id(&req);
            let client_time_zone = config.load_system_settings_config_async()
                .await
                .map(|config| config.timezone)
                .unwrap_or_else(|_| config_ui::default_timezone());

            let mut messages = Vec::new();
            for message in &req.history {
                messages.push(json!({
                    "role": match message.role {
                        models::ChatRole::User => "user",
                        models::ChatRole::Assistant => "assistant",
                        models::ChatRole::System => "system",
                        models::ChatRole::Developer => "developer",
                        models::ChatRole::Tool => "tool",
                    },
                    "content": message.content,
                }));
            }
            if !req.resume_history_only && !req.query.trim().is_empty() {
                messages.push(json!({"role": "user", "content": req.query}));
            }

            let body = json!({
                "messages": messages,
                "model": req.model,
            });
            let options = gateway_request_options(&req, &rpc_id, &client_time_zone);
            // 仅工作流开启才进入 task_plan_run；目标模式由网关固定开启。
            let workflow_mode_active =
                task_plan_run_active(req.workflow_mode_enabled, req.resume_history_only);
            let plan_run_body = match build_task_plan_run_request(&req) {
                Ok(body) => body,
                Err(error) => {
                    let _ = output
                        .send(StreamMessage::AgentStreamError(req.id, error))
                        .await;
                    return;
                }
            };
            let chat_body = gateway_chat_stream::GatewayChatStreamRequest {
                acp_model_source: req.acp_model_source,
                session_id: Some(req.session.clone().into()),
                messages: body
                    .get("messages")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default(),
                system: None,
                model: body
                    .get("model")
                    .cloned()
                    .and_then(|value| serde_json::from_value(value).ok()),
                reasoning_effort: req.reasoning_effort,
                agent: req.agent.clone(),
                allowed_tools: req.allowed_tools.clone(),
                acp_agent: req.acp_test.then(|| req.acp_agent.clone()).flatten(),
                acp_allowed_tools: if req.acp_test { req.acp_allowed_tools.clone() } else { None },
                options: options.clone(),
            };
            let mut stream_done;
            let mut last_plan_run_event: Option<&'static str> = None;
            let mut plan_run_session_ids = std::collections::HashSet::<String>::new();
            let mut plan_run_terminal_error: Option<String> = None;
            let mut plan_run_cancelled = false;
            let mut attempt = 0_u64;
            let mut plan_run_delta_buffer = PlanRunDeltaBuffer::default();
            // 即使普通聊天也会经过这里；浏览器不支持 std::time::Instant。
            let plan_run_delta_clock = web_time::Instant::now();

            info!(
                target: "vw_desktop",
                request_id = req.id,
                session_id = %req.session,
                acp_test = req.acp_test,
                agent = ?req.agent,
                allowed_tools = ?req.allowed_tools,
                require_agent_tool = req.require_agent_tool,
                acp_agent = ?req.acp_agent,
                acp_allowed_tools = ?req.acp_allowed_tools,
                acp_force_new_session = req.acp_force_new_session,
                acp_history_mode = %req.acp_history_mode.as_str(),
                acp_recent_count = req.acp_recent_count,
                full_access_enabled = ?req.full_access_enabled,
                workflow_mode_enabled = req.workflow_mode_enabled,
                goal_mode_enabled = req.goal_mode_enabled,
                workflow_mode_active,
                model = ?req.model,
                has_root = req.root.is_some(),
                history_len = req.history.len(),
                query_len = req.query.len(),
                options = ?options,
                "starting gateway agent stream"
            );

            let mut received_delta = false;
            let mut reconnect = GatewayReconnectState::new(rpc_id);
            stream_done = false;

            loop {
                attempt = attempt.saturating_add(1);
                let mut saw_new_event = false;
                let mut last_forwarded_event_id = None;
                let mut stop_reason = None;

                if plan_run_body.is_some() && plan_run_delta_buffer.has_pending() {
                    match plan_run_delta_buffer.flush(&mut |batch| {
                        try_send_plan_run_delta_batch(&mut output, req.id, batch)
                    }) {
                        Ok(Some(event_id)) => reconnect.record_forwarded_event(event_id),
                        Ok(None) => {}
                        Err(err) => {
                            gateway_stream_stop_after_forward_err(err, &mut stop_reason);
                        }
                    }
                }
                let resume = reconnect.resume();

                let result = if stop_reason.is_some() {
                    Ok(gateway_chat_stream::GatewayStreamAttemptResult::CallerStopped)
                } else if let Some(plan_run_body) = plan_run_body.as_ref() {
                    let root = req.root.as_deref().expect("validated workflow root");
                    stream_task_plan_run_resumable(
                        &endpoint,
                        plan_run_body,
                        &resume,
                        |stream_event| {
                            let event_id = stream_event.id;
                            let event = stream_event.event;
                            if resume.last_event_id.is_none_or(|last| event_id > last) {
                                saw_new_event = true;
                            }
                            let done_event = matches!(
                                event,
                                vw_api_task::TaskPlanRunStreamEventDto::Finished { .. }
                            );
                            let cancelled_event = matches!(
                                event,
                                vw_api_task::TaskPlanRunStreamEventDto::Cancelled { .. }
                            );
                            let blocked_reason = match &event {
                                vw_api_task::TaskPlanRunStreamEventDto::Blocked {
                                    reason,
                                    ..
                                } => Some(reason.clone()),
                                _ => None,
                            };
                            let error_event = match &event {
                                vw_api_task::TaskPlanRunStreamEventDto::Error {
                                    message,
                                    ..
                                } => Some(message.clone()),
                                _ => None,
                            };
                            let event_type = plan_run_event_type(&event);
                            last_plan_run_event = Some(event_type);
                            match &event {
                                vw_api_task::TaskPlanRunStreamEventDto::SessionDelta {
                                    session_id,
                                    delta,
                                    ..
                                } => debug!(
                                    target: "vw_desktop",
                                    request_id = req.id,
                                    event_id,
                                    event_type,
                                    session_id = %session_id,
                                    char_count = delta.chars().count(),
                                    byte_count = delta.len(),
                                    "desktop received task plan-run session delta"
                                ),
                                _ => info!(
                                    target: "vw_desktop",
                                    request_id = req.id,
                                    event_id,
                                    event_type,
                                    "desktop received task plan-run event"
                                ),
                            }

                            if let vw_api_task::TaskPlanRunStreamEventDto::SessionDelta {
                                session_id,
                                delta,
                                ..
                            } = &event
                            {
                                match plan_run_delta_buffer.push_delta(
                                    session_id,
                                    delta,
                                    event_id,
                                    plan_run_delta_clock.elapsed(),
                                    &mut |batch| {
                                        try_send_plan_run_delta_batch(&mut output, req.id, batch)
                                    },
                                ) {
                                    Ok(forwarded_event_id) => {
                                        merge_forwarded_cursor(
                                            &mut last_forwarded_event_id,
                                            forwarded_event_id,
                                        );
                                        return true;
                                    }
                                    Err(err) => {
                                        return gateway_stream_stop_after_forward_err(
                                            err,
                                            &mut stop_reason,
                                        );
                                    }
                                }
                            }

                            match plan_run_delta_buffer.flush(&mut |batch| {
                                try_send_plan_run_delta_batch(&mut output, req.id, batch)
                            }) {
                                Ok(forwarded_event_id) => merge_forwarded_cursor(
                                    &mut last_forwarded_event_id,
                                    forwarded_event_id,
                                ),
                                Err(err) => {
                                    return gateway_stream_stop_after_forward_err(
                                        err,
                                        &mut stop_reason,
                                    );
                                }
                            }

                            if let Some(started_message) = plan_run_session_started_message(
                                &event,
                                req.id,
                                &req.session,
                                root,
                                &req.query,
                            ) {
                                if let Err(err) = output.try_send(started_message) {
                                    return gateway_stream_stop_after_forward_err(
                                        err,
                                        &mut stop_reason,
                                    );
                                }
                                if let Some(session_id) = plan_run_event_session_id(&event) {
                                    plan_run_session_ids.insert(session_id.to_string());
                                }
                            }

                            if let vw_api_task::TaskPlanRunStreamEventDto::SessionWaiting {
                                session_id,
                                ..
                            } = &event
                                && let Err(err) = output.try_send(
                                    StreamMessage::PlanRunSessionWaiting(session_id.clone()))
                            {
                                return gateway_stream_stop_after_forward_err(
                                    err,
                                    &mut stop_reason,
                                );
                            }

                            if let Some(session_id) =
                                plan_run_session_cancelled_id(&event).map(str::to_string)
                            {
                                match output.try_send(
                                    StreamMessage::PlanRunSessionCancelled(
                                        session_id.clone(),
                                    )) {
                                    Ok(()) => {
                                        plan_run_session_ids.remove(&session_id);
                                    }
                                    Err(err) => {
                                        return gateway_stream_stop_after_forward_err(
                                            err,
                                            &mut stop_reason,
                                        );
                                    }
                                }
                            }

                            if let Some(session_id) =
                                plan_run_session_finished_id(&event).map(str::to_string)
                            {
                                match output.try_send(
                                    StreamMessage::PlanRunSessionFinished(
                                        session_id.clone(),
                                    )) {
                                    Ok(()) => {
                                        plan_run_session_ids.remove(&session_id);
                                    }
                                    Err(err) => {
                                        return gateway_stream_stop_after_forward_err(
                                            err,
                                            &mut stop_reason,
                                        );
                                    }
                                }
                            }

                            if let Err(err) = output.try_send(
                                StreamMessage::AgentWorkflowNodeUpdate(
                                    req.id,
                                    plan_run_tool_block(&event, root),
                                )) {
                                return gateway_stream_stop_after_forward_err(
                                    err,
                                    &mut stop_reason,
                                );
                            }

                            last_forwarded_event_id = Some(event_id);
                            if done_event {
                                stream_done = true;
                            } else if cancelled_event {
                                stream_done = true;
                                plan_run_cancelled = true;
                            } else if let Some(error) = error_event {
                                stream_done = true;
                                plan_run_terminal_error = Some(error);
                            } else if let Some(reason) = blocked_reason {
                                stream_done = true;
                                plan_run_terminal_error = Some(if reason.trim().is_empty() {
                                    "工作流执行受阻".to_string()
                                } else {
                                    format!("工作流执行受阻：{reason}")
                                });
                            } else {
                                debug_assert!(!plan_run_event_is_terminal(&event));
                            }
                            true
                        },
                    )
                    .await
                } else {
                    gateway_chat_stream::stream_chat_resumable(
                        &endpoint,
                        req.root.as_deref(),
                        &chat_body,
                        &resume,
                        |stream_event| {
                            let event_id = stream_event.id;
                            let event = stream_event.event;
                            if resume.last_event_id.is_none_or(|last| event_id > last) {
                                saw_new_event = true;
                            }
                            let event_kind = match &event {
                                gateway_chat_stream::GatewayChatStreamEvent::Delta(_) => "delta",
                                gateway_chat_stream::GatewayChatStreamEvent::Done { .. } => "done",
                                gateway_chat_stream::GatewayChatStreamEvent::Error(_) => "error",
                                gateway_chat_stream::GatewayChatStreamEvent::Other(_) => "other",
                            };
                            let terminal_event = matches!(
                                event,
                                gateway_chat_stream::GatewayChatStreamEvent::Done { .. }
                                    | gateway_chat_stream::GatewayChatStreamEvent::Error(_)
                            );
                            let mut handoff = false;

                            let next_message = match event {
                                gateway_chat_stream::GatewayChatStreamEvent::Delta(delta) => {
                                    if !received_delta && !delta.is_empty() {
                                        received_delta = true;
                                        info!(target: "vw_desktop", request_id = req.id, session_id = %req.session, event_id, "desktop received first gateway delta");
                                    }
                                    tracing::debug!(
                                        target: "vw_desktop",
                                        request_id = req.id,
                                        event_id,
                                        delta_len = delta.len(),
                                        "desktop received gateway delta"
                                    );
                                    Some(StreamMessage::AgentStreamDelta(
                                        req.id, delta,
                                    ))
                                }
                                gateway_chat_stream::GatewayChatStreamEvent::Done {
                                    usage,
                                    message_id,
                                    parent_message_id,
                                    ..
                                } => {
                                    info!(
                                        target: "vw_desktop",
                                        request_id = req.id,
                                        event_id,
                                        has_usage = usage.is_some(),
                                        message_id = ?message_id,
                                        parent_message_id = ?parent_message_id,
                                        "desktop received gateway done"
                                    );
                                    Some(StreamMessage::AgentStreamDone(
                                        req.id,
                                        parse_usage(usage.as_ref()),
                                        message_id,
                                        parent_message_id,
                                    ))
                                }
                                gateway_chat_stream::GatewayChatStreamEvent::Error(error)
                                    if error == "cancelled" =>
                                {
                                    Some(StreamMessage::AgentStreamCancelled(req.id))
                                }
                                gateway_chat_stream::GatewayChatStreamEvent::Error(error) => {
                                    info!(
                                        target: "vw_desktop",
                                        request_id = req.id,
                                        event_id,
                                        "desktop received gateway error"
                                    );
                                    Some(StreamMessage::AgentStreamError(
                                        req.id, error,
                                    ))
                                }
                                gateway_chat_stream::GatewayChatStreamEvent::Other(payload) => {
                                    match payload
                                        .get("type")
                                        .and_then(serde_json::Value::as_str)
                                        .unwrap_or_default()
                                    {
                                        "chat.tool" => chat_tool_message(req.id, &payload),
                                        "chat.session_updated" => payload
                                            .get("session")
                                            .and_then(|session| session.get("summary"))
                                            .and_then(|summary| {
                                                serde_json::from_value(summary.clone()).ok()
                                            })
                                            .map(|summary| {

                                                    StreamMessage::AgentSessionDiffSummary(
                                                        req.id, req.session.clone(), summary,
                                                    )
                                            }),
                                        "chat.model_recovery" => chat_model_recovery_message(
                                            req.id,
                                            &req.session,
                                            &payload,
                                        ),
                                        "chat.todo_continuation" => {
                                            Some(
                                                StreamMessage::AgentTodoContinuation(req.id))
                                        }
                                        "chat.compaction" => {
                                            chat_compaction_message(req.id, &payload)
                                        }
                                        "chat.context_loaded" => {
                                            let paths = payload
                                                .get("paths")
                                                .and_then(serde_json::Value::as_array)
                                                .into_iter()
                                                .flatten()
                                                .filter_map(serde_json::Value::as_str)
                                                .map(ToOwned::to_owned)
                                                .collect::<Vec<_>>();
                                            (!paths.is_empty()).then(|| {

                                                    StreamMessage::AgentContextLoaded(
                                                        req.id, paths,
                                                    )
                                            })
                                        }
                                        "chat.step_start" => {
                                            let step_index = payload
                                                .get("step_index")
                                                .and_then(serde_json::Value::as_u64)
                                                .unwrap_or_default()
                                                as u32;
                                            let created_ms = payload
                                                .get("created_ms")
                                                .and_then(serde_json::Value::as_u64)
                                                .unwrap_or_default();
                                            let model = payload
                                                .get("model")
                                                .and_then(serde_json::Value::as_str)
                                                .map(ToOwned::to_owned);
                                            Some(
                                                StreamMessage::AgentStepStart(
                                                    req.id,
                                                    req.session.clone(),
                                                    step_index,
                                                    created_ms,
                                                    model,
                                                ))
                                        }
                                        "chat.step_finish" => {
                                            let step_index = payload
                                                .get("step_index")
                                                .and_then(serde_json::Value::as_u64)
                                                .unwrap_or_default()
                                                as u32;
                                            let finished_ms = payload
                                                .get("finished_ms")
                                                .and_then(serde_json::Value::as_u64)
                                                .unwrap_or_default();
                                            let usage = parse_usage(payload.get("usage"));
                                            let finish_reason = payload
                                                .get("finish_reason")
                                                .and_then(serde_json::Value::as_str)
                                                .map(ToOwned::to_owned);
                                            let model = payload
                                                .get("model")
                                                .and_then(serde_json::Value::as_str)
                                                .map(ToOwned::to_owned);
                                            Some(
                                                StreamMessage::AgentStepFinish(
                                                    req.id,
                                                    req.session.clone(),
                                                    step_index,
                                                    finished_ms,
                                                    usage,
                                                    finish_reason,
                                                    model,
                                                ))
                                        }
                                        "chat.post_tool_round" => {
                                            if !crate::session::agent::take_pending_guide_handoff(
                                                req.id,
                                            ) {
                                                None
                                            } else {
                                                handoff = true;
                                                let step_index = payload
                                                    .get("step_index")
                                                    .and_then(serde_json::Value::as_u64)
                                                    .unwrap_or_default()
                                                    as u32;
                                                Some(
                                                    StreamMessage::AgentPostToolRound(
                                                        req.id,
                                                        req.session.clone(),
                                                        step_index,
                                                    ))
                                            }
                                        }
                                        _ => None,
                                    }
                                }
                            };

                            let Some(next_message) = next_message else {
                                last_forwarded_event_id = Some(event_id);
                                return true;
                            };

                            match output.try_send(next_message) {
                                Ok(()) => {
                                    last_forwarded_event_id = Some(event_id);
                                    if terminal_event {
                                        stream_done = true;
                                    }
                                    if handoff {
                                        stop_reason =
                                            Some(GatewayStreamStopReason::PostToolRoundHandoff);
                                        false
                                    } else {
                                        true
                                    }
                                }
                                Err(err) => {
                                    debug!(
                                        target: "vw_desktop",
                                        request_id = req.id,
                                        event_id,
                                        event_kind,
                                        "desktop agent stream forward failed"
                                    );
                                    gateway_stream_stop_after_forward_err(err, &mut stop_reason)
                                }
                            }
                        },
                    )
                    .await
                };

                if plan_run_body.is_some() && stop_reason.is_none() {
                    match plan_run_delta_buffer.flush(&mut |batch| {
                        try_send_plan_run_delta_batch(&mut output, req.id, batch)
                    }) {
                        Ok(forwarded_event_id) => {
                            merge_forwarded_cursor(&mut last_forwarded_event_id, forwarded_event_id)
                        }
                        Err(err) => {
                            gateway_stream_stop_after_forward_err(err, &mut stop_reason);
                        }
                    }
                }

                match reconnect.finish_attempt(
                    result,
                    saw_new_event,
                    last_forwarded_event_id,
                    stop_reason,
                ) {
                    GatewayStreamNext::Terminal => break,
                    GatewayStreamNext::Cancelled => {
                        debug!(
                            target: "vw_desktop",
                            request_id = req.id,
                            attempt,
                            last_event_id = reconnect.last_event_id,
                            last_plan_run_event,
                            "gateway agent stream cancelled by UI subscription drop"
                        );
                        return;
                    }
                    GatewayStreamNext::PostToolRoundHandoff => {
                        debug!(
                            target: "vw_desktop",
                            request_id = req.id,
                            attempt,
                            "gateway agent stream stopped after post-tool-round handoff"
                        );
                        return;
                    }
                    GatewayStreamNext::Failed(error) => {
                        for session_id in plan_run_session_ids.drain() {
                            let _ = output
                                .send(StreamMessage::PlanRunSessionFinished(
                                    session_id,
                                ))
                                .await;
                        }
                        debug!(
                            target: "vw_desktop",
                            request_id = req.id,
                            attempt,
                            error = %error,
                            "gateway agent stream failed without retry"
                        );
                        let _ = output
                            .send(StreamMessage::AgentStreamError(
                                req.id, error,
                            ))
                            .await;
                        return;
                    }
                    GatewayStreamNext::Retry(delay) => {
                        debug!(
                            target: "vw_desktop",
                            request_id = req.id,
                            stream_kind = if workflow_mode_active { "plan_run" } else { "chat" },
                            attempt,
                            delay_ms = delay.as_millis(),
                            last_event_id = reconnect.last_event_id,
                            last_plan_run_event,
                            "gateway agent stream retry scheduled"
                        );
                        gateway_stream_retry_sleep(delay).await;
                    }
                }
            }

            if workflow_mode_active && stream_done {
                for session_id in plan_run_session_ids.drain() {
                    let _ = output
                        .send(if plan_run_cancelled {
                            StreamMessage::PlanRunSessionCancelled(session_id)
                        } else {
                            StreamMessage::PlanRunSessionFinished(session_id)
                        })
                        .await;
                }
                let terminal_message = if plan_run_cancelled {
                    StreamMessage::AgentStreamCancelled(req.id)
                } else {
                    match plan_run_terminal_error {
                        Some(error) => {
                            StreamMessage::AgentStreamError(req.id, error)
                        }
                        None => StreamMessage::AgentStreamDone(
                            req.id,
                            session_ui_types::TokenUsage::default(),
                            None,
                            None,
                        ),
                    }
                };
                let _ = output.send(terminal_message).await;
            }
        },
    );

    #[cfg(not(target_arch = "wasm32"))]
    return s.boxed();
    #[cfg(target_arch = "wasm32")]
    return s.boxed_local();
}

fn chat_compaction_message(request_id: u64, payload: &serde_json::Value) -> Option<StreamMessage> {
    let completed = match payload.get("phase").and_then(serde_json::Value::as_str)? {
        "started" => false,
        "completed" => true,
        _ => return None,
    };
    Some(StreamMessage::AgentCompactionStatus(request_id, completed))
}

fn parse_usage(value: Option<&serde_json::Value>) -> session_ui_types::TokenUsage {
    let Some(value) = value else {
        return session_ui_types::TokenUsage::default();
    };
    if value.get("prompt_tokens").is_some()
        || value.get("completion_tokens").is_some()
        || value.get("total_tokens").is_some()
    {
        return session_ui_types::TokenUsage {
            input_tokens: value
                .get("prompt_tokens")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0),
            output_tokens: value
                .get("completion_tokens")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0),
            cached_tokens: 0,
            reasoning_tokens: 0,
        };
    }
    serde_json::from_value(value.clone()).unwrap_or_default()
}

#[cfg(test)]
#[path = "agent_stream_tests.rs"]
mod agent_stream_tests;

fn chat_model_recovery_message(
    request_id: u64,
    session_id: &str,
    payload: &serde_json::Value,
) -> Option<StreamMessage> {
    let event: gateway_chat_stream::GatewayModelRecoveryEvent =
        serde_json::from_value(payload.clone()).ok()?;
    (event.session_id == session_id)
        .then_some(StreamMessage::AgentModelRecovery(request_id, event))
}
