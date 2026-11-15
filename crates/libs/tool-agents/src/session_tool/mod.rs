//! 用户可见任务会话的创建工具。

use tool_context::context::current_tool_use_context;
use tool_types::{Tool, ToolResult};
use session_port::{SessionScope, SessionSnapshot, task_session::*};
use session_info as session;
use crate::ports::{SessionHost, TaskCoordinator, SessionStreamExecutor};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use vw_api_chat::chat::GatewayChatStreamRequest;
use {
    vw_api_transport::id::SessionId,
    vw_api_transport::id::TenantScope,
};
use session_text::encode_task_session_source_marker;

pub const MAX_SESSION_READ_CHARS: usize = 64 * 1024;

/// 创建并异步启动同一工作区内的用户可见任务会话。
pub struct SessionTool<H: SessionHost> {
    parent_session_id: Option<String>,
    workspace_dir: PathBuf,
    coordinator: H::Coordinator,
    stream_executor: SessionStreamExecutor<H::Error>,
}

impl<H: SessionHost> SessionTool<H> {
    /// 绑定当前会话和工作目录。
    pub fn new(parent_session_id: Option<String>, workspace_dir: PathBuf) -> Self {
        Self {
            parent_session_id,
            workspace_dir,
            coordinator: H::coordinator(),
            stream_executor: Arc::new(|directory, request| {
                Box::pin(H::collect(directory, request))
            }),
        }
    }

    #[doc(hidden)]
    pub fn with_coordinator(mut self, coordinator: impl Into<H::Coordinator>) -> Self {
        self.coordinator = coordinator.into();
        self
    }

