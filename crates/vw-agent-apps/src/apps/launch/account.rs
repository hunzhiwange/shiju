//! Official account process material. Credential ownership remains in auth.
use super::request::LaunchMaterial;
use crate::auth::credential_file::AccountCredential;
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
}
impl AccountLaunch {
    pub fn run(mut self) -> Result<(), String> {
        let result =
            self.command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
        self.private.close().map_err(|_| "official account temporary cleanup failed")?;
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
    match credential {
        AccountCredential::ClaudeToken(token) => {
            command
                .env("CLAUDE_CONFIG_DIR", private.path())
                .env("CLAUDE_CODE_OAUTH_TOKEN", token)
                .args(["--settings-sources", ""]);
        }
        AccountCredential::CodexAuth(auth) => {
            write_private(&private.path().join("auth.json"), auth.to_string().as_bytes())?;
            write_private(
                &private.path().join("config.toml"),
                b"model_provider = \"openai\"\ncli_auth_credentials_store = \"file\"\n",
            )?;
            command.env("CODEX_HOME", private.path());
        }
    }
    command.args(material.args);
    Ok(AccountLaunch { command, private })
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
