//! Host-owned credential files; never discovers or adopts ambient CLI logins.
use super::{credential_lease::CredentialLease, flow::AuthAccountScope};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use vw_api_types::{
    TimestampMs,
    agent_apps::AppId,
    agent_apps_auth::{AuthRegion, AuthService},
    id::TenantScope,
};

#[cfg(unix)]
mod host;
#[cfg(unix)]
use host::open_root;
#[cfg(target_os = "macos")]
pub(super) use host::remove_account_directory;
#[cfg(unix)]
pub(super) use host::{
    PrivateDirectory, ScopedCredentialPath, check_private, child_name, credential_bytes,
    decode_credential, read_from, register_claude_directory, remove_credential_file, same_file,
    write_to,
};
#[cfg(unix)]
use std::{path::Component, sync::Arc};

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
#[cfg(unix)]
const DIRECTORY_MARKER: &[u8] = b"shiju-managed-claude-directory-v1\n";

/// Host-private xAI token material. Store only under the authorized xAI account lease.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct XaiCredential {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: TimestampMs,
    #[serde(default)]
    pub requires_reauth: bool,
}

/// Host-private Kimi tokens and the device identity used to obtain them.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KimiCredential {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: TimestampMs,
    pub region: AuthRegion,
    pub device_id: String,
}

pub enum AccountCredential {
    ClaudeToken(String),
    ClaudeDirectory(ClaudeAccountDirectory),
    CodexAuth(serde_json::Value),
    CopilotToken(String),
    XaiOauth(XaiCredential),
    KimiOauth(KimiCredential),
}

#[cfg(unix)]
#[derive(Clone, Copy)]
pub(super) enum CredentialKind {
    App(AppId),
    Copilot,
    Xai,
    Kimi,
}
impl AccountCredential {
    pub fn validate(&self, app: AppId) -> Result<(), String> {
        match (app, self) {
            (AppId::Claude, Self::ClaudeDirectory(directory)) => {
                #[cfg(unix)]
                {
                    PrivateDirectory::open(directory.path()).map(|_| ())
                }
                #[cfg(not(unix))]
                {
                    let _ = directory;
                    Err(UNSUPPORTED.into())
                }
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
                if auth.to_string().len() > MAX_BYTES {
                    return Err(INVALID_MATERIAL.into());
                }
                let object = auth.as_object().ok_or(INVALID_MATERIAL)?;
                if object.get("last_refresh").is_some_and(|v| {
                    !v.is_null()
                        && !v
                            .as_str()
                            .is_some_and(|s| s.len() <= 128 && !s.chars().any(char::is_control))
                }) {
                    return Err(INVALID_MATERIAL.into());
                }
                if object.keys().any(|k| {
                    !matches!(
                        k.as_str(),
                        "auth_mode" | "OPENAI_API_KEY" | "tokens" | "last_refresh"
                    )
                }) || object.get("OPENAI_API_KEY").is_some_and(|v| !v.is_null())
                    || object.get("auth_mode").is_some_and(|v| v != "chatgpt")
                {
                    return Err(INVALID_MATERIAL.into());
                }
                let tokens =
                    object.get("tokens").and_then(|v| v.as_object()).ok_or(INVALID_MATERIAL)?;
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
                    return Err(INVALID_MATERIAL.into());
                }
                Ok(())
            }
            (AppId::Agy, _) => {
                Err("agy official accounts require an isolated OS keychain execution host".into())
            }
            _ => Err("official account material does not match application".into()),
        }
    }
    #[cfg(unix)]
    fn bytes(&self) -> Result<Vec<u8>, String> {
        Ok(match self {
            Self::ClaudeToken(s) | Self::CopilotToken(s) => s.as_bytes().to_vec(),
            Self::ClaudeDirectory(_) => DIRECTORY_MARKER.to_vec(),
            Self::CodexAuth(v) => v.to_string().into_bytes(),
            Self::XaiOauth(tokens) => serde_json::to_vec(tokens)
                .ok()
                .filter(|bytes| bytes.len() <= MAX_BYTES)
                .ok_or(INVALID_MATERIAL)?,
            Self::KimiOauth(tokens) => serde_json::to_vec(tokens)
                .ok()
                .filter(|bytes| bytes.len() <= MAX_BYTES)
                .ok_or(INVALID_MATERIAL)?,
        })
    }
}

#[cfg(not(unix))]
const UNSUPPORTED: &str = "official credential storage requires a reviewed host ACL adapter";
const INVALID_MATERIAL: &str = "invalid official account material";
const MAX_BYTES: usize = 100_000;

/// Root is supplied by the authorized execution host, never a client path.
pub struct CredentialFileStore {
    root: PathBuf,
    #[cfg(unix)]
    anchor: Arc<PrivateDirectory>,
    owner: Option<String>,
}

