//! Resolve targets on the authorized execution host, never the desktop client's HOME.
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use vw_api_types::agent_apps::AppId;

use crate::providers::paths;

/// OpenClaw's permanent editor belongs to the workspace domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptOwnership {
    PromptLibrary,
    WorkspaceProjection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptTarget {
    pub app_id: AppId,
    pub file: PathBuf,
    pub ownership: PromptOwnership,
    /// Only Pi has source-proven SYSTEM.md, APPEND_SYSTEM.md and prompts/ support.
    pub pi_native_resources: bool,
}

/// Resolves only a fixed native instruction filename. No content is read or written.
///
/// `home`, overrides and environment must come from authenticated host dispatch.
/// The shared resolver may inspect path metadata. Returned paths are observations,
/// not write authorization: callers must use the native guarded file transaction
/// and recheck symlinks/identity at I/O time, including absent descendants.
pub fn resolve(
    home: &Path,
    app: AppId,
    override_dir: Option<&str>,
    environment: &BTreeMap<String, String>,
) -> Result<PromptTarget, &'static str> {
    use AppId::*;
    if app == ClaudeDesktop {
        return Err("Claude Desktop does not support native prompt files");
    }
    let resolved = paths::read(home, app, override_dir, environment)?;
    let directory = if app == Agy {
        // The provider directory is for CLI settings. Shared customizations live
        // beside it, never in Gemini CLI's GEMINI.md or a private key profile.
        home.join(".gemini/config")
    } else {
        PathBuf::from(resolved.directory)
    };
    let filename = match app {
        Claude => "CLAUDE.md",
        Gemini => "GEMINI.md",
        Hermes => "SOUL.md",
        Codex | GrokBuild | OpenCode | OpenClaw | Pi | Mcode | Dsh | Kimi | Agy | Omp => {
            "AGENTS.md"
        }
        ClaudeDesktop => return Err("Claude Desktop does not support native prompt files"),
    };
    // Also bound the fixed filename, including the Agy customization exception.
    let file = directory.join(filename);
    let file = paths::bounded(home, file.to_str().ok_or("invalid prompt target path")?)?;
    Ok(PromptTarget {
        app_id: app,
        file,
        ownership: if app == OpenClaw {
            PromptOwnership::WorkspaceProjection
        } else {
            PromptOwnership::PromptLibrary
        },
        pi_native_resources: app == Pi,
    })
}
