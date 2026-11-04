//! Executable, project-scoped slash command bindings for native sessions.

use super::{WorkflowRunEvent, WorkflowRuntime, run_workflow_with_events};
use crate::session::processor::{Request, StreamEvent, ToolCallStatus, ToolEvent, ToolEventKind};
use crate::session::ui_types::TokenUsage;
use crate::tools::ToolRuntimeContext;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use vw_api_types::workflow::{WorkflowRunRequest, WorkflowRunStatus};

#[path = "native_agent.rs"]
mod agent;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CommandBindings {
    platform_id: String,
    company_id: String,
    commands: Vec<CommandBinding>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CommandBinding {
    command: String,
    workflow_file: PathBuf,
    #[serde(default)]
    inputs: BTreeMap<String, Value>,
}

fn command_matches(query: &str, command: &str) -> bool {
    query
        .strip_prefix(command)
        .is_some_and(|tail| tail.is_empty() || tail.chars().next().is_some_and(char::is_whitespace))
}

fn command_context(query: &str) -> (&str, Vec<&str>) {
    let query = query.trim();
    let Some((text, context)) = query.split_once("\n\n<session_control_selection>\n") else {
        return (query, vec![]);
    };
    let Some((context, _)) = context.split_once("</session_control_selection>") else {
        return (query, vec![]);
    };
    let selected = context
        .lines()
        .filter_map(|line| line.strip_prefix("技能："))
        .flat_map(|line| line.split(','))
        .map(str::trim)
        .collect();
    (text.trim(), selected)
}

fn bound_request(request: &Request) -> Result<Option<WorkflowRunRequest>, String> {
    let (query, selected) = command_context(&request.query);
    if !query.starts_with('/') && selected.is_empty() {
        return Ok(None);
    }
    let Some(root) = request.root.as_deref() else { return Ok(None) };
    let root = Path::new(root).canonicalize().map_err(|e| format!("工作流项目目录不可用: {e}"))?;
    let path = root.join(".shiju/workflow_commands.json");
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("读取工作流命令绑定失败: {error}")),
    };
    if bytes.len() > 64 * 1024 {
        return Err("工作流命令绑定超过 64 KiB".into());
    }
    let bindings: CommandBindings =
        serde_json::from_slice(&bytes).map_err(|e| format!("工作流命令绑定无效: {e}"))?;
    let mut matches = Vec::new();
    for binding in bindings.commands {
        if !binding.command.starts_with('/')
            || binding.command.trim() != binding.command
            || binding.command.len() < 2
        {
            return Err("工作流命令必须是非空且不含首尾空白的 slash 命令".into());
        }
        let candidate = if query.starts_with('/') {
            query.to_string()
        } else {
            let name = binding.command.split_whitespace().next().unwrap().trim_start_matches('/');
            if !selected.contains(&name) {
                continue;
            }
            format!("/{name} {query}")
        };
        if command_matches(&candidate, &binding.command) {
            matches.push((binding, candidate));
        }
    }
    matches.sort_by_key(|(b, _)| std::cmp::Reverse(b.command.len()));
    let Some((mut binding, query)) = matches.into_iter().next() else { return Ok(None) };
    let tenant = crate::session::ui_store::current_tenant_scope();
    if bindings.platform_id != tenant.platform_id().as_ref()
        || bindings.company_id != tenant.company_id().as_ref()
    {
        return Err("工作流命令绑定不属于当前租户".into());
    }
    if request.options.get("acp_test").and_then(Value::as_bool) == Some(true) {
        return Err("工作流命令仅支持 ShiJu 原生会话".into());
    }
    let source = root
        .join(&binding.workflow_file)
        .canonicalize()
        .map_err(|e| format!("工作流文件不可用: {e}"))?;
    if !source.starts_with(&root) {
        return Err("工作流文件必须位于当前项目目录内".into());
    }
    let yaml = std::fs::read_to_string(source).map_err(|e| format!("读取工作流失败: {e}"))?;
    if yaml.len() > 1024 * 1024 {
        return Err("工作流文件超过 1 MiB".into());
    }
    super::model::parse_workflow_yaml(&yaml)?;
    if binding.inputs.keys().any(|key| key.starts_with("__vw_")) {
        return Err("工作流绑定不能设置运行时权限输入".into());
    }
    binding.inputs.insert("project_directory".into(), json!(root));
    Ok(Some(WorkflowRunRequest {
        workflow_yaml: Some(yaml),
        query: Some(query.to_string()),
        inputs: binding.inputs,
        max_steps: 200,
        ..Default::default()
    }))
}

