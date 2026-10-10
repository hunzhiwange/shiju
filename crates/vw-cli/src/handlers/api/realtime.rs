//! 推理 JSON、SSE、可停止流与 WebSocket chat 命令。

use std::io::{Read, Write};

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use serde_json::{Value, json};
use vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerScopeDto;
use vw_gateway_client::vw_api_types::chat::{GatewayChatStreamEvent, GatewayChatStreamRequest};
use vw_gateway_client::vw_api_types::id::{ProjectId, ProjectScope};
use vw_gateway_client::vw_api_types::task::{TaskPlanRunRequest, TaskPlanRunStreamEventDto};
use vw_gateway_client::{
    GatewayAgentRequest, GatewayChatWebSocketEvent, GatewayJsonSseEvent,
    GatewayStreamAttemptResult, GatewayStreamEvent, GatewayStreamResume,
};

use super::context::ApiContext;
use super::input::read_json_input;
use super::output;

#[derive(Subcommand, Debug)]
pub(super) enum RealtimeCommands {
    Agent {
        #[arg(long, value_parser = required)]
        input: String,
    },
    Openai {
        #[arg(long, value_parser = required)]
        input: String,
    },
    GatewayEvents {
        #[arg(long)]
        limit: Option<usize>,
    },
    GlobalEvents {
        #[arg(long)]
        limit: Option<usize>,
    },
    InstanceEvents {
        #[arg(long)]
        limit: Option<usize>,
    },
    SchedulerEvents {
        #[arg(value_parser = required)]
        project_id: String,
        #[arg(long)]
        after_cursor: Option<i64>,
        #[arg(long)]
        limit: Option<usize>,
    },
    ChatStream {
        #[arg(long, value_parser = required)]
        request_id: String,
        #[arg(long)]
        last_event_id: Option<u64>,
        #[arg(long, value_parser = required)]
        input: String,
    },
    ChatStop {
        #[arg(value_parser = required)]
        request_id: String,
    },
    PlanStream {
        #[arg(long, value_parser = required)]
        request_id: String,
        #[arg(long)]
        last_event_id: Option<u64>,
        #[arg(long, value_parser = required)]
        input: String,
    },
    PlanStop {
        #[arg(value_parser = required)]
        request_id: String,
    },
    Websocket {
        #[arg(long, value_parser = required)]
        input: String,
    },
}

impl RealtimeCommands {
    pub(super) fn dangerous_operation(&self) -> Option<String> {
        match self {
            Self::Agent { .. } => Some("run agent inference".to_string()),
            Self::Openai { .. } => Some("run OpenAI-compatible inference".to_string()),
            Self::ChatStream { request_id, .. } => Some(format!("run chat stream '{request_id}'")),
            Self::ChatStop { request_id } => Some(format!("stop chat stream '{request_id}'")),
            Self::PlanStream { request_id, .. } => {
                Some(format!("run task plan stream '{request_id}'"))
            }
            Self::PlanStop { request_id } => Some(format!("stop task plan stream '{request_id}'")),
            Self::Websocket { .. } => Some("open WebSocket chat session".to_string()),
            Self::GatewayEvents { .. }
            | Self::GlobalEvents { .. }
            | Self::InstanceEvents { .. }
            | Self::SchedulerEvents { .. } => None,
        }
    }
}

