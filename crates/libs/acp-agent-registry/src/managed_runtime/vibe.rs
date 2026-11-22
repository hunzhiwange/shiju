//! Identity checks for the GNU Vibe runtime staged from the architecture's wheel.
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Read,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

pub(super) const MANIFEST: &str =
    include_str!("../../../../../patches/acp-model-source/vibe/linux.json");
pub(super) const LAUNCHER: &str =
    include_str!("../../../../../patches/acp-model-source/vibe/launcher.sh");
pub(super) const INSTALLER: &str =
    include_str!("../../../../../patches/acp-model-source/vibe/install_linux.py");
pub(super) const APPROVAL_PATCH: &str =
    include_str!("../../../../../patches/acp-model-source/vibe/linux-host-approval.patch");

#[derive(Deserialize)]
struct LinuxManifest {
    version: String,
    entry: String,
    launcher_sha256: String,
    approval_files: BTreeMap<String, ApprovalFile>,
    targets: BTreeMap<String, LinuxArtifact>,
}

#[derive(Deserialize)]
struct ApprovalFile {
    after: String,
}

#[derive(Deserialize)]
struct LinuxArtifact {
    files: BTreeMap<String, String>,
}

/// The directory comes from the host's versioned install or installer staging.
pub(super) fn verify_linux(
    directory: &Path,
    version: &str,
    patched_sources: &BTreeMap<String, String>,
) -> Result<PathBuf, String> {
    let manifest: LinuxManifest =
        serde_json::from_str(MANIFEST).map_err(|_| "Invalid Vibe Linux artifact manifest")?;
    let target = format!("{}-unknown-linux-gnu", std::env::consts::ARCH);
    let artifact = manifest.targets.get(&target).ok_or("Unsupported Vibe Linux architecture")?;
    if manifest.version != version {
        return Err("Vibe Linux artifact version mismatch".into());
    }
    let runtime = directory.canonicalize().map_err(|_| "Missing Vibe Linux installation")?;
    let entry = runtime.join(&manifest.entry);
    verify_file(&entry, &manifest.launcher_sha256)?;
    let site = runtime.join("lib/python3.12/site-packages");
    let mut files = patched_sources.clone();
    files.extend(manifest.approval_files.into_iter().map(|(name, file)| (name, file.after)));
    for (name, digest) in files.iter().chain(&artifact.files) {
        verify_file(&site.join(name), digest)?;
    }
    for program in [&entry, &runtime.join("bin/python")] {
        if !fs::metadata(program)
            .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        {
            return Err("Vibe Linux entry or interpreter is not executable".into());
        }
    }
    Ok(entry)
}

fn verify_file(path: &Path, expected: &str) -> Result<(), String> {
    if path.canonicalize().ok().as_deref() != Some(path)
        || !path.symlink_metadata().is_ok_and(|meta| meta.is_file())
    {
        return Err("Vibe Linux artifact is missing or redirected".into());
    }
    let mut file = File::open(path).map_err(|_| "Missing Vibe Linux artifact")?;
    let mut hash = Sha256::new();
    let mut block = [0; 64 * 1024];
    loop {
        let size = file.read(&mut block).map_err(|_| "Could not hash Vibe Linux artifact")?;
        if size == 0 {
            break;
        }
        hash.update(&block[..size]);
    }
    if hex::encode(hash.finalize()) != expected {
        return Err("Vibe Linux artifact checksum mismatch".into());
    }
    Ok(())
}
