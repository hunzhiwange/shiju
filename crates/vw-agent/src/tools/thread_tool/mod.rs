//! 当前项目可见任务的独立工具入口。

use super::session_tool::SessionTool;
use super::traits::{Tool, ToolResult};
use crate::session::runtime::{SessionScope, fork_visible_task_session, load_session, visible_tasks_for_parent};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::path::PathBuf;
use vw_api_types::id::TenantScope;

/// 在当前项目中创建用户可见的 thread。
pub struct CreateThreadTool {
    parent_id: Option<String>,
    directory: PathBuf,
    session_tool: SessionTool,
}

impl CreateThreadTool {
    pub fn new(parent_id: Option<String>, directory: PathBuf) -> Self {
        let session_tool = SessionTool::new(parent_id.clone(), directory.clone());
        Self { parent_id, directory, session_tool }
    }

    #[cfg(test)]
    pub(crate) fn with_coordinator(mut self, coordinator: crate::session::task_session::TaskSessionCoordinator) -> Self {
        self.session_tool = self.session_tool.with_coordinator(coordinator);
        self
    }

    #[cfg(test)]
    pub(crate) fn with_stream_executor(mut self, executor: std::sync::Arc<dyn Fn(String, vw_api_types::chat::GatewayChatStreamRequest) -> futures_util::future::BoxFuture<'static, Result<crate::gateway::api::handlers::session::stream::GatewayChatStreamCollected, crate::gateway::ApiError>> + Send + Sync>) -> Self {
        self.session_tool = self.session_tool.with_stream_executor(executor);
        self
    }
}

#[async_trait]
impl Tool for CreateThreadTool {
    fn name(&self) -> &str { "create_thread" }

    fn description(&self) -> &str { "Create a visible thread in the current project." }

    fn parameters_schema(&self) -> Value {
        json!({"type":"object","properties":{
            "prompt":{"type":"string","minLength":1},
            "title":{"type":"string","minLength":1}
        },"required":["prompt"],"additionalProperties":false})
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let object = args.as_object().ok_or_else(|| anyhow::anyhow!("create_thread arguments must be an object"))?;
        if let Some(field) = object.keys().find(|key| !["prompt", "title"].contains(&key.as_str())) {
            anyhow::bail!("unsupported create_thread field '{field}'");
        }
        let prompt = object.get("prompt").and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow::anyhow!("'prompt' must be a non-empty string"))?;
        let title = match object.get("title") {
            Some(value) => value.as_str().map(str::trim).filter(|s| !s.is_empty())
                .ok_or_else(|| anyhow::anyhow!("'title' must be a non-empty string"))?,
            None => "Task",
        };
        let parent_id = self.parent_id.as_deref().filter(|s| !s.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("create_thread requires a current thread ID"))?;
        let scope = SessionScope::new(TenantScope::default(), &self.directory)?;
        let parent = load_session(scope, parent_id).await?;
        let created = self.session_tool.execute(json!({"action":"create","title":title,"prompt":prompt})).await?;
        let created_data: Value = serde_json::from_str(&created.output)?;
        Ok(ToolResult { success: created.success, output: serde_json::to_string(&json!({
            "thread_id": created_data["session_id"],
            "project_id": parent.info.project_id,
            "directory": created_data["directory"],
            "title": title,
            "state": created_data["state"]
        }))?, error: created.error })
    }

    fn is_concurrency_safe(&self) -> bool { true }
}

/// Continue an idle visible child thread of the current thread.
pub struct SendMessageToThreadTool {
    parent_id: Option<String>,
    directory: PathBuf,
    session_tool: SessionTool,
}

impl SendMessageToThreadTool {
    pub fn new(parent_id: Option<String>, directory: PathBuf) -> Self {
        let session_tool = SessionTool::new(parent_id.clone(), directory.clone());
        Self { parent_id, directory, session_tool }
    }

    #[cfg(test)]
    pub(crate) fn with_coordinator(mut self, coordinator: crate::session::task_session::TaskSessionCoordinator) -> Self {
        self.session_tool = self.session_tool.with_coordinator(coordinator);
        self
    }

    #[cfg(test)]
    pub(crate) fn with_stream_executor(mut self, executor: std::sync::Arc<dyn Fn(String, vw_api_types::chat::GatewayChatStreamRequest) -> futures_util::future::BoxFuture<'static, Result<crate::gateway::api::handlers::session::stream::GatewayChatStreamCollected, crate::gateway::ApiError>> + Send + Sync>) -> Self {
        self.session_tool = self.session_tool.with_stream_executor(executor);
        self
    }
}

