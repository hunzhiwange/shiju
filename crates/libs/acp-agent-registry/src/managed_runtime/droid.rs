//! Pinned executable identity for the Linux Droid runtime.
use super::{Stage, root};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io,
    io::Read,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::Duration,
};

const MANIFEST: &str = include_str!("../../../../../patches/acp-model-source/droid/linux.json");
const INSTALLER: &str =
    include_str!("../../../../../patches/acp-model-source/droid/install_linux.py");
const PATCH: &str = include_str!("../../../../../patches/acp-model-source/droid/apply.py");

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

impl Manifest {
    fn load() -> Result<Self, String> {
        serde_json::from_str(MANIFEST).map_err(|_| "Invalid Droid Linux artifact manifest".into())
    }
}

pub(super) fn reviewed_command() -> Result<Option<(String, Vec<String>)>, String> {
    let manifest = Manifest::load()?;
    let directory = root("droid")?.join(&manifest.revision);
    match directory.symlink_metadata() {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err("Cannot inspect Droid Linux runtime directory".into()),
        Ok(_) => {
            let entry = verify(&directory, &manifest)?;
            let command = entry.to_str().ok_or("Droid Linux executable path is not Unicode")?;
            Ok(Some((command.into(), vec!["exec".into(), "--output-format".into(), "acp".into()])))
        }
    }
}

pub(super) fn verify_command(command: &str) -> Result<(), String> {
    match reviewed_command()? {
        Some((reviewed, _)) if reviewed == command => Ok(()),
        _ => Err("Droid Linux requires the installed reviewed executable".into()),
    }
}

pub(super) async fn install() -> Result<(), String> {
    if reviewed_command()?.is_some() {
        return Ok(());
    }
    let manifest = Manifest::load()?;
    let root = root("droid")?;
    let stage = Stage::new("droid", &root)?;
    for (name, content) in
        [("linux.json", MANIFEST), ("apply.py", PATCH), ("install_linux.py", INSTALLER)]
    {
        fs::write(stage.0.join(name), content).map_err(|_| "Cannot stage Droid Linux installer")?;
    }
    let runtime = stage.0.join(&manifest.revision);
    let script = stage.0.join("install_linux.py");
    crate::install::run_installer(
        "python3",
        &[
            "-E",
            "-s",
            "-B",
            script.to_str().ok_or("Invalid Droid Linux installer path")?,
            runtime.to_str().ok_or("Invalid Droid Linux staging path")?,
        ],
        tokio::time::Instant::now() + Duration::from_secs(300),
    )
    .await?;
    verify(&runtime, &manifest)?;
    let destination = root.join(&manifest.revision);
    if fs::rename(runtime, destination).is_err() && reviewed_command()?.is_none() {
        return Err("Cannot publish Droid Linux runtime".into());
    }
    reviewed_command()?.ok_or("Droid Linux runtime was not installed")?;
    Ok(())
}

fn verify(directory: &Path, manifest: &Manifest) -> Result<PathBuf, String> {
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
