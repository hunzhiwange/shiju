//! 统一的 agent 调用工具。
//!
//! `AgentTool` 为每次 launch 创建并等待持久化子会话，让上层只暴露一个稳定工具入口。

use super::traits::{Tool, ToolCallResult, ToolRenderHint, ToolResult, ToolSpec};
use crate::app::agent::config::DelegateAgentConfig;
use crate::session::child_agent::coordinator::{
    ChildAgentCoordinator, WaitCondition, global_child_agent_coordinator,
};
use crate::session::child_agent::{self, ChildAgentRunInput, ChildAgentTerminalState};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

const DEFAULT_MAX_PARALLEL_SUBAGENTS: usize = 4;
const MAX_PARALLEL_SUBAGENTS: usize = 32;

#[cfg(test)]
type ChildAgentStreamExecutor = Arc<
    dyn Fn(
            vw_api_types::chat::GatewayChatStreamRequest,
        ) -> futures_util::future::BoxFuture<
            'static,
            std::result::Result<
                crate::gateway::api::handlers::session::stream::GatewayChatStreamCollected,
                crate::gateway::ApiError,
            >,
        > + Send
        + Sync,
>;

#[cfg(test)]
async fn run_created_with_optional_executor(
    created: child_agent::CreatedChildAgentRun,
    execute_stream: Option<ChildAgentStreamExecutor>,
) -> anyhow::Result<child_agent::ChildAgentRunResult> {
    if let Some(execute_stream) = execute_stream {
        return child_agent::run_created_with_stream_executor(created, move |request| {
            execute_stream(request)
        })
        .await;
    }
    child_agent::run_created(created).await
}

/// AgentTool 创建持久化子会话后发布的定位信息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentToolChildSessionCreated {
    pub parent_session_id: String,
    pub child_session_id: String,
    pub agent: String,
    pub task: String,
}

pub(crate) type AgentToolChildSessionCreatedCallback =
    Arc<dyn Fn(AgentToolChildSessionCreated) + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AgentToolAction {
    Launch,
    Run,
    Status,
    Wait,
    Stop,
}

struct PreparedChildRun {
    parent_session_id: String,
    agent: String,
    task: String,
    created: child_agent::CreatedChildAgentRun,
}

/// 统一 agent 工具的运行时状态。
pub struct AgentTool {
    agents: Arc<HashMap<String, DelegateAgentConfig>>,
    parent_session_id: Option<String>,
    workspace_dir: std::path::PathBuf,
    parent_model: Option<String>,
    parallelism: Arc<Semaphore>,
    coordinator: ChildAgentCoordinator,
    #[cfg(test)]
    child_agent_stream_executor: Option<ChildAgentStreamExecutor>,
    child_session_created: Option<AgentToolChildSessionCreatedCallback>,
}

impl AgentTool {
    /// 创建新的 `AgentTool`。
    ///
    /// # 参数
    ///
    /// - `agents`: 可调用 agent 配置表。
    /// - `parent_session_id`: 当前父会话 ID，launch 时必须存在。
    /// - `workspace_dir`: 子会话运行工作区。
    pub fn new(
        agents: HashMap<String, DelegateAgentConfig>,
        parent_session_id: Option<String>,
        workspace_dir: std::path::PathBuf,
    ) -> Self {
        Self {
            agents: Arc::new(agents),
            parent_session_id,
            workspace_dir,
            parent_model: None,
            parallelism: Arc::new(Semaphore::new(DEFAULT_MAX_PARALLEL_SUBAGENTS)),
            coordinator: global_child_agent_coordinator(),
            #[cfg(test)]
            child_agent_stream_executor: None,
            child_session_created: None,
        }
    }

    /// 设置同一 AgentTool 实例可同时运行的子智能体数量。
    pub(crate) fn with_max_parallel_subagents(mut self, max_parallel: usize) -> Self {
        self.parallelism = Arc::new(Semaphore::new(max_parallel.clamp(1, MAX_PARALLEL_SUBAGENTS)));
        self
    }

    pub(crate) fn with_parent_model(mut self, model: Option<String>) -> Self {
        self.parent_model = model
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        self
    }

    #[cfg(test)]
    fn with_child_agent_stream_executor(mut self, executor: ChildAgentStreamExecutor) -> Self {
        self.child_agent_stream_executor = Some(executor);
        self
    }

    #[cfg(test)]
    fn with_coordinator(mut self, coordinator: ChildAgentCoordinator) -> Self {
        self.coordinator = coordinator;
        self
    }

