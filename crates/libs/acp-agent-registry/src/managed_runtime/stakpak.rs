//! Verify the upstream Linux executable for ordinary CLI use.
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io, os::unix::fs::PermissionsExt, path::Path};

pub(super) const LINUX_CLI_CONTRACT: &str =
    include_str!("../../../../../patches/acp-model-source/stakpak/linux-cli.json");

#[derive(Deserialize)]
struct CliContract {
    directory: String,
    entry: String,
    args: Vec<String>,
    targets: BTreeMap<String, Target>,
}

#[derive(Deserialize)]
struct Target {
    executable_sha256: String,
}

pub(super) fn cli_local_command(root: &Path) -> Result<Option<(String, Vec<String>)>, String> {
    let contract: CliContract = serde_json::from_str(LINUX_CLI_CONTRACT)
        .map_err(|_| "Invalid Stakpak Linux CLI contract")?;
    let target = contract
        .targets
        .get(std::env::consts::ARCH)
        .ok_or("Unsupported Stakpak Linux CLI architecture")?;
    let root = match root.canonicalize() {
        Ok(root) => root,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("Cannot resolve Stakpak installation root".into()),
    };
    let directory = root.join(contract.directory);
    match directory.symlink_metadata() {
        Ok(metadata) if metadata.is_dir() => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        _ => return Err("Stakpak installation directory is invalid".into()),
    }

    let entry = directory.join(contract.entry);
    let metadata = entry.symlink_metadata().map_err(|_| "Missing Stakpak Linux CLI")?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
        return Err("Stakpak Linux CLI must be a regular executable".into());
    }
    let mut file = fs::File::open(&entry).map_err(|_| "Cannot read Stakpak Linux CLI")?;
    let mut digest = Sha256::new();
    io::copy(&mut file, &mut digest).map_err(|_| "Cannot hash Stakpak Linux CLI")?;
    if hex::encode(digest.finalize()) != target.executable_sha256 {
        return Err("Stakpak Linux CLI checksum mismatch".into());
    }
    let command = entry.to_str().ok_or("Stakpak Linux CLI path is not Unicode")?;
    Ok(Some((command.to_owned(), contract.args)))
}

#[cfg(test)]
#[path = "stakpak_tests.rs"]
mod tests;