impl CredentialFileStore {
    #[cfg(unix)]
    pub fn new(root: &Path) -> Result<Self, String> {
        if !root.is_absolute() || root.components().any(|part| part == Component::ParentDir) {
            return Err("unsafe official credential path".into());
        }
        // Resolve trusted system aliases above the root, e.g. macOS /var.
        // Leave the final component unresolved so the no-follow open checks it.
        let root: PathBuf = root.components().collect();
        let parent = root.parent().ok_or("unsafe official credential path")?;
        let name = root.file_name().ok_or("unsafe official credential path")?;
        let root = parent
            .canonicalize()
            .map_err(|_| "official credential storage unavailable")?
            .join(name);
        let anchor = Arc::new(PrivateDirectory::open(&root)?);
        let store = Self { root, anchor, owner: None };
        open_root(&store.root, &store.anchor)?;
        Ok(store)
    }

    #[cfg(not(unix))]
    pub fn new(_root: &Path) -> Result<Self, String> {
        Err(UNSUPPORTED.into())
    }

    /// Select a managed scope after the caller verifies the principal's authority.
    /// Launch-only credentials live separately and are never consulted by this view.
    pub fn for_owner(&self, principal: &str) -> Result<Self, String> {
        if principal.trim().is_empty() || principal.chars().any(char::is_control) {
            return Err("invalid official account owner".into());
        }
        Ok(Self {
            root: self.root.clone(),
            #[cfg(unix)]
            anchor: self.anchor.clone(),
            owner: Some(scope_component(principal)),
        })
    }

    fn location(
        &self,
        tenant: &TenantScope,
        host: &str,
        app: AppId,
        id: &str,
    ) -> Result<PathBuf, String> {
        validate_account_location(host, id)?;
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
        self.service_location(tenant, host, service, id)
    }

    fn service_location(
        &self,
        tenant: &TenantScope,
        host: &str,
        service: &str,
        id: &str,
    ) -> Result<PathBuf, String> {
        let [platform, company] = [tenant.platform_id().as_ref(), tenant.company_id().as_ref()];
        if [platform, company].iter().any(|value| value.len() > 128) {
            return Err("invalid official account scope".into());
        }
        let mut path = self.root.clone();
        if let Some(owner) = &self.owner {
            path.push(".managed-v1");
            path.extend([scope_component(platform), scope_component(company)]);
            path.push(owner);
            path.extend([scope_component(host), service.to_owned(), scope_component(id)]);
        } else {
            path.extend(
                [platform, company, host, service, id].map(|value| URL_SAFE_NO_PAD.encode(value)),
            );
        }
        Ok(path)
    }

    #[cfg(unix)]
    fn scoped_path(&self, path: PathBuf) -> ScopedCredentialPath {
        ScopedCredentialPath::new(self.root.clone(), self.anchor.clone(), path)
    }

    /// Import only after caller authorization. Never infer ownership from host files.
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

    /// Hold an OS advisory lock through materialization, process exit and rotation writeback.
    pub fn lease(
        &self,
        tenant: &TenantScope,
        host: &str,
        app: AppId,
        id: &str,
        create: bool,
    ) -> Result<CredentialLease, String> {
        let path = self.location(tenant, host, app, id)?;
        #[cfg(unix)]
        {
            let lease =
                CredentialLease::new(self.scoped_path(path), CredentialKind::App(app), create)?;
            #[cfg(target_os = "macos")]
            let lease = if app == AppId::Claude && self.owner.is_some() {
                lease.with_claude_profile_scope(tenant, host)
            } else {
                lease
            };
            Ok(lease)
        }
        #[cfg(not(unix))]
        {
            let _ = (path, create);
            Err(UNSUPPORTED.into())
        }
    }

    /// Copilot has a managed authorization service without a launch application.
    pub fn lease_copilot(
        &self,
        scope: AuthAccountScope<'_>,
        id: &str,
        create: bool,
    ) -> Result<CredentialLease, String> {
        if scope.service != AuthService::GithubCopilot {
            return Err("official credential service mismatch".into());
        }
        validate_account_location(scope.host, id)?;
        let owner = self.for_owner(scope.principal)?;
        let path = owner.service_location(scope.tenant, scope.host, "github_copilot", id)?;
        #[cfg(unix)]
        {
            CredentialLease::new(owner.scoped_path(path), CredentialKind::Copilot, create)
        }
        #[cfg(not(unix))]
        {
            let _ = (path, create);
            Err(UNSUPPORTED.into())
        }
    }

