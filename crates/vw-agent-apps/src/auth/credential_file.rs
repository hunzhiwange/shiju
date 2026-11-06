//! Host-owned credential files; never discovers or adopts ambient CLI logins.
use super::credential_lease::CredentialLease;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use vw_api_types::{agent_apps::AppId, id::TenantScope};

/// Explicit imported material. No Debug/Serialize implementation: never log or return secrets.
/// Only the credential owner can construct this path, from a scoped account record.
pub struct ClaudeAccountDirectory {
    path: PathBuf,
}
impl ClaudeAccountDirectory {
    pub fn path(&self) -> &Path {
        &self.path
    }
}
const DIRECTORY_MARKER: &[u8] = b"shiju-managed-claude-directory-v1\n";

pub enum AccountCredential {
    ClaudeToken(String),
    ClaudeDirectory(ClaudeAccountDirectory),
    CodexAuth(serde_json::Value),
}
impl AccountCredential {
    pub fn validate(&self, app: AppId) -> Result<(), String> {
        match (app, self) {
            (AppId::Claude, Self::ClaudeDirectory(directory)) => {
                check_private(directory.path())?;
                if !directory.path().is_absolute() || !directory.path().is_dir() {
                    return Err("invalid managed Claude directory".into());
                }
                Ok(())
            }
            (AppId::Claude, Self::ClaudeToken(token))
                if !token.is_empty()
                    && token.len() <= 16384
                    && !token.chars().any(char::is_whitespace)
                    && !token.chars().any(char::is_control) =>
            {
                Ok(())
            }
            (AppId::Codex, Self::CodexAuth(auth)) => {
                if auth.to_string().len() > 100000 {
                    return Err("invalid official account material".into());
                }
                let object = auth.as_object().ok_or("invalid official account material")?;
                if object.get("last_refresh").is_some_and(|v| {
                    !v.is_null()
                        && !v
                            .as_str()
                            .is_some_and(|s| s.len() <= 128 && !s.chars().any(char::is_control))
                }) {
                    return Err("invalid official account material".into());
                }
                if object.keys().any(|k| {
                    !matches!(
                        k.as_str(),
                        "auth_mode" | "OPENAI_API_KEY" | "tokens" | "last_refresh"
                    )
                }) || object.get("OPENAI_API_KEY").is_some_and(|v| !v.is_null())
                    || object.get("auth_mode").is_some_and(|v| v != "chatgpt")
                {
                    return Err("invalid official account material".into());
                }
                let tokens = object
                    .get("tokens")
                    .and_then(|v| v.as_object())
                    .ok_or("invalid official account material")?;
                if tokens.keys().any(|k| {
                    !matches!(
                        k.as_str(),
                        "access_token" | "refresh_token" | "id_token" | "account_id"
                    )
                }) || ["access_token", "refresh_token", "id_token", "account_id"].iter().any(
                    |key| {
                        !tokens.get(*key).and_then(|v| v.as_str()).is_some_and(|s| {
                            !s.is_empty()
                                && s.len() <= 16384
                                && !s.chars().any(|c| c.is_whitespace() || c.is_control())
                        })
                    },
                ) {
                    return Err("invalid official account material".into());
                }
                Ok(())
            }
            (AppId::Agy, _) => {
                Err("agy official accounts require an isolated OS keychain execution host".into())
            }
            _ => Err("official account material does not match application".into()),
        }
    }
    pub(super) fn bytes(&self) -> Vec<u8> {
        match self {
            Self::ClaudeToken(s) => s.as_bytes().to_vec(),
            Self::ClaudeDirectory(_) => DIRECTORY_MARKER.to_vec(),
            Self::CodexAuth(v) => v.to_string().into_bytes(),
        }
    }
}

