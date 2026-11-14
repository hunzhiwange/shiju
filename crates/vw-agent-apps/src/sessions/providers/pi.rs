//! Pi session discovery on the authorized execution host.
//!
//! Environment values and project cwd are resolved by the host, never from a
//! request path or the desktop process's ambient environment.
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionLayout {
    Flat,
    ProjectDirectories,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionRootResolution {
    Available { root: PathBuf, layout: SessionLayout },
    RequiresProjectContext { configured_path: String },
    Unavailable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PiSessionDiscovery {
    Available,
    RequiresProjectContext {
        #[serde(rename = "configuredPath")]
        configured_path: String,
    },
    Unavailable { reason: String },
}

/// All paths and environment values must come from the selected execution host.
/// An instance override takes precedence over PI_CODING_AGENT_DIR. A relative
/// sessionDir is usable only with that session's authoritative launch cwd.
pub struct PiSessionContext<'a> {
    pub home: &'a Path,
    pub configured_agent_dir: Option<&'a Path>,
    pub env_agent_dir: Option<&'a str>,
    pub env_session_dir: Option<&'a str>,
    pub project_dir: Option<&'a Path>,
}

pub fn session_discovery(context: &PiSessionContext<'_>) -> PiSessionDiscovery {
    match resolve_session_root(context) {
        SessionRootResolution::Available { .. } => PiSessionDiscovery::Available,
        SessionRootResolution::RequiresProjectContext { configured_path } => {
            PiSessionDiscovery::RequiresProjectContext { configured_path }
        }
        SessionRootResolution::Unavailable { reason } => PiSessionDiscovery::Unavailable { reason },
    }
}

pub fn session_roots(context: &PiSessionContext<'_>) -> SessionRootResolution {
    resolve_session_root(context)
}

pub fn resolve_session_root(context: &PiSessionContext<'_>) -> SessionRootResolution {
    match resolve_root(context) {
        Ok(resolution) => resolution,
        Err(error) => SessionRootResolution::Unavailable { reason: error.to_string() },
    }
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn absolute_path(path: &Path) -> io::Result<PathBuf> {
    if !path.is_absolute() || path.components().any(|part| matches!(part, Component::ParentDir | Component::CurDir)) {
        return Err(invalid("Pi directory must be absolute without traversal"));
    }
    Ok(path.to_path_buf())
}

fn expand_home(value: &str, home: &Path) -> PathBuf {
    if value == "~" {
        home.to_path_buf()
    } else if let Some(suffix) = value.strip_prefix("~/") {
        home.join(suffix)
    } else {
        PathBuf::from(value)
    }
}

// Reject symlinked components before enumerating; a later source-access task
// additionally binds individual reads to the authorized source identity.
fn inspect_directory(path: &Path, allow_missing: bool) -> io::Result<()> {
    let mut current = PathBuf::new();
    for part in path.components() {
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() || !meta.is_dir() => {
                return Err(invalid("Pi directory contains a symlink or non-directory"));
            }
            Ok(_) => {}
            Err(error) if allow_missing && error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        }
    }
    fs::read_dir(path).map(|_| ())
}

fn resolve_root(context: &PiSessionContext<'_>) -> io::Result<SessionRootResolution> {
    absolute_path(context.home)?;
    let agent_dir = match context.configured_agent_dir {
        Some(path) => absolute_path(path)?,
        None => match context.env_agent_dir.filter(|value| !value.is_empty()) {
            Some(value) => absolute_path(&expand_home(value, context.home))?,
            None => context.home.join(".pi/agent"),
        },
    };
    // An explicit session environment override is authoritative even if the
    // native settings file is broken, matching Pi's precedence.
    let configured = match context.env_session_dir.filter(|value| !value.is_empty()) {
        Some(value) => Some(value.to_owned()),
        None => {
            inspect_directory(&agent_dir, true)?;
            let settings = agent_dir.join("settings.json");
            match fs::symlink_metadata(&settings) {
                Ok(meta) if !meta.is_file() || meta.file_type().is_symlink() => {
                    return Err(invalid("Pi settings must be a regular file"));
                }
                Ok(meta) => {
                    if meta.len() > 1024 * 1024 {
                        return Err(invalid("Pi settings exceed discovery size limit"));
                    }
                    let value: Value = serde_json::from_slice(&fs::read(settings)?)
                        .map_err(|_| invalid("Pi settings are invalid JSON"))?;
                    let object = value.as_object().ok_or_else(|| invalid("Pi settings must be an object"))?;
                    match object.get("sessionDir") {
                        None | Some(Value::Null) => None,
                        Some(Value::String(value)) if value.is_empty() => None,
                        Some(Value::String(value)) => Some(value.clone()),
                        Some(_) => return Err(invalid("Pi sessionDir must be a string")),
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                Err(error) => return Err(error),
            }
        }
    };
    if let Some(value) = configured {
        let expanded = expand_home(&value, context.home);
        let root = if expanded.is_absolute() {
            absolute_path(&expanded)?
        } else {
            match context.project_dir {
                Some(project) => absolute_path(&absolute_path(project)?.join(expanded))?,
                None => return Ok(SessionRootResolution::RequiresProjectContext { configured_path: value }),
            }
        };
        inspect_directory(&root, false)?;
        Ok(SessionRootResolution::Available { root, layout: SessionLayout::Flat })
    } else {
        let root = agent_dir.join("sessions");
        inspect_directory(&root, true)?;
        Ok(SessionRootResolution::Available { root, layout: SessionLayout::ProjectDirectories })
    }
}

/// Enumerate all project buckets for the default layout. Explicit sessionDir
/// roots are flat; only per-session fork/subagent trees are walked recursively.
/// Metadata parsing and active branch selection belong to the Pi parser.
pub fn session_files(context: &PiSessionContext<'_>) -> io::Result<Vec<PathBuf>> {
    let (root, layout) = match resolve_root(context)? {
        SessionRootResolution::Available { root, layout } => (root, layout),
        SessionRootResolution::RequiresProjectContext { .. } => {
            return Err(invalid("Pi sessionDir requires an authoritative project cwd"));
        }
        SessionRootResolution::Unavailable { .. } => return Err(invalid("Pi session discovery unavailable")),
    };
    let mut files = Vec::new();
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound && layout == SessionLayout::ProjectDirectories => return Ok(files),
        Err(error) => return Err(error),
    };
    for entry in entries {
        let entry = entry?;
        let kind = entry.file_type()?;
        match layout {
            SessionLayout::Flat if kind.is_file() => push_jsonl(entry.path(), &mut files)?,
            SessionLayout::ProjectDirectories if kind.is_dir() => {
                for session in fs::read_dir(entry.path())? {
                    let session = session?;
                    if session.file_type()?.is_file() { push_jsonl(session.path(), &mut files)?; }
                }
            }
            _ => {}
        }
    }
    let parents = files.clone();
    for path in parents { collect_children(&path.with_extension(""), &mut files)?; }
    files.sort();
    files.dedup();
    Ok(files)
}

fn push_jsonl(path: PathBuf, files: &mut Vec<PathBuf>) -> io::Result<()> {
    if path.extension().and_then(|value| value.to_str()) == Some("jsonl") {
        if files.len() >= 500_000 { return Err(invalid("Pi session discovery limit exceeded")); }
        files.push(path);
    }
    Ok(())
}

fn collect_children(root: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    let mut pending = vec![(root.to_path_buf(), 0usize)];
    while let Some((path, depth)) = pending.pop() {
        match fs::symlink_metadata(&path) {
            Ok(meta) if !meta.is_dir() || meta.file_type().is_symlink() => continue,
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        }
        if depth > 128 { return Err(invalid("Pi session tree depth exceeded")); }
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_dir() && entry.file_name() != "subagent-artifacts" {
                pending.push((entry.path(), depth + 1));
            } else if kind.is_file() { push_jsonl(entry.path(), files)?; }
        }
    }
    Ok(())
}

// Tree parsing is separate from host-context discovery.
pub use super::pi_tree::{load_messages, scan_sessions};

pub(crate) use super::pi_tree::verify_pi_session_id;
