use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

pub(super) const LINUX_CONTRACT: &str =
    include_str!("../../../../../patches/acp-model-source/qoder/linux.json");

#[derive(Deserialize)]
struct LinuxContract {
    #[serde(flatten)]
    runtime: super::Contract,
    ripgrep: BTreeMap<String, Ripgrep>,
}

#[derive(Deserialize)]
struct Ripgrep {
    package: String,
    sha256: String,
}

pub(super) fn verify(directory: &Path) -> Result<PathBuf, String> {
    let review: LinuxContract =
        serde_json::from_str(LINUX_CONTRACT).map_err(|_| "Invalid Qoder Linux runtime contract")?;
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        _ => return Err("Qoder Linux runtime architecture is unsupported".into()),
    };
    let helper = review.ripgrep.get(arch).ok_or("Missing Qoder ripgrep contract")?;
    let parent = directory.parent().ok_or("Missing Qoder Linux runtime root")?;
    let name = directory.file_name().ok_or("Missing Qoder Linux runtime directory")?;
    let directory =
        parent.canonicalize().map_err(|_| "Missing Qoder Linux runtime root")?.join(name);
    let node_modules = directory.join("node_modules");
    let package = node_modules.join(&review.runtime.package);
    for name in review.runtime.files.keys() {
        let path = package.join(name);
        verify_path(&path)?;
    }
    let entry = package.join(&review.runtime.bin);
    verify_executable(&entry)?;
    let rg = node_modules.join(&helper.package).join("bin/rg");
    verify_executable(&rg)?;
    let sha256 =
        hex::encode(Sha256::digest(fs::read(rg).map_err(|_| "Cannot read Qoder ripgrep")?));
    if sha256 != helper.sha256 {
        return Err("Qoder ripgrep checksum mismatch".into());
    }
    super::verify(&directory, &review.runtime)
}

fn verify_path(path: &Path) -> Result<(), String> {
    if path.canonicalize().ok().as_deref() != Some(path) || !path.is_file() {
        return Err("Qoder runtime artifact is missing or redirected".into());
    }
    Ok(())
}

fn verify_executable(path: &Path) -> Result<(), String> {
    verify_path(path)?;
    if fs::metadata(path).map_err(|_| "Cannot inspect Qoder executable")?.permissions().mode()
        & 0o111
        == 0
    {
        return Err("Qoder runtime artifact is not executable".into());
    }
    Ok(())
}
