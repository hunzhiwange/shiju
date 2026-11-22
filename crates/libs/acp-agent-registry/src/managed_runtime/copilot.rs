//! Verify the extracted, architecture-specific Copilot ACP runtime on GNU Linux.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Read,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use super::Contract;

pub(super) const LINUX_ARTIFACTS: &str =
    include_str!("../../../../../patches/acp-model-source/copilot/linux-artifacts.json");

#[derive(Deserialize)]
struct Manifest {
    package: String,
    version: String,
    integrity: String,
    revision: String,
    bin: String,
    files: BTreeMap<String, String>,
    targets: BTreeMap<String, Target>,
}

#[derive(Deserialize)]
struct Target {
    package: String,
    integrity: String,
    files: BTreeMap<String, String>,
}

fn manifest() -> Result<Manifest, String> {
    serde_json::from_str(LINUX_ARTIFACTS).map_err(|_| "Invalid Copilot Linux manifest".into())
}

pub(super) fn installed_directory(parent: &Path) -> Result<PathBuf, String> {
    Ok(parent.join(manifest()?.revision))
}

/// Use for staging, discovery and managed launch. Return the fixed npm alias
/// because the model driver checks the Copilot executable name.
pub(super) fn verify(directory: &Path, contract: &Contract) -> Result<PathBuf, String> {
    let manifest = manifest()?;
    if manifest.package != contract.package
        || manifest.version != contract.version
        || manifest.bin != contract.bin
    {
        return Err("Copilot Linux manifest differs from the reviewed package".into());
    }
    let target = manifest
        .targets
        .get(std::env::consts::ARCH)
        .ok_or("Unsupported Copilot Linux architecture")?;

    let parent = directory.parent().ok_or("Missing Copilot runtime parent")?;
    let name = directory.file_name().ok_or("Missing Copilot runtime directory")?;
    let root = parent.canonicalize().map_err(|_| "Missing Copilot runtime parent")?.join(name);
    let lock = root.join("package-lock.json");
    verify_path(&lock, false)?;
    let lock: serde_json::Value =
        serde_json::from_slice(&fs::read(lock).map_err(|_| "Missing Copilot npm lockfile")?)
            .map_err(|_| "Invalid Copilot npm lockfile")?;
    let packages = &lock["packages"];
    for (package, integrity) in
        [(&manifest.package, &manifest.integrity), (&target.package, &target.integrity)]
    {
        let key = format!("node_modules/{package}");
        let entry = &packages[key.as_str()];
        if entry["version"].as_str() != Some(manifest.version.as_str())
            || entry["integrity"].as_str() != Some(integrity.as_str())
        {
            return Err("Copilot Linux npm package identity differs".into());
        }
    }

    let package = root.join("node_modules").join(&contract.package);
    for (name, expected) in manifest.files.iter().chain(&target.files) {
        let executable = name == &manifest.bin
            || name.ends_with("/copilot-runtime")
            || name.ends_with("/rg")
            || name.ends_with("/tgrep");
        verify_file(&package.join(name), expected, executable)?;
    }
    let alias = root.join("node_modules/.bin/copilot");
    let alias_dir = root.join("node_modules/.bin");
    let entry = package.join(&manifest.bin);
    if alias_dir.canonicalize().ok().as_deref() != Some(alias_dir.as_path())
        || alias.read_link().ok().as_deref()
            != Some(Path::new("../@github/copilot/embedded/index.js"))
        || alias.canonicalize().ok().as_deref() != Some(entry.as_path())
    {
        return Err("Copilot Linux executable does not resolve to its reviewed entry".into());
    }
    Ok(alias)
}

fn verify_path(path: &Path, executable: bool) -> Result<(), String> {
    if path.canonicalize().ok().as_deref() != Some(path) {
        return Err("Copilot Linux artifact is missing or redirected".into());
    }
    let metadata = path.metadata().map_err(|_| "Cannot stat Copilot Linux artifact")?;
    if !metadata.is_file() || (executable && metadata.permissions().mode() & 0o111 == 0) {
        return Err("Invalid Copilot Linux artifact permissions".into());
    }
    Ok(())
}

fn verify_file(path: &Path, expected: &str, executable: bool) -> Result<(), String> {
    verify_path(path, executable)?;
    let mut file = File::open(path).map_err(|_| "Cannot open Copilot Linux artifact")?;
    let mut hash = Sha256::new();
    let mut block = [0; 64 * 1024];
    loop {
        let size = file.read(&mut block).map_err(|_| "Cannot hash Copilot Linux artifact")?;
        if size == 0 {
            break;
        }
        hash.update(&block[..size]);
    }
    if hex::encode(hash.finalize()) != expected {
        return Err("Copilot Linux artifact checksum differs".into());
    }
    Ok(())
}
