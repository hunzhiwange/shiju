//! Verify the fixed Linux VT Code executable for ordinary CLI discovery.
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io, io::Read, os::unix::fs::PermissionsExt, path::Path};

pub(super) const LINUX_CLI_CONTRACT: &str =
    include_str!("../../../../../patches/acp-model-source/vtcode/linux.json");

#[derive(Deserialize)]
struct CliContract {
    directory: String,
    bin: String,
    args: Vec<String>,
    targets: BTreeMap<String, Target>,
}

#[derive(Deserialize)]
struct Target {
    executable_sha256: String,
}

pub(super) fn cli_local_command(root: &Path) -> Result<Option<(String, Vec<String>)>, String> {
    let contract: CliContract = serde_json::from_str(LINUX_CLI_CONTRACT)
        .map_err(|_| "Invalid VT Code Linux CLI contract")?;
    let target = contract
        .targets
        .get(std::env::consts::ARCH)
        .ok_or("Unsupported VT Code Linux CLI architecture")?;
    let root = match root.canonicalize() {
        Ok(root) => root,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("Cannot resolve VT Code installation root".into()),
    };
    let directory = root.join(contract.directory);
    match directory.symlink_metadata() {
        Ok(metadata) if metadata.is_dir() => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        _ => return Err("VT Code installation directory is invalid".into()),
    }
    let entry = directory.join(contract.bin);
    let metadata = entry.symlink_metadata().map_err(|_| "Missing VT Code Linux CLI")?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
        return Err("VT Code Linux CLI must be a regular executable".into());
    }

    let mut native = fs::File::open(&entry).map_err(|_| "Cannot read VT Code Linux CLI")?;
    let mut digest = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let size = native.read(&mut buffer).map_err(|_| "Cannot hash VT Code Linux CLI")?;
        if size == 0 {
            break;
        }
        digest.update(&buffer[..size]);
    }
    if hex::encode(digest.finalize()) != target.executable_sha256 {
        return Err("VT Code Linux CLI checksum mismatch".into());
    }

    let command = entry.to_str().ok_or("VT Code Linux CLI path is not Unicode")?;
    Ok(Some((command.to_owned(), contract.args)))
}
