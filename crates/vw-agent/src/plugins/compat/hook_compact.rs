//! Compact hooks receive metadata only; process failures do not block compaction.
use anyhow::{Result, ensure};
use serde_json::{Value, json};

pub(crate) fn validate(input: &Value) -> Result<()> {
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

pub(crate) fn parse(code: Option<i32>, stdout: &[u8]) -> Result<Value> {
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
