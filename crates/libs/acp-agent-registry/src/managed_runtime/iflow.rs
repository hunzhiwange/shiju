//! Artifact identity shared by Linux iFlow installation, discovery and launch.
use super::Contract;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, os::unix::fs::PermissionsExt, path::Path};

pub(super) const MANIFEST: &str =
    include_str!("../../../../../patches/acp-model-source/iflow/linux-artifacts.json");

#[derive(Deserialize)]
pub(super) struct Manifest {
    pub(super) revision: String,
    package: String,
    version: String,
    bin: String,
    lock_sha256: String,
    patched_files: BTreeMap<String, String>,
    targets: BTreeMap<String, Target>,
}

#[derive(Deserialize)]
struct Target {
    ripgrep: String,
    ripgrep_sha256: String,
    pty_package: String,
    pty_sha256: String,
}

impl Manifest {
    pub(super) fn load() -> Result<Self, String> {
        serde_json::from_str(MANIFEST).map_err(|_| "Invalid iFlow Linux artifact manifest".into())
    }
}

pub(super) fn verify(runtime: &Path, contract: &Contract) -> Result<(), String> {
    let manifest = Manifest::load()?;
    if manifest.package != contract.package
        || manifest.version != contract.version
        || manifest.bin != contract.bin
        || !manifest.patched_files.contains_key(&manifest.bin)
    {
        return Err("iFlow Linux contract identity mismatch".into());
    }
    let target =
        manifest.targets.get(std::env::consts::ARCH).ok_or("Missing iFlow Linux target")?;
    let parent = runtime.parent().ok_or("Missing iFlow runtime parent")?;
    let directory = runtime.file_name().ok_or("Missing iFlow runtime directory")?;
    let expected =
        parent.canonicalize().map_err(|_| "Missing iFlow runtime parent")?.join(directory);
    let modules = expected.join("node_modules");
    let package = modules.join(&manifest.package);
    let metadata = package.join("package.json");
    if metadata.canonicalize().ok().as_ref() != Some(&metadata) {
        return Err("iFlow Linux package metadata is missing or redirected".into());
    }
    for (path, digest) in [
        (expected.join("package-lock.json"), &manifest.lock_sha256),
        (package.join(&target.ripgrep), &target.ripgrep_sha256),
        (modules.join(&target.pty_package).join("pty.node"), &target.pty_sha256),
    ]
    .into_iter()
    .chain(manifest.patched_files.iter().map(|(file, hash)| (package.join(file), hash)))
    {
        if path.canonicalize().ok().as_ref() != Some(&path)
            || !path.symlink_metadata().is_ok_and(|meta| meta.is_file())
        {
            return Err("iFlow Linux artifact is missing or redirected".into());
        }
        let bytes = fs::read(path).map_err(|_| "Cannot read iFlow Linux artifact")?;
        if hex::encode(Sha256::digest(bytes)) != *digest {
            return Err("iFlow Linux artifact checksum mismatch".into());
        }
    }
    let rg = fs::metadata(package.join(&target.ripgrep)).map_err(|_| "Missing iFlow ripgrep")?;
    let shim = modules.join(".bin/iflow");
    if rg.permissions().mode() & 0o111 == 0
        || !shim.metadata().is_ok_and(|meta| meta.permissions().mode() & 0o111 != 0)
        || shim.canonicalize().ok() != Some(package.join(&manifest.bin))
    {
        return Err("iFlow Linux executable does not match the reviewed entry".into());
    }
    Ok(())
}