#[async_trait]
impl Tool for SendMessageToThreadTool {
    fn name(&self) -> &str { "send_message_to_thread" }
    fn description(&self) -> &str { "Send a message to an idle visible child thread." }
    fn parameters_schema(&self) -> Value {
        json!({"type":"object","properties":{"thread_id":{"type":"string","minLength":1},"prompt":{"type":"string","minLength":1}},"required":["thread_id","prompt"],"additionalProperties":false})
    }
    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let object = query_args(&args, &["thread_id", "prompt"])?;
        let id = object.get("thread_id").and_then(Value::as_str).map(str::trim).filter(|id| !id.is_empty())
            .ok_or_else(|| anyhow::anyhow!("thread_id must be a non-empty string"))?;
        let prompt = object.get("prompt").and_then(Value::as_str).map(str::trim).filter(|prompt| !prompt.is_empty())
            .ok_or_else(|| anyhow::anyhow!("prompt must be a non-empty string"))?;
        query_child(&self.parent_id, &self.directory, id).await?;
        let result = self.session_tool.execute(json!({"action":"send","session_id":id,"prompt":prompt})).await?;
        let output: Value = serde_json::from_str(&result.output)?;
        Ok(ToolResult { success: result.success, output: json!({"thread_id":id,"state":output["state"]}).to_string(), error: result.error })
    }
    fn is_concurrency_safe(&self) -> bool { true }
}

macro_rules! query_tool {
    ($name:ident, $tool_name:literal, $description:literal, $schema:expr, $body:expr) => {
        #[derive(Clone)]
        pub struct $name { parent_id: Option<String>, directory: PathBuf }
        impl $name {
            pub fn new(parent_id: Option<String>, directory: PathBuf) -> Self { Self { parent_id, directory } }
        }
        #[async_trait]
        impl Tool for $name {
            fn name(&self) -> &str { $tool_name }
            fn description(&self) -> &str { $description }
            fn parameters_schema(&self) -> Value { $schema }
            async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> { ($body)(self.clone(), args).await }
            fn is_concurrency_safe(&self) -> bool { true }
        }
    };
}

fn query_args<'a>(args: &'a Value, allowed: &[&str]) -> anyhow::Result<&'a serde_json::Map<String, Value>> {
    let object = args.as_object().ok_or_else(|| anyhow::anyhow!("thread arguments must be an object"))?;
    if let Some(field) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        anyhow::bail!("unsupported thread query field '{field}'");
    }
    Ok(object)
}

async fn query_parent(parent_id: &Option<String>, directory: &PathBuf) -> anyhow::Result<(String, SessionScope, String)> {
    let id = parent_id.as_deref().filter(|id| !id.trim().is_empty()).ok_or_else(|| anyhow::anyhow!("current thread ID required"))?;
    let scope = SessionScope::new(TenantScope::default(), directory)?;
    let parent = load_session(scope.clone(), id).await?;
    Ok((id.to_string(), scope, parent.info.project_id))
}

async fn query_child(parent_id: &Option<String>, directory: &PathBuf, id: &str) -> anyhow::Result<crate::session::runtime::SessionSnapshot> {
    let (parent, scope, project_id) = query_parent(parent_id, directory).await?;
    let child = load_session(scope, id).await?;
    if child.info.parent_id.as_deref() != Some(&parent) || child.info.project_id != project_id || !child.info.is_visible_task() {
        anyhow::bail!("thread is not a visible child of the current thread");
    }
    Ok(child)
}

fn run_sync<T: Send>(operation: impl FnOnce() -> anyhow::Result<T> + Send) -> anyhow::Result<T> {
    std::thread::scope(|scope| {
        scope.spawn(operation).join().map_err(|panic| {
            let message = panic
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| panic.downcast_ref::<String>().map(String::as_str))
                .unwrap_or("unknown panic");
            anyhow::anyhow!("thread metadata operation panicked: {message}")
        })?
    })
}

query_tool!(ForkThreadTool, "fork_thread", "Fork a visible child thread in the current project.",
    json!({"type":"object","properties":{"thread_id":{"type":"string","minLength":1},"message_id":{"type":"string","minLength":1}},"required":["thread_id"],"additionalProperties":false}),
    |tool: ForkThreadTool, args: Value| async move {
        let object = query_args(&args, &["thread_id", "message_id"])?;
        let id = object.get("thread_id").and_then(Value::as_str).filter(|id| !id.is_empty()).ok_or_else(|| anyhow::anyhow!("thread_id required"))?;
        let message_id = match object.get("message_id") {
            Some(value) => Some(value.as_str().filter(|id| !id.is_empty()).ok_or_else(|| anyhow::anyhow!("message_id must be a non-empty string"))?),
            None => None,
        };
        let (parent, scope, _) = query_parent(&tool.parent_id, &tool.directory).await?;
        let created = fork_visible_task_session(scope, &parent, id, message_id).await?;
        Ok(ToolResult { success: true, output: json!({"thread_id":created.id,"project_id":created.project_id,"parent_thread_id":parent}).to_string(), error: None })
    });

