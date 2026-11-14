//! Bounded, incremental integrity checks for downloaded synchronization artifacts.

use sha2::{Digest, Sha256};

pub const MAX_ARTIFACT_SIZE: u64 = 512 * 1024 * 1024;

/// A manifest declaration, independent of transport Content-Length.
#[derive(Clone)]
pub struct ArtifactDeclaration {
    pub name: String,
    pub sha256: String,
    pub size: u64,
}

impl ArtifactDeclaration {
    fn validate(&self) -> Result<(), &'static str> {
        if !matches!(self.name.as_str(), "config.json" | "skills.zip") {
            return Err("unsupported synchronization artifact");
        }
        if self.size > MAX_ARTIFACT_SIZE {
            return Err("artifact declared size exceeds limit");
        }
        if self.sha256.len() != 64
            || !self.sha256.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("artifact SHA256 must be lowercase hexadecimal");
        }
        Ok(())
    }
}

/// Only `finish` can produce this proof. It covers the bytes fed to the verifier,
/// not a mutable path: consumers must apply those same staged bytes after EOF.
pub struct VerifiedArtifact {
    declaration: ArtifactDeclaration,
}

impl VerifiedArtifact {
    pub fn declaration(&self) -> &ArtifactDeclaration {
        &self.declaration
    }
}

/// Feed each received chunk before storing it. After any error this verifier is
/// permanently invalid, even if the caller accidentally attempts to finish it.
pub struct ArtifactVerifier {
    declaration: ArtifactDeclaration,
    received: u64,
    hasher: Sha256,
    failed: bool,
}

impl ArtifactVerifier {
    pub fn new(
        declaration: ArtifactDeclaration,
        content_length: Option<u64>,
    ) -> Result<Self, &'static str> {
        declaration.validate()?;
        if let Some(length) = content_length {
            if length > MAX_ARTIFACT_SIZE || length != declaration.size {
                return Err("artifact Content-Length disagrees with declaration");
            }
        }
        Ok(Self {
            declaration,
            received: 0,
            hasher: Sha256::new(),
            failed: false,
        })
    }

    pub fn update(&mut self, chunk: &[u8]) -> Result<(), &'static str> {
        if self.failed {
            return Err("artifact verification already failed");
        }
        let length = u64::try_from(chunk.len())
            .ok()
            .and_then(|length| self.received.checked_add(length));
        let Some(length) = length.filter(|length| {
            *length <= MAX_ARTIFACT_SIZE && *length <= self.declaration.size
        }) else {
            self.failed = true;
            return Err("artifact received size exceeds declaration or limit");
        };
        self.hasher.update(chunk);
        self.received = length;
        Ok(())
    }

    /// Call only after a successful transport EOF; interrupted downloads must
    /// discard the verifier and staging file rather than enter the apply phase.
    pub fn finish(self) -> Result<VerifiedArtifact, &'static str> {
        if self.failed {
            return Err("artifact verification already failed");
        }
        if self.received != self.declaration.size {
            return Err("artifact download is truncated");
        }
        let digest = format!("{:x}", self.hasher.finalize());
        if digest != self.declaration.sha256 {
            return Err("artifact SHA256 mismatch");
        }
        Ok(VerifiedArtifact { declaration: self.declaration })
    }
}

/// Stable content identity of the two fixed artifacts, independent of timestamps
/// and device display names. This validates declarations, not downloaded bytes.
pub fn compute_snapshot_id(artifacts: &[ArtifactDeclaration]) -> Result<String, &'static str> {
    if artifacts.len() != 2 {
        return Err("snapshot requires config.json and skills.zip");
    }
    for artifact in artifacts {
        artifact.validate()?;
    }
    let mut sorted = artifacts.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| left.name.cmp(&right.name));
    if sorted[0].name == sorted[1].name {
        return Err("snapshot contains duplicate artifacts");
    }
    let artifacts = super::manifest::SyncArtifacts {
        config: super::manifest::SyncArtifact {
            sha256: sorted[0].sha256.clone(),
            size: sorted[0].size,
        },
        skills: super::manifest::SyncArtifact {
            sha256: sorted[1].sha256.clone(),
            size: sorted[1].size,
        },
    };
    artifacts.snapshot_id()
}