    /// Lease xAI OAuth credentials in the authenticated execution-host scope.
    pub fn lease_xai(
        &self,
        scope: AuthAccountScope<'_>,
        id: &str,
        create: bool,
    ) -> Result<CredentialLease, String> {
        if scope.service != AuthService::XaiOauth {
            return Err("official credential service mismatch".into());
        }
        validate_account_location(scope.host, id)?;
        let owner = self.for_owner(scope.principal)?;
        let path = owner.service_location(scope.tenant, scope.host, "xai_oauth", id)?;
        #[cfg(unix)]
        {
            CredentialLease::new(owner.scoped_path(path), CredentialKind::Xai, create)
        }
        #[cfg(not(unix))]
        {
            let _ = (path, create);
            Err(UNSUPPORTED.into())
        }
    }

    /// Lease Kimi OAuth credentials in the authenticated execution-host scope.
    pub fn lease_kimi(
        &self,
        scope: AuthAccountScope<'_>,
        id: &str,
        create: bool,
    ) -> Result<CredentialLease, String> {
        if scope.service != AuthService::KimiOauth {
            return Err("official credential service mismatch".into());
        }
        validate_account_location(scope.host, id)?;
        let owner = self.for_owner(scope.principal)?;
        let path = owner.service_location(scope.tenant, scope.host, "kimi_oauth", id)?;
        #[cfg(unix)]
        {
            CredentialLease::new(owner.scoped_path(path), CredentialKind::Kimi, create)
        }
        #[cfg(not(unix))]
        {
            let _ = (path, create);
            Err(UNSUPPORTED.into())
        }
    }

    /// Allocate a new, owner-scoped Claude directory. Never adopts an arbitrary host path.
    pub fn register_claude_directory(
        &self,
        tenant: &TenantScope,
        host: &str,
        id: &str,
    ) -> Result<(), String> {
        self.lease(tenant, host, AppId::Claude, id, true)?.register_claude_directory()
    }

    /// Remove a newly allocated Claude login directory before its account is published.
    /// The caller must have confirmed that this ID has no published metadata.
    #[cfg(target_os = "macos")]
    pub fn discard_unpublished_claude(
        &self,
        scope: AuthAccountScope<'_>,
        id: &str,
        keychain: &dyn super::claude_keychain::ClaudeKeychain,
    ) -> Result<(), vw_api_types::agent_apps_auth::AuthErrorCode> {
        use crate::providers::claude_login::LoginMaterial;
        use sha2::{Digest, Sha256};
        use vw_api_types::agent_apps_auth::{AuthErrorCode, AuthService};
        scope.validate()?;
        if scope.service != AuthService::ClaudeOfficial {
            return Err(AuthErrorCode::Unsupported);
        }
        let storage = |_| AuthErrorCode::StorageUnavailable;
        let owner = self.for_owner(scope.principal).map_err(storage)?;
        let lease =
            owner.lease(scope.tenant, scope.host, AppId::Claude, id, false).map_err(storage)?;
        let AccountCredential::ClaudeDirectory(directory) = lease.read().map_err(storage)? else {
            return Err(AuthErrorCode::StorageUnavailable);
        };
        let material = LoginMaterial::from_account(directory.path(), keychain)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let identity = material.identity().map_err(|_| AuthErrorCode::IdentityMismatch)?;
        let hash = identity.map_or([0; 32], |identity| Sha256::digest(identity.as_bytes()).into());
        lease.remove_claude(keychain, hash)
    }

    /// Read only a host-private generation for a scoped Codex usage cache lookup.
    #[cfg(target_os = "macos")]
    pub fn codex_usage_version(
        &self,
        scope: AuthAccountScope<'_>,
        id: &str,
    ) -> Result<Option<[u8; 32]>, vw_api_types::agent_apps_auth::AuthErrorCode> {
        self.codex_usage_file(scope, id, |_, _, _, version| Ok(version))
    }

    /// Load the exact credential bundle associated with the generation used by a query.
    #[cfg(target_os = "macos")]
    pub fn codex_usage_snapshot(
        &self,
        scope: AuthAccountScope<'_>,
        id: &str,
    ) -> Result<Option<(serde_json::Value, [u8; 32])>, vw_api_types::agent_apps_auth::AuthErrorCode>
    {
        use vw_api_types::agent_apps_auth::AuthErrorCode;
        self.codex_usage_file(scope, id, |directory, path, bytes, version| {
            let AccountCredential::CodexAuth(auth) =
                decode_credential(directory, path, CredentialKind::App(AppId::Codex), bytes)
                    .map_err(|_| AuthErrorCode::ReauthRequired)?
            else {
                return Err(AuthErrorCode::ReauthRequired);
            };
            Ok((auth, version))
        })
    }

