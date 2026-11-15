//! Observation-only events carry host metadata, never provider options or messages.
use serde_json::{Value, json};

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

pub use plugin_hooks::hook_model::{validate, token_usage};
