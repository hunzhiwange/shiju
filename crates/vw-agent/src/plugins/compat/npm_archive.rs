//! Bounded extraction into a private temporary directory, followed by audited installation.
use crate::plugins::{NpmPluginArchive, PluginIdentity, PluginInstallResult, load_plugin_manifest};
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fs, io::Read, path::Path};

const MAX_EXTRACTED: u64 = 250 * 1024 * 1024;
const MAX_TAR_BYTES: u64 = MAX_EXTRACTED + 16 * 1024 * 1024;

pub(crate) fn install_archive(
    archive: NpmPluginArchive,
    destination: &Path,
    marketplace: Option<&str>,
) -> Result<PluginInstallResult> {
    if archive.bytes.len() > 50 * 1024 * 1024
        || hex::encode(Sha256::digest(&archive.bytes)) != archive.sha256
    {
        bail!("npm archive hash mismatch or size limit exceeded");
    }
    let staging = tempfile::tempdir()?;
    let decoder = flate2::read::GzDecoder::new(archive.bytes.as_slice());
    let mut tar = tar::Archive::new(decoder.take(MAX_TAR_BYTES + 1));
    let mut seen = HashSet::new();
    let mut extracted = 0u64;
    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let raw = path.to_str().context("unsafe npm archive: non UTF-8 path")?;
        let raw = raw.strip_suffix('/').unwrap_or(raw);
        if raw.contains(['\\', ':'])
            || raw.split('/').any(|p| p.is_empty() || p == "." || p == "..")
            || !(raw == "package" || raw.starts_with("package/"))
            || !seen.insert(raw.to_owned())
            || seen.len() > 10000
        {
            bail!("unsafe npm archive path or duplicate entry");
        }
        let kind = entry.header().entry_type();
        if !kind.is_file() && !kind.is_dir() {
            bail!("unsafe npm archive entry type");
        }
        let output = staging.path().join(raw);
        if kind.is_dir() {
            if entry.size() != 0 {
                bail!("unsafe npm archive directory size");
            }
            fs::create_dir_all(&output)?;
            continue;
        }
        if raw == "package" {
            bail!("unsafe npm archive root");
        }
        extracted =
            extracted.checked_add(entry.size()).context("unsafe npm archive size overflow")?;
        if extracted > MAX_EXTRACTED {
            bail!("unsafe npm archive exceeds extracted size limit");
        }
        fs::create_dir_all(output.parent().context("unsafe npm archive parent")?)?;
        let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&output)?;
        std::io::copy(&mut entry, &mut file)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = if entry.header().mode()? & 0o111 != 0 { 0o755 } else { 0o644 };
            fs::set_permissions(&output, fs::Permissions::from_mode(mode))?;
        }
    }
    // Drain to validate gzip CRC and bound even padding/extension overhead.
    let mut remaining = tar.into_inner();
    std::io::copy(&mut remaining, &mut std::io::sink())?;
    if remaining.limit() == 0 {
        bail!("unsafe npm archive exceeds decoded size limit");
    }
    let root = staging.path().join("package");
    let mut metadata_bytes = Vec::new();
    fs::File::open(root.join("package.json"))?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut metadata_bytes)?;
    if metadata_bytes.len() > 1024 * 1024 {
        bail!("npm package metadata exceeds size limit");
    }
    let metadata: serde_json::Value = serde_json::from_slice(&metadata_bytes)?;
    if metadata.get("name").and_then(|v| v.as_str()) != Some(&archive.package)
        || metadata.get("version").and_then(|v| v.as_str()) != Some(&archive.version)
    {
        bail!("npm package metadata mismatch");
    }
    let manifest = load_plugin_manifest(&root)?;
    let identity = match marketplace {
        Some(m) => PluginIdentity::marketplace(&manifest.name, m)?,
        None => PluginIdentity::local(&manifest.name),
    };
    let provenance = serde_json::json!({"npm":archive.package,"version":archive.version,"registry":archive.registry,"sha256":archive.sha256}).to_string();
    super::transaction_install::install_package_with_source(
        &manifest,
        identity,
        destination,
        Some(&provenance),
    )
}

pub(crate) fn install(
    package: &str,
    version: Option<&str>,
    registry: Option<&str>,
    destination: &Path,
    marketplace: Option<&str>,
) -> Result<PluginInstallResult> {
    let package = package.to_owned();
    let version = version.map(str::to_owned);
    let registry = registry.map(str::to_owned);
    // The existing installation API is synchronous and may be invoked inside a Tokio runtime.
    let archive = std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(
            super::npm_source::fetch_npm_plugin(&package, version.as_deref(), registry.as_deref()),
        )
    })
    .join()
    .map_err(|_| anyhow::anyhow!("npm download worker panicked"))??;
    install_archive(archive, destination, marketplace)
}
