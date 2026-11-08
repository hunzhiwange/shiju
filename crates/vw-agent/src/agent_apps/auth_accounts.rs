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
    codex_refresh::CodexRefresh,
    copilot::{CopilotAuth, CopilotDeviceStart},
    copilot_token::CopilotSession,
    credential_file::{KimiCredential, XaiCredential},
    flow::AuthFlows,
    kimi::{KimiDeviceAuth, KimiDeviceStart, KimiLoginTarget},
    xai::{XaiDeviceAuth, XaiDeviceStart},
    xai_refresh::XaiRefresh,
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
use vw_api_types::agent_apps_auth::{AuthAccount, AuthFlow, AuthService, AuthStartRequest};
use vw_api_types::{
    TimestampMs,
    agent_apps::{AppId, AppInstallation, LaunchRequest},
    agent_apps_auth::{AuthErrorCode, AuthRegion},
    id::TenantScope,
};

#[cfg(not(target_arch = "wasm32"))]
mod defaults;
#[cfg(not(target_arch = "wasm32"))]
pub use defaults::NativeAccountDefault;

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
    pub region: Option<AuthRegion>,
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
            region: None,
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
        let result = launch.run_with_lease_using(&lease, execute);
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

    /// Resolve a managed Codex account's access token on the authorized execution host.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn codex_access_token(
        &self,
        adapter: &CodexRefresh,
        scope: AuthAccountScope<'_>,
        account_id: &str,
    ) -> Result<String, AuthErrorCode> {
        if scope.service != AuthService::CodexOauth {
            return Err(AuthErrorCode::Unsupported);
        }
        crate::global_database::run_sync_database(|| Ok(self.read_metadata(&scope, account_id)))
            .map_err(|_| AuthErrorCode::StorageUnavailable)??;
        adapter.access_token(&self.credentials, scope, account_id).await
    }

    /// Resolve a managed xAI account's access token on the authorized execution host.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn xai_access_token(
        &self,
        adapter: &XaiRefresh,
        scope: AuthAccountScope<'_>,
        account_id: &str,
    ) -> Result<String, AuthErrorCode> {
        if scope.service != AuthService::XaiOauth {
            return Err(AuthErrorCode::Unsupported);
        }
        crate::global_database::run_sync_database(|| Ok(self.read_metadata(&scope, account_id)))
            .map_err(|_| AuthErrorCode::StorageUnavailable)??;
        adapter.access_token(&self.credentials, scope, account_id).await
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
            return self.replace_oauth_credential(
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
    fn replace_oauth_credential(
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

    /// Advance xAI device authorization and publish the scoped account and private tokens.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn poll_xai_device(
        &self,
        adapter: &XaiDeviceAuth,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        started: &XaiDeviceStart,
    ) -> Result<AuthFlow, AuthErrorCode> {
        adapter
            .poll(flows, scope, started, |bound, account, identity, credential| {
                crate::global_database::run_sync_database(|| {
                    Ok(self.store_xai_device_account(bound, account, identity, credential))
                })
                .map_err(|_| AuthErrorCode::StorageUnavailable)?
            })
            .await
    }

    /// Select an existing Kimi profile on the authorized execution host.
    #[cfg(target_os = "macos")]
    pub async fn use_kimi_account(
        &self,
        adapter: &KimiDeviceAuth,
        scope: AuthAccountScope<'_>,
        account_id: &str,
        native: vw_agent_apps::auth::kimi_account::KimiNativeTarget<'_>,
    ) -> Result<(), AuthErrorCode> {
        let account = self.kimi_account_target(scope, account_id)?;
        adapter.use_account(&self.credentials, account, native).await
    }

    #[cfg(target_os = "macos")]
    pub(crate) async fn use_bound_kimi_account(
        &self,
        scope: AuthAccountScope<'_>,
        account_id: &str,
        native: vw_agent_apps::auth::kimi_account::KimiNativeTarget<'_>,
        connection: &serde_json::Value,
        catalog_unchanged: impl Fn() -> bool + Send + 'static,
    ) -> Result<(), AuthErrorCode> {
        let account = self.kimi_account_target(scope, account_id)?;
        let adapter = KimiDeviceAuth::new().map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
        adapter
            .use_bound_account(&self.credentials, account, native, connection, catalog_unchanged)
            .await
    }

    #[cfg(target_os = "macos")]
    fn kimi_account_target<'a>(
        &self,
        scope: AuthAccountScope<'a>,
        account_id: &'a str,
    ) -> Result<vw_agent_apps::auth::kimi_account::KimiAccountTarget<'a>, AuthErrorCode> {
        use vw_agent_apps::auth::kimi_account::KimiAccountTarget;
        if scope.service != AuthService::KimiOauth {
            return Err(AuthErrorCode::Unsupported);
        }
        let metadata =
            crate::global_database::run_sync_database(
                || Ok(self.read_metadata(&scope, account_id)),
            )
            .map_err(|_| AuthErrorCode::StorageUnavailable)??;
        Ok(KimiAccountTarget {
            scope,
            account_id,
            region: metadata.region.ok_or(AuthErrorCode::StorageUnavailable)?,
            identity_sha256: metadata.external_identity_sha256,
        })
    }

    #[cfg(target_os = "macos")]
    pub(crate) async fn bind_kimi_provider(
        &self,
        scope: AuthAccountScope<'_>,
        account_id: &str,
        native: &vw_agent_apps::auth::kimi_account::KimiNativeTarget<'_>,
        expected_account: Option<&str>,
        connection: &serde_json::Value,
        catalog_unchanged: impl Fn() -> bool + Send + 'static,
    ) -> Result<(), AuthErrorCode> {
        use vw_agent_apps::auth::kimi_account::KimiAccountTarget;

        let key = Self::managed_key(&scope, account_id)?;
        let read = || self.metadata.get(scope.tenant, MANAGED_ACCOUNT_NAMESPACE, &key);
        let raw = crate::global_database::run_sync_database(read)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?
            .ok_or(AuthErrorCode::NotFound)?;
        let metadata: AuthAccountMetadata =
            serde_json::from_str(&raw).map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let target = KimiAccountTarget {
            scope,
            account_id,
            region: metadata.region.ok_or(AuthErrorCode::StorageUnavailable)?,
            identity_sha256: metadata.external_identity_sha256,
        };
        let store = self.metadata.clone();
        let tenant = scope.tenant.clone();
        native
            .bind_account(&self.credentials, target, expected_account, connection, move || {
                catalog_unchanged()
                    && crate::global_database::run_sync_database(|| {
                        store.get(&tenant, MANAGED_ACCOUNT_NAMESPACE, &key)
                    })
                    .is_ok_and(|latest| latest.as_ref() == Some(&raw))
            })
            .await
    }

    /// Resolve a targeted re-login only from the same authorized host account.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn start_kimi_device(
        &self,
        adapter: &KimiDeviceAuth,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        request: AuthStartRequest,
    ) -> Result<KimiDeviceStart, AuthErrorCode> {
        adapter
            .start(flows, scope, request, |id| {
                crate::global_database::run_sync_database(|| Ok(self.kimi_login_target(scope, id)))
                    .map_err(|_| AuthErrorCode::StorageUnavailable)?
            })
            .await
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn kimi_login_target(
        &self,
        scope: AuthAccountScope<'_>,
        id: &str,
    ) -> Result<KimiLoginTarget, AuthErrorCode> {
        let metadata = self.read_metadata(&scope, id)?;
        let lease = self
            .credentials
            .lease_kimi(scope, id, false)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let AccountCredential::KimiOauth(saved) =
            lease.read().map_err(|_| AuthErrorCode::StorageUnavailable)?
        else {
            return Err(AuthErrorCode::StorageUnavailable);
        };
        if metadata.region != Some(saved.region) {
            return Err(AuthErrorCode::IdentityMismatch);
        }
        Ok(KimiLoginTarget { region: saved.region, device_id: saved.device_id })
    }

    /// Advance the device exchange and publish the authorized account and private tokens.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn poll_kimi_device(
        &self,
        adapter: &KimiDeviceAuth,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        started: &KimiDeviceStart,
    ) -> Result<AuthFlow, AuthErrorCode> {
        adapter
            .poll(flows, scope, started, |bound, target, account, identity, credential| {
                crate::global_database::run_sync_database(|| {
                    Ok(self.store_kimi_device_account(bound, target, account, identity, credential))
                })
                .map_err(|_| AuthErrorCode::StorageUnavailable)?
            })
            .await
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn store_kimi_device_account(
        &self,
        scope: AuthAccountScope<'_>,
        target: Option<&str>,
        account: &AuthAccount,
        identity: &str,
        credential: KimiCredential,
    ) -> Result<(), AuthErrorCode> {
        let authenticated_at = account.authenticated_at.ok_or(AuthErrorCode::InvalidInput)?;
        if account.region != Some(credential.region)
            || target.is_some_and(|target| target != account.account_id)
        {
            return Err(AuthErrorCode::IdentityMismatch);
        }
        let mut metadata = AuthAccountMetadata::new(identity, authenticated_at)?;
        metadata.label = Some(account.label.clone());
        metadata.region = account.region;
        let id = &account.account_id;
        let lease = self
            .credentials
            .lease_kimi(scope, id, true)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        self.store_device_credential(
            &scope,
            id,
            &metadata,
            &lease,
            &AccountCredential::KimiOauth(credential),
            target.is_some(),
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn store_xai_device_account(
        &self,
        scope: AuthAccountScope<'_>,
        account: &AuthAccount,
        identity: &str,
        credential: XaiCredential,
    ) -> Result<(), AuthErrorCode> {
        let authenticated_at = account.authenticated_at.ok_or(AuthErrorCode::InvalidInput)?;
        let mut metadata = AuthAccountMetadata::new(identity, authenticated_at)?;
        metadata.label = Some(account.label.clone());
        let id = &account.account_id;
        let lease = self
            .credentials
            .lease_xai(scope, id, true)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        self.store_device_credential(
            &scope,
            id,
            &metadata,
            &lease,
            &AccountCredential::XaiOauth(credential),
            false,
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn store_device_credential(
        &self,
        scope: &AuthAccountScope<'_>,
        id: &str,
        metadata: &AuthAccountMetadata,
        lease: &CredentialLease,
        credential: &AccountCredential,
        require_existing: bool,
    ) -> Result<(), AuthErrorCode> {
        let key = Self::managed_key(scope, id)?;
        if let Some(previous) = self
            .metadata
            .get(scope.tenant, MANAGED_ACCOUNT_NAMESPACE, &key)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?
        {
            let saved: AuthAccountMetadata =
                serde_json::from_str(&previous).map_err(|_| AuthErrorCode::StorageUnavailable)?;
            if saved.external_identity_sha256 != metadata.external_identity_sha256
                || saved.region != metadata.region
            {
                return Err(AuthErrorCode::IdentityMismatch);
            }
            if saved.authenticated_at > metadata.authenticated_at {
                return Err(AuthErrorCode::Conflict);
            }
            let updated =
                serde_json::to_string(metadata).map_err(|_| AuthErrorCode::StorageUnavailable)?;
            return self.replace_oauth_credential(
                scope.tenant,
                &key,
                &previous,
                &updated,
                lease,
                credential,
            );
        }
        if require_existing {
            return Err(AuthErrorCode::NotFound);
        }
        self.publish_oauth_credential(scope, id, metadata, lease, credential)
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
    pub(crate) fn managed_official_lease(
        &self,
        scope: AuthAccountScope<'_>,
        id: &str,
    ) -> Result<CredentialLease, AuthErrorCode> {
        let app = match scope.service {
            AuthService::ClaudeOfficial => AppId::Claude,
            AuthService::CodexOauth => AppId::Codex,
            _ => return Err(AuthErrorCode::Unsupported),
        };
        self.read_metadata(&scope, id)?;
        let lease = self
            .credentials
            .for_owner(scope.principal)
            .and_then(|owner| owner.lease(scope.tenant, scope.host, app, id, false))
            .map_err(|error| {
                if error == "official account is in use" {
                    AuthErrorCode::AccountInUse
                } else {
                    AuthErrorCode::StorageUnavailable
                }
            })?;
        self.read_metadata(&scope, id)?;
        Ok(lease)
    }

    pub(crate) fn claude_managed_directory(
        &self,
        lease: &CredentialLease,
    ) -> Result<vw_agent_apps::auth::credential_file::ClaudeAccountDirectory, AuthErrorCode> {
        match lease.read().map_err(|_| AuthErrorCode::StorageUnavailable)? {
            AccountCredential::ClaudeDirectory(directory) => Ok(directory),
            _ => Err(AuthErrorCode::ReauthRequired),
        }
    }

    pub(crate) fn prepare_bound_official_launch(
        &self,
        scope: AuthAccountScope<'_>,
        id: &str,
        model: Option<&str>,
        material: vw_agent_apps::apps::launch::request::LaunchMaterial,
        unchanged: impl FnOnce() -> bool,
    ) -> Result<AuthAccountLaunch, AuthErrorCode> {
        let lease = self.managed_official_lease(scope, id)?;
        let metadata = self.read_metadata(&scope, id)?;
        let credential = lease.read().map_err(|_| AuthErrorCode::ReauthRequired)?;
        let (app, identity) = match &credential {
            AccountCredential::ClaudeDirectory(directory)
                if scope.service == AuthService::ClaudeOfficial =>
            {
                let identity = vw_agent_apps::providers::claude_login::LoginMaterial::from_account(
                    directory.path(),
                    &vw_agent_apps::auth::claude_keychain::MacOsKeychain,
                )
                .and_then(|material| material.identity())
                .map_err(|_| AuthErrorCode::KeychainUnavailable)?
                .ok_or(AuthErrorCode::ReauthRequired)?;
                (AppId::Claude, identity)
            }
            AccountCredential::CodexAuth(auth) if scope.service == AuthService::CodexOauth => {
                (AppId::Codex, vw_agent_apps::auth::codex_device::managed_identity(auth)?)
            }
            _ => return Err(AuthErrorCode::Unsupported),
        };
        let actual: [u8; 32] = Sha256::digest(identity.as_bytes()).into();
        if actual != metadata.external_identity_sha256 {
            return Err(AuthErrorCode::IdentityMismatch);
        }
        if !unchanged() {
            return Err(AuthErrorCode::Conflict);
        }
        let launch = vw_agent_apps::apps::launch::account::prepare_account_launch_with_model(
            app,
            credential,
            material,
            Path::new("/tmp"),
            model,
        )
        .map_err(|_| AuthErrorCode::InvalidInput)?;
        Ok(AuthAccountLaunch { launch, lease })
    }

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
