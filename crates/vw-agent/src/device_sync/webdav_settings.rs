//! Host-local WebDAV settings; only ciphertext is persisted alongside the read model.
use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio_util::sync::CancellationToken;
use vw_agent_apps::device_sync::webdav_config::WebDavConfig;
use vw_api_types::{
    device_sync_config::{SyncSecretUpdate, UpdateWebDavSyncConfigRequest, WebDavSyncConfig},
    id::TenantScope,
};
use crate::{
    gateway::settings_store::GatewaySettingsStore,
    global_database::GlobalDatabase,
    security::SecretStore,
};
use super::operation_lock::acquire_sync_lock;

const NAMESPACE: &str = "device_sync.webdav.local.v1";

// Deliberately private and without Debug: neither plaintext nor ciphertext is a read DTO.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredConnection {
    config: WebDavSyncConfig,
    password_ciphertext: Option<String>,
}

pub struct WebDavSettingsStore {
    settings: GatewaySettingsStore,
    secrets: SecretStore,
}

impl WebDavSettingsStore {
    /// root is the existing host secret-store root, supplied by the gateway owner.
    pub fn new(database: GlobalDatabase, root: &Path) -> Result<Self> {
        Ok(Self {
            settings: GatewaySettingsStore::new(database)?,
            secrets: SecretStore::new(root, true),
        })
    }

    /// Scope comes from authorization, never from the update payload.
    pub fn read(&self, tenant: &TenantScope, host: &str) -> Result<Option<WebDavSyncConfig>> {
        let key = scope_key(host)?;
        let raw = self.settings.get(tenant, NAMESPACE, &key)
            .map_err(|_| anyhow::anyhow!("WebDAV settings unavailable"))?;
        raw.as_deref().map(decode).transpose().map(|row| row.map(|row| row.config))
    }

    /// Opaque local revision includes encrypted credential state, never returned to clients.
    pub(crate) fn revision(&self, tenant: &TenantScope, host: &str) -> Result<String> {
        use sha2::{Digest, Sha256};
        let raw = self.settings.get(tenant, NAMESPACE, &scope_key(host)?)?
            .ok_or_else(|| anyhow::anyhow!("device sync not configured"))?;
        let _ = decode(&raw)?;
        Ok(hex::encode(Sha256::digest(raw.as_bytes())))
    }

    /// Resolve only under the shared operation lock; never expose credentials as DTOs.
    pub(crate) fn connection(&self, tenant: &TenantScope, host: &str) -> Result<WebDavConfig> {
        let raw = self.settings.get(tenant, NAMESPACE, &scope_key(host)?)?
            .ok_or_else(|| anyhow::anyhow!("device sync not configured"))?;
        let row = decode(&raw)?;
        ensure!(row.config.enabled, "device sync disabled");
        let encrypted = row.password_ciphertext.ok_or_else(|| anyhow::anyhow!("device sync credential missing"))?;
        let secret = self.secrets.decrypt(&encrypted)
            .map_err(|_| anyhow::anyhow!("device sync credential unavailable"))?;
        WebDavConfig {
            enabled: row.config.enabled,
            auto_sync: row.config.auto_sync,
            base_url: row.config.base_url,
            username: row.config.username,
            remote_root: row.config.remote_root,
            profile: row.config.profile,
            password: secret,
        }.normalize().map_err(anyhow::Error::msg)
    }

    /// Resolve a draft without publishing settings or encrypting replacement material.
    pub fn resolve_probe(&self, tenant: &TenantScope, host: &str,
        request: UpdateWebDavSyncConfigRequest) -> Result<WebDavConfig> {
        let mut config = WebDavConfig {
            enabled: request.enabled,
            auto_sync: request.auto_sync,
            base_url: request.base_url,
            username: request.username,
            remote_root: request.remote_root,
            profile: request.profile,
            password: String::new(),
        }.normalize().map_err(anyhow::Error::msg)?;
        config.password = match request.secret {
            SyncSecretUpdate::Replace { value } => {
                ensure!(!value.is_empty(), "empty replacement credential");
                value
            }
            SyncSecretUpdate::Clear {} => String::new(),
            SyncSecretUpdate::Keep {} => {
                let raw = self.settings.get(tenant, NAMESPACE, &scope_key(host)?)?
                    .ok_or_else(|| anyhow::anyhow!("no saved connection for Keep"))?;
                let old = decode(&raw)?;
                ensure!(old.config.base_url == config.base_url
            && old.config.username == config.username
            && old.config.remote_root == config.remote_root
            && old.config.profile == config.profile, "Keep requires the same connection identity");
                match old.password_ciphertext {
                    Some(value) => self.secrets.decrypt(&value)
                        .map_err(|_| anyhow::anyhow!("credential decryption failed"))?,
                    None => String::new(),
                }
            }
        };
        Ok(config)
    }

