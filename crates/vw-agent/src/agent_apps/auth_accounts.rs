//! Official account owner: metadata in shared tenant storage, secrets in private host files.
use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use vw_agent_apps::{
    apps::launch::{
        account::{AccountLaunch, prepare_account_launch},
        request::validate_launch_request,
    },
    auth::{
        credential_file::{AccountCredential, CredentialFileStore},
        credential_lease::CredentialLease,
    },
};
use vw_api_types::{
    TimestampMs,
    agent_apps::{AppId, AppInstallation, LaunchRequest},
    agent_apps_auth::{AuthErrorCode, AuthService},
    id::TenantScope,
};

// Launch-only rows lack a principal and are never read as managed accounts.
const MANAGED_ACCOUNT_NAMESPACE: &str = "agent_apps.auth_accounts.v2";

/// Ownership resolved by the caller from trusted tenant membership and host policy.
/// Construct only after authorizing the selected service and execution host.
pub struct AuthAccountScope<'a> {
    pub tenant: &'a TenantScope,
    pub principal: &'a str,
    pub host: &'a str,
    pub service: AuthService,
}

/// Persisted account identity from a verified upstream login, without credential material.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthAccountMetadata {
    pub external_identity_sha256: [u8; 32],
    pub authenticated_at: TimestampMs,
}

impl AuthAccountMetadata {
    /// Hash the service's verified, canonical identity, including issuer or region if applicable.
    pub fn new(
        external_identity: &str,
        authenticated_at: TimestampMs,
    ) -> Result<Self, AuthErrorCode> {
        if external_identity.trim().is_empty() || external_identity.chars().any(char::is_control) {
            return Err(AuthErrorCode::InvalidInput);
        }
        Ok(Self {
            external_identity_sha256: Sha256::digest(external_identity).into(),
            authenticated_at,
        })
    }
}

/// Internal authorized-host service. Not a client-selected directory or account discovery API.
pub struct AuthAccountStore {
    metadata: GatewaySettingsStore,
    credentials: CredentialFileStore,
}
/// Holds the auth-owner lease through child exit, rotation writeback and temporary cleanup.
pub struct AuthAccountLaunch {
    launch: AccountLaunch,
    lease: CredentialLease,
}
impl AuthAccountLaunch {
    pub fn run(self) -> Result<(), String> {
        self.run_with(|command| {
            command
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
        })
    }
    pub fn run_with(
        self,
        execute: impl FnOnce(&mut std::process::Command) -> std::io::Result<std::process::ExitStatus>,
    ) -> Result<(), String> {
        let Self { launch, lease } = self;
        let result = launch.run_with_refresh_using(|credential| lease.save(&credential), execute);
        drop(lease);
        result
    }
}
impl AuthAccountStore {
    pub fn new(database: GlobalDatabase, credential_root: &Path) -> Result<Self, String> {
        Ok(Self {
            metadata: GatewaySettingsStore::new(database)
                .map_err(|_| "official account metadata unavailable")?,
            credentials: CredentialFileStore::new(credential_root)?,
        })
    }
    /// Publish metadata after the authorized login verifies the service identity.
    pub fn insert_metadata(
        &self,
        scope: &AuthAccountScope<'_>,
        account_id: &str,
        metadata: &AuthAccountMetadata,
    ) -> Result<(), AuthErrorCode> {
        let key = Self::managed_key(scope, account_id)?;
        let payload =
            serde_json::to_string(metadata).map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let inserted = self
            .metadata
            .insert_if_absent(scope.tenant, MANAGED_ACCOUNT_NAMESPACE, &key, &payload)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        if !inserted {
            return Err(AuthErrorCode::Conflict);
        }
        Ok(())
    }

    /// Read a managed account in the authorized scope.
    pub fn read_metadata(
        &self,
        scope: &AuthAccountScope<'_>,
        account_id: &str,
    ) -> Result<AuthAccountMetadata, AuthErrorCode> {
        let key = Self::managed_key(scope, account_id)?;
        let payload = self
            .metadata
            .get(scope.tenant, MANAGED_ACCOUNT_NAMESPACE, &key)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?
            .ok_or(AuthErrorCode::NotFound)?;
        serde_json::from_str(&payload).map_err(|_| AuthErrorCode::StorageUnavailable)
    }

