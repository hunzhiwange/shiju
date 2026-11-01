//! Observation-only events carry host metadata, never provider options or messages.
use anyhow::{Result, ensure};
use serde_json::{Value, json};

pub(crate) fn validate(input: &Value) -> Result<()> {
    ensure!(
        ["session_id", "turn_id", "model"]
            .iter()
            .all(|k| input[k].as_str().is_some_and(|s| !s.trim().is_empty()))
            && input["cwd"].as_str().is_some_and(|s| std::path::Path::new(s).is_absolute()),
        "invalid model request metadata"
    );
    ensure!(
        input["hook_event_name"] == "PreModelRequest"
            || (input["hook_event_name"] == "PostModelRequest"
                && matches!(input["status"].as_str(), Some("completed" | "failed" | "cancelled"))
                && input.get("token_usage").is_some()),
        "invalid model request event"
    );
    Ok(())
}

pub(crate) async fn emit(status: Option<&str>, usage: Value) {
    let Some(context) = crate::tools::current_tool_use_context() else {
        return;
    };
    let mut input = json!({"session_id":context.session(), "turn_id":context.turn_id(),
        "model":context.model(), "cwd":context.workspace_root(), "transcript_path":null,
        "hook_event_name":if status.is_some() {"PostModelRequest"} else {"PreModelRequest"}});
    if let Some(status) = status {
        input["status"] = json!(status);
        input["error"] =
            if status == "failed" { json!("model request failed") } else { Value::Null };
        input["token_usage"] = usage;
    }
    if validate(&input).is_ok()
        && let Some(runner) = context.hook_runner()
    {
        runner.model_request(&input).await;
    }
}

/// Codex usage includes cached input within input and reasoning within output.
pub(crate) fn token_usage(input: i64, output: i64, cached: i64, reasoning: i64) -> Value {
    json!({"input_tokens": input, "output_tokens":output, "cached_input_tokens":cached,
        "cache_write_input_tokens":0, "reasoning_output_tokens":reasoning,
        "total_tokens":input.saturating_add(output)})
}
