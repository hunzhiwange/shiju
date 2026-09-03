//! 用户可见任务会话的创建工具。

use super::context::current_tool_use_context;
use super::traits::{Tool, ToolResult};
use crate::gateway::api::handlers::session::stream::{
    GatewayChatStreamCollected, collect_gateway_chat_stream,
};
use crate::session::runtime::{
    CreateSessionInput, SessionScope, SessionSnapshot, create_visible_task_session, load_session,
};
use crate::session::task_session::{
    MAX_TASK_SESSION_WAIT, TaskSessionCoordinator, TaskSessionWaitCondition,
    global_task_session_coordinator,
};
use async_trait::async_trait;
use futures_util::future::BoxFuture;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use vw_api_types::chat::GatewayChatStreamRequest;
use vw_api_types::id::{SessionId, TenantScope};
use vw_shared::session::session_utils::encode_task_session_source_marker;

pub(crate) const MAX_SESSION_READ_CHARS: usize = 64 * 1024;

type SessionStreamExecutor = Arc<
    dyn Fn(
            String,
            GatewayChatStreamRequest,
        )
            -> BoxFuture<'static, Result<GatewayChatStreamCollected, crate::gateway::ApiError>>
        + Send
        + Sync,
>;

/// 创建并异步启动同一工作区内的用户可见任务会话。
pub struct SessionTool {
    parent_session_id: Option<String>,
    workspace_dir: PathBuf,
    coordinator: TaskSessionCoordinator,
    stream_executor: SessionStreamExecutor,
}

