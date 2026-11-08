//! Scoped bindings for stock Codex cards consumed by the existing isolated launcher.
use super::*;
use crate::agent_apps::auth_accounts::{AuthAccountScope, AuthAccountStore};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use vw_agent_apps::auth::{
    codex_device::managed_identity, credential_file::AccountCredential,
    credential_lease::CredentialLease,
};
use vw_api_types::{
    agent_apps_auth::{AuthErrorCode, AuthService},
    app_providers::codex_catalog::CodexProfile,
};

const FIELD: &str = "managedCodexLogin";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CodexBinding {
    host: String,
    principal: String,
    account_id: String,
}

impl AppProviderStore {
    pub fn bind_codex_account(
        &self,
        scope: AuthAccountScope<'_>,
        root: &Path,
        provider_id: &str,
        expected_account: Option<&str>,
        account_id: Option<&str>,
    ) -> Result<(), AuthErrorCode> {
        check_scope(scope)?;
        let key = Self::key(AppId::Codex, provider_id).map_err(|_| AuthErrorCode::InvalidInput)?;
        let raw = self
            .settings
            .get(scope.tenant, NAMESPACE, &key)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?
            .ok_or(AuthErrorCode::NotFound)?;
        let mut row: Value =
            serde_json::from_str(&raw).map_err(|_| AuthErrorCode::StorageUnavailable)?;
        official(&row)?;
        let current = account(&row, scope)?.map(|(id, _)| id);
        if current.as_deref() == account_id {
            return Ok(());
        }
        if current.as_deref() != expected_account {
            return Err(AuthErrorCode::Conflict);
        }
        let accounts = AuthAccountStore::new(self.database.clone(), root)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let _old_lease =
            current.as_deref().map(|id| accounts.managed_official_lease(scope, id)).transpose()?;
        let _new_lease = if let Some(id) = account_id {
            let lease = accounts.managed_official_lease(scope, id)?;
            let AccountCredential::CodexAuth(auth) =
                lease.read().map_err(|_| AuthErrorCode::ReauthRequired)?
            else {
                return Err(AuthErrorCode::ReauthRequired);
            };
            let identity: [u8; 32] = Sha256::digest(managed_identity(&auth)?.as_bytes()).into();
            if accounts.read_metadata(&scope, id)?.external_identity_sha256 != identity {
                return Err(AuthErrorCode::IdentityMismatch);
            }
            row[FIELD] = serde_json::to_value(CodexBinding {
                host: scope.host.into(),
                principal: scope.principal.into(),
                account_id: id.into(),
            })
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
            Some(lease)
        } else {
            row.as_object_mut().ok_or(AuthErrorCode::StorageUnavailable)?.remove(FIELD);
            None
        };
        self.settings
            .compare_exchange(scope.tenant, NAMESPACE, &key, &raw, &row.to_string())
            .map_err(|_| AuthErrorCode::StorageUnavailable)?
            .then_some(())
            .ok_or(AuthErrorCode::Conflict)
    }

    pub fn codex_account_binding(
        &self,
        scope: AuthAccountScope<'_>,
        id: &str,
    ) -> Result<Option<String>, AuthErrorCode> {
        check_scope(scope)?;
        let row = self
            .codex_record(scope.tenant, id)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?
            .ok_or(AuthErrorCode::NotFound)?;
        Ok(account(&row, scope)?.map(|(id, _)| id))
    }

    pub fn with_codex_provider_unreferenced<T>(
        &self,
        scope: AuthAccountScope<'_>,
        root: &Path,
        account_id: &str,
        mutate: impl FnOnce(&CredentialLease) -> Result<T, AuthErrorCode>,
    ) -> Result<T, AuthErrorCode> {
        check_scope(scope)?;
        let accounts = AuthAccountStore::new(self.database.clone(), root)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let lease = accounts.managed_official_lease(scope, account_id)?;
        for row in self
            .settings
            .list(scope.tenant, NAMESPACE)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?
        {
            if !row.config_key.starts_with("\"codex\":") {
                continue;
            }
            let row: Value = serde_json::from_str(&row.payload_json)
                .map_err(|_| AuthErrorCode::StorageUnavailable)?;
            if let Some(reference) = binding(&row)?
                && reference.host == scope.host
                && reference.principal == scope.principal
                && reference.account_id == account_id
            {
                return Err(AuthErrorCode::AccountInUse);
            }
        }
        mutate(&lease)
    }
}

pub(super) fn account(
    row: &Value,
    scope: AuthAccountScope<'_>,
) -> Result<Option<(String, Option<String>)>, AuthErrorCode> {
    let Some(binding) = binding(row)? else {
        return Ok(None);
    };
    if binding.host != scope.host || binding.principal != scope.principal {
        return Err(AuthErrorCode::Forbidden);
    }
    Ok(Some((binding.account_id, official(row)?)))
}

pub(super) fn saved_reference(row: &Value) -> Option<&Value> {
    row.get(FIELD)
}

pub(super) fn preserve_reference(reference: Option<Value>, row: &mut Value) -> Result<(), String> {
    if let Some(reference) = reference {
        official(row).map_err(
            |_| "Codex managed binding must be removed before replacing the official profile",
        )?;
        row[FIELD] = reference;
    }
    Ok(())
}

pub(crate) fn has_codex_binding(row: &Value) -> bool {
    row.get(FIELD).is_some()
}

fn binding(row: &Value) -> Result<Option<CodexBinding>, AuthErrorCode> {
    row.get(FIELD)
        .map(|raw| {
            serde_json::from_value(raw.clone()).map_err(|_| AuthErrorCode::StorageUnavailable)
        })
        .transpose()
}

fn official(row: &Value) -> Result<Option<String>, AuthErrorCode> {
    let profile: CodexProfile =
        serde_json::from_value(row.get("codex").cloned().ok_or(AuthErrorCode::Unsupported)?)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
    let config: toml::Table = profile.config.parse().map_err(|_| AuthErrorCode::Unsupported)?;
    if profile.api_key.is_some()
        || config.get("model_provider").is_some_and(|value| value.as_str() != Some("openai"))
        || config
            .get("cli_auth_credentials_store")
            .is_some_and(|value| value.as_str() != Some("file"))
        || config.keys().any(|key| {
            !matches!(key.as_str(), "model" | "model_provider" | "cli_auth_credentials_store")
        })
        || row.get("modelCatalogRevision").is_some_and(|value| !value.is_null())
        || row.get("requestOptions").is_some_and(|value| !value.is_null())
        || row.pointer("/meta/authBinding").is_some()
        || row.pointer("/meta/codexAccountId").is_some()
        || row.pointer("/meta/liveConfigManaged").is_some_and(|value| value == true)
        || row.get("settingsConfig")
            != Some(&vw_agent_apps::providers::codex_catalog::settings(&profile))
    {
        return Err(AuthErrorCode::Unsupported);
    }
    config
        .get("model")
        .map(|model| {
            model
                .as_str()
                .filter(|model| {
                    !model.is_empty()
                        && model.len() <= 256
                        && !model.starts_with('-')
                        && !model.chars().any(char::is_control)
                })
                .map(str::to_owned)
                .ok_or(AuthErrorCode::Unsupported)
        })
        .transpose()
}

fn check_scope(scope: AuthAccountScope<'_>) -> Result<(), AuthErrorCode> {
    scope.validate()?;
    if scope.service != AuthService::CodexOauth {
        return Err(AuthErrorCode::Unsupported);
    }
    Ok(())
}
