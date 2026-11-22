//! Linux-only verification for the pinned Codex bridge and its engine.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::Read,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

const MANIFEST: &str =
    include_str!("../../../../../patches/acp-model-source/codex/linux-engine.json");
const BRIDGE: &str = "@agentclientprotocol/codex-acp/dist/index.js";
const WRAPPER: &str = "@openai/codex/bin/codex.js";

#[derive(Deserialize)]
struct Manifest {
    bridge_sha256: String,
    wrapper_sha256: String,
    targets: BTreeMap<String, Target>,
}

#[derive(Deserialize)]
struct Target {
    package: String,
    files: BTreeMap<String, String>,
}

/// Match the installer and Node wrapper on native architecture. Only Linux GNU
/// x64 and ARM64 should compile this module.
pub(super) fn verify(directory: &Path) -> Result<PathBuf, String> {
    let target = match std::env::consts::ARCH {
        "x86_64" => "x86_64-unknown-linux-gnu",
        "aarch64" => "aarch64-unknown-linux-gnu",
        _ => return Err("Unsupported managed Codex Linux architecture".into()),
    };
    let manifest: Manifest = serde_json::from_str(MANIFEST)
        .map_err(|_| "Invalid reviewed Codex Linux artifact manifest")?;
    let target = manifest.targets.get(target).ok_or("Missing reviewed Codex Linux target")?;
    let parent = directory.parent().ok_or("Missing managed Codex runtime parent")?;
    let name = directory.file_name().ok_or("Missing managed Codex runtime directory")?;
    let canonical_parent =
        parent.canonicalize().map_err(|_| "Missing managed Codex runtime parent")?;
    let modules = canonical_parent.join(name).join("node_modules");

    let bridge = modules.join(BRIDGE);
    verify_file(&bridge, &manifest.bridge_sha256, false)?;
    verify_file(&modules.join(WRAPPER), &manifest.wrapper_sha256, false)?;
    let engine = modules.join(&target.package);
    for (file, hash) in &target.files {
        verify_file(&engine.join(file), hash, file != "package.json")?;
    }
    Ok(bridge)
}

fn verify_file(path: &Path, expected: &str, executable: bool) -> Result<(), String> {
    if path.canonicalize().ok().as_deref() != Some(path) {
        return Err("A reviewed Codex Linux artifact is missing or redirected".into());
    }
    let metadata = path.metadata().map_err(|_| "Cannot stat reviewed Codex Linux artifact")?;
    if !metadata.is_file() || (executable && metadata.permissions().mode() & 0o111 == 0) {
        return Err("Invalid reviewed Codex Linux artifact permissions".into());
    }
    let mut file = File::open(path).map_err(|_| "Cannot open reviewed Codex Linux artifact")?;
    let mut digest = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let count =
            file.read(&mut buffer).map_err(|_| "Cannot hash reviewed Codex Linux artifact")?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    if hex::encode(digest.finalize()) != expected {
        return Err("Reviewed Codex Linux artifact checksum mismatch".into());
    }
    Ok(())
}

#[cfg(test)]
#[path = "codex_tests.rs"]
mod tests;
