//! Start the dedicated launcher using only host-owned Kilocode profile inputs.
use super::*;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct KilocodeSandbox {
    pub directory: PathBuf,
    pub workspace: PathBuf,
}

impl KilocodeSandbox {
    pub(super) fn command(
        &self,
        executable: &str,
        args: &[String],
        env: &std::collections::HashMap<String, String>,
    ) -> Result<tokio::process::Command, AcpError> {
        if !cfg!(all(target_os = "macos", target_arch = "aarch64")) || args != ["acp", "--pure"] {
            return Err(AcpError::Initialize("Unsupported restricted Kilocode launch".into()));
        }
        crate::verify_reviewed_agent_executable("kilocode", executable)
            .map_err(AcpError::Initialize)?;
        let entry = Path::new(executable).canonicalize().map_err(AcpError::Spawn)?;
        let mut command = tokio::process::Command::new("/usr/bin/python3");
        command
            .current_dir(&self.workspace)
            .arg("-IB")
            .arg(self.directory.join("kilocode-launcher.py"))
            .arg(entry)
            .arg(&self.directory)
            .env_clear()
            .env("HOME", self.directory.join("home"))
            .env("TMPDIR", self.directory.join("home"));
        for name in [
            "KILO_CONFIG_CONTENT",
            "XDG_CONFIG_HOME",
            "XDG_DATA_HOME",
            "XDG_CACHE_HOME",
            "XDG_STATE_HOME",
            "VW_KILO_RELAY_TOKEN",
            "SHIJU_KILO_SKILL_ROOT",
            "SHIJU_KILO_SKILL_SOURCES",
        ] {
            let value = env.get(name).ok_or_else(|| {
                AcpError::Initialize("Missing restricted Kilocode configuration".into())
            })?;
            command.env(name, value);
        }
        for name in [
            "KILO_DISABLE_CLAUDE_CODE",
            "KILO_DISABLE_CLAUDE_CODE_PROMPT",
            "KILO_DISABLE_CLAUDE_CODE_SKILLS",
            "KILO_DISABLE_EXTERNAL_SKILLS",
        ] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        Ok(command)
    }
}