    fn managed_key(
        scope: &AuthAccountScope<'_>,
        account_id: &str,
    ) -> Result<String, AuthErrorCode> {
        if scope.principal.trim().is_empty() || scope.principal.chars().any(char::is_control) {
            return Err(AuthErrorCode::InvalidInput);
        }
        for value in [scope.host, account_id] {
            vw_agent_apps::providers::validate_identity(value)
                .map_err(|_| AuthErrorCode::InvalidInput)?;
        }
        serde_json::to_string(&(scope.principal, scope.host, scope.service, account_id))
            .map_err(|_| AuthErrorCode::InvalidInput)
    }

    fn key(host: &str, app: AppId, account_id: &str) -> Result<String, String> {
        for value in [host, account_id] {
            vw_agent_apps::providers::validate_identity(value)
                .map_err(|_| "invalid official account identity")?;
        }
        serde_json::to_string(&(host, app, account_id))
            .map_err(|_| "invalid official account identity".into())
    }
    /// Read scoped metadata through the unique owner and validate its private material.
    /// Unsupported account services and missing records produce no selectable identities.
    pub fn list(
        &self,
        tenant: &TenantScope,
        host: &str,
        app: AppId,
    ) -> Result<Vec<String>, String> {
        if !matches!(app, AppId::Claude | AppId::Codex) {
            return Ok(vec![]);
        }
        let mut ids = Vec::new();
        for entry in self
            .metadata
            .list(tenant, "agent_apps.auth_accounts")
            .map_err(|_| "official account metadata unavailable")?
        {
            let (record_host, record_app, id): (String, AppId, String) =
                serde_json::from_str(&entry.config_key)
                    .map_err(|_| "invalid stored official account metadata")?;
            if record_host == host
                && record_app == app
                && entry.payload_json == "{\"configured\":true}"
            {
                Self::key(host, app, &id)?;
                // Invalid/missing material is not advertised as an available account.
                if self.credentials.read(tenant, host, app, &id).is_ok() {
                    ids.push(id);
                }
            }
        }
        ids.sort();
        Ok(ids)
    }
    /// Caller must authorize this explicit import for the tenant/host; no ambient adoption.
    pub fn save(
        &self,
        tenant: &TenantScope,
        host: &str,
        app: AppId,
        account_id: &str,
        credential: &AccountCredential,
    ) -> Result<(), String> {
        let key = Self::key(host, app, account_id)?;
        self.credentials.validate_material_scope(tenant, host, app, account_id, credential)?;
        let lease = self.credentials.lease(tenant, host, app, account_id, true)?;
        self.metadata
            .upsert(tenant, "agent_apps.auth_accounts", &key, "{\"configured\":true}")
            .map_err(|_| "official account metadata unavailable")?;
        lease.save(credential)
    }
    pub fn register_claude_directory(
        &self,
        tenant: &TenantScope,
        host: &str,
        account_id: &str,
    ) -> Result<(), String> {
        let key = Self::key(host, AppId::Claude, account_id)?;
        self.credentials.register_claude_directory(tenant, host, account_id)?;
        self.metadata
            .upsert(tenant, "agent_apps.auth_accounts", &key, "{\"configured\":true}")
            .map_err(|_| "official account metadata unavailable")?;
        Ok(())
    }
    /// An explicit scoped selection; never interprets the machine's current login as a default.
    pub fn set_default(
        &self,
        tenant: &TenantScope,
        host: &str,
        app: AppId,
        account_id: &str,
    ) -> Result<(), String> {
        let key = Self::key(host, app, account_id)?;
        if self
            .metadata
            .get(tenant, "agent_apps.auth_accounts", &key)
            .map_err(|_| "official account metadata unavailable")?
            .is_none()
        {
            return Err("official account not found".into());
        }
        self.credentials.read(tenant, host, app, account_id)?;
        let default_key =
            serde_json::to_string(&(host, app)).map_err(|_| "invalid official account identity")?;
        let value =
            serde_json::to_string(account_id).map_err(|_| "invalid official account identity")?;
        self.metadata
            .upsert(tenant, "agent_apps.auth_defaults", &default_key, &value)
            .map_err(|_| "official account metadata unavailable")?;
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_default_launch(
        &self,
        tenant: &TenantScope,
        host: &str,
        request: &LaunchRequest,
        installations: &[AppInstallation],
        roots: &[PathBuf],
        private_temp_root: &Path,
    ) -> Result<AuthAccountLaunch, String> {
        vw_agent_apps::providers::validate_identity(host)
            .map_err(|_| "invalid official account identity")?;
        let key = serde_json::to_string(&(host, request.app_id))
            .map_err(|_| "invalid official account identity")?;
        let value = self
            .metadata
            .get(tenant, "agent_apps.auth_defaults", &key)
            .map_err(|_| "official account metadata unavailable")?
            .ok_or("default official account not configured")?;
        let account_id: String =
            serde_json::from_str(&value).map_err(|_| "invalid stored default official account")?;
        self.prepare_launch(
            tenant,
            host,
            &account_id,
            request,
            installations,
            roots,
            private_temp_root,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_launch(
        &self,
        tenant: &TenantScope,
        host: &str,
        account_id: &str,
        request: &LaunchRequest,
        installations: &[AppInstallation],
        roots: &[PathBuf],
        private_temp_root: &Path,
    ) -> Result<AuthAccountLaunch, String> {
        let key = Self::key(host, request.app_id, account_id)?;
        if self
            .metadata
            .get(tenant, "agent_apps.auth_accounts", &key)
            .map_err(|_| "official account metadata unavailable")?
            .as_deref()
            != Some("{\"configured\":true}")
        {
            return Err("official account not found".into());
        }
        let material = validate_launch_request(request, host, installations, roots, false)?;
        let lease = self.credentials.lease(tenant, host, request.app_id, account_id, false)?;
        let credential = lease.read()?;
        let launch =
            prepare_account_launch(request.app_id, credential, material, private_temp_root)?;
        Ok(AuthAccountLaunch { launch, lease })
    }
}
#[cfg(test)]
#[path = "auth_accounts_tests.rs"]
mod tests;

#[cfg(target_os = "macos")]
impl AuthAccountStore {
    /// Resolve only a pre-existing tenant/host account and retain the owner's launch lease.
    pub(crate) fn claude_login_account(
        &self,
        tenant: &TenantScope,
        host: &str,
        id: &str,
    ) -> Result<
        (CredentialLease, vw_agent_apps::auth::credential_file::ClaudeAccountDirectory),
        String,
    > {
        let key = Self::key(host, AppId::Claude, id)?;
        if self
            .metadata
            .get(tenant, "agent_apps.auth_accounts", &key)
            .map_err(|_| "official account storage unavailable")?
            .as_deref()
            != Some("{\"configured\":true}")
        {
            return Err("official account not found".into());
        }
        let lease = self.credentials.lease(tenant, host, AppId::Claude, id, false)?;
        let AccountCredential::ClaudeDirectory(directory) = lease.read()? else {
            return Err("native login requires an existing directory account".into());
        };
        Ok((lease, directory))
    }
    /// Preserve a rotated/previous default in the same owner before overwriting the OS slot.
    pub(crate) fn archive_claude_default(
        &self,
        tenant: &TenantScope,
        host: &str,
        material: &vw_agent_apps::providers::claude_login::LoginMaterial,
        keychain: &dyn vw_agent_apps::auth::claude_keychain::ClaudeKeychain,
    ) -> Result<(), vw_agent_apps::providers::live::write_guard::WriteError> {
        use vw_agent_apps::providers::{
            claude_login::LoginMaterial, live::write_guard::WriteError,
        };
        let Some(identity) = material.identity()? else {
            return Ok(());
        };
        for id in self.list(tenant, host, AppId::Claude).map_err(|_| WriteError::Io)? {
            if let AccountCredential::ClaudeDirectory(dir) = self
                .credentials
                .read(tenant, host, AppId::Claude, &id)
                .map_err(|_| WriteError::Io)?
                && LoginMaterial::from_account(dir.path(), keychain)?.identity()?.as_ref()
                    == Some(&identity)
            {
                let (_lease, dir) = self
                    .claude_login_account(tenant, host, &id)
                    .map_err(|_| WriteError::Conflict)?;
                return material.archive(dir.path(), keychain);
            }
        }
        let id = format!(
            "native_{}",
            Sha256::digest(identity.as_bytes())
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        // This is an auth-owner import, not a second account registry or a client path.
        self.register_claude_directory(tenant, host, &id).map_err(|_| WriteError::Io)?;
        let (_lease, dir) =
            self.claude_login_account(tenant, host, &id).map_err(|_| WriteError::Conflict)?;
        material.archive(dir.path(), keychain)
    }
}