    #[cfg(target_os = "macos")]
    fn codex_usage_file<T>(
        &self,
        scope: AuthAccountScope<'_>,
        id: &str,
        project: impl FnOnce(
            &PrivateDirectory,
            &Path,
            Vec<u8>,
            [u8; 32],
        ) -> Result<T, vw_api_types::agent_apps_auth::AuthErrorCode>,
    ) -> Result<Option<T>, vw_api_types::agent_apps_auth::AuthErrorCode> {
        use std::os::unix::fs::MetadataExt;
        use vw_api_types::agent_apps_auth::AuthErrorCode;

        scope.validate()?;
        if scope.service != AuthService::CodexOauth {
            return Err(AuthErrorCode::Unsupported);
        }
        validate_account_location(scope.host, id).map_err(|_| AuthErrorCode::InvalidInput)?;
        let owner = self.for_owner(scope.principal).map_err(|_| AuthErrorCode::InvalidInput)?;
        let path = owner
            .service_location(scope.tenant, scope.host, "codex", id)
            .map_err(|_| AuthErrorCode::InvalidInput)?;
        let location = owner.scoped_path(path);
        let name = child_name(&location.path).map_err(|_| AuthErrorCode::InvalidInput)?;
        let directory = location.directory(false).map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let Some(file) = directory.file(&name).map_err(|_| AuthErrorCode::StorageUnavailable)?
        else {
            return Ok(None);
        };
        let stat = file.metadata().map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let bytes = credential_bytes(&file).map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let mut generation = Sha256::new();
        generation.update(stat.dev().to_le_bytes());
        generation.update(stat.ino().to_le_bytes());
        generation.update(stat.ctime().to_le_bytes());
        generation.update(stat.ctime_nsec().to_le_bytes());
        generation.update(&bytes);
        project(&directory, &location.path, bytes, generation.finalize().into()).map(Some)
    }

    /// Observe managed material without acquiring a write lease or adopting a native login.
    pub fn read_managed_account(
        &self,
        scope: AuthAccountScope<'_>,
        id: &str,
    ) -> Result<Option<AccountCredential>, vw_api_types::agent_apps_auth::AuthErrorCode> {
        use vw_api_types::agent_apps_auth::AuthErrorCode;
        scope.validate()?;
        validate_account_location(scope.host, id).map_err(|_| AuthErrorCode::InvalidInput)?;
        #[cfg(unix)]
        {
            let (service, kind) = match scope.service {
                AuthService::GithubCopilot => ("github_copilot", CredentialKind::Copilot),
                AuthService::CodexOauth => ("codex", CredentialKind::App(AppId::Codex)),
                AuthService::XaiOauth => ("xai_oauth", CredentialKind::Xai),
                AuthService::KimiOauth => ("kimi_oauth", CredentialKind::Kimi),
                AuthService::ClaudeOfficial => ("claude", CredentialKind::App(AppId::Claude)),
                _ => return Err(AuthErrorCode::Unsupported),
            };
            let owner = self.for_owner(scope.principal).map_err(|_| AuthErrorCode::InvalidInput)?;
            let path = owner
                .service_location(scope.tenant, scope.host, service, id)
                .map_err(|_| AuthErrorCode::InvalidInput)?;
            let location = owner.scoped_path(path);
            match location.directory(false).and_then(|dir| read_from(&dir, &location.path, kind)) {
                Ok(credential) => Ok(Some(credential)),
                Err(error) if error == "official account not found" => Ok(None),
                Err(_) => Err(AuthErrorCode::StorageUnavailable),
            }
        }
        #[cfg(not(unix))]
        Err(AuthErrorCode::Unsupported)
    }

    pub fn read(
        &self,
        tenant: &TenantScope,
        host: &str,
        app: AppId,
        id: &str,
    ) -> Result<AccountCredential, String> {
        let path = self.location(tenant, host, app, id)?;
        #[cfg(unix)]
        {
            let location = self.scoped_path(path);
            read_from(&location.directory(false)?, &location.path, CredentialKind::App(app))
        }
        #[cfg(not(unix))]
        {
            let _ = path;
            Err(UNSUPPORTED.into())
        }
    }
}

fn validate_account_location(host: &str, id: &str) -> Result<(), String> {
    for value in [host, id] {
        crate::providers::validate_identity(value)
            .map_err(|_| "invalid official account identity")?;
    }
    Ok(())
}

fn scope_component(value: &str) -> String {
    // Lowercase hexadecimal remains injective for the digest on case-insensitive APFS.
    Sha256::digest(value.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(super) fn managed_config_path(path: &Path) -> PathBuf {
    path.with_extension("claude-config")
}

// Read the rotated auth file produced inside our private process directory.
pub(crate) fn read_credential(path: &Path, app: AppId) -> Result<AccountCredential, String> {
    #[cfg(unix)]
    {
        let parent = path.parent().ok_or("unsafe official credential path")?;
        read_from(&PrivateDirectory::open(parent)?, path, CredentialKind::App(app))
    }
    #[cfg(not(unix))]
    {
        let _ = (path, app);
        Err(UNSUPPORTED.into())
    }
}
