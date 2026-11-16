//! 原生工作流的宿主请求、运行时工厂与事件适配。
use crate::session::processor::{Request, StreamEvent};
use crate::session::ui_types::TokenUsage;
use crate::tools::ToolRuntimeContext;
use workflow_engine::{WorkflowRuntime, native_command::{BoundCommandRequest, CommandEvent, NativeCommandHost}};
use vw_api_workflow::WorkflowRunRequest;
use std::sync::Arc;

#[path = "native_agent.rs"]
mod agent;

struct NativeCommandAdapter<'a> {
    request: &'a Request,
    context: &'a ToolRuntimeContext,
}

impl NativeCommandHost for NativeCommandAdapter<'_> {
    type Event = StreamEvent;
    fn now_ms() -> u64 { crate::session::session::now_ms() }
    fn event(event: CommandEvent) -> StreamEvent {
        match event {
            CommandEvent::Tool(event) => StreamEvent::Tool(event),
            CommandEvent::StepStart(created_ms) => StreamEvent::StepStart { step_index: 0, created_ms, model: None },
            CommandEvent::StepFinish(finished_ms) => StreamEvent::StepFinish {
                step_index: 0, finished_ms, usage: TokenUsage::default(), finish_reason: Some("workflow".into()), model: None,
            },
            CommandEvent::Delta(text) => StreamEvent::Delta(text),
            CommandEvent::Done => StreamEvent::Done(TokenUsage::default()),
            CommandEvent::Error(error) => StreamEvent::Error(error),
        }
    }
    fn default_runtime(&self, tx: tokio::sync::mpsc::UnboundedSender<StreamEvent>, cancellation: tokio_util::sync::CancellationToken) -> WorkflowRuntime {
        let request = self.request;
        let context = self.context;
        let tools = Arc::new(agent::NativeTools {
            context: context.clone().with_session_tools(request.session_tools.clone()),
            allowed_tools: request.options.get("allowed_tools").cloned(),
        });
        WorkflowRuntime {
            video_edit_budget: crate::workflow::budget::VideoEditRunBudget::default()
                .with_cancellation(cancellation.clone()),
            provider: Arc::new(crate::providers::session_llm::SessionLlmProvider::new()),
            model: request.model.clone().unwrap_or_default(),
            temperature: 0.2,
            tool_provider: Some(tools),
            agent_provider: Some(Arc::new(agent::NativeAgent {
                parent: request.clone(),
                events: tx.clone(),
                cancellation: cancellation.clone(),
            })),
            knowledge_provider: None,
            document_extractor: None,
            pause_store: None,
        }
    }
}

pub(crate) fn run_bound_command(
    request: &Request,
    context: &ToolRuntimeContext,
    emit: &mut (impl FnMut(StreamEvent) -> bool + Send),
) -> bool {
    run_command(request, context, emit, None)
}

pub(crate) fn run_video_command(
    request: &Request,
    context: &ToolRuntimeContext,
    skills: &[crate::skills::Skill],
    emit: &mut (impl FnMut(StreamEvent) -> bool + Send),
) -> bool {
    let query = super::video_edit::entry::command_query(&request.query);
    match super::video_edit::parse_video_edit_command(&query) {
        Ok(None) => return false,
        Err(error) => {
            emit(StreamEvent::Error(format!("{error}。用法：/shiju 剪辑 \"视频路径\" [要求]")));
            return true;
        }
        Ok(Some(_)) => {}
    }
    let prepared =
        crate::session::prompt::block_on(super::video_edit::entry::build(request, context, skills));
    match prepared {
        Ok(prepared) => run_command(request, context, emit, Some(prepared)),
        Err(error) => {
            emit(StreamEvent::Error(format!("云端剪辑入口失败: {error:#}")));
            true
        }
    }
}

fn run_command(
    request: &Request,
    context: &ToolRuntimeContext,
    emit: &mut (impl FnMut(StreamEvent) -> bool + Send),
    prepared: Option<(WorkflowRuntime, WorkflowRunRequest)>,
) -> bool {
    let host = NativeCommandAdapter { request, context };
    let binding = BoundCommandRequest {
        query: &request.query,
        root: request.root.as_deref(),
        acp_test: request.options.get("acp_test").and_then(serde_json::Value::as_bool) == Some(true),
        current_tenant: crate::session::ui_store::current_tenant_scope,
    };
    crate::session::prompt::block_on(workflow_engine::native_command::run_command(
        &host, binding, request.abort_token.clone().unwrap_or_default(), emit, prepared,
    ))
}

#[cfg(test)]
#[path = "native_tests.rs"]
mod tests;
