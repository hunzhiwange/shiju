//! Official account process material. Credential ownership remains in auth.
use super::request::LaunchMaterial;
use crate::auth::{credential_file::AccountCredential, credential_lease::CredentialLease};
use serde_json::Value;
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};
use vw_api_types::agent_apps::AppId;

/// Not Debug/serializable. Private material lives until the child has been reaped.
pub struct AccountLaunch {
    command: Command,
    private: tempfile::TempDir,
    codex_auth: Option<Value>,
}
impl AccountLaunch {
    /// Run with the scoped lease retained by the authorized account owner.
    pub fn run_with_lease_using(
        self,
        lease: &CredentialLease,
        execute: impl FnOnce(&mut Command) -> std::io::Result<std::process::ExitStatus>,
    ) -> Result<(), String> {
        let _refresh = self.codex_auth.as_ref().map(|_| lease.lock_refresh()).transpose()?;
        self.run_with_rotation_using(
            |expected, next| {
                lease.save_codex_generation(expected, &AccountCredential::CodexAuth(next))
            },
            execute,
        )
    }

    /// Process-only convenience for independent process-boundary tests.
    #[cfg(test)]
    pub(super) fn run(self) -> Result<(), String> {
        self.run_with_refresh(|_| Ok(()))
    }
    /// Return validated rotated credentials to their owner before cleaning temporary resources,
    /// even when the process exits unsuccessfully. Never return the child's output.
    pub fn run_with_refresh(
        self,
        persist: impl FnOnce(AccountCredential) -> Result<(), String>,
    ) -> Result<(), String> {
        self.run_with_refresh_using(persist, |command| {
            command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status()
        })
    }
    /// The transport must reap the real CLI before returning, including on disconnect.
    pub fn run_with_refresh_using(
        self,
        persist: impl FnOnce(AccountCredential) -> Result<(), String>,
        execute: impl FnOnce(&mut Command) -> std::io::Result<std::process::ExitStatus>,
    ) -> Result<(), String> {
        self.run_with_rotation_using(|_, next| persist(AccountCredential::CodexAuth(next)), execute)
    }

    fn run_with_rotation_using(
        mut self,
        persist: impl FnOnce(&Value, Value) -> Result<(), String>,
        execute: impl FnOnce(&mut Command) -> std::io::Result<std::process::ExitStatus>,
    ) -> Result<(), String> {
        let result = execute(&mut self.command);
        let rotation = match (&result, &self.codex_auth) {
            (Ok(_), Some(expected)) => crate::auth::credential_file::read_credential(
                &self.private.path().join("auth.json"),
                AppId::Codex,
            )
            .and_then(|credential| {
                let AccountCredential::CodexAuth(auth) = credential else {
                    return Err("invalid rotated official account material".into());
                };
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if crate::auth::codex_refresh::should_adopt_cli_rotation(expected, &auth)? {
                        return persist(expected, auth);
                    }
                    Ok(())
                }
                #[cfg(target_arch = "wasm32")]
                {
                    let _ = (expected, auth, persist);
                    Err("official account launch requires a reviewed host ACL adapter".into())
                }
            }),
            _ => Ok(()),
        };
        let cleanup = self.private.close().map_err(|_| "official account temporary cleanup failed");
        cleanup?;
        rotation?;
        match result {
            Ok(status) if status.success() => Ok(()),
            Ok(_) => Err("official account process exited unsuccessfully".into()),
            Err(_) => Err("official account process could not be started".into()),
        }
    }
}

/// Call only with execution-host validated material and auth-owner scoped credentials.
/// Never falls back to default HOME credentials or switches the machine keychain.
pub fn prepare_account_launch(
    app: AppId,
    credential: AccountCredential,
    material: LaunchMaterial,
    temp: &Path,
) -> Result<AccountLaunch, String> {
    prepare_account_launch_with_model(app, credential, material, temp, None)
}

