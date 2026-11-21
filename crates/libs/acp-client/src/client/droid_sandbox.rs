//! Request-owned Darwin sandbox for Factory Droid's reviewed ACP daemon.
use super::*;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct DroidSandbox {
    pub workspace: PathBuf,
    pub home: PathBuf,
    pub skill_roots: Vec<PathBuf>,
    pub relay_port: u16,
}

impl DroidSandbox {
    pub(super) fn command(
        &self,
        executable: &str,
        args: &[String],
    ) -> Result<tokio::process::Command, AcpError> {
        let canonical = Path::new(executable).canonicalize().map_err(AcpError::Spawn)?;
        let settings = self.home.join("settings.json");
        let args_ok = matches!(args, [exec, output, acp, flag, path, model, id, spec, spec_id]
            if exec == "exec" && output == "--output-format" && acp == "acp"
                && flag == "--settings" && Path::new(path).canonicalize().ok().as_ref() == Some(&settings)
                && model == "--model" && id == "custom:Session-Selected-0"
                && spec == "--spec-model" && spec_id == id);
        if !cfg!(all(target_os = "macos", target_arch = "aarch64")) || !args_ok {
            return Err(AcpError::Initialize("Unsupported restricted Droid launch".into()));
        }
        acp_agent_registry::verify_droid_restricted_executable(executable)
            .map_err(AcpError::Initialize)?;
        let quote = |path: &Path| {
            let path = path
                .to_str()
                .filter(|p| {
                    path.is_absolute()
                        && path.parent().is_some()
                        && !p.chars().any(char::is_control)
                })
                .ok_or_else(|| AcpError::Initialize("Invalid Droid sandbox path".into()))?;
            serde_json::to_string(path).map_err(|error| AcpError::Initialize(error.to_string()))
        };
        let mut reads = String::new();
        for root in self.skill_roots.iter().chain(std::iter::once(&self.home)) {
            reads.push_str(&format!(" (subpath {})", quote(root)?));
        }
        let policy = format!(
            r#"(version 1)
(allow default)
(deny file-read-data)
(allow file-read-data (literal "/") (subpath "/System") (subpath "/usr/lib") (subpath "/usr/share"))
(deny file-read-data (subpath "/System/Volumes/Data"))
(allow file-read-data (literal "/dev/null") (literal "/dev/urandom")
    (literal "/private/etc/hosts") (literal "/private/etc/localtime")
    (literal {}) (literal {}) (literal {}) (literal {}) {reads})
(deny file-write*)
(allow file-write* (subpath {}) (literal "/dev/null"))
(deny process-exec)
(allow process-exec (literal {}))
(deny network*)
(allow network-outbound (remote ip "localhost:{}"))
"#,
            quote(&canonical)?,
            quote(&self.workspace)?,
            quote(
                self.home
                    .parent()
                    .ok_or_else(|| AcpError::Initialize("Missing Droid launch parent".into()))?
            )?,
            quote(&self.workspace.join(".factory/settings.json"))?,
            quote(&self.home)?,
            quote(&canonical)?,
            self.relay_port,
        );
        let mut command = tokio::process::Command::new("/usr/bin/sandbox-exec");
        command
            .current_dir(&self.home)
            .args(["-p", &policy])
            .arg(canonical)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", &self.home)
            .env("TMPDIR", &self.home)
            .env("FACTORY_HOME_OVERRIDE", &self.home)
            .env("FACTORY_AIRGAP_ENABLED", "true")
            .env("FACTORY_DISABLE_DYNAMIC_CONFIG", "true")
            .env("FACTORY_DISABLE_KEYRING", "true")
            .env("NO_PROXY", "127.0.0.1,localhost");
        Ok(command)
    }
}
