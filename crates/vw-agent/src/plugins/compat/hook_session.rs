//! Host-owned lifecycle, independent of individual turns.
use crate::tools::ToolUseContext;
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, OnceLock},
};

type Sessions = HashMap<String, Arc<ToolUseContext>>;
fn sessions() -> &'static tokio::sync::Mutex<Sessions> {
    static SESSIONS: OnceLock<tokio::sync::Mutex<Sessions>> = OnceLock::new();
    SESSIONS.get_or_init(Default::default)
}
fn key(session: &str, root: &Path) -> String {
    serde_json::to_string(&(
        crate::session::ui_store::current_tenant_scope(),
        root.canonicalize().unwrap_or_else(|_| root.to_owned()),
        session,
    ))
    .expect("session identity")
}
pub(crate) fn validate(input: &Value) -> Result<()> {
    ensure!(
        input["hook_event_name"] == "SessionStart"
            && input["session_id"].as_str().is_some_and(|s| !s.trim().is_empty())
            && input["cwd"].as_str().is_some_and(|s| Path::new(s).is_absolute())
            && matches!(input["source"].as_str(), Some("startup" | "resume")),
        "invalid session start input"
    );
    Ok(())
}
pub(crate) async fn start(context: Arc<ToolUseContext>, resumed: bool) -> Result<()> {
    let root =
        context.workspace_root().ok_or_else(|| anyhow::anyhow!("session requires directory"))?;
    let input = json!({"hook_event_name":"SessionStart", "session_id":context.session(),
        "cwd":root, "model":context.model(), "transcript_path":null,
        "source":if resumed {"resume"} else {"startup"}, "permission_mode":"default"});
    validate(&input)?;
    let mut sessions = sessions().lock().await;
    let key = key(context.session(), &root);
    if sessions.contains_key(&key) {
        return Ok(());
    }
    if let Some(runner) = context.hook_runner() {
        if let Err(error) =
            crate::tools::scope_tool_use_context(context.clone(), runner.start_session(&input))
                .await
        {
            crate::tools::scope_tool_use_context(
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
pub(crate) async fn close(session: &str, root: &Path) {
    let context = sessions().lock().await.remove(&key(session, root));
    if let Some(context) = context
        && let Some(runner) = context.hook_runner()
    {
        crate::tools::scope_tool_use_context(
            context.clone(),
            runner.fire_session_end(session, context.channel().unwrap_or_default()),
        )
        .await;
    }
}