pub(super) async fn run(
    command: RealtimeCommands,
    context: &ApiContext,
    stdin: &mut impl Read,
    writer: &mut impl Write,
) -> Result<()> {
    match command {
        RealtimeCommands::Agent { input } => {
            let request = parse::<GatewayAgentRequest>(&input, stdin, "agent")?;
            let response =
                context.client().inference_agent(&request).await.map_err(anyhow::Error::msg)?;
            output::write_json(writer, &response, context.output_format())
        }
        RealtimeCommands::Openai { input } => {
            let request = read_json_input(&input, stdin)?;
            let response = context
                .client()
                .inference_openai_chat(&request)
                .await
                .map_err(anyhow::Error::msg)?;
            output::write_json(writer, &response, context.output_format())
        }
        RealtimeCommands::GatewayEvents { limit } => {
            let mut sink = JsonEventSink::new(writer, limit);
            let result = context.client().stream_gateway_events(|event| sink.write(event)).await;
            sink.finish(result)
        }
        RealtimeCommands::GlobalEvents { limit } => {
            let mut sink = JsonEventSink::new(writer, limit);
            let result = context.client().stream_global_events(|event| sink.write(event)).await;
            sink.finish(result)
        }
        RealtimeCommands::InstanceEvents { limit } => {
            let mut sink = JsonEventSink::new(writer, limit);
            let result = context
                .client()
                .stream_instance_events(context.directory(), |event| sink.write(event))
                .await;
            sink.finish(result)
        }
        RealtimeCommands::SchedulerEvents { project_id, after_cursor, limit } => {
            let scope = scheduler_scope(context, project_id)?;
            let mut sink = JsonEventSink::new(writer, limit);
            let result = context
                .client()
                .stream_agent_scheduler_events(&scope, after_cursor, |event| sink.write(event))
                .await;
            sink.finish(result)
        }
        RealtimeCommands::ChatStream { request_id, last_event_id, input } => {
            let request = parse::<GatewayChatStreamRequest>(&input, stdin, "chat stream")?;
            run_chat_stream(context, request_id, last_event_id, &request, writer).await
        }
        RealtimeCommands::ChatStop { request_id } => {
            let cancelled =
                context.client().chat_stream_stop(&request_id).await.map_err(anyhow::Error::msg)?;
            output::write_json(writer, &json!({"cancelled":cancelled}), context.output_format())
        }
        RealtimeCommands::PlanStream { request_id, last_event_id, input } => {
            let request = parse::<TaskPlanRunRequest>(&input, stdin, "task plan stream")?;
            run_plan_stream(context, request_id, last_event_id, &request, writer).await
        }
        RealtimeCommands::PlanStop { request_id } => {
            let cancelled = context
                .client()
                .task_plan_run_stream_stop(&request_id)
                .await
                .map_err(anyhow::Error::msg)?;
            output::write_json(writer, &json!({"cancelled":cancelled}), context.output_format())
        }
        RealtimeCommands::Websocket { input } => {
            let messages = parse::<Vec<Value>>(&input, stdin, "WebSocket messages")?;
            run_websocket(context, &messages, writer).await
        }
    }
}

struct JsonEventSink<'a, W> {
    writer: &'a mut W,
    limit: Option<usize>,
    count: usize,
    write_error: Option<anyhow::Error>,
}

impl<'a, W: Write> JsonEventSink<'a, W> {
    fn new(writer: &'a mut W, limit: Option<usize>) -> Self {
        Self { writer, limit, count: 0, write_error: None }
    }

    fn write(&mut self, event: GatewayJsonSseEvent) -> bool {
        if let Err(error) = output::write_jsonl_item(self.writer, &event) {
            self.write_error = Some(error);
            return false;
        }
        self.count += 1;
        self.limit.is_none_or(|limit| self.count < limit)
    }

    fn finish(self, result: Result<GatewayStreamAttemptResult, String>) -> Result<()> {
        let result = result.map_err(anyhow::Error::msg)?;
        if let Some(error) = self.write_error {
            return Err(error);
        }
        output::write_jsonl_item(
            self.writer,
            &json!({"type":"stream_end","status":attempt_status(result)}),
        )
    }
}

async fn run_chat_stream(
    context: &ApiContext,
    request_id: String,
    last_event_id: Option<u64>,
    request: &GatewayChatStreamRequest,
    writer: &mut impl Write,
) -> Result<()> {
    let resume = GatewayStreamResume { request_id, last_event_id };
    let mut stream_error = None;
    let mut write_error = None;
    let result = vw_gateway_client::GatewayClient::stream_chat_resumable(
        context.client().endpoint(),
        context.directory(),
        request,
        &resume,
        |event| {
            if let GatewayChatStreamEvent::Error(error) = &event.event {
                stream_error = Some(error.clone());
            }
            if let Err(error) = output::write_jsonl_item(writer, &chat_event_json(&event)) {
                write_error = Some(error);
                return false;
            }
            true
        },
    )
    .await
    .map_err(anyhow::Error::msg)?;
    finish_protocol_stream(writer, result, write_error, stream_error)
}

