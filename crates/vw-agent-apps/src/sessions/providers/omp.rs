//! Oh My Pi history on the authorized host. No ambient desktop environment.
use std::fs;
use std::path::{Component, Path, PathBuf};
use vw_api_types::external_session::*;
use super::pi::{SessionLayout, SessionRootResolution};
use super::pi_tree::{self, Dialect};

const OMP: Dialect = Dialect { label: "Oh My Pi", title_slot: true, model_selector: true };

/// Values come from the selected instance's native path resolver and execution
/// host environment. Retain these values for controlled resume generation;
/// never substitute the terminal process's profile or XDG environment.
pub struct OmpSessionContext<'a> {
    pub agent_dir: &'a Path,
    pub native_agent_dir: &'a Path,
    pub profile: Option<&'a str>,
    pub xdg_data_home: Option<&'a Path>,
    pub env_session_dir: Option<&'a str>,
}

fn validate_path(path: &Path) -> Result<(), String> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir | Component::CurDir)) {
        return Err("OMP session directory must be absolute without traversal".into());
    }
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(meta) if !meta.is_dir() || meta.file_type().is_symlink() => return Err("OMP directory must not traverse symlinks".into()),
            Ok(_) => {},
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(())
}

pub fn session_roots(context: &OmpSessionContext<'_>) -> SessionRootResolution {
    match resolve_root(context) {
        Ok(root) => root,
        Err(reason) => SessionRootResolution::Unavailable { reason },
    }
}

fn resolve_root(context: &OmpSessionContext<'_>) -> Result<SessionRootResolution, String> {
    if let Some(raw) = context.env_session_dir.filter(|s| !s.is_empty()) {
        let root = PathBuf::from(raw);
        if !root.is_absolute() {
            return Ok(SessionRootResolution::RequiresProjectContext { configured_path: raw.to_owned() });
        }
        validate_path(&root)?;
        return Ok(SessionRootResolution::Available { root, layout: SessionLayout::Flat });
    }
    validate_path(context.agent_dir)?;
    validate_path(context.native_agent_dir)?;
    let mut data = context.agent_dir.to_path_buf();
    if context.agent_dir == context.native_agent_dir {
        if let Some(xdg) = context.xdg_data_home {
            validate_path(xdg)?;
            let mut candidate = xdg.join("omp");
            if let Some(profile) = context.profile.filter(|p| !p.is_empty() && *p != "default") {
                if profile.len() > 64 || !profile.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b)) || profile == "." || profile == ".." {
                    return Err("Invalid OMP profile".into());
                }
                candidate = candidate.join("profiles").join(profile);
            }
            validate_path(&candidate)?;
            if candidate.is_dir() { data = candidate; }
        }
    }
    let root = data.join("sessions");
    validate_path(&root)?;
    Ok(SessionRootResolution::Available { root, layout: SessionLayout::ProjectDirectories })
}

/// OMP excludes child artifact trees: only direct files in project buckets (or
/// in an explicit flat override) are interactive sessions.
pub fn session_files(context: &OmpSessionContext<'_>) -> Result<Vec<PathBuf>, String> {
    let (root, layout) = match resolve_root(context)? {
        SessionRootResolution::Available { root, layout } => (root, layout),
        SessionRootResolution::RequiresProjectContext { .. } => return Err("OMP session directory cannot be globally resolved without project context".into()),
        SessionRootResolution::Unavailable { reason } => return Err(reason),
    };
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.to_string()),
    };
    let mut files = Vec::new();
    let mut count = 0usize;
    for entry in entries {
        count += 1;
        if count > 500_000 { return Err("OMP discovery limit exceeded".into()); }
        let entry = entry.map_err(|e| e.to_string())?;
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        match layout {
            SessionLayout::Flat if kind.is_file() => push_file(entry.path(), &mut files),
            SessionLayout::ProjectDirectories if kind.is_dir() => {
                for child in fs::read_dir(entry.path()).map_err(|e| e.to_string())? {
                    count += 1;
                    if count > 500_000 { return Err("OMP discovery limit exceeded".into()); }
                    let child = child.map_err(|e| e.to_string())?;
                    if child.file_type().map_err(|e| e.to_string())?.is_file() { push_file(child.path(), &mut files); }
                }
            }
            _ => {},
        }
    }
    files.sort();
    Ok(files)
}
fn push_file(path: PathBuf, files: &mut Vec<PathBuf>) {
    if path.extension().and_then(|s| s.to_str()) == Some("jsonl") { files.push(path); }
}

pub fn scan_sessions(context: &OmpSessionContext<'_>, identity: &ExternalSessionIdentity) -> Result<Vec<SessionMeta>, String> {
    if identity.provider != SessionProvider::Omp { return Err("Expected an OMP application identity".into()); }
    let files = session_files(context)?;
    let root = match resolve_root(context)? {
        SessionRootResolution::Available { root, .. } => root,
        _ => return Err("OMP session root is unavailable".into()),
    };
    pi_tree::scan_files_for(&OMP, &root, files, identity)
}

pub fn load_messages(context: &OmpSessionContext<'_>, path: &Path) -> Result<Vec<SessionMessage>, String> {
    if !session_files(context)?.iter().any(|p| p == path) { return Err("OMP session is outside the authorized layout".into()); }
    pi_tree::load_messages_for(&OMP, path)
}

pub(crate) use super::pi_tree::verify_omp_session_id;
