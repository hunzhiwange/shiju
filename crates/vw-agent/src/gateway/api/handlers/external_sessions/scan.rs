//! Adapter dispatch over administrator-bound instance roots, without ambient HOME.
use std::path::Path;
use vw_agent_apps::sessions::providers as p;
use vw_api_types::external_session::*;
pub(super) fn scan(root: &Path, home: &Path, identity: &ExternalSessionIdentity) -> Result<(Vec<SessionMeta>, bool), String> {
    // Reject root symlinks before any adapter reads metadata.
    for ancestor in root.ancestors() {
        match std::fs::symlink_metadata(ancestor) {
            Ok(m) if m.file_type().is_symlink() => return Err("Invalid instance root".into()),
            Ok(_) => {},
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {},
            Err(_) => return Err("Instance unavailable".into()),
        }
    }
    let sessions = match identity.provider {
        SessionProvider::Claude => p::claude::scan_sessions(root, identity).map_err(|e| e.to_string())?,
        SessionProvider::Codex => {
            let config = match std::fs::read_to_string(root.join("config.toml")) {
                Ok(s) => s, Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(), Err(e) => return Err(e.to_string()),
            };
            p::codex::scan_sessions(root, &config, home, None, identity).map_err(|e| e.to_string())?
        }
        SessionProvider::Opencode => p::opencode_sqlite::scan_sessions(root, identity)?,
        SessionProvider::Hermes => {
            let scan = p::hermes_sqlite::scan_sessions(root, identity)?;
            return Ok((scan.sessions, scan.sqlite_truncated));
        }
        SessionProvider::Pi => p::pi::scan_sessions(&p::pi::PiSessionContext { home, configured_agent_dir: Some(root), env_agent_dir: None, env_session_dir: None, project_dir: None }, identity)?,
        SessionProvider::Omp => p::omp::scan_sessions(&p::omp::OmpSessionContext { agent_dir: root, native_agent_dir: root, profile: None, xdg_data_home: None, env_session_dir: None }, identity)?,
        SessionProvider::Gemini => p::gemini::scan_sessions(root, identity)?,
        SessionProvider::Openclaw => p::openclaw::scan_sessions(root, identity).map_err(|e| e.to_string())?,
        SessionProvider::Grokbuild => p::grokbuild::scan_sessions(root, identity)?,
        SessionProvider::Kimi => p::kimi::scan_sessions(root, identity)?,
        SessionProvider::Mcode => p::mcode::scan_sessions(root, identity)?,
        SessionProvider::Dsh => p::dsh::scan_sessions(root, identity)?,
        SessionProvider::Agy => p::agy::scan_sessions(&[root.to_path_buf()], identity)?,
    };
    Ok((sessions, false))
}
