//! Launch only the host-owned restricted OpenCode profile.
use super::*;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct OpenCodeSandbox {
    pub directory: PathBuf,
    pub workspace: PathBuf,
}

impl OpenCodeSandbox {
    pub(super) fn command(
        &self,
        executable: &str,
        args: &[String],
        env: &std::collections::HashMap<String, String>,
    ) -> Result<tokio::process::Command, AcpError> {
        if !cfg!(all(target_os = "macos", target_arch = "aarch64")) || args != ["acp"] {
            return Err(AcpError::Initialize("Unsupported restricted OpenCode launch".into()));
        }
        crate::verify_reviewed_agent_executable("opencode", executable)
            .map_err(AcpError::Initialize)?;
        let entry = Path::new(executable).canonicalize().map_err(AcpError::Spawn)?;
        let mut command = tokio::process::Command::new("/usr/bin/python3");
        command
            .current_dir(&self.workspace)
            .arg("-IB")
            .arg(self.directory.join("opencode-launcher.py"))
            .arg(entry)
            .arg(&self.directory)
            .env_clear()
            .env("HOME", self.directory.join("home"))
            .env("TMPDIR", self.directory.join("home"));
        for name in [
            "OPENCODE_CONFIG_CONTENT",
            "OPENCODE_CONFIG_DIR",
            "XDG_CONFIG_HOME",
            "XDG_DATA_HOME",
            "XDG_CACHE_HOME",
            "XDG_STATE_HOME",
            "VW_OPENCODE_RELAY_TOKEN",
            "SHIJU_OPENCODE_SKILL_ROOT",
            "SHIJU_OPENCODE_SKILL_SOURCES",
        ] {
            let value = env.get(name).ok_or_else(|| {
                AcpError::Initialize("Missing restricted OpenCode configuration".into())
            })?;
            command.env(name, value);
        }
        // Preserve the host's explicit OpenCode skill disables and native permission policy.
        for name in [
            "OPENCODE_DISABLE_EXTERNAL_SKILLS",
            "OPENCODE_DISABLE_CLAUDE_CODE_SKILLS",
            "OPENCODE_DISABLE_CLAUDE_CODE",
            "OPENCODE_PERMISSION",
        ] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        Ok(command)
    }
}