query_tool!(HandoffThreadTool, "handoff_thread", "Report that thread handoff is unsupported by this runtime.",
    json!({"type":"object","properties":{"thread_id":{"type":"string","minLength":1}},"required":["thread_id"],"additionalProperties":false}),
    |tool: HandoffThreadTool, args: Value| async move {
        let object = query_args(&args, &["thread_id"])?;
        let id = object.get("thread_id").and_then(Value::as_str).map(str::trim).filter(|id| !id.is_empty())
            .ok_or_else(|| anyhow::anyhow!("thread_id must be a non-empty string"))?;
        query_child(&tool.parent_id, &tool.directory, id).await?;
        anyhow::bail!("handoff_thread is unsupported: this runtime cannot transfer threads between hosts or worktrees")
    });

query_tool!(ListThreadsTool, "list_threads", "List visible threads in the current project and parent thread.",
    json!({"type":"object","properties":{},"additionalProperties":false}),
    |tool: ListThreadsTool, args: Value| async move {
        query_args(&args, &[])?;
        let (parent, scope, project_id) = query_parent(&tool.parent_id, &tool.directory).await?;
        let pinned_project_id = project_id.clone();
        let pinned = run_sync(move || {
            crate::session::ui_store::load_pinned_threads(
                &TenantScope::default(),
                &pinned_project_id,
            )
        })?;
        let threads = visible_tasks_for_parent(scope, &parent).await?.into_iter().filter(|info| info.time.archived.is_none()).map(|info| json!({
            "thread_id": info.id, "project_id": project_id, "title": info.title,
            "parent_thread_id": parent, "updated_ms": info.time.updated,
            "pinned": pinned.contains(&info.id)
        })).collect::<Vec<_>>();
        Ok(ToolResult { success: true, output: json!({"threads":threads}).to_string(), error: None })
    });

query_tool!(SetThreadPinnedTool, "set_thread_pinned", "Pin or unpin a visible child thread in the current project.",
    json!({"type":"object","properties":{"thread_id":{"type":"string","minLength":1},"pinned":{"type":"boolean"}},"required":["thread_id","pinned"],"additionalProperties":false}),
    |tool: SetThreadPinnedTool, args: Value| async move {
        let object = query_args(&args, &["thread_id", "pinned"])?;
        let id = object.get("thread_id").and_then(Value::as_str).map(str::trim).filter(|id| !id.is_empty())
            .ok_or_else(|| anyhow::anyhow!("thread_id must be a non-empty string"))?;
        let pinned = object.get("pinned").and_then(Value::as_bool)
            .ok_or_else(|| anyhow::anyhow!("pinned must be a boolean"))?;
        let child = query_child(&tool.parent_id, &tool.directory, id).await?;
        let project_id = child.info.project_id.clone();
        let thread_id = id.to_string();
        let actual = run_sync({
            let project_id = project_id.clone();
            let thread_id = thread_id.clone();
            move || crate::session::ui_store::set_thread_pinned(
                &TenantScope::default(),
                &project_id,
                &thread_id,
                pinned,
            )
        })?;
        Ok(ToolResult { success: true, output: json!({"thread_id":thread_id,"project_id":project_id,"pinned":actual}).to_string(), error: None })
    });

query_tool!(ReadThreadTool, "read_thread", "Read a visible child thread.",
    json!({"type":"object","properties":{"thread_id":{"type":"string"},"max_chars":{"type":"integer","minimum":1}},"required":["thread_id"],"additionalProperties":false}),
    |tool: ReadThreadTool, args: Value| async move {
        let object = query_args(&args, &["thread_id", "max_chars"])?;
        let id = object.get("thread_id").and_then(Value::as_str).filter(|id| !id.is_empty()).ok_or_else(|| anyhow::anyhow!("thread_id required"))?;
        query_child(&tool.parent_id, &tool.directory, id).await?;
        let mut legacy = json!({"action":"read","session_id":id});
        if let Some(max_chars) = object.get("max_chars") { legacy["max_chars"] = max_chars.clone(); }
        let result = SessionTool::new(tool.parent_id.clone(), tool.directory.clone()).execute(legacy).await?;
        let mut output: Value = serde_json::from_str(&result.output)?;
        output.as_object_mut().expect("read result object").remove("session_id");
        output["thread_id"] = json!(id);
        Ok(ToolResult { success: true, output: output.to_string(), error: None })
    });