fn node_event(run_id: &str, event: WorkflowRunEvent) -> Option<StreamEvent> {
    if let WorkflowRunEvent::Child { context, event } = event {
        let mut result = node_event(run_id, *event)?;
        if let StreamEvent::Tool(ref mut tool) = result {
            tool.call_id = context.call_id;
            tool.payload["parent_node_id"] = json!(context.parent_node_id);
            tool.payload["chunk_index"] = json!(context.chunk_index);
            tool.payload["chunk_id"] = json!(context.chunk_id);
            tool.payload["node_id"] = json!(context.node_id);
        }
        return Some(result);
    }
    let (node_id, title, kind, status, payload) = match event {
        WorkflowRunEvent::NodeStarted(event) => (
            event.node_id,
            event.title,
            ToolEventKind::Start,
            ToolCallStatus::Running,
            json!({"node_type":event.node_type}),
        ),
        WorkflowRunEvent::NodeFinished(event) => {
            let status =
                if event.node.status == vw_api_types::workflow::WorkflowNodeRunStatus::Failed {
                    ToolCallStatus::Failed
                } else {
                    ToolCallStatus::Completed
                };
            (
                event.node.node_id,
                event.node.title,
                ToolEventKind::Complete,
                status,
                json!({"elapsed_ms":event.node.elapsed_ms}),
            )
        }
        _ => return None,
    };
    Some(StreamEvent::Tool(ToolEvent {
        call_id: format!("workflow:{run_id}:{node_id}"),
        name: "workflow".into(),
        kind,
        status,
        occurred_ms: crate::session::session::now_ms(),
        payload: json!({"workflow_run_id":run_id,"node_id":node_id,"title":title,"details":payload}),
    }))
}

pub(crate) fn run_bound_command(
    request: &Request,
    context: &ToolRuntimeContext,
    emit: &mut (impl FnMut(StreamEvent) -> bool + Send + 'static),
) -> bool {
    let workflow_request = match bound_request(request) {
        Ok(Some(request)) => request,
        Ok(None) => return false,
        Err(error) => {
            emit(StreamEvent::Error(error));
            return true;
        }
    };
    let cancellation = request.abort_token.clone().unwrap_or_default();
    if cancellation.is_cancelled() {
        emit(StreamEvent::Error("cancelled".into()));
        return true;
    }
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let tools = Arc::new(agent::NativeTools {
        context: context.clone(),
        allowed_tools: request.options.get("allowed_tools").cloned(),
    });
    let runtime = WorkflowRuntime {
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
    };
    if !emit(StreamEvent::StepStart {
        step_index: 0,
        created_ms: crate::session::session::now_ms(),
        model: None,
    }) {
        return true;
    }
    let result = crate::session::prompt::block_on(async {
        let events = tx.clone();
        let mut run_id = String::new();
        let run = run_workflow_with_events(runtime, workflow_request, move |event| {
            if let WorkflowRunEvent::WorkflowStarted { run_id: id } = event {
                run_id = id;
            } else if let Some(event) = node_event(&run_id, event) {
                let _ = events.send(event);
            }
        });
        tokio::pin!(run);
        loop {
            tokio::select! {
                result = &mut run => {
                    while let Ok(event) = rx.try_recv() {
                        if !emit(event) { cancellation.cancel(); return Err("cancelled".to_string()) }
                    }
                    return result;
                }
                Some(event) = rx.recv() => {
                    if !emit(event) { cancellation.cancel(); return Err("cancelled".to_string()) }
                }
                _ = cancellation.cancelled() => return Err("cancelled".to_string()),
            }
        }
    });
    match result {
        Ok(response) if response.status == WorkflowRunStatus::Succeeded => {
            emit(StreamEvent::StepFinish {
                step_index: 0,
                finished_ms: crate::session::session::now_ms(),
                usage: TokenUsage::default(),
                finish_reason: Some("workflow".into()),
                model: None,
            });
            emit(StreamEvent::Delta(response.answer.unwrap_or_default()));
            emit(StreamEvent::Done(TokenUsage::default()));
        }
        Ok(response) => {
            emit(StreamEvent::Error(response.error.unwrap_or_else(|| "工作流未完成".into())));
        }
        Err(error) => {
            emit(StreamEvent::Error(error));
        }
    }
    true
}

#[cfg(test)]
#[path = "native_tests.rs"]
mod tests;