    /// Publish the configuration and encrypted password in one conditional database write.
    /// A concurrent process causes an explicit conflict rather than a lost update.
    pub async fn save(
        &self,
        tenant: &TenantScope,
        host: &str,
        request: UpdateWebDavSyncConfigRequest,
        cancellation: &CancellationToken,
    ) -> Result<WebDavSyncConfig> {
        let key = scope_key(host)?;
        let lock_tenant = serde_json::to_string(&(
            tenant.platform_id().as_ref(), tenant.company_id().as_ref(),
        ))?;
        let _guard = acquire_sync_lock(&lock_tenant, host, cancellation).await?;
        let normalized = WebDavConfig {
            enabled: request.enabled,
            auto_sync: request.auto_sync,
            base_url: request.base_url,
            username: request.username,
            password: String::new(),
            remote_root: request.remote_root,
            profile: request.profile,
        }.normalize().map_err(anyhow::Error::msg)?;
        let previous = self.settings.get(tenant, NAMESPACE, &key)
            .map_err(|_| anyhow::anyhow!("WebDAV settings unavailable"))?;
        let old = previous.as_deref().map(decode).transpose()?;
        let same_identity = old.as_ref().is_some_and(|old| {
            old.config.base_url == normalized.base_url
                && old.config.username == normalized.username
                && old.config.remote_root == normalized.remote_root
                && old.config.profile == normalized.profile
        });
        let password_ciphertext = match request.secret {
            SyncSecretUpdate::Keep {} if same_identity => {
                old.and_then(|old| old.password_ciphertext)
            }
            SyncSecretUpdate::Keep {} | SyncSecretUpdate::Clear {} => None,
            SyncSecretUpdate::Replace { value } => {
                ensure!(!value.is_empty(), "empty WebDAV password; use Clear explicitly");
                let encrypted = self.secrets.encrypt(&value)
                    .map_err(|_| anyhow::anyhow!("WebDAV credential encryption failed"))?;
                ensure!(SecretStore::is_secure_encrypted(&encrypted),
                    "WebDAV requires encrypted credentials");
                Some(encrypted)
            }
        };
        let config = WebDavSyncConfig {
            enabled: normalized.enabled,
            auto_sync: normalized.auto_sync,
            base_url: normalized.base_url,
            username: normalized.username,
            remote_root: normalized.remote_root,
            profile: normalized.profile,
            secret_present: password_ciphertext.is_some(),
        };
        let payload = serde_json::to_string(&StoredConnection {
            config: config.clone(), password_ciphertext,
        })?;
        let changed = match previous {
            Some(previous) => self.settings.compare_exchange(
                tenant, NAMESPACE, &key, &previous, &payload,
            ),
            None => self.settings.insert_if_absent(tenant, NAMESPACE, &key, &payload),
        }.map_err(|_| anyhow::anyhow!("WebDAV settings publication failed"))?;
        ensure!(changed, "WebDAV connection changed concurrently; reload before saving");
        Ok(config)
    }
}

fn scope_key(host: &str) -> Result<String> {
    vw_agent_apps::providers::validate_identity(host)
        .map_err(|_| anyhow::anyhow!("invalid authorized WebDAV host"))?;
    Ok(serde_json::to_string(&(host, "default"))?)
}

fn decode(raw: &str) -> Result<StoredConnection> {
    let row: StoredConnection = serde_json::from_str(raw)
        .map_err(|_| anyhow::anyhow!("invalid stored WebDAV settings"))?;
    if row.config.secret_present != row.password_ciphertext.is_some()
        || row.password_ciphertext.as_deref().is_some_and(|value| {
            !SecretStore::is_secure_encrypted(value)
        })
    {
        bail!("invalid stored WebDAV credential state");
    }
    let canonical = WebDavConfig {
        enabled: row.config.enabled,
        auto_sync: row.config.auto_sync,
        base_url: row.config.base_url.clone(),
        username: row.config.username.clone(),
        password: String::new(),
        remote_root: row.config.remote_root.clone(),
        profile: row.config.profile.clone(),
    }.normalize().map_err(|_| anyhow::anyhow!("invalid stored WebDAV connection"))?;
    ensure!(canonical.base_url == row.config.base_url
        && canonical.username == row.config.username
        && canonical.remote_root == row.config.remote_root
        && canonical.profile == row.config.profile,
        "stored WebDAV connection is not canonical");
    Ok(row)
}
