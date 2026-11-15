//! Plugin hook protocol implementation.
use anyhow::{Result, bail};

pub fn interpret(code: Option<i32>, stdout: &[u8]) -> Result<serde_json::Value> {
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

pub fn prepare_data_directory(
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
