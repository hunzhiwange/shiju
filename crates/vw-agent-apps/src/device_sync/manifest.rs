//! Closed native wire format. Compatibility is supplied by the snapshot owner.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const FORMAT: &str = "shiju-device-sync";
pub const PROTOCOL_VERSION: u32 = 1;
pub const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
pub const CONFIG_ARTIFACT: &str = "config.json";
pub const SKILLS_ARTIFACT: &str = "skills.zip";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyncArtifact {
    pub sha256: String,
    pub size: u64,
}

/// Required fields close the set and let serde reject missing/duplicate names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyncArtifacts {
    #[serde(rename = "config.json")]
    pub config: SyncArtifact,
    #[serde(rename = "skills.zip")]
    pub skills: SyncArtifact,
}

impl SyncArtifacts {
    /// Lexicographic artifact order is part of the protocol, independent of JSON order.
    pub fn snapshot_id(&self) -> Result<String, &'static str> {
        if !is_sha256(&self.config.sha256) || !is_sha256(&self.skills.sha256) {
            return Err("Invalid artifact SHA256");
        }
        let identity = format!(
            "{CONFIG_ARTIFACT}:{}|{SKILLS_ARTIFACT}:{}",
            self.config.sha256, self.skills.sha256
        );
        Ok(format!("{:x}", Sha256::digest(identity.as_bytes())))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyncManifest {
    pub format: String,
    pub protocol_version: u32,
    pub data_compat_version: u32,
    pub device_name: String,
    pub created_at: String,
    pub artifacts: SyncArtifacts,
    pub snapshot_id: String,
}

impl SyncManifest {
    /// Network callers must also cap the response while reading, before buffering.
    /// Never substitutes a guessed compatibility version when the owner is absent.
    pub fn from_slice(
        bytes: &[u8],
        supported_data_version: u32,
    ) -> Result<Self, &'static str> {
        if bytes.len() > MAX_MANIFEST_BYTES {
            return Err("Manifest exceeds 1 MiB");
        }
        let manifest: Self =
            serde_json::from_slice(bytes).map_err(|_| "Invalid native sync manifest")?;
        manifest.normalize(supported_data_version)
    }

    /// Also validate locally constructed manifests before publication.
    pub fn normalize(mut self, supported_data_version: u32) -> Result<Self, &'static str> {
        if self.format != FORMAT {
            return Err("Unsupported sync manifest format");
        }
        if self.protocol_version != PROTOCOL_VERSION {
            return Err("Unsupported sync protocol version");
        }
        if self.data_compat_version != supported_data_version {
            return Err("Unsupported sync data compatibility version");
        }
        if !is_sha256(&self.snapshot_id) || self.snapshot_id != self.artifacts.snapshot_id()? {
            return Err("Invalid sync snapshot identity");
        }
        if chrono::DateTime::parse_from_rfc3339(&self.created_at).is_err() {
            return Err("Invalid sync creation time");
        }
        self.device_name = self
            .device_name
            .chars()
            .filter(|c| !c.is_control())
            .take(64)
            .collect();
        // Size limits apply to locally built values as well as downloaded bytes.
        let encoded = serde_json::to_vec(&self).map_err(|_| "Invalid native sync manifest")?;
        if encoded.len() > MAX_MANIFEST_BYTES {
            return Err("Manifest exceeds 1 MiB");
        }
        Ok(self)
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
