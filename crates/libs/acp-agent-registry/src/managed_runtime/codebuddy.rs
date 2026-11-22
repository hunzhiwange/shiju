//! Additional artifact checks for the fixed CodeBuddy npm runtime on Linux GNU.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Read,
    os::unix::fs::PermissionsExt,
    path::Path,
};

use super::Contract;

pub(super) const LINUX_ARTIFACTS: &str =
    include_str!("../../../../../patches/acp-model-source/codebuddy/linux-artifacts.json");

#[derive(Deserialize)]
struct Manifest {
    package: String,
    version: String,
    integrity: String,
    common_dependencies: Vec<Dependency>,
    targets: BTreeMap<String, Target>,
}

#[derive(Deserialize)]
struct Target {
    package_files: BTreeMap<String, String>,
    package_executables: Vec<String>,
    dependencies: Vec<Dependency>,
}

#[derive(Deserialize)]
struct Dependency {
    package: String,
    version: String,
    integrity: String,
    files: BTreeMap<String, String>,
    executables: Vec<String>,
}

/// Use after the base contract validates the patched npm entry and bundles.
pub(super) fn verify(directory: &Path, contract: &Contract) -> Result<(), String> {
    let manifest: Manifest =
        serde_json::from_str(LINUX_ARTIFACTS).map_err(|_| "Invalid CodeBuddy Linux manifest")?;
    let target = manifest
        .targets
        .get(std::env::consts::ARCH)
        .ok_or("Unsupported CodeBuddy Linux architecture")?;
    if (manifest.package.as_str(), manifest.version.as_str())
        != (contract.package.as_str(), contract.version.as_str())
    {
        return Err("CodeBuddy Linux manifest differs from the reviewed package".into());
    }
    let parent = directory.parent().ok_or("Missing CodeBuddy runtime parent")?;
    let name = directory.file_name().ok_or("Missing CodeBuddy runtime directory")?;
    let root = parent.canonicalize().map_err(|_| "Missing CodeBuddy runtime parent")?.join(name);
    let lock = root.join("package-lock.json");
    verify_path(&lock, false)?;
    let lock: serde_json::Value =
        serde_json::from_slice(&fs::read(lock).map_err(|_| "Missing CodeBuddy npm lockfile")?)
            .map_err(|_| "Invalid CodeBuddy npm lockfile")?;
    let packages = &lock["packages"];
    check_lock(packages, &manifest.package, &manifest.version, &manifest.integrity)?;

    let modules = root.join("node_modules");
    let package = modules.join(&manifest.package);
    for name in contract.files.keys() {
        verify_path(&package.join(name), name == &contract.bin)?;
    }
    for (name, hash) in &target.package_files {
        verify_file(&package.join(name), hash, target.package_executables.contains(name))?;
    }
    for dependency in manifest.common_dependencies.iter().chain(&target.dependencies) {
        check_lock(packages, &dependency.package, &dependency.version, &dependency.integrity)?;
        let path = modules.join(&dependency.package);
        for (name, hash) in &dependency.files {
            verify_file(&path.join(name), hash, dependency.executables.contains(name))?;
        }
    }
    // The reviewed npm layout resolves helpers from the installation root.
    for package in [package, modules.join("@lydell/node-pty")] {
        match fs::symlink_metadata(package.join("node_modules")) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            _ => return Err("Unexpected nested CodeBuddy dependency directory".into()),
        }
    }
    Ok(())
}

fn check_lock(
    packages: &serde_json::Value,
    package: &str,
    version: &str,
    integrity: &str,
) -> Result<(), String> {
    let key = format!("node_modules/{package}");
    let entry = &packages[key.as_str()];
    if entry["version"].as_str() != Some(version) || entry["integrity"].as_str() != Some(integrity)
    {
        return Err(format!("CodeBuddy Linux npm identity mismatch for {package}"));
    }
    Ok(())
}

fn verify_path(path: &Path, executable: bool) -> Result<(), String> {
    if path.canonicalize().ok().as_deref() != Some(path) {
        return Err("A reviewed CodeBuddy Linux artifact is missing or redirected".into());
    }
    let metadata = path.metadata().map_err(|_| "Cannot stat CodeBuddy Linux artifact")?;
    if !metadata.is_file() || (executable && metadata.permissions().mode() & 0o111 == 0) {
        return Err("Invalid CodeBuddy Linux artifact permissions".into());
    }
    Ok(())
}

fn verify_file(path: &Path, expected: &str, executable: bool) -> Result<(), String> {
    verify_path(path, executable)?;
    let mut file = File::open(path).map_err(|_| "Cannot open CodeBuddy Linux artifact")?;
    let mut hash = Sha256::new();
    let mut block = [0; 64 * 1024];
    loop {
        let size = file.read(&mut block).map_err(|_| "Cannot hash CodeBuddy Linux artifact")?;
        if size == 0 {
            break;
        }
        hash.update(&block[..size]);
    }
    if hex::encode(hash.finalize()) != expected {
        return Err("CodeBuddy Linux artifact checksum mismatch".into());
    }
    Ok(())
}