fn chat_event_json(event: &GatewayStreamEvent<GatewayChatStreamEvent>) -> Value {
    let payload = match &event.event {
        GatewayChatStreamEvent::Delta(delta) => json!({"type":"chat.delta","delta":delta}),
        GatewayChatStreamEvent::Done { finish_reason, usage, message_id, parent_message_id } => {
            json!({
                "type":"chat.done", "finish_reason":finish_reason, "usage":usage,
                "message_id":message_id, "parent_message_id":parent_message_id
            })
        }
        GatewayChatStreamEvent::Error(error) => json!({"type":"chat.error","error":error}),
        GatewayChatStreamEvent::Other(payload) => payload.clone(),
    };
    json!({"id":event.id,"event":payload})
}

async fn run_plan_stream(
    context: &ApiContext,
    request_id: String,
    last_event_id: Option<u64>,
    request: &TaskPlanRunRequest,
    writer: &mut impl Write,
) -> Result<()> {
    let resume = GatewayStreamResume { request_id, last_event_id };
    let mut stream_error = None;
    let mut write_error = None;
    let result = vw_gateway_client::GatewayClient::stream_task_plan_run_resumable(
        context.client().endpoint(),
        request,
        &resume,
        |event: GatewayStreamEvent<TaskPlanRunStreamEventDto>| {
            if let TaskPlanRunStreamEventDto::Error { message, .. } = &event.event {
                stream_error = Some(message.clone());
            }
            if let Err(error) = output::write_jsonl_item(writer, &event) {
                write_error = Some(error);
                return false;
            }
            true
        },
    )
    .await
    .map_err(anyhow::Error::msg)?;
    finish_protocol_stream(writer, result, write_error, stream_error)
}

fn finish_protocol_stream(
    writer: &mut impl Write,
    result: GatewayStreamAttemptResult,
    write_error: Option<anyhow::Error>,
    stream_error: Option<String>,
) -> Result<()> {
    if let Some(error) = write_error {
        return Err(error);
    }
    output::write_jsonl_item(
        writer,
        &json!({"type":"stream_end","status":attempt_status(result)}),
    )?;
    if let Some(error) = stream_error {
        bail!(error);
    }
    if result == GatewayStreamAttemptResult::Incomplete {
        bail!("stream ended before a terminal event");
    }
    Ok(())
}

async fn run_websocket(
    context: &ApiContext,
    messages: &[Value],
    writer: &mut impl Write,
) -> Result<()> {
    if messages.is_empty() {
        bail!("WebSocket messages must not be empty");
    }
    let mut socket = context.client().connect_chat_websocket().await.map_err(anyhow::Error::msg)?;
    for message in messages {
        socket.send_json(message).await.map_err(anyhow::Error::msg)?;
    }
    let mut protocol_error = None;
    loop {
        let Some(event) = socket.next_event().await.map_err(anyhow::Error::msg)? else {
            bail!("chat WebSocket ended without a close frame");
        };
        if let GatewayChatWebSocketEvent::Text { data } = &event
            && data.get("type").and_then(Value::as_str) == Some("error")
        {
            protocol_error = Some(
                data.get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("WebSocket error")
                    .to_string(),
            );
        }
        let closed = event.is_closed();
        let abnormal = matches!(
            &event,
            GatewayChatWebSocketEvent::Closed { code: Some(code), .. }
                if !matches!(*code, 1000 | 1001)
        );
        output::write_jsonl_item(writer, &event)?;
        if closed {
            if abnormal {
                bail!("chat WebSocket closed abnormally");
            }
            break;
        }
    }
    if let Some(error) = protocol_error {
        bail!(error);
    }
    Ok(())
}

fn scheduler_scope(context: &ApiContext, project_id: String) -> Result<AgentSchedulerScopeDto> {
    let scope = ProjectScope::new(
        context.client().endpoint().tenant_scope().clone(),
        ProjectId::from(project_id),
    )
    .context("invalid scheduler project scope")?;
    Ok(AgentSchedulerScopeDto::new(scope))
}

fn attempt_status(result: GatewayStreamAttemptResult) -> &'static str {
    match result {
        GatewayStreamAttemptResult::Terminated => "terminated",
        GatewayStreamAttemptResult::CallerStopped => "caller_stopped",
        GatewayStreamAttemptResult::Incomplete => "incomplete",
    }
}

fn parse<T: serde::de::DeserializeOwned>(
    source: &str,
    stdin: &mut impl Read,
    operation: &str,
) -> Result<T> {
    serde_json::from_value(read_json_input(source, stdin)?)
        .with_context(|| format!("invalid realtime {operation} JSON input"))
}

fn required(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("value must not be empty".to_string());
    }
    Ok(value.to_string())
}
