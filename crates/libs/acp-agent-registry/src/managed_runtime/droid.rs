//! Pinned executable identity for the Linux Droid runtime.
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::Read,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

const MANIFEST: &str = include_str!("../../../../../patches/acp-model-source/droid/linux.json");

#[derive(Deserialize)]
struct Manifest {
    revision: String,
    bin: String,
    targets: BTreeMap<String, Target>,
}

#[derive(Deserialize)]
struct Target {
    patched_sha256: String,
}

/// Verify the selected installed binary from the installer, discovery, or launch.
pub(super) fn verify(directory: &Path) -> Result<PathBuf, String> {
    let manifest: Manifest =
        serde_json::from_str(MANIFEST).map_err(|_| "Invalid Droid Linux artifact manifest")?;
    let target = manifest
        .targets
        .get(std::env::consts::ARCH)
        .ok_or("Missing reviewed Droid Linux target")?;
    if directory.file_name().and_then(|name| name.to_str()) != Some(manifest.revision.as_str()) {
        return Err("Unexpected Droid Linux runtime directory".into());
    }
    let parent = directory
        .parent()
        .ok_or("Missing Droid Linux runtime parent")?
        .canonicalize()
        .map_err(|_| "Missing Droid Linux runtime parent")?;
    let path = parent.join(&manifest.revision).join(&manifest.bin);
    if path.canonicalize().ok().as_deref() != Some(path.as_path()) {
        return Err("Droid Linux executable is missing or redirected".into());
    }
    let metadata = path.metadata().map_err(|_| "Cannot stat Droid Linux executable")?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
        return Err("Invalid Droid Linux executable permissions".into());
    }
    let mut file = File::open(&path).map_err(|_| "Cannot open Droid Linux executable")?;
    let mut digest = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|_| "Cannot hash Droid Linux executable")?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    if hex::encode(digest.finalize()) != target.patched_sha256 {
        return Err("Droid Linux executable checksum mismatch".into());
    }
    Ok(path)
}