    #[doc(hidden)]
    pub fn with_stream_executor(mut self, executor: SessionStreamExecutor<H::Error>) -> Self {
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

    fn tenant_scope() -> TenantScope {
        current_tool_use_context()
            .map_or_else(H::tenant, |context| context.tenant_scope())
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

    #[doc(hidden)]
    pub async fn validated_target(&self, session_id: &str) -> anyhow::Result<SessionSnapshot> {
        let current = self.parent_session_id()?;
        if session_id == current {
            anyhow::bail!("SessionTool cannot target the current session");
        }
        let scope = SessionScope::new(Self::tenant_scope(), self.workspace_dir()?)?;
        let snapshot = H::load(scope, session_id).await?;
        if !snapshot.info.is_visible_task() {
            anyhow::bail!("session '{session_id}' is not a visible task session");
        }
        if snapshot.info.parent_id.as_deref() != Some(current) {
            anyhow::bail!("session '{session_id}' parent session mismatch");
        }
        Ok(snapshot)
    }

    async fn recover_tasks(&self) -> anyhow::Result<()> {
        let parent = self.parent_session_id()?;
        let scope = SessionScope::new(Self::tenant_scope(), self.workspace_dir()?)?;
        let tasks = H::visible(scope.clone(), parent).await?;
        let Some(project_id) = tasks.first().map(|task| task.project_id.as_str()) else {
            return Ok(());
        };
        let tenant = scope.tenant().clone();
        let project_id = project_id.to_string();
        let persisted = run_sync(move || H::load_states(&tenant, &project_id))?;
        for task in tasks {
            let saved = persisted.get(&task.id).filter(|state| state.parent_session_id == parent);
            let mut snapshot = saved.cloned().unwrap_or_else(|| TaskSessionSnapshot {
                parent_session_id: parent.to_string(),
                task_session_id: task.id.clone(),
                title: None,
                state: TaskSessionState::Failed,
                error: Some("task turn state unavailable after restart".to_string()),
                created_ms: task.time.created,
                updated_ms: task.time.updated,
            });
            if !snapshot.state.is_terminal() {
                snapshot.state = TaskSessionState::Failed;
                snapshot.error = Some("task turn interrupted by restart".to_string());
                snapshot.updated_ms = H::now_ms();
            }
            self.coordinator.restore(snapshot);
        }
        Ok(())
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
        task: session::Info,
        prompt: String,
    ) -> anyhow::Result<()> {
        let pending = task.is_first_response_pending();
        let session::Info { id: task_session_id, project_id, directory, .. } = task;
        let tenant = Self::tenant_scope();
        let coordinator = self.coordinator.clone();
        let executor = self.stream_executor.clone();
        let model = Self::inherited_model_ref();
        let (startup_tx, startup_rx) = std::sync::mpsc::sync_channel(1);
        let thread_parent = parent_session_id.clone();
        let thread_task = task_session_id.clone();
        let thread_project = project_id.clone();
        let thread_tenant = tenant.clone();
        let spawned = std::thread::Builder::new()
            .name("vw-visible-task-session".to_string())
            .spawn(move || {
                let startup = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(anyhow::Error::from)
                    .and_then(|runtime| {
                        coordinator.mark_running(&thread_parent, &thread_task)?;
                        persist_task_state::<H>(
                            &coordinator, &thread_tenant, &thread_parent, &thread_task, &thread_project,
                        )?;
                        Ok(runtime)
                    });
                let runtime = match startup {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        let _ = startup_tx.send(Err(error.to_string()));
                        return;
                    }
                };
                let _ = startup_tx.send(Ok(()));
                let request = GatewayChatStreamRequest {
                    session_id: Some(SessionId::from(thread_task.as_str())),
                    messages: vec![json!({"role": "user", "content": prompt})],
                    model,
                    ..GatewayChatStreamRequest::default()
                };
                let result = runtime.block_on(H::with_tenant(
                    thread_tenant.clone(),
                    async move { executor(directory, request).await },
                ));
                let error = match result {
                    Ok(_) if pending => match H::committed(
                        &thread_tenant, &thread_project, &thread_task,
                    ) {
                        Ok(true) => None,
                        Ok(false) => Some("task turn did not commit a successful first response".into()),
                        Err(error) => Some(error.to_string()),
                    },
                    Ok(_) => None,
                    Err(error) => Some(error.to_string()),
                };
                if let Err(error) = finish_task_turn::<H>(
                    &coordinator, &thread_tenant, &thread_parent, &thread_task, &thread_project, error,
                ) {
                    tracing::error!(target: "vw_agent", error = %error, "failed to persist terminal visible task state");
                }
            });

        let startup_error = match spawned {
            Err(error) => format!("spawn visible task session runtime: {error}"),
            Ok(_) => match startup_rx.recv() {
                Ok(Ok(())) => return Ok(()),
                Ok(Err(error)) => format!("start visible task session: {error}"),
                Err(error) => format!("visible task session stopped before startup: {error}"),
            },
        };
        if let Err(error) = finish_task_turn::<H>(
            &self.coordinator,
            &tenant,
            &parent_session_id,
            &task_session_id,
            &project_id,
            Some(startup_error.clone()),
        ) {
            tracing::error!(target: "vw_agent", %error, "failed to record visible task startup failure");
        }
        anyhow::bail!(startup_error)
    }

    async fn create(&self, args: &Value) -> anyhow::Result<ToolResult> {
        Self::validate_fields(args, &["action", "title", "prompt"])?;
        let parent_session_id = self.parent_session_id()?.to_string();
        let directory = self.workspace_dir()?;
        let title = Self::required_text(args, "title")?.to_string();
        let prompt = Self::required_text(args, "prompt")?;
        let prompt = encode_task_session_source_marker(&parent_session_id, prompt)
            .ok_or_else(|| anyhow::anyhow!("current session ID cannot be encoded as a source"))?;
        let scope = SessionScope::new(Self::tenant_scope(), &directory)?;
        let created = H::reserve(scope, parent_session_id.clone()).await?;
        let session_id = created.info.id.clone();
        let directory = created.info.directory.clone();
        self.coordinator.register(&parent_session_id, &session_id, Some(title.clone()))?;
        self.spawn_background_turn(parent_session_id.clone(), created.info, prompt)?;
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
        self.recover_tasks().await?;
        let parent = self.parent_session_id()?;
        let snapshots = self.coordinator.snapshots(parent, &[])?;
        let mut tasks = Vec::new();
        for state in snapshots {
            let target = self.validated_target(&state.task_session_id).await?;
            let title = if target.info.is_first_response_pending() {
                state.title.as_deref().unwrap_or_default()
            } else {
                &target.info.title
            };
            tasks.push(json!({"session_id": target.info.id, "title": title, "state": state.state, "updated_ms": state.updated_ms}));
        }
        Self::result(json!({"action": "list", "tasks": tasks}))
    }

    async fn status(&self, args: &Value) -> anyhow::Result<ToolResult> {
        Self::validate_fields(args, &["action", "session_ids"])?;
        self.recover_tasks().await?;
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
        self.recover_tasks().await?;
        let parent = self.parent_session_id()?.to_string();
        let id = Self::required_text(args, "session_id")?.to_string();
        let prompt = Self::required_text(args, "prompt")?;
        let target = self.validated_target(&id).await?;
        let prompt = encode_task_session_source_marker(&parent, prompt)
            .ok_or_else(|| anyhow::anyhow!("current session ID cannot be encoded as a source"))?;
        self.coordinator.begin_turn(&parent, &id)?;
        self.spawn_background_turn(parent, target.info, prompt)?;
        Self::result(json!({"action": "send", "session_id": id, "state": "running"}))
    }

    async fn wait(&self, args: &Value) -> anyhow::Result<ToolResult> {
        Self::validate_fields(args, &["action", "session_ids", "condition", "timeout_ms"])?;
        self.recover_tasks().await?;
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

#[doc(hidden)]
pub fn persist_task_state<H: SessionHost>(
    coordinator: &H::Coordinator,
    tenant: &TenantScope,
    parent_id: &str,
    task_id: &str,
    project_id: &str,
) -> anyhow::Result<()> {
    let snapshot = coordinator.snapshots(parent_id, &[task_id.to_string()])?.remove(0);
    run_sync(move || H::save_state(tenant, project_id, &snapshot))
}

fn finish_task_turn<H: SessionHost>(
    coordinator: &H::Coordinator,
    tenant: &TenantScope,
    parent_id: &str,
    task_id: &str,
    project_id: &str,
    error: Option<String>,
) -> anyhow::Result<()> {
    coordinator.finish_turn(parent_id, task_id, error, |snapshot| {
        run_sync(move || H::save_state(tenant, project_id, snapshot))
    })?;
    Ok(())
}

fn run_sync<T: Send>(operation: impl FnOnce() -> anyhow::Result<T> + Send) -> anyhow::Result<T> {
    std::thread::scope(|scope| {
        scope.spawn(operation).join().map_err(|panic| {
            let message = panic
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| panic.downcast_ref::<String>().map(String::as_str))
                .unwrap_or("unknown panic");
            anyhow::anyhow!("visible task state thread panicked: {message}")
        })?
    })
}

#[doc(hidden)]
pub fn bounded_read_output(
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
impl<H: SessionHost> Tool for SessionTool<H> {
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