query_tool!(SetThreadTitleTool, "set_thread_title", "Update the title of a visible child thread.",
    json!({"type":"object","properties":{"thread_id":{"type":"string","minLength":1},"title":{"type":"string","minLength":1}},"required":["thread_id","title"],"additionalProperties":false}),
    |tool: SetThreadTitleTool, args: Value| async move {
        let object = query_args(&args, &["thread_id", "title"])?;
        let id = object.get("thread_id").and_then(Value::as_str).map(str::trim).filter(|id| !id.is_empty())
            .ok_or_else(|| anyhow::anyhow!("thread_id must be a non-empty string"))?;
        let title = object.get("title").and_then(Value::as_str).map(str::trim).filter(|title| !title.is_empty())
            .ok_or_else(|| anyhow::anyhow!("title must be a non-empty string"))?;
        query_child(&tool.parent_id, &tool.directory, id).await?;
        let tenant = TenantScope::default();
        let directory = tool.directory.clone();
        let id = id.to_string();
        let title = title.to_string();
        let updated = crate::project::instance::provide(directory, None, move || {
            let id = id.clone();
            let title = title.clone();
            Box::pin(async move {
                crate::session::ui_store::with_tenant_scope(tenant, async move {
                    crate::session::session::update(&id, |info| info.title = title).await
                }).await
            })
        }).await??;
        Ok(ToolResult { success: true, output: json!({"thread_id":updated.id,"project_id":updated.project_id,"title":updated.title}).to_string(), error: None })
    });

query_tool!(SetThreadArchivedTool, "set_thread_archived", "Archive or restore a visible child thread.",
    json!({"type":"object","properties":{"thread_id":{"type":"string","minLength":1},"archived":{"type":"boolean"}},"required":["thread_id","archived"],"additionalProperties":false}),
    |tool: SetThreadArchivedTool, args: Value| async move {
        let object = query_args(&args, &["thread_id", "archived"])?;
        let id = object.get("thread_id").and_then(Value::as_str).map(str::trim).filter(|id| !id.is_empty())
            .ok_or_else(|| anyhow::anyhow!("thread_id must be a non-empty string"))?;
        let archived = object.get("archived").and_then(Value::as_bool)
            .ok_or_else(|| anyhow::anyhow!("archived must be a boolean"))?;
        query_child(&tool.parent_id, &tool.directory, id).await?;
        let tenant = TenantScope::default();
        let directory = tool.directory.clone();
        let id = id.to_string();
        let updated = crate::project::instance::provide(directory, None, move || {
            let id = id.clone();
            Box::pin(async move {
                crate::session::ui_store::with_tenant_scope(tenant, async move {
                    crate::session::session::update(&id, |info| {
                        info.time.archived = archived.then(crate::session::session::now_ms);
                    }).await
                }).await
            })
        }).await??;
        Ok(ToolResult { success: true, output: json!({"thread_id":updated.id,"project_id":updated.project_id,"archived":updated.time.archived.is_some()}).to_string(), error: None })
    });

query_tool!(WaitThreadsTool, "wait_threads", "Wait for visible child threads.",
    json!({"type":"object","properties":{"thread_ids":{"type":"array","items":{"type":"string"},"minItems":1},"timeout_ms":{"type":"integer","minimum":0}},"required":["thread_ids"],"additionalProperties":false}),
    |tool: WaitThreadsTool, args: Value| async move {
        let object = query_args(&args, &["thread_ids", "timeout_ms"])?;
        let ids = object.get("thread_ids").and_then(Value::as_array).ok_or_else(|| anyhow::anyhow!("thread_ids required"))?;
        if ids.is_empty() { anyhow::bail!("thread_ids must not be empty"); }
        for id in ids {
            let id = id.as_str().filter(|id| !id.is_empty()).ok_or_else(|| anyhow::anyhow!("thread_ids must contain strings"))?;
            query_child(&tool.parent_id, &tool.directory, id).await?;
        }
        let mut legacy = json!({"action":"wait","session_ids":ids});
        if let Some(timeout_ms) = object.get("timeout_ms") { legacy["timeout_ms"] = timeout_ms.clone(); }
        let result = SessionTool::new(tool.parent_id.clone(), tool.directory.clone()).execute(legacy).await?;
        let mut output: Value = serde_json::from_str(&result.output)?;
        if let Some(tasks) = output["tasks"].as_array_mut() {
            for task in tasks { if let Some(id) = task.as_object_mut().and_then(|task| task.remove("task_session_id")) { task["thread_id"] = id; } }
        }
        Ok(ToolResult { success: true, output: output.to_string(), error: None })
    });

#[cfg(test)]
mod tests;
