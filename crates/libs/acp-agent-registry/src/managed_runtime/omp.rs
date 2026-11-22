//! Identity checks for the pinned Linux OMP standalone executable.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::Read,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

pub(super) const MANIFEST: &str =
    include_str!("../../../../../patches/acp-model-source/omp/linux.json");

#[derive(Deserialize)]
struct Manifest {
    targets: BTreeMap<String, Artifact>,
}

#[derive(Deserialize)]
struct Artifact {
    sha256: String,
    size: u64,
}

pub(super) fn verify(directory: &Path) -> Result<PathBuf, String> {
    let target = match std::env::consts::ARCH {
        "x86_64" => "x86_64-unknown-linux-gnu",
        "aarch64" => "aarch64-unknown-linux-gnu",
        _ => return Err("Unsupported managed OMP Linux architecture".into()),
    };
    let manifest: Manifest =
        serde_json::from_str(MANIFEST).map_err(|_| "Invalid reviewed OMP Linux manifest")?;
    let artifact = manifest.targets.get(target).ok_or("Missing reviewed OMP Linux target")?;
    let parent = directory.parent().ok_or("Missing managed OMP runtime parent")?;
    let name = directory.file_name().ok_or("Missing managed OMP runtime directory")?;
    let parent = parent.canonicalize().map_err(|_| "Missing managed OMP runtime parent")?;
    let entry = parent.join(name).join("omp");
    if entry.canonicalize().ok().as_deref() != Some(&entry) {
        return Err("Reviewed OMP Linux executable is missing or redirected".into());
    }
    let metadata = entry.symlink_metadata().map_err(|_| "Cannot stat OMP Linux executable")?;
    if !metadata.is_file()
        || metadata.permissions().mode() & 0o111 == 0
        || metadata.len() != artifact.size
    {
        return Err("Invalid reviewed OMP Linux executable metadata".into());
    }
    let mut file =
        File::open(&entry).map_err(|_| "Cannot open OMP Linux executable")?.take(artifact.size + 1);
    let mut digest = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    let mut size = 0;
    loop {
        let count = file.read(&mut buffer).map_err(|_| "Cannot hash OMP Linux executable")?;
        if count == 0 {
            break;
        }
        size += count as u64;
        digest.update(&buffer[..count]);
    }
    if size != artifact.size || hex::encode(digest.finalize()) != artifact.sha256 {
        return Err("Reviewed OMP Linux executable checksum mismatch".into());
    }
    Ok(entry)
}
