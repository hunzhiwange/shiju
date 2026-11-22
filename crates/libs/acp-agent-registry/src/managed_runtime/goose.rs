//! Identity checks for the Goose executable produced by the managed Linux installer.

use super::Contract;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::Read,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

pub(super) const TARGET: &str = if cfg!(target_arch = "x86_64") {
    "x86_64-unknown-linux-gnu"
} else {
    "aarch64-unknown-linux-gnu"
};

pub(super) const LINUX_PATCH_SHA256: &str =
    "ad6eda848802728e2efadab5677d61f740e3dc64a823e18625d807cf37a63ebe";

pub(super) fn verify(directory: &Path, contract: &Contract) -> Result<PathBuf, String> {
    let metadata = fs::symlink_metadata(directory).map_err(|_| "Missing Goose installation")?;
    if !metadata.is_dir() || metadata.permissions().mode() & 0o022 != 0 {
        return Err("Goose installation is redirected or writable by other users".into());
    }
    let directory = directory.canonicalize().map_err(|_| "Missing Goose installation")?;
    let executable = directory.join(&contract.bin);
    let receipt_path = directory.join("build-receipt.json");
    for path in [&executable, &receipt_path] {
        let metadata = fs::symlink_metadata(path).map_err(|_| "Missing Goose managed artifact")?;
        if !metadata.is_file() || metadata.permissions().mode() & 0o022 != 0 {
            return Err("Goose managed artifact is redirected or writable by other users".into());
        }
    }
    if fs::metadata(&executable).map_err(|_| "Missing Goose executable")?.permissions().mode()
        & 0o111
        == 0
    {
        return Err("Goose managed artifact is not executable".into());
    }
    let receipt: serde_json::Value = serde_json::from_slice(
        &fs::read(receipt_path).map_err(|_| "Cannot read Goose build receipt")?,
    )
    .map_err(|_| "Invalid Goose build receipt")?;
    if receipt["target"] != TARGET
        || receipt["linux_patch_sha256"] != LINUX_PATCH_SHA256
        || receipt["version"] != contract.version
        || receipt["source_sha256"].as_str() != contract.source_sha256.as_deref()
        || receipt["patch_sha256"].as_str() != contract.patch_sha256.as_deref()
    {
        return Err("Goose build receipt differs from the Linux managed contract".into());
    }

    let mut file = File::open(&executable).map_err(|_| "Cannot open Goose executable")?;
    let mut header = [0; 20];
    file.read_exact(&mut header).map_err(|_| "Missing Goose ELF header")?;
    let machine = if cfg!(target_arch = "x86_64") { 62 } else { 183 };
    if &header[..7] != b"\x7fELF\x02\x01\x01"
        || !matches!(u16::from_le_bytes([header[16], header[17]]), 2 | 3)
        || u16::from_le_bytes([header[18], header[19]]) != machine
    {
        return Err("Goose managed artifact has an unexpected ELF target".into());
    }
    let mut digest = Sha256::new();
    digest.update(header);
    let mut buffer = [0; 64 * 1024];
    loop {
        let size = file.read(&mut buffer).map_err(|_| "Cannot hash Goose executable")?;
        if size == 0 {
            break;
        }
        digest.update(&buffer[..size]);
    }
    if receipt["sha256"] != hex::encode(digest.finalize()) {
        return Err("Goose executable checksum differs from the Linux build receipt".into());
    }
    Ok(executable)
}
