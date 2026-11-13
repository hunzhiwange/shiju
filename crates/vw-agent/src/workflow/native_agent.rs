//! Native processor adapter for executable workflow agent nodes.

use crate::session::processor::{Request, StreamEvent};
use crate::session::runtime::{PromptEvent, PromptTermination};
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

pub(super) fn check_allowed(name: &str, allowed: Option<&Value>) -> Result<(), String> {
    if let Some(allowed) = allowed {
        let allowed = allowed.as_array().ok_or("allowed_tools 必须是数组")?;
        if !allowed.iter().any(|value| value.as_str() == Some(name)) {
            return Err(format!("工作流工具不在当前会话能力范围内: {name}"));
        }
    }
    Ok(())
}

#[async_trait::async_trait]
impl WorkflowToolProvider for NativeTools {
    async fn call(&self, request: WorkflowToolRequest) -> Result<WorkflowToolResult, String> {
        if request.provider != "shiju"
            || request.action.is_some()
            || request.credential_id.is_some()
        {
            return Err("原生工作流工具仅支持 shiju，不能覆盖 action 或 credential".into());
        }
        check_allowed(&request.tool_name, self.allowed_tools.as_ref())?;
        let args = serde_json::to_string(&request.inputs).map_err(|e| e.to_string())?;
        let result = execute_tool_call(&request.tool_name, &args, &self.context).map_err(
            |error| match error {
                crate::tools::ToolCallError::Denied { message, .. }
                | crate::tools::ToolCallError::Failed(message) => message,
            },
        )?;
        // Shell terminal snapshots remain protocol data even on failure.
        // A workflow must still stop before downstream side effects.
        if !result.is_success() {
            return Err(result.error_text().unwrap_or_else(|| "workflow tool failed".into()));
        }
        Ok(WorkflowToolResult {
            text: Some(result.model_text()),
            json: Some(result.data.clone()),
            result: result.data,
            files: vec![],
        })
    }
}

pub(super) struct NativeAgent {
    pub(super) parent: Request,
    pub(super) events: tokio::sync::mpsc::UnboundedSender<StreamEvent>,
    pub(super) cancellation: CancellationToken,
}

#[async_trait::async_trait]
impl WorkflowAgentProvider for NativeAgent {
    async fn run(
        &self,
        request: WorkflowAgentRequest,
        _tools: Arc<dyn WorkflowToolProvider>,
    ) -> Result<WorkflowAgentResult, String> {
        let mut allowed = Vec::new();
        for tool in &request.tools {
            if tool.provider != "shiju" || tool.action.is_some() || tool.credential_id.is_some() {
                return Err("原生工作流 agent 仅支持当前会话的 shiju 工具".into());
            }
            check_allowed(&tool.tool_name, self.parent.options.get("allowed_tools"))?;
            allowed.push(tool.tool_name.clone());
        }
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
        let cancellation = self.cancellation.child_token();
        parent.abort_token = Some(cancellation.clone());
        let tenant = crate::session::ui_store::current_tenant_scope();
        let events = self.events.clone();
        tokio::task::spawn_blocking(move || {
            crate::session::ui_store::with_tenant_scope_sync(tenant, || {
                let turn = crate::session::runtime::start_prompt(parent);
                let mut output = String::new();
                let mut iterations = 0;
                while let Ok(event) = turn.events.recv() {
                    match event {
                        PromptEvent::Text(delta) => output.push_str(&delta),
                        PromptEvent::Tool(event) => {
                            let _ = events.send(StreamEvent::Tool(event));
                        }
                        PromptEvent::StepStart { .. } => {
                            // Agent nodes expose the final turn, not earlier tool-round commentary.
                            output.clear();
                            iterations += 1;
                            if iterations > request.max_iterations {
                                turn.cancel.cancel();
                                return Err("workflow agent 超过最大迭代次数".to_string());
                            }
                        }
                        PromptEvent::Terminated(PromptTermination::Completed(_)) => {
                            let answer =
                                crate::session::child_agent::visible_answer_from_execution_output(
                                    &output,
                                );
                            return Ok(WorkflowAgentResult {
                                answer,
                                tool_outputs: vec![],
                                reasoning: None,
                                iterations,
                                success: true,
                            });
                        }
                        PromptEvent::Terminated(PromptTermination::Failed(error)) => {
                            return Err(error);
                        }
                        PromptEvent::Terminated(PromptTermination::Cancelled) => {
                            return Err("cancelled".into());
                        }
                        _ => {}
                    }
                }
                cancellation.cancel();
                Err("workflow agent 未返回终态".into())
            })
        })
        .await
        .map_err(|e| format!("workflow agent 执行失败: {e}"))?
    }
}
