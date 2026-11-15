//! Compact hooks receive metadata only; process failures do not block compaction.
use anyhow::{Result, ensure};
use serde_json::json;

pub(crate) async fn emit(
    session: &str,
    turn: &str,
    model: &str,
    auto: bool,
    post: bool,
) -> Result<()> {
    let Some(context) = crate::tools::current_tool_use_context() else {
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

/// Manual requests use the same trusted execution factory as normal turns.
pub(crate) async fn context(
    session: &str,
    turn: &str,
    model: &str,
) -> Result<std::sync::Arc<crate::tools::ToolUseContext>> {
    if let Some(context) = crate::tools::current_tool_use_context() {
        ensure!(context.session() == session, "compact hook session mismatch");
        return Ok(context);
    }
    let root = crate::project::instance::directory();
    ensure!(std::path::Path::new(&root).is_absolute(), "compaction requires project context");
    let config = crate::config::get_project_for_tenant(
        &crate::session::ui_store::current_tenant_scope(),
        std::path::Path::new(&root),
    )
    .await?;
    let runner = crate::hooks::execution_hook_runner(&config)?;
    let mut context =
        crate::tools::ToolUseContext::new(session, Some(root)).with_turn_id(turn).with_model(model);
    if let Some(runner) = runner {
        context = context.with_hook_runner(runner);
    }
    Ok(std::sync::Arc::new(context))
}

pub use plugin_hooks::hook_compact::{validate, parse};
