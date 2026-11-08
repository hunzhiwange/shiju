//! Official account owner: metadata in shared tenant storage, secrets in private host files.
use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
pub use vw_agent_apps::auth::flow::AuthAccountScope;
#[cfg(not(target_arch = "wasm32"))]
use vw_agent_apps::auth::{
    codex_browser::CodexBrowserStart,
    codex_device::{CodexDeviceAuth, CodexDeviceStart, codex_account_id},
    copilot::{CopilotAuth, CopilotDeviceStart},
    copilot_token::CopilotSession,
    flow::AuthFlows,
};
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
#[cfg(not(target_arch = "wasm32"))]
use vw_api_types::agent_apps_auth::{AuthAccount, AuthFlow, AuthService};
use vw_api_types::{
    TimestampMs,
    agent_apps::{AppId, AppInstallation, LaunchRequest},
    agent_apps_auth::AuthErrorCode,
    id::TenantScope,
};

// Launch-only rows lack a principal and are never read as managed accounts.
const MANAGED_ACCOUNT_NAMESPACE: &str = "agent_apps.auth_accounts.v2";

/// Persisted account identity from a verified upstream login, without credential material.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthAccountMetadata {
    pub external_identity_sha256: [u8; 32],
    pub authenticated_at: TimestampMs,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github_domain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
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
            github_domain: None,
            label: None,
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

    /// Resolve a managed Copilot account for host-internal requests.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn copilot_session(
        &self,
        adapter: &CopilotAuth,
        scope: AuthAccountScope<'_>,
        account_id: &str,
    ) -> Result<CopilotSession, AuthErrorCode> {
        if scope.service != AuthService::GithubCopilot {
            return Err(AuthErrorCode::Unsupported);
        }
        let metadata =
            crate::global_database::run_sync_database(
                || Ok(self.read_metadata(&scope, account_id)),
            )
            .map_err(|_| AuthErrorCode::StorageUnavailable)??;
        let authority = metadata.github_domain.ok_or(AuthErrorCode::StorageUnavailable)?;
        adapter.session(&self.credentials, scope, account_id, &authority).await
    }

    /// Advance a due Codex attempt and publish its token bundle on the execution host.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn poll_codex_device(
        &self,
        adapter: &CodexDeviceAuth,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        started: &CodexDeviceStart,
    ) -> Result<AuthFlow, AuthErrorCode> {
        adapter
            .poll(flows, scope, started, |bound, target, account, identity, credential| {
                self.commit_codex_account(bound, target, account, identity, credential)
            })
            .await
    }

    /// Advance a browser callback using the same scoped Codex account transaction.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn poll_codex_browser(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        started: &CodexBrowserStart,
    ) -> Result<AuthFlow, AuthErrorCode> {
        started
            .poll(flows, scope, |bound, target, account, identity, credential| {
                self.commit_codex_account(bound, target, account, identity, credential)
            })
            .await
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn commit_codex_account(
        &self,
        scope: AuthAccountScope<'_>,
        target: Option<&str>,
        account: &AuthAccount,
        identity: &str,
        credential: &AccountCredential,
    ) -> Result<(), AuthErrorCode> {
        crate::global_database::run_sync_database(|| {
            Ok(self.store_codex_account(scope, target, account, identity, credential))
        })
        .map_err(|_| AuthErrorCode::StorageUnavailable)?
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn store_codex_account(
        &self,
        scope: AuthAccountScope<'_>,
        target: Option<&str>,
        account: &AuthAccount,
        identity: &str,
        credential: &AccountCredential,
    ) -> Result<(), AuthErrorCode> {
        let authenticated_at = account.authenticated_at.ok_or(AuthErrorCode::InvalidInput)?;
        let mut metadata = AuthAccountMetadata::new(identity, authenticated_at)?;
        metadata.label = Some(account.label.clone());
        let id = &account.account_id;
        if target.is_some_and(|target| target != id) {
            return Err(AuthErrorCode::IdentityMismatch);
        }
        let owner = self
            .credentials
            .for_owner(scope.principal)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let canonical_id = codex_account_id(scope, identity)?;
        // Serialize commits for this identity even when an existing account uses a different ID.
        let acquire = |id| {
            owner
                .lease(scope.tenant, scope.host, AppId::Codex, id, true)
                .map_err(|_| AuthErrorCode::StorageUnavailable)
        };
        let identity_lease = acquire(&canonical_id)?;
        let account_lease;
        let lease = if *id == canonical_id {
            &identity_lease
        } else {
            account_lease = acquire(id)?;
            &account_lease
        };
        let key = Self::managed_key(&scope, id)?;
        let previous = self
            .metadata
            .get(scope.tenant, MANAGED_ACCOUNT_NAMESPACE, &key)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        if target.is_some() && previous.is_none() {
            return Err(AuthErrorCode::NotFound);
        }
        if self.codex_identity_exists_elsewhere(scope, id, metadata.external_identity_sha256)? {
            return Err(AuthErrorCode::CodexOauthDuplicateAccount);
        }
        if let Some(previous) = previous {
            let saved: AuthAccountMetadata =
                serde_json::from_str(&previous).map_err(|_| AuthErrorCode::StorageUnavailable)?;
            if saved.external_identity_sha256 != metadata.external_identity_sha256 {
                return Err(AuthErrorCode::IdentityMismatch);
            }
            if target.is_none() {
                return Err(AuthErrorCode::CodexOauthDuplicateAccount);
            }
            if saved.authenticated_at > authenticated_at {
                return Err(AuthErrorCode::Conflict);
            }
            // Identity, display preferences and bindings belong to the existing account.
            let updated = AuthAccountMetadata { authenticated_at, ..saved };
            let updated =
                serde_json::to_string(&updated).map_err(|_| AuthErrorCode::StorageUnavailable)?;
            return self.replace_codex_credential(
                scope.tenant,
                &key,
                &previous,
                &updated,
                lease,
                credential,
            );
        }

        self.publish_oauth_credential(&scope, id, &metadata, lease, credential)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn codex_identity_exists_elsewhere(
        &self,
        scope: AuthAccountScope<'_>,
        id: &str,
        identity_hash: [u8; 32],
    ) -> Result<bool, AuthErrorCode> {
        let rows = self
            .metadata
            .list(scope.tenant, MANAGED_ACCOUNT_NAMESPACE)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        for row in rows {
            let (principal, host, service, other_id): (String, String, AuthService, String) =
                serde_json::from_str(&row.config_key)
                    .map_err(|_| AuthErrorCode::StorageUnavailable)?;
            if principal == scope.principal
                && host == scope.host
                && service == scope.service
                && other_id != id
            {
                let other: AuthAccountMetadata = serde_json::from_str(&row.payload_json)
                    .map_err(|_| AuthErrorCode::StorageUnavailable)?;
                if other.external_identity_sha256 == identity_hash {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn replace_codex_credential(
        &self,
        tenant: &TenantScope,
        key: &str,
        previous: &str,
        updated: &str,
        lease: &CredentialLease,
        credential: &AccountCredential,
    ) -> Result<(), AuthErrorCode> {
        let old_credential = match lease.read() {
            Ok(saved) => Some(saved),
            Err(error) if error == "official account not found" => None,
            Err(_) => return Err(AuthErrorCode::StorageUnavailable),
        };
        lease.save(credential).map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let error = match self.metadata.compare_exchange(
            tenant,
            MANAGED_ACCOUNT_NAMESPACE,
            key,
            previous,
            updated,
        ) {
            Ok(true) => return Ok(()),
            Ok(false) => AuthErrorCode::Conflict,
            Err(_) => match self.metadata.get(tenant, MANAGED_ACCOUNT_NAMESPACE, key) {
                Ok(Some(current)) if current == updated => return Ok(()),
                Ok(_) => AuthErrorCode::StorageUnavailable,
                // Publication may have succeeded. As with new accounts, keep the file when
                // the database cannot confirm the state of its metadata.
                Err(_) => return Err(AuthErrorCode::StorageUnavailable),
            },
        };
        match old_credential {
            Some(saved) => lease.save(&saved),
            None => lease.remove_unpublished_oauth(),
        }
        .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        Err(error)
    }

    /// Advance a due Copilot attempt on the authorized execution host.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn poll_copilot(
        &self,
        adapter: &CopilotAuth,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        started: &CopilotDeviceStart,
    ) -> Result<AuthFlow, AuthErrorCode> {
        adapter
            .poll(flows, scope, started, |bound, account, identity, token| {
                self.commit_copilot(bound, account, identity, token)
            })
            .await
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn commit_copilot(
        &self,
        scope: AuthAccountScope<'_>,
        account: &AuthAccount,
        identity: &str,
        token: &str,
    ) -> Result<(), AuthErrorCode> {
        crate::global_database::run_sync_database(|| {
            Ok(self.store_copilot(scope, account, identity, token))
        })
        .map_err(|_| AuthErrorCode::StorageUnavailable)?
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn store_copilot(
        &self,
        scope: AuthAccountScope<'_>,
        account: &AuthAccount,
        identity: &str,
        token: &str,
    ) -> Result<(), AuthErrorCode> {
        let authenticated_at = account.authenticated_at.ok_or(AuthErrorCode::InvalidInput)?;
        let mut metadata = AuthAccountMetadata::new(identity, authenticated_at)?;
        metadata.github_domain =
            Some(account.github_domain.clone().ok_or(AuthErrorCode::InvalidInput)?);
        metadata.label = Some(account.label.clone());
        let id = &account.account_id;
        let lease = self
            .credentials
            .lease_copilot(scope, id, true)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let existing = match self.read_metadata(&scope, id) {
            Ok(saved) if saved.external_identity_sha256 == metadata.external_identity_sha256 => {
                true
            }
            Ok(_) => return Err(AuthErrorCode::IdentityMismatch),
            Err(AuthErrorCode::NotFound) => false,
            Err(error) => return Err(error),
        };

        let credential = AccountCredential::CopilotToken(token.to_owned());
        if existing {
            lease.save(&credential).map_err(|_| AuthErrorCode::StorageUnavailable)
        } else {
            self.publish_oauth_credential(&scope, id, &metadata, &lease, &credential)
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn publish_oauth_credential(
        &self,
        scope: &AuthAccountScope<'_>,
        id: &str,
        metadata: &AuthAccountMetadata,
        lease: &CredentialLease,
        credential: &AccountCredential,
    ) -> Result<(), AuthErrorCode> {
        lease.save(credential).map_err(|_| AuthErrorCode::StorageUnavailable)?;
        if let Err(error) = self.insert_metadata(scope, id, metadata) {
            // Confirm ambiguous database errors before removing a published credential.
            match self.read_metadata(scope, id) {
                Ok(saved) if saved == *metadata => return Ok(()),
                Ok(_) | Err(AuthErrorCode::NotFound) => lease
                    .remove_unpublished_oauth()
                    .map_err(|_| AuthErrorCode::StorageUnavailable)?,
                Err(_) => {
                    // Preserve the private file if the database cannot tell us whether
                    // it committed. A later attempt checks the same account under its lease.
                }
            }
            return Err(error);
        }
        Ok(())
    }

    fn managed_key(
        scope: &AuthAccountScope<'_>,
        account_id: &str,
    ) -> Result<String, AuthErrorCode> {
        scope.validate()?;
        vw_agent_apps::providers::validate_identity(account_id)
            .map_err(|_| AuthErrorCode::InvalidInput)?;
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
