//! Launch the host-owned OpenClaw profile without inheriting user credentials.
use super::*;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct OpenClawSandbox {
    pub directory: PathBuf,
    pub workspace: PathBuf,
    pub node: PathBuf,
}

impl OpenClawSandbox {
    pub(super) fn command(
        &self,
        executable: &str,
        args: &[String],
        env: &std::collections::HashMap<String, String>,
    ) -> Result<tokio::process::Command, AcpError> {
        if !cfg!(all(target_os = "macos", target_arch = "aarch64")) || args != ["acp"] {
            return Err(AcpError::Initialize("Unsupported restricted OpenClaw launch".into()));
        }
        acp_agent_registry::verify_openclaw_restricted_executable(executable)
            .map_err(AcpError::Initialize)?;
        let entry = Path::new(executable).canonicalize().map_err(AcpError::Spawn)?;
        let token = env.get("VW_OPENCLAW_RELAY_TOKEN").ok_or_else(|| {
            AcpError::Initialize("Missing restricted OpenClaw relay binding".into())
        })?;
        let mut command = tokio::process::Command::new("/usr/bin/python3");
        command
            .current_dir(&self.workspace)
            .arg("-IB")
            .arg(self.directory.join("openclaw-launcher.py"))
            .arg(entry)
            .arg(&self.directory)
            .arg(&self.node)
            .env_clear()
            .env("HOME", self.directory.join("native"))
            .env("TMPDIR", self.directory.join("native"))
            .env("VW_OPENCLAW_RELAY_TOKEN", token);
        Ok(command)
    }
}
