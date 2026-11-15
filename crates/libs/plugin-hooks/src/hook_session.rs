//! Plugin hook protocol implementation.
use anyhow::{Result, ensure};
use serde_json::Value;
use std::path::Path;
use crate::host::HookHost;
use tool_context::context::{ToolUseContext, scope_tool_use_context};
use serde_json::json;
use std::{collections::HashMap, sync::Arc};

pub fn validate(input: &Value) -> Result<()> {
    ensure!(
        input["hook_event_name"] == "SessionStart"
            && input["session_id"].as_str().is_some_and(|s| !s.trim().is_empty())
            && input["cwd"].as_str().is_some_and(|s| Path::new(s).is_absolute())
            && matches!(input["source"].as_str(), Some("startup" | "resume")),
        "invalid session start input"
    );
    Ok(())
}

pub struct HookSessions<H: HookHost>(tokio::sync::Mutex<HashMap<String, Arc<ToolUseContext<H>>>>);
impl<H: HookHost> Default for HookSessions<H> { fn default() -> Self { Self(Default::default()) } }
fn key<H: HookHost>(session: &str, root: &Path) -> String {
    serde_json::to_string(&(
        H::current_tenant_scope(),
        root.canonicalize().unwrap_or_else(|_| root.to_owned()),
        session,
    ))
    .expect("session identity")
}

pub async fn start<H: HookHost>(context: Arc<ToolUseContext<H>>, resumed: bool) -> Result<()> {
    let root =
        context.workspace_root().ok_or_else(|| anyhow::anyhow!("session requires directory"))?;
    let input = json!({"hook_event_name":"SessionStart", "session_id":context.session(),
        "cwd":root, "model":context.model(), "transcript_path":null,
        "source":if resumed {"resume"} else {"startup"}, "permission_mode":"default"});
    validate(&input)?;
    let mut sessions = H::sessions().0.lock().await;
    let key = key::<H>(context.session(), &root);
    if sessions.contains_key(&key) {
        return Ok(());
    }
    if let Some(runner) = context.hook_runner() {
        if let Err(error) =
            scope_tool_use_context(context.clone(), runner.start_session(&input))
                .await
        {
            scope_tool_use_context(
                context.clone(),
                runner.fire_session_end(context.session(), context.channel().unwrap_or_default()),
            )
            .await;
            return Err(error);
        }
    }
    sessions.insert(key, context);
    Ok(())
}
pub async fn close<H: HookHost>(session: &str, root: &Path) {
    let context = H::sessions().0.lock().await.remove(&key::<H>(session, root));
    if let Some(context) = context
        && let Some(runner) = context.hook_runner()
    {
        scope_tool_use_context(
            context.clone(),
            runner.fire_session_end(session, context.channel().unwrap_or_default()),
        )
        .await;
    }
}
