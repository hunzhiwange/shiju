use super::protocol::Frame;
use crate::server::BackendError;
use std::{path::Path, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout},
};

pub fn failure(_: impl std::fmt::Display) -> BackendError {
    // Do not echo upstream diagnostics, credentials, tool arguments or model payloads.
    BackendError::Internal(
        "aionrs transport or protocol failure; check the installed CLI revision".into(),
    )
}
pub(super) fn reviewed_ready(frame: Frame) -> bool {
    frame.kind == "ready"
        && frame.version.as_deref() == Some("0.2.11")
        && frame.capabilities.is_some_and(|c| c.tool_approval && c.current_mode == "default")
}

pub struct Process {
    #[cfg(unix)]
    pub mcp: Option<super::mcp::Broker>,
    pub child: Child,
    pub workspace: std::path::PathBuf,
    pub managed_skills: bool,
    pub skill_root: Option<std::path::PathBuf>,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    buffer: Vec<u8>,
    _home: tempfile::TempDir,
}
impl Process {
    pub async fn spawn(cwd: &Path) -> Result<Self, BackendError> {
        let managed = std::env::var_os("SHIJU_AIONRS_SANDBOX");
        let home = if managed.is_some() {
            let workspace = std::env::var_os("SHIJU_AIONRS_WORKSPACE")
                .ok_or_else(|| failure("missing workspace"))?;
            if cwd != Path::new(&workspace) {
                return Err(failure("wrong workspace"));
            }
            let state =
                std::env::var_os("SHIJU_AIONRS_STATE").ok_or_else(|| failure("missing state"))?;
            tempfile::tempdir_in(state)
        } else {
            tempfile::tempdir()
        }
        .map_err(failure)?;
        let config = "[tools]\nauto_approve = false\nallow_list = []\n[session]\nenabled = false\n";
        std::fs::write(home.path().join(".aionrs.toml"), config).map_err(failure)?;
        // dirs::config_dir differs on Linux, macOS and Windows. Populate all three.
        for dir in [
            home.path().join(".config/aionrs"),
            home.path().join("Library/Application Support/aionrs"),
            home.path().join("AppData/Roaming/aionrs"),
        ] {
            std::fs::create_dir_all(&dir).map_err(failure)?;
            std::fs::write(dir.join("config.toml"), config).map_err(failure)?;
        }
        let sibling = std::env::current_exe().map_err(failure)?.with_file_name("aionrs");
        let skill_root = managed.as_ref().and_then(|_| {
            std::env::var_os("SHIJU_AIONRS_SKILL_ROOT").map(std::path::PathBuf::from)
        });
        let mut command = if let Some(policy) = managed {
            if !sibling.is_file() || !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
                return Err(failure("missing managed engine"));
            }
            let mut command = tokio::process::Command::new("/usr/bin/sandbox-exec");
            command.args([std::ffi::OsStr::new("-f"), &policy]).arg(sibling);
            command.env_clear();
            for key in ["API_KEY", "PROVIDER", "MODEL", "BASE_URL"] {
                command.env(
                    key,
                    std::env::var_os(key).ok_or_else(|| failure("missing relay binding"))?,
                );
            }
            if let Some(root) = &skill_root {
                command.env("SHIJU_AIONRS_READ_ONLY_SKILLS", "1")
                    .env("SHIJU_AIONRS_SKILL_ROOT", root);
            }
            command.env("PATH", "/usr/bin:/bin").env("NO_PROXY", "127.0.0.1,localhost");
            command
        } else {
            let mut command = tokio::process::Command::new(if sibling.is_file() {
                sibling
            } else {
                "aionrs".into()
            });
            command.env_clear();
            for key in [
                "PATH",
                "SystemRoot",
                "WINDIR",
                "PATHEXT",
                "API_KEY",
                "PROVIDER",
                "MODEL",
                "BASE_URL",
                "ANTHROPIC_API_KEY",
                "ANTHROPIC_BASE_URL",
                "OPENAI_API_KEY",
                "OPENAI_BASE_URL",
            ] {
                if let Some(value) = std::env::var_os(key) {
                    command.env(key, value);
                }
            }
            command
        };
        command
            .env("HOME", home.path())
            .env("USERPROFILE", home.path())
            .env("CFFIXED_USER_HOME", home.path())
            .env("TMPDIR", home.path())
            .env("XDG_CONFIG_HOME", home.path().join(".config"))
            .env("APPDATA", home.path().join("AppData/Roaming"))
            .args(["--json-stream", "--no-color", "--project-dir"])
            .arg(home.path())
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        #[cfg(unix)]
        command.process_group(0);
        let mut child = command.spawn().map_err(failure)?;
        let input = child.stdin.take().ok_or_else(|| failure("stdin"))?;
        let output = BufReader::new(child.stdout.take().ok_or_else(|| failure("stdout"))?);
        let mut process = Self {
            #[cfg(unix)]
            mcp: None,
            child,
            workspace: cwd.to_path_buf(),
            managed_skills: skill_root.is_some(),
            skill_root: skill_root.map(|root| root.join(".aionrs/skills")).filter(|root| root.is_dir()),
            input,
            output,
            buffer: Vec::new(),
            _home: home,
        };
        let ready = process.read().await?;
        if !reviewed_ready(ready) {
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
                if available.is_empty() {
                    return Err(failure("EOF"));
                }
                let end = available.iter().position(|b| *b == b'\n');
                let count = end.map_or(available.len(), |n| n + 1);
                if self.buffer.len() + count > 1024 * 1024 {
                    return Err(failure("oversized frame"));
                }
                self.buffer.extend_from_slice(&available[..count]);
                self.output.consume(count);
                if end.is_some() {
                    let frame = serde_json::from_slice(&self.buffer).map_err(failure);
                    self.buffer.clear();
                    return frame;
                }
            }
        })
        .await
        .map_err(failure)?
    }
    #[cfg(unix)]
    pub async fn attach_mcp(
        &mut self,
        servers: Vec<agent_client_protocol::McpServer>,
    ) -> Result<(), BackendError> {
        let socket = std::env::var_os("SHIJU_AIONRS_MCP")
            .ok_or_else(|| BackendError::InvalidRequest("aionrs MCP requires the managed model relay".into()))?;
        let endpoint = std::env::var("BASE_URL").map_err(failure)?;
        let token = std::env::var("API_KEY").map_err(failure)?;
        let endpoint = endpoint.strip_suffix("/v1").ok_or_else(|| failure("invalid managed MCP relay"))?;
        let broker_token = std::env::var("SHIJU_AIONRS_MCP_TOKEN").map_err(failure)?;
        let count = servers.len();
        self.mcp = Some(super::mcp::Broker::start(socket.into(), broker_token, &self.workspace, servers)?);
        for mount in 0..count {
            let name = format!("mount-{mount}");
            self.send(serde_json::json!({
                "type": "add_mcp_server",
                "name": name,
                "transport": "streamable-http",
                "url": format!("{endpoint}/shiju/aionrs/mcp/{mount}"),
                "headers": { "Authorization": format!("Bearer {token}") },
            })).await?;
            let ready = tokio::time::timeout(Duration::from_secs(20), self.read())
                .await.map_err(failure)??;
            if ready.kind != "mcp_ready" || ready.name.as_deref() != Some(name.as_str())
                || ready.tools.as_ref().is_none_or(|tools| tools.is_empty())
            {
                return Err(failure("MCP discovery failed"));
            }
        }
        Ok(())
    }

    pub async fn kill(&mut self) {
        #[cfg(unix)]
        drop(self.mcp.take());
        self.kill_tree();
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
    }
    fn kill_tree(&mut self) {
        if let Some(id) = self.child.id() {
            #[cfg(unix)]
            // The child leads its own process group; never signal the host group.
            unsafe {
                libc::kill(-(id as i32), libc::SIGKILL);
            }
            #[cfg(windows)]
            {
                let _ = std::process::Command::new("taskkill")
                    .args(["/F", "/T", "/PID", &id.to_string()])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        self.kill_tree();
    }
}
