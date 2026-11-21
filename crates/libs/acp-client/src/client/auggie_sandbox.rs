//! Read-only Darwin launch for the reviewed Auggie session-model runtime.
use super::*;
use std::path::PathBuf;

#[derive(Clone)]
pub struct AuggieSandbox {
    pub workspace: PathBuf,
    pub home: PathBuf,
    pub launch: PathBuf,
    pub history: PathBuf,
    pub skill_roots: Vec<PathBuf>,
    pub relay_port: u16,
}

impl AuggieSandbox {
    pub(super) fn command(&self, executable: &str) -> Result<tokio::process::Command, AcpError> {
        acp_agent_registry::verify_auggie_restricted_executable(executable)
            .map_err(AcpError::Initialize)?;
        let entry = PathBuf::from(executable).canonicalize().map_err(AcpError::Spawn)?;
        let node = shell_command::resolve_executable("node")
            .filter(|path| path.is_absolute())
            .ok_or_else(|| AcpError::Initialize("Restricted Auggie requires Node.js".into()))?
            .canonicalize()
            .map_err(AcpError::Spawn)?;
        if node.starts_with(&self.workspace) {
            return Err(AcpError::Initialize(
                "Auggie Node executable overlaps the workspace".into(),
            ));
        }
        let quote = |path: &std::path::Path| {
            if !path.is_absolute() || path.parent().is_none() {
                return Err(AcpError::Initialize("Invalid Auggie sandbox path".into()));
            }
            let text = path
                .to_str()
                .filter(|text| !text.chars().any(char::is_control))
                .ok_or_else(|| AcpError::Initialize("Invalid Auggie sandbox path".into()))?;
            serde_json::to_string(text).map_err(|error| AcpError::Initialize(error.to_string()))
        };
        let mut roots = String::new();
        for root in
            [&self.workspace, &self.home, &self.history].into_iter().chain(&self.skill_roots)
        {
            roots.push_str(&format!(" (subpath {})", quote(root)?));
        }
        let bridge = entry.with_file_name("session-model.mjs");
        let policy = format!(
            r#"(version 1)
(allow default)
(deny file-read-data)
(allow file-read-data (literal "/") (subpath "/System") (subpath "/usr/lib") (subpath "/usr/share"))
(deny file-read-data (subpath "/System/Volumes/Data"))
(allow file-read-data (literal "/dev/null") (literal "/dev/urandom")
    (literal "/private/etc/hosts") (literal "/private/etc/localtime")
    (literal {}) (literal {}) (literal {}) (literal {}) (literal {}) {roots})
(deny file-write*)
(allow file-write* (subpath {}) (subpath {}) (literal "/dev/null"))
(deny process-fork process-exec)
(allow process-exec (literal {}))
(deny network*)
(allow network-outbound (remote ip "localhost:{}"))
"#,
            quote(&node)?,
            quote(&entry)?,
            quote(&bridge)?,
            quote(&self.launch.join("model.json"))?,
            quote(&self.launch.join("session.json"))?,
            quote(&self.home)?,
            quote(&self.history)?,
            quote(&node)?,
            self.relay_port
        );
        let mut command = tokio::process::Command::new("/usr/bin/sandbox-exec");
        command.current_dir(&self.workspace).arg("-p").arg(policy).arg(node).arg(entry);
        Ok(command)
    }
}
