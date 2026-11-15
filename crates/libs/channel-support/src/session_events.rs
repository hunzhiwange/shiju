//! 通道响应事件收集；仅消费已启动处理器的强类型事件及工作线程句柄。
use session_port::{PromptEvent, PromptTermination, ToolEventKind};
use session_port::ui_types::ChatSessionStep;
use anyhow::{Context, Result};
#[derive(Default)]
pub struct ChannelProcessorResponse {
    pub text: String,
    pub completed: bool,
    pub user_id: Option<String>,
    pub assistant_id: Option<String>,
    pub steps: Vec<session_port::ui_types::ChatSessionStep>,
    pub raw_artifacts: Vec<session_port::llm_raw::LlmRawArtifact>,
}

impl From<&str> for ChannelProcessorResponse {
    fn from(text: &str) -> Self {
        Self { text: text.into(), completed: true, ..Self::default() }
    }
}

pub async fn collect_channel_events(
    mut event_rx: tokio::sync::mpsc::UnboundedReceiver<PromptEvent>,
    mut response: ChannelProcessorResponse,
    delta_tx: Option<tokio::sync::mpsc::Sender<String>>,
    processor_task: tokio::task::JoinHandle<()>,
    successful_finish_reason: fn(Option<&str>) -> bool,
) -> Result<ChannelProcessorResponse> {
    // 处理流式事件
    let mut open_tools = std::collections::HashSet::new();
    let mut step_started_ms = None;
    let mut final_step_has_text = false;
    let terminal = loop {
        let Some(ev) = event_rx.recv().await else {
            break Err(anyhow::anyhow!("session processor exited without terminal event"));
        };
        match ev {
            PromptEvent::LlmRawArtifact(artifact) => response.raw_artifacts.push(artifact),
            PromptEvent::Text(delta) => {
                final_step_has_text |= !delta.trim().is_empty();
                response.text.push_str(&delta);
                if let Some(tx) = delta_tx.as_ref() {
                    let _ = tx.send(delta).await;
                }
            }
            PromptEvent::Tool(event) => {
                if event.kind == ToolEventKind::Complete {
                    open_tools.remove(&event.call_id);
                } else {
                    open_tools.insert(event.call_id);
                }
                let delta = format!("tool {}\n{}\n", event.name, event.payload);
                response.text.push_str(&delta);
                if let Some(tx) = delta_tx.as_ref() {
                    let _ = tx.send(delta).await;
                }
            }
            PromptEvent::Terminated(PromptTermination::Completed(_)) => {
                let finish_reason =
                    response.steps.last().and_then(|step| step.finish_reason.as_deref());
                response.completed = open_tools.is_empty()
                    && final_step_has_text
                    && successful_finish_reason(finish_reason);
                break Ok(response);
            }
            PromptEvent::Terminated(PromptTermination::Failed(err)) => {
                break Err(anyhow::anyhow!(err));
            }
            PromptEvent::Terminated(PromptTermination::Cancelled) => {
                break Err(anyhow::anyhow!("session processor cancelled"));
            }
            PromptEvent::StepStart { created_ms, .. } => {
                final_step_has_text = false;
                step_started_ms = Some(created_ms);
            }
            PromptEvent::StepFinish { step_index, finished_ms, usage, finish_reason, model } => {
                response.steps.push(ChatSessionStep {
                    index: step_index,
                    turn_id: response.assistant_id.clone(),
                    started_ms: step_started_ms.take().unwrap_or(finished_ms),
                    finished_ms: Some(finished_ms),
                    start_snapshot_path: None,
                    finish_snapshot_path: None,
                    usage,
                    cost_usd: None,
                    finish_reason,
                    model,
                    tool_timings: Vec::new(),
                });
            }
            PromptEvent::PostToolRound { .. }
            | PromptEvent::ModelRecovery(_)
            | PromptEvent::TodoContinuation
            | PromptEvent::Compaction { .. }
            | PromptEvent::ContextLoaded { .. } => {}
        }
    };
    // 终态事件可能早于最终工件持久化，必须等待阻塞处理器真正返回。
    processor_task.await.context("channel session processor crashed")?;
    terminal
}
pub fn to_session_history(
    turns: &[model_types::ChatMessage],
) -> Vec<session_port::ui_types::ChatMessage> {
    turns
        .iter()
        .filter_map(|turn| {
            let role = match turn.role.as_str() {
                "user" => session_port::ui_types::ChatRole::User,
                "assistant" => session_port::ui_types::ChatRole::Assistant,
                "system" => session_port::ui_types::ChatRole::System,
                "developer" => session_port::ui_types::ChatRole::Developer,
                "tool" => session_port::ui_types::ChatRole::Tool,
                _ => return None,
            };
            Some(session_port::ui_types::ChatMessage {
                role,
                content: turn.content.clone(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            })
        })
        .collect()
}
