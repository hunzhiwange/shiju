//! Shared Codex state database location resolution on the execution host.
//!
//! This resolves candidates only; callers must authorize each root and use the
//! session storage boundary before opening a file. It never reads ambient HOME
//! or a desktop client's environment and does not probe or create databases.

use std::path::{Component, Path, PathBuf};

use toml_edit::DocumentMut;

pub const CODEX_STATE_DB_FILENAME: &str = "state_5.sqlite";

/// Include CODEX_HOME's database, then the configured SQLite directory's database.
/// A nonempty `sqlite_home` in config takes precedence over CODEX_SQLITE_HOME.
/// `execution_home` and `execution_sqlite_home` must come from the trusted target
/// host, rather than from request payloads. Malformed config is an error, not
/// permission to silently fall back to a different database.
pub fn codex_state_db_paths(
    config_dir: &Path,
    config_text: &str,
    execution_home: &Path,
    execution_sqlite_home: Option<&str>,
) -> Result<Vec<PathBuf>, &'static str> {
    let config_dir = normalized_absolute(config_dir)?;
    let document = config_text
        .parse::<DocumentMut>()
        .map_err(|_| "invalid Codex configuration TOML")?;
    let configured = match document.get("sqlite_home") {
        Some(value) => Some(
            value
                .as_str()
                .ok_or("Codex sqlite_home must be a string")?
                .trim(),
        ),
        None => None,
    }
    .filter(|raw| !raw.is_empty());

    let mut paths = vec![config_dir.join(CODEX_STATE_DB_FILENAME)];
    if let Some(raw) = configured.or_else(|| {
        execution_sqlite_home
            .map(str::trim)
            .filter(|raw| !raw.is_empty())
    }) {
        let sqlite_home = resolve_user_path(raw, execution_home)?;
        let candidate = sqlite_home.join(CODEX_STATE_DB_FILENAME);
        if !paths.contains(&candidate) {
            paths.push(candidate);
        }
    }
    Ok(paths)
}

fn resolve_user_path(raw: &str, execution_home: &Path) -> Result<PathBuf, &'static str> {
    let path = if raw == "~" {
        normalized_absolute(execution_home)?
    } else if let Some(rest) = raw.strip_prefix("~/").or_else(|| raw.strip_prefix("~\\")) {
        normalized_absolute(execution_home)?.join(rest)
    } else {
        PathBuf::from(raw)
    };
    normalized_absolute(&path)
}

fn normalized_absolute(path: &Path) -> Result<PathBuf, &'static str> {
    if !path.is_absolute()
        || path.as_os_str().len() > 4096
        || path.to_string_lossy().chars().any(char::is_control)
    {
        return Err("Codex state directory must be a valid absolute execution-host path");
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                return Err("Codex state directory cannot contain parent traversal");
            }
            Component::CurDir => {}
            other => normalized.push(other.as_os_str()),
        }
    }
    Ok(normalized)
}
