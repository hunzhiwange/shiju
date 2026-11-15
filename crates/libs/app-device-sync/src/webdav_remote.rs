//! Read-only remote previews. Gateway supplies credentials and authorizes network use.

use super::{
    manifest::{PROTOCOL_VERSION, SyncManifest},
    webdav_config::WebDavConfig,
    webdav_transport::{WebDavError, download::WebDavDownloadTarget},
    webdav_url::build_artifact_url,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteCompatibility {
    Compatible,
    UnsupportedProtocol,
    UnsupportedDataVersion,
}

/// Metadata is an observation, not proof of artifact integrity or a write lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteSnapshot {
    pub device_name: String,
    pub created_at: String,
    pub snapshot_id: String,
    pub protocol_version: u32,
    pub data_compat_version: u32,
    pub compatibility: RemoteCompatibility,
    /// Encoded native manifest path only; excludes host, credentials and query.
    pub manifest_path: String,
    pub etag: String,
    pub manifest_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteSnapshotQuery {
    Empty,
    InvalidManifest { reason: &'static str },
    Snapshot(RemoteSnapshot),
}

/// Reads one bounded manifest GET, whose response already supplies the ETag.
/// No artifact HEAD is needed for this preview; no config/skills are downloaded.
/// Only manifest 404 is empty. Transport/authentication failures remain errors.
/// The version comes from the shared snapshot owner; no legacy paths are probed.
pub async fn query_remote_snapshot(
    config: WebDavConfig,
    supported_data_version: u32,
) -> Result<RemoteSnapshotQuery, WebDavError> {
    let config = config.normalize().map_err(query_error)?;
    let manifest_path = build_artifact_url(
        &config.base_url,
        &config.remote_root,
        &config.profile,
        supported_data_version,
        "manifest.json",
    )
    .map_err(query_error)?
    .path()
    .to_owned();
    let target = WebDavDownloadTarget::new(config, supported_data_version)?;
    let Some(download) = target.download_manifest().await? else {
        return Ok(RemoteSnapshotQuery::Empty);
    };
    let manifest = match SyncManifest::inspect_slice(&download.bytes) {
        Ok(manifest) => manifest,
        Err(reason) => return Ok(RemoteSnapshotQuery::InvalidManifest { reason }),
    };
    let compatibility = if manifest.protocol_version != PROTOCOL_VERSION {
        RemoteCompatibility::UnsupportedProtocol
    } else if manifest.data_compat_version != supported_data_version {
        RemoteCompatibility::UnsupportedDataVersion
    } else {
        RemoteCompatibility::Compatible
    };
    Ok(RemoteSnapshotQuery::Snapshot(RemoteSnapshot {
        device_name: manifest.device_name,
        created_at: manifest.created_at,
        snapshot_id: manifest.snapshot_id,
        protocol_version: manifest.protocol_version,
        data_compat_version: manifest.data_compat_version,
        compatibility,
        manifest_path,
        etag: download.metadata.etag,
        manifest_sha256: {
            use sha2::{Digest, Sha256};
            format!("{:x}", Sha256::digest(&download.bytes))
        },
    }))
}

fn query_error(message: &'static str) -> WebDavError {
    WebDavError { operation: "query", status: None, message }
}