impl SessionTool {
    /// 绑定当前会话和工作目录。
    pub fn new(parent_session_id: Option<String>, workspace_dir: PathBuf) -> Self {
        Self {
            parent_session_id,
            workspace_dir,
            coordinator: global_task_session_coordinator(),
            stream_executor: Arc::new(|directory, request| {
                Box::pin(collect_gateway_chat_stream(directory, request))
            }),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_coordinator(mut self, coordinator: TaskSessionCoordinator) -> Self {
        self.coordinator = coordinator;
        self
    }

    #[cfg(test)]
    pub(crate) fn with_stream_executor(mut self, executor: SessionStreamExecutor) -> Self {
        self.stream_executor = executor;
        self
    }

    fn parent_session_id(&self) -> anyhow::Result<&str> {
        self.parent_session_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow::anyhow!("SessionTool requires a current session ID"))
    }

    fn workspace_dir(&self) -> anyhow::Result<PathBuf> {
        if self.workspace_dir.as_os_str().is_empty() {
            anyhow::bail!("SessionTool requires a current session directory");
        }
        Ok(self.workspace_dir.clone())
    }

    fn required_text<'a>(args: &'a Value, key: &str) -> anyhow::Result<&'a str> {
        args.get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow::anyhow!("'{key}' must be a non-empty string"))
    }

    fn validate_fields(args: &Value, allowed: &[&str]) -> anyhow::Result<()> {
        let object = args
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("SessionTool arguments must be an object"))?;
        if let Some(field) = object.keys().find(|field| !allowed.contains(&field.as_str())) {
            anyhow::bail!("Unknown field '{field}' for SessionTool action");
        }
        Ok(())
    }

    fn required_ids(args: &Value) -> anyhow::Result<Vec<String>> {
        let values = args
            .get("session_ids")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("'session_ids' must be a non-empty array"))?;
        let mut ids = Vec::new();
        for value in values {
            let id =
                value.as_str().map(str::trim).filter(|id| !id.is_empty()).ok_or_else(|| {
                    anyhow::anyhow!("'session_ids' entries must be non-empty strings")
                })?;
            if !ids.iter().any(|existing| existing == id) {
                ids.push(id.to_string());
            }
        }
        if ids.is_empty() {
            anyhow::bail!("'session_ids' must be a non-empty array");
        }
        Ok(ids)
    }

    async fn validated_target(&self, session_id: &str) -> anyhow::Result<SessionSnapshot> {
        let current = self.parent_session_id()?;
        if session_id == current {
            anyhow::bail!("SessionTool cannot target the current session");
        }
        let scope = SessionScope::new(TenantScope::default(), self.workspace_dir()?)?;
        let snapshot = load_session(scope, session_id).await?;
        if !snapshot.info.is_visible_task() {
            anyhow::bail!("session '{session_id}' is not a visible task session");
        }
        Ok(snapshot)
    }

    fn inherited_model_ref() -> Option<String> {
        let context = current_tool_use_context()?;
        if context.model_selected_by_auto() {
            return None;
        }
        let model = context.model()?.trim();
        if model.is_empty() {
            return None;
        }
        Some(match context.provider().map(str::trim).filter(|value| !value.is_empty()) {
            Some(provider) => format!("{provider}/{model}"),
            None => model.to_string(),
        })
    }

    fn spawn_background_turn(
        &self,
        parent_session_id: String,
        task_session_id: String,
        directory: String,
        prompt: String,
    ) -> anyhow::Result<()> {
        let coordinator = self.coordinator.clone();
        let executor = self.stream_executor.clone();
        let model = Self::inherited_model_ref();
        let thread_parent = parent_session_id.clone();
        let thread_task = task_session_id.clone();
        let spawned = std::thread::Builder::new()
            .name("vw-visible-task-session".to_string())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("failed to build visible task session runtime");
                runtime.block_on(async move {
                    if let Err(error) = coordinator.mark_running(&thread_parent, &thread_task) {
                        tracing::error!(target: "vw_agent", error = %error, "failed to start visible task session state");
                        return;
                    }
                    let request = GatewayChatStreamRequest {
                        session_id: Some(SessionId::from(thread_task.as_str())),
                        messages: vec![json!({"role": "user", "content": prompt})],
                        model,
                        ..GatewayChatStreamRequest::default()
                    };
                    match executor(directory, request).await {
                        Ok(_) => {
                            let _ = coordinator.mark_succeeded(&thread_parent, &thread_task);
                        }
                        Err(error) => {
                            let _ = coordinator.mark_failed(
                                &thread_parent,
                                &thread_task,
                                error.to_string(),
                            );
                        }
                    }
                });
            });

        if let Err(error) = spawned {
            let message = format!("spawn visible task session runtime: {error}");
            let _ =
                self.coordinator.mark_failed(&parent_session_id, &task_session_id, message.clone());
            anyhow::bail!(message);
        }
        Ok(())
    }

    async fn create(&self, args: &Value) -> anyhow::Result<ToolResult> {
        Self::validate_fields(args, &["action", "title", "prompt"])?;
        let parent_session_id = self.parent_session_id()?.to_string();
        let directory = self.workspace_dir()?;
        let title = Self::required_text(args, "title")?.to_string();
        let prompt = Self::required_text(args, "prompt")?;
        let prompt = encode_task_session_source_marker(&parent_session_id, prompt)
            .ok_or_else(|| anyhow::anyhow!("current session ID cannot be encoded as a source"))?;
        let scope = SessionScope::new(TenantScope::default(), &directory)?;
        let created = create_visible_task_session(CreateSessionInput {
            scope,
            parent_id: Some(parent_session_id.clone()),
            title: Some(title.clone()),
            permission: None,
        })
        .await?;
        let session_id = created.info.id;
        self.coordinator.register(&parent_session_id, &session_id)?;
        let directory = created.info.directory;
        self.spawn_background_turn(
            parent_session_id.clone(),
            session_id.clone(),
            directory.clone(),
            prompt,
        )?;
        let output = serde_json::to_string(&json!({
            "action": "create",
            "state": "running",
            "session_id": session_id,
            "title": title,
            "directory": directory,
            "parent_session_id": parent_session_id,
        }))?;
        Ok(ToolResult { success: true, output, error: None })
    }

    async fn list(&self, args: &Value) -> anyhow::Result<ToolResult> {
        Self::validate_fields(args, &["action"])?;
        let parent = self.parent_session_id()?;
        let snapshots = self.coordinator.snapshots(parent, &[])?;
        let mut tasks = Vec::new();
        for state in snapshots {
            let target = self.validated_target(&state.task_session_id).await?;
            tasks.push(json!({"session_id": target.info.id, "title": target.info.title, "state": state.state, "updated_ms": state.updated_ms}));
        }
        Self::result(json!({"action": "list", "tasks": tasks}))
    }

    async fn status(&self, args: &Value) -> anyhow::Result<ToolResult> {
        Self::validate_fields(args, &["action", "session_ids"])?;
        let ids = Self::required_ids(args)?;
        for id in &ids {
            self.validated_target(id).await?;
        }
        let tasks = self.coordinator.snapshots(self.parent_session_id()?, &ids)?;
        Self::result(json!({"action": "status", "tasks": tasks}))
    }

    async fn read(&self, args: &Value) -> anyhow::Result<ToolResult> {
        Self::validate_fields(args, &["action", "session_id", "max_chars"])?;
        let id = Self::required_text(args, "session_id")?;
        let snapshot = self.validated_target(id).await?;
        let max_chars = args
            .get("max_chars")
            .map(|value| {
                value
                    .as_u64()
                    .ok_or_else(|| anyhow::anyhow!("'max_chars' must be a positive integer"))
            })
            .transpose()?
            .unwrap_or(MAX_SESSION_READ_CHARS as u64);
        if max_chars == 0 {
            anyhow::bail!("'max_chars' must be a positive integer");
        }
        let messages = snapshot
            .history
            .into_iter()
            .map(|message| serde_json::to_value(message).map_err(anyhow::Error::from))
            .collect::<Result<Vec<_>, _>>()?;
        let output = bounded_read_output(
            id,
            messages,
            usize::try_from(max_chars).unwrap_or(usize::MAX).min(MAX_SESSION_READ_CHARS),
        )?;
        Ok(ToolResult { success: true, output, error: None })
    }

    async fn send(&self, args: &Value) -> anyhow::Result<ToolResult> {
        Self::validate_fields(args, &["action", "session_id", "prompt"])?;
        let parent = self.parent_session_id()?.to_string();
        let id = Self::required_text(args, "session_id")?.to_string();
        let prompt = Self::required_text(args, "prompt")?;
        let target = self.validated_target(&id).await?;
        self.coordinator.begin_turn(&parent, &id)?;
        let prompt = encode_task_session_source_marker(&parent, prompt)
            .ok_or_else(|| anyhow::anyhow!("current session ID cannot be encoded as a source"))?;
        if let Err(error) =
            self.spawn_background_turn(parent, id.clone(), target.info.directory, prompt)
        {
            return Err(error);
        }
        Self::result(json!({"action": "send", "session_id": id, "state": "running"}))
    }

    async fn wait(&self, args: &Value) -> anyhow::Result<ToolResult> {
        Self::validate_fields(args, &["action", "session_ids", "condition", "timeout_ms"])?;
        let ids = Self::required_ids(args)?;
        for id in &ids {
            self.validated_target(id).await?;
        }
        let condition = match args.get("condition").and_then(Value::as_str).unwrap_or("any") {
            "any" => TaskSessionWaitCondition::Any,
            "all" => TaskSessionWaitCondition::All,
            other => anyhow::bail!("Unknown wait condition '{other}'. Supported: any, all"),
        };
        let timeout_ms = args
            .get("timeout_ms")
            .map(|value| {
                value
                    .as_u64()
                    .ok_or_else(|| anyhow::anyhow!("'timeout_ms' must be a non-negative integer"))
            })
            .transpose()?
            .unwrap_or(MAX_TASK_SESSION_WAIT.as_millis() as u64);
        let outcome = self
            .coordinator
            .wait(
                self.parent_session_id()?,
                &ids,
                condition,
                Duration::from_millis(timeout_ms).min(MAX_TASK_SESSION_WAIT),
            )
            .await?;
        Self::result(
            json!({"action": "wait", "tasks": outcome.tasks, "timed_out": outcome.timed_out}),
        )
    }

    fn result(value: Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: serde_json::to_string(&value)?, error: None })
    }
}

