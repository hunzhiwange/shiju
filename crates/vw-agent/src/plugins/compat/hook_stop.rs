//! Completion control is scoped to a host-owned turn.
use crate::tools::ToolUseContext;
use anyhow::Result;
use serde_json::json;
use std::sync::Arc;

pub use plugin_hooks::hook_stop::Outcome;

pub(crate) async fn emit(
    context: Arc<ToolUseContext>,
    interrupt: bool,
    active: bool,
    message: Option<&str>,
) -> Result<Outcome> {
    // Cleanup is host-owned and must run even when no plugin hooks are installed.
    #[cfg(unix)]
    if let Some(runtime) = crate::tools::computer_control::NativeControlRuntime::shared() {
        runtime.stop_context(&context).await;
    }
    let mut input = json!({"session_id":context.session(), "turn_id":context.turn_id(),
        "cwd":context.workspace_root(), "model":context.model(), "permission_mode":if context.full_access_enabled() {"bypassPermissions"} else {"default"},
        "transcript_path":null, "hook_event_name":if interrupt {"Interrupt"} else {"Stop"},
        "stop_hook_active":active, "last_assistant_message":message});
    if interrupt {
        let object = input.as_object_mut().expect("host hook input is an object");
        object.remove("stop_hook_active");
        object.remove("last_assistant_message");
    }
    if let Some(runner) = context.hook_runner() {
        validate(&input)?;
        let run = crate::tools::scope_tool_use_context(context.clone(), runner.turn_end(&input));
        if !interrupt && let Some(token) = context.abort_token() {
            return tokio::select! { biased; _ = token.cancelled() => Ok(Outcome::default()), result = run => result };
        }
        if interrupt {
            return tokio::time::timeout(std::time::Duration::from_secs(3), run).await.map_err(
                |_| anyhow::anyhow!("Interrupt hooks exceeded cancellation cleanup deadline"),
            )?;
        }
        return run.await;
    }
    Ok(Outcome::default())
}

pub use plugin_hooks::hook_stop::{validate, parse, outcome};
