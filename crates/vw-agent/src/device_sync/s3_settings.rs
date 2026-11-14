//! Host-local S3 settings; only ciphertext is persisted alongside the read model.
use super::operation_lock::acquire_sync_lock;
use crate::{
    gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase,
    security::SecretStore,
};
use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio_util::sync::CancellationToken;
use vw_agent_apps::device_sync::s3_config::S3Config;
use vw_api_types::{
    device_sync_config::{S3SyncConfig, SyncSecretUpdate, UpdateS3SyncConfigRequest},
    id::TenantScope,
};

const NAMESPACE: &str = "device_sync.s3.local.v1";

// Deliberately private and without Debug: neither plaintext nor ciphertext is a read DTO.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredConnection {
    config: S3SyncConfig,
    secret_ciphertext: Option<String>,
}

pub struct S3SettingsStore {
    settings: GatewaySettingsStore,
    secrets: SecretStore,
}

impl S3SettingsStore {
    /// root is the existing host secret-store root, supplied by the gateway owner.
    pub fn new(database: GlobalDatabase, root: &Path) -> Result<Self> {
        Ok(Self {
            settings: GatewaySettingsStore::new(database)?,
            secrets: SecretStore::new(root, true),
        })
    }

    /// Scope comes from authorization, never from the update payload.
    pub fn read(&self, tenant: &TenantScope, host: &str) -> Result<Option<S3SyncConfig>> {
        let key = scope_key(host)?;
        let raw = self
            .settings
            .get(tenant, NAMESPACE, &key)
            .map_err(|_| anyhow::anyhow!("S3 settings unavailable"))?;
        raw.as_deref().map(decode).transpose().map(|row| row.map(|row| row.config))
    }

    /// Resolve a draft without publishing settings or encrypting replacement material.
    pub fn resolve_probe(&self, tenant: &TenantScope, host: &str,
        request: UpdateS3SyncConfigRequest) -> Result<S3Config> {
        let mut config = S3Config {
            enabled: false,
            auto_sync: request.auto_sync,
            bucket: request.bucket,
            region: request.region,
            access_key_id: request.access_key_id,
            endpoint: request.endpoint,
            remote_root: request.remote_root,
            profile: request.profile,
            secret_access_key: String::new(),
        }.normalize().map_err(anyhow::Error::msg)?;
        config.secret_access_key = match request.secret {
            SyncSecretUpdate::Replace { value } => {
                ensure!(!value.is_empty(), "empty replacement credential");
                value
            }
            SyncSecretUpdate::Clear {} => String::new(),
            SyncSecretUpdate::Keep {} => {
                let raw = self.settings.get(tenant, NAMESPACE, &scope_key(host)?)?
                    .ok_or_else(|| anyhow::anyhow!("no saved connection for Keep"))?;
                let old = decode(&raw)?;
                ensure!(old.config.bucket == config.bucket
            && old.config.region == config.region
            && old.config.access_key_id == config.access_key_id
            && old.config.endpoint == config.endpoint
            && old.config.remote_root == config.remote_root
            && old.config.profile == config.profile, "Keep requires the same connection identity");
                match old.secret_ciphertext {
                    Some(value) => self.secrets.decrypt(&value)
                        .map_err(|_| anyhow::anyhow!("credential decryption failed"))?,
                    None => String::new(),
                }
            }
        };
        Ok(config)
    }