    pub(crate) fn with_child_session_created_callback(
        mut self,
        callback: AgentToolChildSessionCreatedCallback,
    ) -> Self {
        self.child_session_created = Some(callback);
        self
    }

    async fn run_child_agent(
        &self,
        created: child_agent::CreatedChildAgentRun,
    ) -> anyhow::Result<child_agent::ChildAgentRunResult> {
        #[cfg(test)]
        if let Some(execute_stream) = &self.child_agent_stream_executor {
            let execute_stream = Arc::clone(execute_stream);
            return child_agent::run_created_with_stream_executor(created, move |request| {
                execute_stream(request)
            })
            .await;
        }
        child_agent::run_created(created).await
    }

    fn spawn_background_child(
        &self,
        prepared: PreparedChildRun,
        cancellation: tokio_util::sync::CancellationToken,
    ) -> anyhow::Result<()> {
        let PreparedChildRun { parent_session_id, agent: _, task: _, created } = prepared;
        let child_session_id = created.child_session_id.clone();
        let coordinator = self.coordinator.clone();
        #[cfg(test)]
        let execute_stream = self.child_agent_stream_executor.clone();
        let thread_parent_session_id = parent_session_id.clone();
        let thread_child_session_id = child_session_id.clone();

        let spawned = std::thread::Builder::new()
            .name("vw-child-agent".to_string())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("failed to build child agent runtime");
                runtime.block_on(async move {
                    let _ =
                        coordinator.mark_running(&thread_parent_session_id, &thread_child_session_id);
                    let run_created = created.clone();
                    #[cfg(test)]
                    let execution = tokio::select! {
                        _ = cancellation.cancelled() => {
                            child_agent::cancel_created(created, "cancelled by parent agent").await
                        }
                        result = run_created_with_optional_executor(run_created, execute_stream) => {
                            result
                        }
                    };
                    #[cfg(not(test))]
                    let execution = tokio::select! {
                        _ = cancellation.cancelled() => {
                            child_agent::cancel_created(created, "cancelled by parent agent").await
                        }
                        result = child_agent::run_created(run_created) => {
                            result
                        }
                    };

                    match execution {
                        Ok(result) => match result.terminal_state {
                            ChildAgentTerminalState::Succeeded => {
                                let _ = coordinator.mark_succeeded(
                                    &thread_parent_session_id,
                                    &thread_child_session_id,
                                    result.output.unwrap_or_default(),
                                );
                            }
                            ChildAgentTerminalState::Failed => {
                                let _ = coordinator.mark_failed(
                                    &thread_parent_session_id,
                                    &thread_child_session_id,
                                    result.error.unwrap_or_else(|| {
                                        "child agent execution failed".to_string()
                                    }),
                                );
                            }
                            ChildAgentTerminalState::Cancelled => {}
                        },
                        Err(error) => {
                            let _ = coordinator.mark_failed(
                                &thread_parent_session_id,
                                &thread_child_session_id,
                                error.to_string(),
                            );
                        }
                    }
                });
            });

        if let Err(error) = spawned {
            let message = format!("spawn child agent runtime: {error}");
            let _ =
                self.coordinator.mark_failed(&parent_session_id, &child_session_id, &message);
            anyhow::bail!(message);
        }
        Ok(())
    }

    fn available_agents_description(&self) -> String {
        let mut agents = self.agents.iter().collect::<Vec<_>>();
        agents.sort_by_key(|(name, _)| *name);
        if agents.is_empty() {
            return "（未配置）".to_string();
        }

        agents
            .into_iter()
            .map(|(name, config)| {
                let label = config.label.as_deref().map(str::trim).filter(|value| !value.is_empty());
                let description =
                    config.description.as_deref().map(str::trim).filter(|value| !value.is_empty());
                match (label, description) {
                    (Some(label), Some(description)) => {
                        format!("{name} [{label}]: {description}")
                    }
                    (Some(label), None) => format!("{name} [{label}]"),
                    (None, Some(description)) => format!("{name}: {description}"),
                    (None, None) => name.clone(),
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn available_agent_names(&self) -> Vec<String> {
        let mut names = self.agents.keys().cloned().collect::<Vec<_>>();
        names.sort();
        names
    }

    fn action_from_args(&self, args: &Value) -> anyhow::Result<AgentToolAction> {
        let explicit = args
            .get("action")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_ascii_lowercase());

        match explicit.as_deref() {
            Some("launch" | "spawn") | None => Ok(AgentToolAction::Launch),
            Some("run") => Ok(AgentToolAction::Run),
            Some("status") => Ok(AgentToolAction::Status),
            Some("wait") => Ok(AgentToolAction::Wait),
            Some("stop") => Ok(AgentToolAction::Stop),
            Some(other) => anyhow::bail!(
                "Unknown AgentTool action '{other}'. Supported actions: launch, run, spawn, status, wait, stop"
            ),
        }
    }

    fn parent_session_id(&self) -> anyhow::Result<&str> {
        self.parent_session_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow::anyhow!("AgentTool requires a current parent session ID"))
    }

    fn child_session_ids_from_args(&self, args: &Value) -> anyhow::Result<Vec<String>> {
        let mut ids = Vec::new();
        if let Some(value) = args.get("child_session_id") {
            let Some(value) = value.as_str().map(str::trim).filter(|value| !value.is_empty()) else {
                anyhow::bail!("'child_session_id' must be a non-empty string");
            };
            ids.push(value.to_string());
        }
        if let Some(values) = args.get("child_session_ids") {
            let Some(values) = values.as_array() else {
                anyhow::bail!("'child_session_ids' must be an array of non-empty strings");
            };
            for value in values {
                let Some(value) =
                    value.as_str().map(str::trim).filter(|value| !value.is_empty())
                else {
                    anyhow::bail!("'child_session_ids' must contain only non-empty strings");
                };
                if !ids.iter().any(|existing| existing == value) {
                    ids.push(value.to_string());
                }
            }
        }
        Ok(ids)
    }

    fn wait_condition_from_args(&self, args: &Value) -> anyhow::Result<WaitCondition> {
        match args
            .get("condition")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("any") => Ok(WaitCondition::Any),
            Some("all") | None => Ok(WaitCondition::All),
            Some(other) => anyhow::bail!("Unknown wait condition '{other}'. Supported: any, all"),
        }
    }

    fn wait_timeout_from_args(&self, args: &Value) -> anyhow::Result<Duration> {
        const DEFAULT_TIMEOUT_MS: u64 = 30_000;
        const MAX_TIMEOUT_MS: u64 = 60_000;
        let timeout_ms = match args.get("timeout_ms") {
            Some(value) => value
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("'timeout_ms' must be a non-negative integer"))?,
            None => DEFAULT_TIMEOUT_MS,
        };
        Ok(Duration::from_millis(timeout_ms.min(MAX_TIMEOUT_MS)))
    }

    fn agent_name_from_args<'a>(&self, args: &'a Value) -> Option<&'a str> {
        args.get("agent")
            .and_then(Value::as_str)
            .or_else(|| args.get("subagent_type").and_then(Value::as_str))
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    fn prompt_from_args<'a>(&self, args: &'a Value) -> Option<&'a str> {
        args.get("prompt").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty())
    }

    fn task_summary(prompt: &str) -> String {
        const MAX_CHARS: usize = 160;
        let mut chars = prompt.chars();
        let summary = chars.by_ref().take(MAX_CHARS).collect::<String>();
        if chars.next().is_some() { format!("{summary}…") } else { summary }
    }

    async fn prepare_child_run(
        &self,
        args: &Value,
    ) -> anyhow::Result<Result<PreparedChildRun, ToolResult>> {
        let agent = self
            .agent_name_from_args(args)
            .ok_or_else(|| anyhow::anyhow!("Missing 'agent' parameter"))?;
        let prompt = self
            .prompt_from_args(args)
            .ok_or_else(|| anyhow::anyhow!("Missing 'prompt' parameter"))?;
        let max_depth = match self.agents.get(agent) {
            Some(config) => usize::try_from(config.max_depth).unwrap_or(usize::MAX),
            None => {
                return Ok(Err(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Unknown agent '{agent}'")),
                }));
            }
        };
        let parent_session_id = self.parent_session_id()?;
        let current_depth = child_agent::session_depth(
            self.workspace_dir.to_string_lossy().to_string(),
            parent_session_id.to_string(),
        )
        .await?;
        if current_depth >= max_depth {
            return Ok(Err(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "Agent '{agent}' delegation depth limit reached: current depth {current_depth}, max {max_depth}"
                )),
            }));
        }
        let context = args
            .get("context")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| vec![json!({"role": "system", "content": value})])
            .unwrap_or_default();
        let task = Self::task_summary(prompt);
        let created = child_agent::create(ChildAgentRunInput {
            parent_session_id: parent_session_id.to_string(),
            directory: self.workspace_dir.to_string_lossy().to_string(),
            agent_key: agent.to_string(),
            model: self.parent_model.clone(),
            prompt: prompt.to_string(),
            context,
        })
        .await?;
        Ok(Ok(PreparedChildRun {
            parent_session_id: parent_session_id.to_string(),
            agent: agent.to_string(),
            task,
            created,
        }))
    }

    fn report_child_created(&self, prepared: &PreparedChildRun) {
        if let Some(callback) = &self.child_session_created {
            callback(AgentToolChildSessionCreated {
                parent_session_id: prepared.parent_session_id.clone(),
                child_session_id: prepared.created.child_session_id.clone(),
                agent: prepared.agent.clone(),
                task: prepared.task.clone(),
            });
        }
    }

    async fn handle_launch(&self, args: &Value) -> anyhow::Result<ToolResult> {
        let prepared = match self.prepare_child_run(args).await? {
            Ok(prepared) => prepared,
            Err(result) => return Ok(result),
        };
        let child_session_id = prepared.created.child_session_id.clone();
        let cancellation = self.coordinator.register(
            &prepared.parent_session_id,
            &child_session_id,
            &prepared.agent,
            &prepared.task,
        )?;
        self.report_child_created(&prepared);
        let output = serde_json::to_string(&json!({
            "action": "launch",
            "state": "running",
            "parent_session_id": prepared.parent_session_id,
            "child_session_id": child_session_id,
            "agent": prepared.agent,
            "task": prepared.task,
        }))?;
        self.spawn_background_child(prepared, cancellation)?;
        Ok(ToolResult { success: true, output, error: None })
    }

    async fn handle_run(&self, args: &Value) -> anyhow::Result<ToolResult> {
        let _parallel_permit = self
            .parallelism
            .acquire()
            .await
            .map_err(|_| anyhow::anyhow!("AgentTool parallel limiter is unavailable"))?;
        let prepared = match self.prepare_child_run(args).await? {
            Ok(prepared) => prepared,
            Err(result) => return Ok(result),
        };
        self.report_child_created(&prepared);
        let PreparedChildRun { parent_session_id, agent, task, created } = prepared;
        let result = self.run_child_agent(created).await?;
        let terminal_state = match result.terminal_state {
            ChildAgentTerminalState::Succeeded => "succeeded",
            ChildAgentTerminalState::Failed => "failed",
            ChildAgentTerminalState::Cancelled => "cancelled",
        };
        let output = serde_json::to_string(&json!({
            "action": "run",
            "state": terminal_state,
            "parent_session_id": parent_session_id,
            "child_session_id": result.child_session_id,
            "agent": agent,
            "task": task,
            "terminal_state": terminal_state,
            "output": result.output,
            "error": result.error,
        }))?;
        Ok(ToolResult {
            success: result.terminal_state == ChildAgentTerminalState::Succeeded,
            output,
            error: result.error,
        })
    }

    fn handle_status(&self, args: &Value) -> anyhow::Result<ToolResult> {
        let parent_session_id = self.parent_session_id()?;
        let child_session_ids = self.child_session_ids_from_args(args)?;
        let tasks = self.coordinator.snapshots(parent_session_id, &child_session_ids)?;
        let output = serde_json::to_string(&json!({
            "action": "status",
            "tasks": tasks,
        }))?;
        Ok(ToolResult { success: true, output, error: None })
    }

    async fn handle_wait(&self, args: &Value) -> anyhow::Result<ToolResult> {
        let parent_session_id = self.parent_session_id()?;
        let child_session_ids = self.child_session_ids_from_args(args)?;
        let tasks = self.coordinator.snapshots(parent_session_id, &child_session_ids)?;
        if tasks.is_empty() {
            let output = serde_json::to_string(&json!({
                "action": "wait",
                "condition": "all",
                "timed_out": false,
                "tasks": [],
            }))?;
            return Ok(ToolResult { success: true, output, error: None });
        }
        let condition = self.wait_condition_from_args(args)?;
        let outcome = self
            .coordinator
            .wait(
                parent_session_id,
                &child_session_ids,
                condition,
                self.wait_timeout_from_args(args)?,
            )
            .await?;
        let output = serde_json::to_string(&json!({
            "action": "wait",
            "condition": match condition {
                WaitCondition::Any => "any",
                WaitCondition::All => "all",
            },
            "timed_out": outcome.timed_out,
            "tasks": outcome.tasks,
        }))?;
        Ok(ToolResult { success: true, output, error: None })
    }

    fn handle_stop(&self, args: &Value) -> anyhow::Result<ToolResult> {
        let parent_session_id = self.parent_session_id()?;
        let child_session_ids = self.child_session_ids_from_args(args)?;
        if child_session_ids.is_empty() {
            anyhow::bail!("AgentTool stop requires 'child_session_id' or 'child_session_ids'");
        }
        for child_session_id in &child_session_ids {
            self.coordinator.cancel(parent_session_id, child_session_id)?;
        }
        let tasks = self.coordinator.snapshots(parent_session_id, &child_session_ids)?;
        let output = serde_json::to_string(&json!({
            "action": "stop",
            "tasks": tasks,
        }))?;
        Ok(ToolResult { success: true, output, error: None })
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for AgentTool {
    fn name(&self) -> &str {
        "AgentTool"
    }

    fn description(&self) -> &str {
        "Delegate complex work to persistent sub-agents; skip only genuinely simple single-step tasks. Use launch (or spawn) to start independent work in the background and in parallel, run when the result is an immediate dependency, status to inspect tasks, wait for event-driven any/all completion with a bounded timeout, and stop to cancel owned tasks. Completed run/wait outputs contain each child's final handoff report, including evidence for research tasks and validation for execution tasks. Continue useful parent work after launch, then verify and synthesize required results before answering."
    }

    fn parameters_schema(&self) -> Value {
        let agent_names = self.available_agent_names();
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["launch", "run", "spawn", "status", "wait", "stop"],
                    "description": "操作。省略时默认 launch；launch/spawn 后台启动，run 前台等待。"
                },
                "agent": {
                    "type": "string",
                    "enum": agent_names,
                    "description": format!("要调用的 agent 名称。可用候选（名称、标签与用途）：{}", self.available_agents_description())
                },
                "subagent_type": {
                    "type": "string",
                    "enum": self.available_agent_names(),
                    "description": "Claude Code 兼容别名，等价于 agent；必须从 agent 候选中选择。"
                },
                "prompt": {
                    "type": "string",
                    "description": "A bounded, self-contained task with the expected deliverable, relevant constraints, and validation requirements."
                },
                "context": {
                    "type": "string",
                    "description": "Optional task-specific context not already included in prompt."
                },
                "child_session_id": {
                    "type": "string",
                    "description": "status、wait 或 stop 的单个子会话 ID。"
                },
                "child_session_ids": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "status、wait 或 stop 的子会话 ID 列表；status/wait 省略时使用当前父会话全部任务。"
                },
                "condition": {
                    "type": "string",
                    "enum": ["any", "all"],
                    "description": "wait 返回条件，默认 all。"
                },
                "timeout_ms": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 60000,
                    "description": "wait 的有界超时毫秒数，默认 30000，最大 60000。"
                }
            }
        })
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(self.name(), self.description(), self.parameters_schema())
            .with_display_name("AgentTool")
            .with_aliases(vec!["Agent".to_string(), "agent_tool".to_string(), "agent".to_string()])
            .with_read_only(false)
            .with_destructive(false)
            .with_concurrency_safe(true)
            .with_requires_user_interaction(false)
            .with_strict(true)
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        match self.action_from_args(&args)? {
            AgentToolAction::Launch => self.handle_launch(&args).await,
            AgentToolAction::Run => self.handle_run(&args).await,
            AgentToolAction::Status => self.handle_status(&args),
            AgentToolAction::Wait => self.handle_wait(&args).await,
            AgentToolAction::Stop => self.handle_stop(&args),
        }
    }

    fn render_hint(&self, result: &ToolCallResult) -> Option<ToolRenderHint> {
        let output = result.data.get("output").and_then(Value::as_str)?;
        let body: Value = serde_json::from_str(output).ok()?;
        let details = if body.get("child_session_id").is_some() {
            &body
        } else {
            let tasks = body.get("tasks")?.as_array()?;
            if tasks.len() != 1 {
                return None;
            }
            &tasks[0]
        };
        let metadata = json!({
            "parent_session_id": details.get("parent_session_id")?,
            "child_session_id": details.get("child_session_id")?,
            "agent": details.get("agent")?,
            "task": details.get("task")?,
        });
        Some(ToolRenderHint {
            title: Some("AgentTool".to_string()),
            kind: Some("agent_tool".to_string()),
            summary: details.get("task").and_then(Value::as_str).map(str::to_owned),
            metadata,
        })
    }
}
#[cfg(test)]
mod tests;
