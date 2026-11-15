//! Read-only S3 previews; Gateway authorizes network use and supplies credentials.

use super::{
    artifact::MAX_ARTIFACT_SIZE,
    manifest::{PROTOCOL_VERSION, SyncManifest},
    s3_config::S3Config,
    s3_transport::{S3DownloadError, S3DownloadTarget},
};

pub use super::webdav_remote::{RemoteCompatibility, RemoteSnapshot, RemoteSnapshotQuery};

/// Fetch only the fixed native manifest, bounded by the download transport.
/// Only HTTP 404 is empty; permission and transport errors remain errors.
/// Unsupported positive versions remain displayable, without fetching artifacts.
/// The supported data version must come from the shared snapshot owner.
pub async fn query_remote_snapshot(
    config: S3Config,
    supported_data_version: u32,
) -> Result<RemoteSnapshotQuery, S3DownloadError> {
    let target = S3DownloadTarget::new(config, supported_data_version)?;
    let manifest_path = target.manifest_path()?;
    let Some(download) = target.download_manifest().await? else {
        return Ok(RemoteSnapshotQuery::Empty);
    };
    let manifest = match SyncManifest::inspect_slice(&download.bytes) {
        Ok(manifest) => manifest,
        Err(reason) => return Ok(RemoteSnapshotQuery::InvalidManifest { reason }),
    };
    if manifest.protocol_version == 0 || manifest.data_compat_version == 0 {
        return Ok(RemoteSnapshotQuery::InvalidManifest {
            reason: "Invalid sync manifest version",
        });
    }
    if manifest.artifacts.config.size > MAX_ARTIFACT_SIZE
        || manifest.artifacts.skills.size > MAX_ARTIFACT_SIZE
    {
        return Ok(RemoteSnapshotQuery::InvalidManifest {
            reason: "Artifact declared size exceeds limit",
        });
    }
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
