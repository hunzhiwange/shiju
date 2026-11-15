//! 原生工作流命令绑定、执行与类型化生命周期。
use crate::{WorkflowRunEvent, WorkflowRuntime, run_workflow_with_events};
use crate::native_video_progress as video_progress;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::{Path, PathBuf}, sync::Arc};
use session_port::{ToolEvent, ToolEventKind, ToolCallStatus};
use tokio_util::sync::CancellationToken;
use vw_api_workflow::{WorkflowRunRequest, WorkflowRunStatus};
use vw_api_transport::TenantScope;

pub struct BoundCommandRequest<'a> {
    pub query: &'a str,
    pub root: Option<&'a str>,
    pub acp_test: bool,
    pub current_tenant: fn() -> TenantScope,
}

pub enum CommandEvent {
    Tool(ToolEvent),
    StepStart(u64),
    StepFinish(u64),
    Delta(String),
    Done,
    Error(String),
}

pub trait NativeCommandHost {
    type Event: Send;
    fn event(event: CommandEvent) -> Self::Event;
    fn now_ms() -> u64;
    fn default_runtime(&self, events: tokio::sync::mpsc::UnboundedSender<Self::Event>, cancellation: CancellationToken) -> WorkflowRuntime;
}
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

pub fn bound_request(request: BoundCommandRequest<'_>) -> Result<Option<WorkflowRunRequest>, String> {
    let (query, selected) = command_context(request.query);
    if !query.starts_with('/') && selected.is_empty() {
        return Ok(None);
    }
    let Some(root) = request.root else { return Ok(None) };
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
    let tenant = (request.current_tenant)();
    if bindings.platform_id != tenant.platform_id().as_ref()
        || bindings.company_id != tenant.company_id().as_ref()
    {
        return Err("工作流命令绑定不属于当前租户".into());
    }
    if request.acp_test {
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
    workflow_core::model::parse_workflow_yaml(&yaml)?;
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

fn node_event(run_id: &str, event: WorkflowRunEvent, now_ms: fn() -> u64) -> Option<ToolEvent> {
    if let WorkflowRunEvent::Child { context, event } = event {
        let mut result = node_event(run_id, *event, now_ms)?;
        {
            let tool = &mut result;
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
                if event.node.status == vw_api_workflow::WorkflowNodeRunStatus::Failed {
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
    Some(ToolEvent {
        call_id: format!("workflow:{run_id}:{node_id}"),
        name: "workflow".into(),
        kind,
        status,
        occurred_ms: now_ms(),
        payload: json!({"workflow_run_id":run_id,"node_id":node_id,"title":title,"details":payload}),
    })
}

pub async fn run_command<H: NativeCommandHost>(
    host: &H,
    request: BoundCommandRequest<'_>,
    cancellation: CancellationToken,
    emit: &mut (impl FnMut(H::Event) -> bool + Send),
    prepared: Option<(WorkflowRuntime, WorkflowRunRequest)>,
) -> bool {
    let video_run = prepared.is_some();
    let (prepared_runtime, workflow_request) = match prepared {
        Some((runtime, request)) => (Some(runtime), request),
        None => (
            None,
            match bound_request(request) {
                Ok(Some(request)) => request,
                Ok(None) => return false,
                Err(error) => {
                    emit(H::event(CommandEvent::Error(error)));
                    return true;
                }
            },
        ),
    };
    if cancellation.is_cancelled() {
        emit(H::event(CommandEvent::Error("cancelled".into())));
        return true;
    }
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let runtime = prepared_runtime.unwrap_or_else(|| host.default_runtime(tx.clone(), cancellation.clone()));
    if !emit(H::event(CommandEvent::StepStart(H::now_ms()))) { return true; }
    let video_budget = runtime.video_edit_budget.clone();
    let progress = Arc::new(std::sync::Mutex::new(video_progress::Progress::new(
        workflow_request.workflow_yaml.as_deref().unwrap_or_default(),
    )));
    let result = async {
        let events = tx.clone();
        let mut run_id = String::new();
        let updates = progress.clone();
        let budget = video_budget.clone();
        let run = run_workflow_with_events(runtime, workflow_request, move |event| {
            if video_run && let Ok(mut progress) = updates.lock() {
                progress.observe(&event);
                if let Some(event) = progress.event_changed(&budget, H::now_ms) {
                    let _ = events.send(H::event(CommandEvent::Tool(event)));
                }
            }
            if let WorkflowRunEvent::WorkflowStarted { run_id: id } = event {
                run_id = id;
            } else if let Some(event) = node_event(&run_id, event, H::now_ms) {
                let _ = events.send(H::event(CommandEvent::Tool(event)));
            }
        });
        tokio::pin!(run);
        let mut tick = tokio::time::interval(std::time::Duration::from_millis(250));
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
                _ = tick.tick(), if video_run => {
                    if let Ok(mut progress) = progress.lock() && progress.ready() {
                        if let Some(event) = progress.event_changed(&video_budget, H::now_ms) && !emit(H::event(CommandEvent::Tool(event))) { cancellation.cancel(); return Err("cancelled".into()); }
                    }
                }
                _ = cancellation.cancelled() => return Err("cancelled".to_string()),
            }
        }
    }.await;
    if video_run
        && let Ok(mut progress) = progress.lock()
        && progress.ready()
    {
        progress.finish(
            result.as_ref().is_ok_and(|r| r.status == WorkflowRunStatus::Succeeded),
            cancellation.is_cancelled(),
        );
        if let Some(event) = progress.event_changed(&video_budget, H::now_ms) {
            emit(H::event(CommandEvent::Tool(event)));
        }
    }
    match result {
        Ok(response) if response.status == WorkflowRunStatus::Succeeded => {
            emit(H::event(CommandEvent::StepFinish(H::now_ms())));
            emit(H::event(CommandEvent::Delta(response.answer.unwrap_or_default())));
            emit(H::event(CommandEvent::Done));
        }
        Ok(response) => {
            emit(H::event(CommandEvent::Error(response.error.unwrap_or_else(|| "工作流未完成".into()))));
        }
        Err(error) => {
            emit(H::event(CommandEvent::Error(error)));
        }
    }
    true
}
