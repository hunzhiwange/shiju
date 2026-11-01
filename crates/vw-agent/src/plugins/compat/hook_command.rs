//! Synchronous command control effects; diagnostics never expose raw command output.
use anyhow::{Result, bail};

pub(crate) fn interpret(code: Option<i32>, stdout: &[u8]) -> Result<serde_json::Value> {
    match code {
        Some(2) => bail!("Codex hook blocked execution (exit status 2)"),
        Some(0) => {}
        Some(_) => {
            tracing::warn!("Codex hook returned a nonblocking exit status");
            return Ok(serde_json::Value::Null);
        }
        None => bail!("Codex hook terminated without an exit status"),
    }
    if stdout.iter().all(u8::is_ascii_whitespace) {
        return Ok(serde_json::Value::Null);
    }
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(stdout) else {
        // Plain stdout is permitted by the command protocol.
        return Ok(serde_json::Value::Null);
    };
    if value.get("continue").and_then(serde_json::Value::as_bool) == Some(false) {
        bail!("Codex hook stopped execution");
    }
    if value.get("decision").and_then(serde_json::Value::as_str) == Some("block")
        || value
            .pointer("/hookSpecificOutput/permissionDecision")
            .and_then(serde_json::Value::as_str)
            == Some("deny")
    {
        bail!("Codex hook blocked execution");
    }
    Ok(value)
}

/// Bind only host-owned tool context; no caller-provided payload can replace it.
pub(crate) fn bind_input(
    payload: &mut serde_json::Value,
    event: crate::plugins::PluginHookEvent,
) -> Result<()> {
    use crate::plugins::PluginHookEvent as E;
    use serde_json::json;
    let event_name = match event {
        E::PermissionRequest => "PermissionRequest",
        E::BeforeToolCall => "PreToolUse",
        E::AfterToolCall => "PostToolUse",
        E::BeforeLlmCall => "PreModelRequest",
        _ => bail!("unsupported synchronous Codex event"),
    };
    let object =
        payload.as_object_mut().ok_or_else(|| anyhow::anyhow!("hook input must be an object"))?;
    object.insert("hook_event_name".into(), json!(event_name));
    if let Some(tool) = object.get("tool").cloned() {
        object.insert("tool_name".into(), tool["name"].clone());
        if let Some(input) = tool.get("args") {
            object.insert("tool_input".into(), input.clone());
        }
    }
    {
        let context = crate::tools::current_tool_use_context()
            .ok_or_else(|| anyhow::anyhow!("Codex hook requires active session context"))?;
        anyhow::ensure!(
            !context.session().is_empty()
                && context.turn_id().is_some()
                && context.model().is_some()
                && context.workspace_root().is_some(),
            "Codex hook session context is incomplete"
        );
        if matches!(event, E::BeforeToolCall | E::AfterToolCall) {
            anyhow::ensure!(context.tool_call_id().is_some(), "Codex hook requires a tool call id");
        }
        if event == E::AfterToolCall {
            object.insert(
                "tool_input".into(),
                context.hook_input().cloned().unwrap_or(serde_json::Value::Null),
            );
            if let Some(response) = object.get("result").cloned() {
                object.insert("tool_response".into(), response);
            }
        }
        object.insert("session_id".into(), json!(context.session()));
        object.insert("turn_id".into(), json!(context.turn_id()));
        object.insert("tool_use_id".into(), json!(context.tool_call_id()));
        object.insert("model".into(), json!(context.model()));
        object.insert("cwd".into(), json!(context.workspace_root()));
        object.insert("transcript_path".into(), serde_json::Value::Null);
        object.insert(
            "permission_mode".into(),
            json!(if context.full_access_enabled() { "bypassPermissions" } else { "default" }),
        );
    }
    object.retain(|key, _| {
        matches!(
            key.as_str(),
            "session_id"
                | "turn_id"
                | "transcript_path"
                | "cwd"
                | "hook_event_name"
                | "model"
                | "permission_mode"
                | "tool_name"
                | "tool_input"
                | "tool_response"
                | "tool_use_id"
        )
    });
    if event == E::BeforeLlmCall {
        object.remove("permission_mode");
        object.remove("tool_use_id");
    }
    Ok(())
}

/// Resolve each existing prefix before creating host state; reject symlinks.
pub(crate) fn prepare_data_directory(
    path: &std::path::Path,
    package: &std::path::Path,
) -> Result<std::path::PathBuf> {
    use anyhow::Context;
    let package = package.canonicalize()?;
    anyhow::ensure!(
        path.is_absolute() && !path.starts_with(&package),
        "hook data directory overlaps package"
    );
    for ancestor in path.ancestors() {
        if ancestor.exists() {
            anyhow::ensure!(
                ancestor.canonicalize()? == ancestor,
                "hook data directory contains a symlink"
            );
        }
    }
    std::fs::create_dir_all(path).context("failed to create hook data directory")?;
    let resolved = path.canonicalize()?;
    anyhow::ensure!(
        resolved == path && !package.starts_with(&resolved),
        "hook data directory escapes host state"
    );
    Ok(resolved)
}

/// Keep hook context in the owning tool turn; never share it through handler state.
pub(crate) fn retain_context(
    text: &str,
    limit: Option<usize>,
    data: &std::path::Path,
) -> Result<()> {
    use anyhow::Context;
    use std::io::Write;
    let context = crate::tools::current_tool_use_context()
        .context("hook context requires an active tool turn")?;
    let limit = limit.unwrap_or(2500);
    let visible = if limit == 0 || text.len().div_ceil(4) <= limit {
        text.to_owned()
    } else {
        let directory = data.join("hook-outputs");
        anyhow::ensure!(
            !directory.exists() || directory.canonicalize()? == directory,
            "hook context directory contains a symlink"
        );
        std::fs::create_dir_all(&directory)?;
        let mut file = tempfile::NamedTempFile::new_in(directory)
            .context("failed to create hook context file")?;
        file.write_all(text.as_bytes()).context("failed to preserve hook context")?;
        let (_, path) = file.keep().context("failed to retain hook context file")?;
        let footer = format!("\n\nFull hook output saved to: {}", path.display());
        let budget = limit.saturating_sub(footer.len().div_ceil(4)).saturating_mul(4);
        let left = text.floor_char_boundary(budget / 2 + budget % 2);
        let right = text.ceil_char_boundary(text.len().saturating_sub(budget / 2));
        let removed = text.len().saturating_sub(budget).div_ceil(4);
        format!(
            "Warning: truncated output (original token count: {})\nTotal output lines: {}\n\n{}…{removed} tokens truncated…{}{footer}",
            text.len().div_ceil(4),
            text.lines().count(),
            &text[..left],
            &text[right..],
        )
    };
    context.push_hook_context(visible)
}