    /// Publish the configuration and encrypted secret access key in one conditional database write.
    /// A concurrent process causes an explicit conflict rather than a lost update.
    pub async fn save(
        &self,
        tenant: &TenantScope,
        host: &str,
        request: UpdateS3SyncConfigRequest,
        cancellation: &CancellationToken,
    ) -> Result<S3SyncConfig> {
        let key = scope_key(host)?;
        let lock_tenant =
            serde_json::to_string(&(tenant.platform_id().as_ref(), tenant.company_id().as_ref()))?;
        let _guard = acquire_sync_lock(&lock_tenant, host, cancellation).await?;
        // Normalize metadata without inventing a plaintext credential for Keep.
        let normalized = S3Config {
            enabled: false,
            auto_sync: request.auto_sync,
            bucket: request.bucket,
            region: request.region,
            access_key_id: request.access_key_id,
            secret_access_key: String::new(),
            endpoint: request.endpoint,
            remote_root: request.remote_root,
            profile: request.profile,
        }
        .normalize()
        .map_err(anyhow::Error::msg)?;
        let previous = self
            .settings
            .get(tenant, NAMESPACE, &key)
            .map_err(|_| anyhow::anyhow!("S3 settings unavailable"))?;
        let old = previous.as_deref().map(decode).transpose()?;
        let same_identity = old.as_ref().is_some_and(|old| {
            old.config.bucket == normalized.bucket
                && old.config.region == normalized.region
                && old.config.access_key_id == normalized.access_key_id
                && old.config.endpoint == normalized.endpoint
                && old.config.remote_root == normalized.remote_root
                && old.config.profile == normalized.profile
        });
        let secret_ciphertext = match request.secret {
            SyncSecretUpdate::Keep {} if same_identity => old.and_then(|old| old.secret_ciphertext),
            SyncSecretUpdate::Keep {} | SyncSecretUpdate::Clear {} => None,
            SyncSecretUpdate::Replace { value } => {
                ensure!(!value.is_empty(), "empty S3 secret access key; use Clear explicitly");
                let encrypted = self
                    .secrets
                    .encrypt(&value)
                    .map_err(|_| anyhow::anyhow!("S3 credential encryption failed"))?;
                ensure!(
                    SecretStore::is_secure_encrypted(&encrypted),
                    "S3 requires encrypted credentials"
                );
                Some(encrypted)
            }
        };
        ensure!(
            !request.enabled
                || (!normalized.bucket.is_empty()
                    && !normalized.region.is_empty()
                    && !normalized.access_key_id.is_empty()
                    && secret_ciphertext.is_some()),
            "enabled S3 requires bucket, region, access key ID and secret access key"
        );
        ensure!(!cancellation.is_cancelled(), "S3 settings save cancelled");
        let config = S3SyncConfig {
            enabled: request.enabled,
            auto_sync: normalized.auto_sync,
            bucket: normalized.bucket,
            region: normalized.region,
            access_key_id: normalized.access_key_id,
            endpoint: normalized.endpoint,
            remote_root: normalized.remote_root,
            profile: normalized.profile,
            secret_present: secret_ciphertext.is_some(),
        };
        let payload =
            serde_json::to_string(&StoredConnection { config: config.clone(), secret_ciphertext })?;
        let changed = match previous {
            Some(previous) => {
                self.settings.compare_exchange(tenant, NAMESPACE, &key, &previous, &payload)
            }
            None => self.settings.insert_if_absent(tenant, NAMESPACE, &key, &payload),
        }
        .map_err(|_| anyhow::anyhow!("S3 settings publication failed"))?;
        ensure!(changed, "S3 connection changed concurrently; reload before saving");
        Ok(config)
    }
}

fn scope_key(host: &str) -> Result<String> {
    vw_agent_apps::providers::validate_identity(host)
        .map_err(|_| anyhow::anyhow!("invalid authorized S3 host"))?;
    Ok(serde_json::to_string(&(host, "default"))?)
}

fn decode(raw: &str) -> Result<StoredConnection> {
    let row: StoredConnection =
        serde_json::from_str(raw).map_err(|_| anyhow::anyhow!("invalid stored S3 settings"))?;
    if row.config.secret_present != row.secret_ciphertext.is_some()
        || row
            .secret_ciphertext
            .as_deref()
            .is_some_and(|value| !SecretStore::is_secure_encrypted(value))
    {
        bail!("invalid stored S3 credential state");
    }
    let canonical = S3Config {
        enabled: false,
        auto_sync: row.config.auto_sync,
        bucket: row.config.bucket.clone(),
        region: row.config.region.clone(),
        access_key_id: row.config.access_key_id.clone(),
        secret_access_key: String::new(),
        endpoint: row.config.endpoint.clone(),
        remote_root: row.config.remote_root.clone(),
        profile: row.config.profile.clone(),
    }
    .normalize()
    .map_err(|_| anyhow::anyhow!("invalid stored S3 connection"))?;
    ensure!(
        canonical.bucket == row.config.bucket
            && canonical.region == row.config.region
            && canonical.access_key_id == row.config.access_key_id
            && canonical.endpoint == row.config.endpoint
            && canonical.remote_root == row.config.remote_root
            && canonical.profile == row.config.profile,
        "stored S3 connection is not canonical"
    );
    ensure!(
        !row.config.enabled
            || (!canonical.bucket.is_empty()
                && !canonical.region.is_empty()
                && !canonical.access_key_id.is_empty()
                && row.secret_ciphertext.is_some()),
        "invalid enabled S3 connection"
    );
    Ok(row)
}
