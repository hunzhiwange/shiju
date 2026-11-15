//! Plugin hook protocol implementation.
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use crate::host::HookHost;
use tool_context::context::{ToolUseContext, current_tool_use_context};

pub fn validate(input: &Value) -> Result<()> {
    ensure!(
        ["session_id", "turn_id", "model"]
            .iter()
            .all(|key| input[key].as_str().is_some_and(|value| !value.trim().is_empty()))
            && input["cwd"].as_str().is_some_and(|value| std::path::Path::new(value).is_absolute())
            && matches!(input["trigger"].as_str(), Some("auto" | "manual"))
            && matches!(input["hook_event_name"].as_str(), Some("PreCompact" | "PostCompact")),
        "invalid compact hook metadata"
    );
    Ok(())
}

pub fn parse(code: Option<i32>, stdout: &[u8]) -> Result<Value> {
    if code != Some(0) {
        tracing::warn!("compact hook command failed");
        return Ok(Value::Null);
    }
    let value = serde_json::from_slice::<Value>(stdout).unwrap_or(Value::Null);
    let valid = value.as_object().is_some_and(|object| {
        object.iter().all(|(key, value)| match key.as_str() {
            "continue" | "suppressOutput" => value.is_boolean(),
            "stopReason" | "systemMessage" => value.is_null() || value.is_string(),
            _ => false,
        })
    });
    if !valid {
        if !stdout.iter().all(u8::is_ascii_whitespace) {
            tracing::warn!("invalid compact hook output");
        }
        return Ok(Value::Null);
    }
    if value["continue"] == false {
        anyhow::bail!(
            "compact hook stopped: {}",
            value["stopReason"].as_str().unwrap_or("hook stopped execution")
        );
    }
    Ok(value)
}

pub async fn emit<H: HookHost>(
    session: &str,
    turn: &str,
    model: &str,
    auto: bool,
    post: bool,
) -> Result<()> {
    let Some(context) = current_tool_use_context::<H>() else {
        return Ok(());
    };
    ensure!(context.session() == session, "compact hook session mismatch");
    let input = json!({"session_id":session, "turn_id":context.turn_id().unwrap_or(turn),
        "model":model, "cwd":context.workspace_root(), "transcript_path":null,
        "hook_event_name":if post {"PostCompact"}else{"PreCompact"},
        "trigger":if auto {"auto"}else{"manual"}});
    validate(&input)?;
    if let Some(runner) = context.hook_runner() {
        runner.compact(&input).await?;
    }
    Ok(())
}

pub async fn context<H: HookHost>(
    session: &str,
    turn: &str,
    model: &str,
) -> Result<std::sync::Arc<ToolUseContext<H>>> {
    if let Some(context) = current_tool_use_context::<H>() {
        ensure!(context.session() == session, "compact hook session mismatch");
        return Ok(context);
    }
    let root = H::directory();
    ensure!(std::path::Path::new(&root).is_absolute(), "compaction requires project context");
    let runner = H::execution_runner(std::path::Path::new(&root)).await?;
    let mut context =
        ToolUseContext::<H>::new(session, Some(root)).with_turn_id(turn).with_model(model);
    if let Some(runner) = runner {
        context = context.with_hook_runner(runner);
    }
    Ok(std::sync::Arc::new(context))
}
