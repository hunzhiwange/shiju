use std::{path::Path, process::Stdio, time::Duration};
use tokio::{io::{AsyncBufReadExt, AsyncWriteExt, BufReader}, process::{Child, ChildStdin, ChildStdout}};
use super::protocol::Frame;
use crate::server::BackendError;

pub fn failure(_: impl std::fmt::Display) -> BackendError {
    // Do not echo upstream diagnostics, credentials, tool arguments or model payloads.
    BackendError::Internal("aionrs transport or protocol failure; check the installed CLI revision".into())
}
pub struct Process {
    pub child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    buffer: Vec<u8>,
    _home: tempfile::TempDir,
}
impl Process {
    pub async fn spawn(cwd: &Path) -> Result<Self, BackendError> {
        let home = tempfile::tempdir().map_err(failure)?;
        let config = "[tools]\nauto_approve = false\nallow_list = []\n[session]\nenabled = false\n";
        std::fs::write(home.path().join(".aionrs.toml"), config).map_err(failure)?;
        // dirs::config_dir differs on Linux, macOS and Windows. Populate all three.
        for dir in [home.path().join(".config/aionrs"), home.path().join("Library/Application Support/aionrs"), home.path().join("AppData/Roaming/aionrs")] {
            std::fs::create_dir_all(&dir).map_err(failure)?;
            std::fs::write(dir.join("config.toml"), config).map_err(failure)?;
        }
        let mut command = tokio::process::Command::new("aionrs");
        command.env_clear();
        for key in ["PATH", "SystemRoot", "WINDIR", "PATHEXT", "API_KEY", "PROVIDER", "MODEL", "BASE_URL", "ANTHROPIC_API_KEY", "ANTHROPIC_BASE_URL", "OPENAI_API_KEY", "OPENAI_BASE_URL"] {
            if let Some(value) = std::env::var_os(key) { command.env(key, value); }
        }
        command.env("HOME", home.path()).env("USERPROFILE", home.path())
            .env("XDG_CONFIG_HOME", home.path().join(".config"))
            .env("APPDATA", home.path().join("AppData/Roaming"))
            .args(["--json-stream", "--no-color", "--project-dir"])
            .arg(home.path()).current_dir(cwd)
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).kill_on_drop(true);
        #[cfg(unix)]
        command.process_group(0);
        let mut child = command.spawn().map_err(failure)?;
        let input = child.stdin.take().ok_or_else(|| failure("stdin"))?;
        let output = BufReader::new(child.stdout.take().ok_or_else(|| failure("stdout"))?);
        let mut process = Self { child, input, output, buffer: Vec::new(), _home: home };
        let ready = process.read().await?;
        if ready.kind != "ready" || ready.version.as_deref() != Some("0.2.0")
            || !ready.capabilities.is_some_and(|c| c.tool_approval && c.current_mode == "default") {
            return Err(failure("unsupported handshake"));
        }
        Ok(process)
    }
    pub async fn send(&mut self, value: serde_json::Value) -> Result<(), BackendError> {
        let mut bytes = serde_json::to_vec(&value).map_err(failure)?;
        bytes.push(b'\n');
        self.input.write_all(&bytes).await.map_err(failure)?;
        self.input.flush().await.map_err(failure)
    }
    pub async fn read(&mut self) -> Result<Frame, BackendError> {
        tokio::time::timeout(Duration::from_secs(180), async {
            // Bound each NDJSON frame without allowing read_line to allocate without limit.
            loop {
                let available = self.output.fill_buf().await.map_err(failure)?;
                if available.is_empty() { return Err(failure("EOF")); }
                let end = available.iter().position(|b| *b == b'\n');
                let count = end.map_or(available.len(), |n| n + 1);
                if self.buffer.len() + count > 1024 * 1024 { return Err(failure("oversized frame")); }
                self.buffer.extend_from_slice(&available[..count]);
                self.output.consume(count);
                if end.is_some() { let frame = serde_json::from_slice(&self.buffer).map_err(failure); self.buffer.clear(); return frame; }
            }
        }).await.map_err(failure)?
    }
    pub async fn kill(&mut self) {
        self.kill_tree();
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
    }
    fn kill_tree(&mut self) {
        if let Some(id) = self.child.id() {
            #[cfg(unix)]
            // The child leads its own process group; never signal the host group.
            unsafe { libc::kill(-(id as i32), libc::SIGKILL); }
            #[cfg(windows)]
            { let _ = std::process::Command::new("taskkill").args(["/F", "/T", "/PID", &id.to_string()])
                .stdout(Stdio::null()).stderr(Stdio::null()).status(); }
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) { self.kill_tree(); }
}
