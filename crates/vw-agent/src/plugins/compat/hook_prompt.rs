//! Current-input dispatch; restored history is never replayed through hooks.
use crate::tools::ToolUseContext;
use anyhow::Result;
use serde_json::json;
use std::sync::Arc;

pub(crate) async fn submit(context: Arc<ToolUseContext>, prompt: &str) -> Result<()> {
    let input = json!({"hook_event_name":"UserPromptSubmit", "session_id":context.session(),
        "turn_id":context.turn_id(), "cwd":context.workspace_root(), "model":context.model(),
        "prompt":prompt, "transcript_path":null,
        "permission_mode":if context.full_access_enabled() {"bypassPermissions"} else {"default"}});
    validate(&input)?;
    if let Some(runner) = context.hook_runner() {
        let result = crate::tools::scope_tool_use_context(
            context.clone(),
            runner.user_prompt_submit(&input),
        )
        .await;
        if result.is_err() {
            context.take_hook_contexts();
        }
        result?;
    }
    Ok(())
}

pub use plugin_hooks::hook_prompt::{validate, parse};