/// Root is supplied by the authorized execution host, never a client path.
/// Scope includes platform, company, host, service and account. No default-account fallback.
pub struct CredentialFileStore {
    root: PathBuf,
}
impl CredentialFileStore {
    pub fn new(root: &Path) -> Result<Self, String> {
        if !root.is_absolute() || !root.is_dir() {
            return Err("official credential root unavailable".into());
        }
        check_private(root)?;
        Ok(Self { root: root.canonicalize().map_err(|_| "official credential root unavailable")? })
    }
    fn location(
        &self,
        tenant: &TenantScope,
        host: &str,
        app: AppId,
        id: &str,
    ) -> Result<PathBuf, String> {
        for value in [host, id] {
            crate::providers::validate_identity(value)
                .map_err(|_| "invalid official account identity")?;
        }
        let service = match app {
            AppId::Claude => "claude",
            AppId::Codex => "codex",
            AppId::Agy => {
                return Err(
                    "agy official accounts require an isolated OS keychain execution host".into()
                );
            }
            _ => return Err("official account service is not implemented".into()),
        };
        let components =
            [tenant.platform_id().as_ref(), tenant.company_id().as_ref(), host, service];
        if components.iter().any(|value| value.len() > 128) {
            return Err("invalid official account scope".into());
        }
        let mut path = self.root.clone();
        for value in components {
            path.push(URL_SAFE_NO_PAD.encode(value));
        }
        Ok(path.join(URL_SAFE_NO_PAD.encode(id)))
    }
    fn path(
        &self,
        tenant: &TenantScope,
        host: &str,
        app: AppId,
        id: &str,
        create: bool,
    ) -> Result<PathBuf, String> {
        let target = self.location(tenant, host, app, id)?;
        let parent = target.parent().ok_or("invalid official credential path")?;
        let relative =
            parent.strip_prefix(&self.root).map_err(|_| "invalid official credential path")?;
        let mut path = self.root.clone();
        for component in relative.components() {
            path.push(component);
            if create && !path.exists() {
                let mut builder = fs::DirBuilder::new();
                #[cfg(unix)]
                {
                    use std::os::unix::fs::DirBuilderExt;
                    builder.mode(0o700);
                }
                match builder.create(&path) {
                    Ok(()) => (),
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
                    Err(_) => return Err("official credential storage unavailable".into()),
                }
            }
            check_private(&path)?;
        }
        Ok(target)
    }
    /// Import only after caller authorization; never infer ownership from host files.
    pub fn save(
        &self,
        tenant: &TenantScope,
        host: &str,
        app: AppId,
        id: &str,
        credential: &AccountCredential,
    ) -> Result<(), String> {
        self.validate_material_scope(tenant, host, app, id, credential)?;
        self.lease(tenant, host, app, id, true)?.save(credential)
    }
    /// Validate identity and a directory's opaque ownership before metadata/filesystem writes.
    pub fn validate_material_scope(
        &self,
        tenant: &TenantScope,
        host: &str,
        app: AppId,
        id: &str,
        credential: &AccountCredential,
    ) -> Result<(), String> {
        credential.validate(app)?;
        let path = self.location(tenant, host, app, id)?;
        if let AccountCredential::ClaudeDirectory(directory) = credential
            && directory.path() != managed_config_path(&path)
        {
            return Err("managed Claude directory scope mismatch".into());
        }
        Ok(())
    }
    /// An OS advisory lock is held through materialization, process exit and rotation writeback.
    /// No stale lock directory: the kernel releases the lock on process termination.
    pub fn lease(
        &self,
        tenant: &TenantScope,
        host: &str,
        app: AppId,
        id: &str,
        create: bool,
    ) -> Result<CredentialLease, String> {
        // Validate unsupported services and identity before creating any files.
        let path = self.path(tenant, host, app, id, create)?;
        CredentialLease::new(path, app)
    }
    /// Allocate a new, owner-scoped Claude directory. Never adopts an arbitrary host path.
    pub fn register_claude_directory(
        &self,
        tenant: &TenantScope,
        host: &str,
        id: &str,
    ) -> Result<(), String> {
        let lease = self.lease(tenant, host, AppId::Claude, id, true)?;
        let directory = managed_config_path(lease.path());
        if directory.exists() {
            return match lease.read()? {
                AccountCredential::ClaudeDirectory(_) => Ok(()),
                _ => Err("managed Claude directory already exists".into()),
            };
        }
        // Refuse replacing a token-based record with a different account representation.
        if lease.path().exists() {
            return Err("official account already exists".into());
        }
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&directory).map_err(|_| "cannot create managed Claude directory")?;
        let credential =
            AccountCredential::ClaudeDirectory(ClaudeAccountDirectory { path: directory.clone() });
        if let Err(error) = lease.save(&credential) {
            let _ = fs::remove_dir(&directory);
            return Err(error);
        }
        Ok(())
    }
    pub fn read(
        &self,
        tenant: &TenantScope,
        host: &str,
        app: AppId,
        id: &str,
    ) -> Result<AccountCredential, String> {
        let path = self.path(tenant, host, app, id, false)?;
        read_credential(&path, app)
    }
}
pub(super) fn managed_config_path(path: &Path) -> PathBuf {
    path.with_extension("claude-config")
}