fn bounded_read_output(
    session_id: &str,
    mut messages: Vec<Value>,
    max_chars: usize,
) -> anyhow::Result<String> {
    let mut truncated = false;
    loop {
        let output = serde_json::to_string(
            &json!({"action": "read", "session_id": session_id, "messages": messages, "truncated": truncated}),
        )?;
        if output.chars().count() <= max_chars || messages.is_empty() {
            return Ok(output);
        }
        messages.remove(0);
        truncated = true;
    }
}

#[async_trait]
impl Tool for SessionTool {
    fn name(&self) -> &str {
        "SessionTool"
    }

    fn description(&self) -> &str {
        "Create, list, inspect, continue, and wait for user-visible task sessions in this workspace."
    }

    fn parameters_schema(&self) -> Value {
        json!({"oneOf": [
            {"type":"object","properties":{"action":{"const":"create"},"title":{"type":"string","minLength":1},"prompt":{"type":"string","minLength":1}},"required":["action","title","prompt"],"additionalProperties":false},
            {"type":"object","properties":{"action":{"const":"list"}},"required":["action"],"additionalProperties":false},
            {"type":"object","properties":{"action":{"const":"status"},"session_ids":{"type":"array","minItems":1,"items":{"type":"string","minLength":1}}},"required":["action","session_ids"],"additionalProperties":false},
            {"type":"object","properties":{"action":{"const":"read"},"session_id":{"type":"string","minLength":1},"max_chars":{"type":"integer","minimum":1,"maximum":MAX_SESSION_READ_CHARS}},"required":["action","session_id"],"additionalProperties":false},
            {"type":"object","properties":{"action":{"const":"send"},"session_id":{"type":"string","minLength":1},"prompt":{"type":"string","minLength":1}},"required":["action","session_id","prompt"],"additionalProperties":false},
            {"type":"object","properties":{"action":{"const":"wait"},"session_ids":{"type":"array","minItems":1,"items":{"type":"string","minLength":1}},"condition":{"type":"string","enum":["any","all"]},"timeout_ms":{"type":"integer","minimum":0,"maximum":60000}},"required":["action","session_ids"],"additionalProperties":false}
        ]})
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        match args.get("action").and_then(Value::as_str).map(str::trim) {
            Some("create") => self.create(&args).await,
            Some("list") => self.list(&args).await,
            Some("status") => self.status(&args).await,
            Some("read") => self.read(&args).await,
            Some("send") => self.send(&args).await,
            Some("wait") => self.wait(&args).await,
            Some(other) => anyhow::bail!(
                "Unknown SessionTool action '{other}'. Supported: create, list, status, read, send, wait"
            ),
            None => anyhow::bail!("Missing 'action' parameter"),
        }
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests;
