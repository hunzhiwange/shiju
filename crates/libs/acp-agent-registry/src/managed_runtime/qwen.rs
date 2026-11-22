//! Linux Qwen files beyond the shared npm package checks.
use acp_access_policy::access_policy::qwen;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const MANIFEST: &str = include_str!("../../../../../patches/acp-model-source/qwen/linux.json");

#[derive(serde::Deserialize)]
struct Manifest {
    package: String,
    entry: String,
    targets: BTreeMap<String, Artifacts>,
}

#[derive(serde::Deserialize)]
struct Artifacts {
    files: BTreeMap<String, String>,
}

pub(super) fn verify(directory: &Path) -> Result<PathBuf, String> {
    let manifest: Manifest = serde_json::from_str(MANIFEST).map_err(|_| "Invalid Qwen manifest")?;
    let target = match std::env::consts::ARCH {
        "x86_64" => "x86_64-unknown-linux-gnu",
        "aarch64" => "aarch64-unknown-linux-gnu",
        _ => return Err("Unsupported Qwen Linux architecture".into()),
    };
    let artifacts = manifest.targets.get(target).ok_or("Missing Qwen Linux files")?;
    let base = directory
        .parent()
        .ok_or("Missing Qwen runtime root")?
        .canonicalize()
        .map_err(|_| "Missing Qwen runtime root")?;
    let package = base
        .join(directory.file_name().ok_or("Missing Qwen runtime directory")?)
        .join("node_modules")
        .join(manifest.package);
    let entry = package.join(manifest.entry);
    verify_file(&entry, true)?;
    let command = entry.to_str().ok_or("Qwen entry path is not Unicode")?;
    if !qwen::verified_linux_package(command) {
        return Err("Qwen managed package fingerprint mismatch".into());
    }
    for (name, expected) in &artifacts.files {
        let mut file = verify_file(&package.join(name), !name.ends_with(".so"))?;
        let mut digest = Sha256::new();
        let mut buffer = [0; 64 * 1024];
        loop {
            let count = file.read(&mut buffer).map_err(|_| "Cannot read Qwen Linux artifact")?;
            if count == 0 {
                break;
            }
            digest.update(&buffer[..count]);
        }
        if hex::encode(digest.finalize()) != *expected {
            return Err("Qwen Linux artifact checksum mismatch".into());
        }
    }
    Ok(entry)
}

fn verify_file(path: &Path, executable: bool) -> Result<File, String> {
    if path.canonicalize().ok().as_deref() != Some(path) {
        return Err("Qwen Linux artifact is missing or redirected".into());
    }
    let metadata = fs::metadata(path).map_err(|_| "Cannot inspect Qwen Linux artifact")?;
    if !metadata.is_file() || (executable && metadata.permissions().mode() & 0o111 == 0) {
        return Err(
            "Qwen Linux artifact is not a regular file with the required permissions".into()
        );
    }
    File::open(path).map_err(|_| "Cannot open Qwen Linux artifact".into())
}
