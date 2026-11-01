//! Trusted parent execution carries child identity across worker threads.
use crate::tools::ToolUseContext;
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::sync::Arc;

pub(crate) fn validate(input: &Value) -> Result<()> {
    ensure!(
        ["session_id", "agent_id", "agent_type"]
            .iter()
            .all(|key| input[key].as_str().is_some_and(|s| !s.trim().is_empty()))
            && input["session_id"] != input["agent_id"]
            && input["cwd"].as_str().is_some_and(|s| std::path::Path::new(s).is_absolute())
            && matches!(input["hook_event_name"].as_str(), Some("SubagentStart" | "SubagentStop")),
        "invalid subagent hook identity"
    );
    Ok(())
}

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

pub(crate) fn contexts(output: &Value, event: &str) -> Result<Vec<String>> {
    let text = if event == "SubagentStart" {
        output.pointer("/hookSpecificOutput/additionalContext").and_then(Value::as_str)
    } else if output["continue"] != false && output["decision"] == "block" {
        output["reason"].as_str()
    } else {
        None
    };
    Ok(text.filter(|s| !s.trim().is_empty()).map(|s| vec![s.to_owned()]).unwrap_or_default())
}

pub(crate) fn parse(code: Option<i32>, stdout: &[u8], stderr: &[u8], stop: bool) -> Result<Value> {
    if stop && code == Some(2) {
        let reason = String::from_utf8_lossy(stderr);
        return Ok(json!({"decision":"block", "reason":reason.trim()}));
    }
    if code != Some(0) {
        tracing::warn!("subagent hook command failed");
        return Ok(Value::Null);
    }
    if stdout.iter().all(u8::is_ascii_whitespace) {
        return Ok(Value::Null);
    }
    let value = match serde_json::from_slice::<Value>(stdout) {
        Ok(value) => value,
        Err(_) if !stop && !stdout.starts_with(b"{") => {
            return Ok(
                json!({"hookSpecificOutput":{"additionalContext":String::from_utf8_lossy(stdout).trim()}}),
            );
        }
        Err(_) => {
            tracing::warn!("invalid subagent hook output");
            return Ok(Value::Null);
        }
    };
    let valid = value.as_object().is_some_and(|object| {
        object.iter().all(|(key, item)| match key.as_str() {
            "continue" | "suppressOutput" => item.is_boolean(),
            "stopReason" | "systemMessage" => item.is_null() || item.is_string(),
            "decision" if stop => item.is_null() || item == "block",
            "reason" if stop => item.is_null() || item.is_string(),
            "hookSpecificOutput" if !stop => {
                item.is_null()
                    || item.as_object().is_some_and(|specific| {
                        specific.contains_key("hookEventName")
                            && specific.iter().all(|(key, value)| match key.as_str() {
                                "hookEventName" => value == "SubagentStart",
                                "additionalContext" => value.is_null() || value.is_string(),
                                _ => false,
                            })
                    })
            }
            _ => false,
        })
    });
    if !valid
        || (stop
            && value["decision"] == "block"
            && !value["reason"].as_str().is_some_and(|s| !s.trim().is_empty()))
    {
        tracing::warn!("invalid subagent hook output");
        return Ok(Value::Null);
    }
    // SubagentStart only injects context; universal continue:false is ignored by the baseline.
    Ok(value)
}
