//! 原生工作流 agent 的权限检查、轮次与终态处理。
use crate::{check_allowed, WorkflowAgentRequest, WorkflowAgentResult};
use serde_json::Value;
use session_port::{PromptEvent, PromptTermination, PromptTurn, ToolEvent};
use tokio_util::sync::CancellationToken;

/// 宿主保留强类型请求、租户作用域及会话启动；领域层消费原提示事件。
pub trait NativeAgentHost: Send + Sync + 'static {
    type Stage: Send + 'static;
    fn allowed_tools(&self) -> Option<&Value>;
    fn prepare_stage(&self, request: WorkflowAgentRequest, allowed: Vec<String>, cancellation: CancellationToken) -> Result<Self::Stage, String>;
    fn with_stage<F>(stage: Self::Stage, consume: F) -> Result<WorkflowAgentResult, String>
    where F: FnOnce(PromptTurn) -> Result<WorkflowAgentResult, String>;
    fn emit_tool(&self, event: ToolEvent);
    fn visible_answer(output: &str) -> String;
}

pub async fn run_agent<H: NativeAgentHost>(host: H, request: WorkflowAgentRequest, cancellation: CancellationToken) -> Result<WorkflowAgentResult, String> {
        let mut allowed = Vec::new();
        for tool in &request.tools {
            if tool.provider != "shiju" || tool.action.is_some() || tool.credential_id.is_some() {
                return Err("原生工作流 agent 仅支持当前会话的 shiju 工具".into());
            }
            check_allowed(&tool.tool_name, host.allowed_tools())?;
            allowed.push(tool.tool_name.clone());
        }
    let max_iterations = request.max_iterations;
    let cancellation = cancellation.child_token();
    let stage = host.prepare_stage(request, allowed, cancellation.clone())?;
    tokio::task::spawn_blocking(move || {
        H::with_stage(stage, |turn| {
                let mut output = String::new();
                let mut iterations = 0;
                while let Ok(event) = turn.events.recv() {
                    match event {
                        PromptEvent::Text(delta) => output.push_str(&delta),
                        PromptEvent::Tool(event) => {
                            host.emit_tool(event);
                        }
                        PromptEvent::StepStart { .. } => {
                            // Agent nodes expose the final turn, not earlier tool-round commentary.
                            output.clear();
                            iterations += 1;
                            if iterations > max_iterations {
                                turn.cancel.cancel();
                                return Err("workflow agent 超过最大迭代次数".to_string());
                            }
                        }
                        PromptEvent::Terminated(PromptTermination::Completed(_)) => {
                            let answer =
                                H::visible_answer(
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
    }).await.map_err(|e| format!("workflow agent 执行失败: {e}"))?
}
