//! Trusted parent execution carries child identity across worker threads.
use crate::tools::ToolUseContext;
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::sync::Arc;

#[cfg(test)]
pub(crate) async fn emit(
    context: Option<Arc<ToolUseContext>>,
    parent: &str,
    child: &str,
    agent: &str,
    output: Option<&str>,
    stop: bool,
) -> Result<Vec<String>> {
    emit_boundary(context, parent, child, agent, output, stop.then_some(false)).await
}

pub(crate) async fn emit_boundary(
    context: Option<Arc<ToolUseContext>>,
    parent: &str,
    child: &str,
    agent: &str,
    output: Option<&str>,
    stop: Option<bool>,
) -> Result<Vec<String>> {
    let Some(context) = context else {
        return Ok(Vec::new());
    };
    ensure!(context.session() == parent, "subagent parent session mismatch");
    let mut isolated = context.as_ref().clone().with_turn_id(child);
    isolated.async_hooks = context.async_hooks.clone();
    let context = Arc::new(isolated);
    let mut input = json!({"session_id":parent, "agent_id":child, "agent_type":agent,
        "turn_id":context.turn_id(), "cwd":context.workspace_root(), "model":context.model(),
        "transcript_path":null, "permission_mode":"default",
        "hook_event_name":if stop.is_some() {"SubagentStop"} else {"SubagentStart"}});
    if let Some(active) = stop {
        input["agent_transcript_path"] = Value::Null;
        input["stop_hook_active"] = json!(active);
        input["last_assistant_message"] = json!(output);
    }
    validate(&input)?;
    if let Some(runner) = context.hook_runner() {
        return crate::tools::scope_tool_use_context(context.clone(), runner.subagent(&input))
            .await;
    }
    Ok(Vec::new())
}

pub use plugin_hooks::hook_subagent::{validate, contexts, parse};