pub(crate) fn read_credential(path: &Path, app: AppId) -> Result<AccountCredential, String> {
    check_private(path)?;
    let metadata = fs::symlink_metadata(path).map_err(|_| "official account not found")?;
    if !metadata.is_file() || metadata.len() > 100000 {
        return Err("invalid official account material".into());
    }
    let bytes = fs::read(path).map_err(|_| "official credential storage unavailable")?;
    let credential = match app {
        AppId::Claude if bytes == DIRECTORY_MARKER => {
            AccountCredential::ClaudeDirectory(ClaudeAccountDirectory {
                path: managed_config_path(path),
            })
        }
        AppId::Claude => AccountCredential::ClaudeToken(
            String::from_utf8(bytes).map_err(|_| "invalid official account material")?,
        ),
        AppId::Codex => AccountCredential::CodexAuth(
            serde_json::from_slice(&bytes).map_err(|_| "invalid official account material")?,
        ),
        _ => return Err("official account service is not implemented".into()),
    };
    credential.validate(app)?;
    Ok(credential)
}
pub(super) fn write_credential(
    path: &Path,
    app: AppId,
    credential: &AccountCredential,
) -> Result<(), String> {
    credential.validate(app)?;
    if let AccountCredential::ClaudeDirectory(directory) = credential
        && directory.path() != managed_config_path(path)
    {
        return Err("managed Claude directory scope mismatch".into());
    }
    if path.exists() {
        let previous = read_credential(path, app)?;
        match (&previous, credential) {
            (AccountCredential::ClaudeToken(_), AccountCredential::ClaudeToken(_))
            | (AccountCredential::ClaudeDirectory(_), AccountCredential::ClaudeDirectory(_)) => (),
            (AccountCredential::CodexAuth(before), AccountCredential::CodexAuth(after))
                if before["tokens"]["account_id"] == after["tokens"]["account_id"] => {}
            _ => return Err("official account identity or representation changed".into()),
        }
    }
    let mut file =
        tempfile::NamedTempFile::new_in(path.parent().ok_or("invalid official credential path")?)
            .map_err(|_| "official credential storage unavailable")?;
    file.write_all(&credential.bytes())
        .and_then(|_| file.as_file().sync_all())
        .map_err(|_| "official credential storage unavailable")?;
    file.persist(path).map_err(|_| "official credential storage unavailable")?;
    Ok(())
}

pub(super) fn check_private(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|_| "official account not found")?;
    if metadata.file_type().is_symlink() {
        return Err("unsafe official credential path".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err("official credential permissions are not private".into());
        }
    }
    #[cfg(not(unix))]
    {
        return Err("official credential storage requires a reviewed host ACL adapter".into());
    }
    #[allow(unreachable_code)]
    Ok(())
}