/// An executor-validated official provider may select a model in the private process.
pub fn prepare_account_launch_with_model(
    app: AppId,
    credential: AccountCredential,
    material: LaunchMaterial,
    temp: &Path,
    model: Option<&str>,
) -> Result<AccountLaunch, String> {
    credential.validate(app)?;
    if !material.program.is_absolute()
        || !material.program.is_file()
        || !material.cwd.is_absolute()
        || !material.cwd.is_dir()
        || !temp.is_absolute()
        || !temp.is_dir()
        || material.program.to_string_lossy().chars().any(char::is_control)
        || material.cwd.to_string_lossy().chars().any(char::is_control)
    {
        return Err("official account launch requires validated host paths".into());
    }
    if material.skip_permissions {
        return Err("official account permission skipping is not implemented".into());
    }
    if material
        .program
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("cmd") || s.eq_ignore_ascii_case("bat"))
        || material.args.len() > 32
        || material.args.iter().any(|arg| {
            arg.starts_with('-')
                || arg.len() > 8192
                || arg.chars().any(|c| {
                    c.is_control()
                        || matches!(c, ';' | '&' | '|' | '<' | '>' | '$' | '`' | '%' | '!')
                })
        })
    {
        return Err("unsafe official account launch input".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if material
            .program
            .metadata()
            .map_err(|_| "official account program unavailable")?
            .permissions()
            .mode()
            & 0o111
            == 0
        {
            return Err("official account program is not executable".into());
        }
    }
    if app == AppId::Codex {
        for ancestor in material.cwd.ancestors() {
            if ancestor.join(".codex/config.toml").exists() {
                return Err("project configuration conflicts with official account launch".into());
            }
            if ancestor.join(".git").exists() {
                break;
            }
        }
    }
    let mut builder = tempfile::Builder::new();
    builder.prefix("shiju-account-");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(fs::Permissions::from_mode(0o700));
    }
    #[cfg(not(unix))]
    {
        return Err("official account launch requires a reviewed host ACL adapter".into());
    }
    let private =
        builder.tempdir_in(temp).map_err(|_| "cannot create private official account directory")?;
    let mut command = Command::new(material.program);
    command.current_dir(material.cwd).env_clear();
    for name in [
        "PATH",
        "HOME",
        "USERPROFILE",
        "SystemRoot",
        "WINDIR",
        "TEMP",
        "TMP",
        "LANG",
        "LC_ALL",
        "LC_CTYPE",
        "TERM",
        "COLORTERM",
    ] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    let mut codex_auth = None;
    match credential {
        AccountCredential::CopilotToken(_) => {
            return Err("Copilot authorization has no launch application".into());
        }
        AccountCredential::XaiOauth(_) => {
            return Err("xAI authorization has no launch application".into());
        }
        AccountCredential::KimiOauth(_) => {
            return Err("Kimi authorization requires a reviewed native account adapter".into());
        }
        AccountCredential::ClaudeToken(token) => {
            command
                .env("CLAUDE_CONFIG_DIR", private.path())
                .env("CLAUDE_CODE_OAUTH_TOKEN", token)
                .args(["--settings-sources", ""]);
        }
        AccountCredential::ClaudeDirectory(directory) => {
            command.env("CLAUDE_CONFIG_DIR", directory.path()).args(["--settings-sources", ""]);
        }
        AccountCredential::CodexAuth(auth) => {
            write_private(&private.path().join("auth.json"), auth.to_string().as_bytes())?;
            codex_auth = Some(auth);
            write_private(
                &private.path().join("config.toml"),
                b"model_provider = \"openai\"\ncli_auth_credentials_store = \"file\"\n",
            )?;
            command.env("CODEX_HOME", private.path());
        }
    }
    if let Some(model) = model {
        if model.is_empty()
            || model.len() > 256
            || model.starts_with('-')
            || model.chars().any(char::is_control)
        {
            return Err("invalid official provider model".into());
        }
        command.args(["--model", model]);
    }
    command.args(material.args);
    Ok(AccountLaunch { command, private, codex_auth })
}
fn write_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
        .open(path)
        .and_then(|mut f| f.write_all(bytes))
        .map_err(|_| "cannot materialize official account credential".into())
}
