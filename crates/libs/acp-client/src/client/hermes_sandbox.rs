//! Read-only Darwin launch for the reviewed Hermes session-model runtime.
use super::*;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct HermesSandbox {
    pub workspace: PathBuf,
    pub home: PathBuf,
    pub history: PathBuf,
    pub skill_roots: Vec<PathBuf>,
    pub relay_port: u16,
}

impl HermesSandbox {
    pub(super) fn command(
        &self,
        executable: &str,
        args: &[String],
    ) -> Result<tokio::process::Command, AcpError> {
        if !cfg!(all(target_os = "macos", target_arch = "aarch64"))
            || args != ["acp", "--session-model-binding"]
        {
            return Err(AcpError::Initialize("Unsupported restricted Hermes launch".into()));
        }
        crate::verify_reviewed_agent_executable("hermes", executable)
            .map_err(AcpError::Initialize)?;
        let entry = Path::new(executable).canonicalize().map_err(AcpError::Spawn)?;
        let python = entry.with_file_name("python");
        let real_python = python.canonicalize().map_err(AcpError::Spawn)?;
        let runtime = entry.parent().and_then(Path::parent);
        let interpreter = real_python.parent().and_then(Path::parent);
        let (Some(runtime), Some(interpreter)) = (runtime, interpreter) else {
            return Err(AcpError::Initialize("Invalid Hermes runtime layout".into()));
        };
        if runtime.parent().is_none()
            || !interpreter.join("lib/python3.12").is_dir()
            || self.workspace.parent().is_none()
            || self.home.parent().is_none()
            || self.history.parent().is_none()
        {
            return Err(AcpError::Initialize("Invalid Hermes sandbox paths".into()));
        }
        let quote = |path: &Path| {
            serde_json::to_string(&path.to_str().ok_or_else(|| {
                AcpError::Initialize("Hermes sandbox requires Unicode paths".into())
            })?)
            .map_err(|error| AcpError::Initialize(error.to_string()))
        };
        let mut reads = String::new();
        for path in [runtime, interpreter, &self.home, &self.history]
            .into_iter()
            .chain(self.skill_roots.iter().map(PathBuf::as_path))
        {
            if !path.is_absolute() || path.parent().is_none() {
                return Err(AcpError::Initialize("Invalid Hermes read scope".into()));
            }
            reads.push_str(&format!(" (subpath {})", quote(path)?));
        }
        let policy = format!(
            r#"(version 1)
(allow default)
(deny file-read-data)
(allow file-read-data (literal "/") (subpath "/System") (subpath "/usr/lib") (subpath "/usr/share"))
(deny file-read-data (subpath "/System/Volumes/Data"))
(allow file-read-data (literal "/dev/null") (literal "/dev/urandom")
    (literal "/private/etc/hosts") (literal "/private/etc/localtime") (literal {}) {reads})
(deny file-write*)
(allow file-write* (subpath {}) (subpath {}) (literal "/dev/null"))
(deny process-fork process-exec)
(allow process-exec (literal {}) (literal {}))
(deny network*)
(allow network-outbound (remote ip "localhost:{}"))
"#,
            quote(&self.workspace)?,
            quote(&self.home)?,
            quote(&self.history)?,
            quote(&python)?,
            quote(&real_python)?,
            self.relay_port
        );
        let mut command = tokio::process::Command::new("/usr/bin/sandbox-exec");
        command
            .current_dir(&self.workspace)
            .arg("-p")
            .arg(policy)
            .arg(python)
            .arg("-IB")
            .arg(entry)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", &self.home)
            .env("TMPDIR", &self.home)
            .env("NO_PROXY", "*")
            .env("no_proxy", "*")
            .env("SHIJU_HERMES_HISTORY", &self.history);
        Ok(command)
    }
}
