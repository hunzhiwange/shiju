//! Native processor adapter for executable workflow agent nodes.

use crate::session::processor::{Request, StreamEvent};

use crate::session::ui_types::{ChatMessage, ChatRole};
use crate::tools::{ToolRuntimeContext, execute_tool_call};
use crate::workflow::{
    WorkflowAgentProvider, WorkflowAgentRequest, WorkflowAgentResult, WorkflowToolProvider,
    WorkflowToolRequest, WorkflowToolResult,
};
use serde_json::Value;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub(super) struct NativeTools {
    pub(super) context: ToolRuntimeContext,
    pub(super) allowed_tools: Option<Value>,
}

#[cfg(test)]
pub(super) use workflow_engine::check_allowed;

impl workflow_engine::native_tools::NativeToolHost for NativeTools {
    fn allowed_tools(&self) -> Option<&Value> { self.allowed_tools.as_ref() }
    fn execute(&self, name: &str, args: &str) -> Result<tool_types::ToolCallResult, String> {
        execute_tool_call(name, args, &self.context).map_err(|error| match error {
            crate::tools::ToolCallError::Denied { message, .. }
            | crate::tools::ToolCallError::Failed(message) => message,
        })
    }
}

#[async_trait::async_trait]
impl WorkflowToolProvider for NativeTools {
    async fn call(&self, request: WorkflowToolRequest) -> Result<WorkflowToolResult, String> {
        workflow_engine::native_tools::call_tool(self, request)
    }
}

#[derive(Clone)]
pub(super) struct NativeAgent {
    pub(super) parent: Request,
    pub(super) events: tokio::sync::mpsc::UnboundedSender<StreamEvent>,
    pub(super) cancellation: CancellationToken,
}

impl workflow_engine::native_agent::NativeAgentHost for NativeAgent {
    type Stage = (Request, vw_api_transport::TenantScope);

    fn allowed_tools(&self) -> Option<&Value> {
        self.parent.options.get("allowed_tools")
    }

    fn prepare_stage(&self, request: WorkflowAgentRequest, allowed: Vec<String>, cancellation: CancellationToken) -> Result<Self::Stage, String> {
        let mut parent = self.parent.clone();
        parent.session = format!("workflow-stage-{}", uuid::Uuid::new_v4());
        parent.model = Some(request.model);
        parent.history = request
            .messages
            .iter()
            .map(|message| {
                let role = match message.role.as_str() {
                    "user" => ChatRole::User,
                    "assistant" => ChatRole::Assistant,
                    "system" => ChatRole::System,
                    "developer" => ChatRole::Developer,
                    _ => return Err("工作流 agent 消息角色不受支持".to_string()),
                };
                Ok(ChatMessage {
                    role,
                    content: message.content.clone(),
                    model: None,
                    think_timing: vec![],
                    processing_timing: None,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let last = parent.history.pop().ok_or("工作流 agent 缺少消息")?;
        if !matches!(last.role, ChatRole::User) {
            return Err("工作流 agent 最后一条消息必须是 user".into());
        }
        parent.query = last.content;
        parent.options["allowed_tools"] = serde_json::json!(allowed);
        parent.assistant_message_id = None;
        parent.persist_app_session_artifacts = false;
        parent.app_session_scope = None;
        parent.abort_token = Some(cancellation.clone());
        Ok((parent, crate::session::ui_store::current_tenant_scope()))
    }

    fn with_stage<F>(stage: Self::Stage, consume: F) -> Result<WorkflowAgentResult, String>
    where F: FnOnce(session_port::PromptTurn) -> Result<WorkflowAgentResult, String> {
        let (parent, tenant) = stage;
        crate::session::ui_store::with_tenant_scope_sync(tenant, || {
            consume(crate::session::runtime::start_prompt(parent))
        })
    }

    fn emit_tool(&self, event: session_port::ToolEvent) {
        let _ = self.events.send(StreamEvent::Tool(event));
    }

    fn visible_answer(output: &str) -> String {
        crate::session::child_agent::visible_answer_from_execution_output(output)
    }
}

#[async_trait::async_trait]
impl WorkflowAgentProvider for NativeAgent {
    async fn run(&self, request: WorkflowAgentRequest, _tools: Arc<dyn WorkflowToolProvider>) -> Result<WorkflowAgentResult, String> {
        workflow_engine::native_agent::run_agent(self.clone(), request, self.cancellation.clone()).await
    }
}
