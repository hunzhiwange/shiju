//! Portable, filtered owner records. No database discovery or host backup bytes.
use super::{backup_create_files, portable_policy::{filter_portable_item, MigrationLocation},
    safety_snapshot::{SafetySnapshotError as Error, SafetySnapshotScope, MAX_SNAPSHOT_BYTES}};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{io::{Cursor, Read, Write}, path::Path};
use vw_api_types::agent_apps::data_management::{DataArtifactKind, DataCategory};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortableCategory {
    pub category: DataCategory,
    pub items: Vec<Value>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format_version: u32,
    kind: DataArtifactKind,
    categories: Vec<DataCategory>,
    data_sha256: [u8; 32],
}
/// Safe metadata only. Neither digest nor filename authorizes a download.
pub struct PortableExportReceipt {
    pub id: String,
    pub digest: [u8; 32],
    pub excluded_fields: usize,
}

fn selection(categories: &[DataCategory]) -> Result<(), Error> {
    if categories.is_empty() || categories.len() > 7
        || categories.iter().enumerate().any(|(i, c)| categories[..i].contains(c)) {
        return Err(Error::InvalidSelection);
    }
    Ok(())
}

/// Capture must read the authorized tenant/project/principal/host through each
/// domain owner, with owner guards held until this call returns. Unsupported
/// categories must fail, not return an empty replacement. Values are normalized
/// individual objects, never opaque config strings or rollback preimages.
/// Destination is an existing executor-owned 0700 directory, never a client path.
/// Scope is deliberately excluded from the portable bytes.
pub fn export_portable_package(
    root: &Path,
    scope: &SafetySnapshotScope,
    categories: &[DataCategory],
    mut capture: impl FnMut(&SafetySnapshotScope, DataCategory) -> Result<Vec<Value>, Error>,
) -> Result<PortableExportReceipt, Error> {
    selection(categories)?;
    if scope.host_id.trim().is_empty() || scope.principal_id.trim().is_empty() {
        return Err(Error::InvalidSelection);
    }
    let directory = backup_create_files::open_root(root)?;
    let mut records = Vec::new();
    let mut excluded_fields = 0;
    let mut total = 0;
    for category in categories {
        let items = capture(scope, *category)?;
        total += items.len();
        if total > 10_000 { return Err(Error::TooLarge); }
        let mut filtered = Vec::new();
        for item in items {
            if !item.is_object() { return Err(Error::InvalidSelection); }
            let item = filter_portable_item(&item, MigrationLocation::CrossDevice);
            excluded_fields += item.excluded.len();
            filtered.push(item.value);
        }
        records.push(PortableCategory { category: *category, items: filtered });
    }
    let data = serde_json::to_vec(&records).map_err(|_| Error::Corrupt)?;
    if data.len() > MAX_SNAPSHOT_BYTES { return Err(Error::TooLarge); }
    let manifest = Manifest {
        format_version: 1, kind: DataArtifactKind::PortableExport,
        categories: categories.to_vec(), data_sha256: Sha256::digest(&data).into(),
    };
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored).unix_permissions(0o600);
    zip.start_file("manifest.json", options).map_err(|_| Error::Corrupt)?;
    zip.write_all(&serde_json::to_vec(&manifest).map_err(|_| Error::Corrupt)?)
        .map_err(|_| Error::StorageUnavailable)?;
    zip.start_file("data.json", options).map_err(|_| Error::Corrupt)?;
    zip.write_all(&data).map_err(|_| Error::StorageUnavailable)?;
    let bytes = zip.finish().map_err(|_| Error::Corrupt)?.into_inner();
    if bytes.len() > MAX_SNAPSHOT_BYTES { return Err(Error::TooLarge); }
    let digest = Sha256::digest(&bytes).into();
    let id = uuid::Uuid::new_v4().to_string();
    super::export_files::publish(&directory, &id, &bytes, |reread| {
        verify_portable_package(reread, &digest).map(|_| ())
    })?;
    Ok(PortableExportReceipt { id, digest, excluded_fields })
}

/// Validate a bounded package against a separately retained digest. This is
/// integrity checking, not authentication or authorization to import records.
pub fn verify_portable_package(bytes: &[u8], digest: &[u8; 32]) -> Result<Vec<PortableCategory>, Error> {
    if bytes.len() > MAX_SNAPSHOT_BYTES { return Err(Error::TooLarge); }
    if <[u8; 32]>::from(Sha256::digest(bytes)) != *digest { return Err(Error::Corrupt); }
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| Error::Corrupt)?;
    if archive.len() != 2 { return Err(Error::Corrupt); }
    let mut parts = Vec::new();
    for (index, name) in ["manifest.json", "data.json"].iter().enumerate() {
        let entry = archive.by_index(index).map_err(|_| Error::Corrupt)?;
        if entry.name() != *name || !entry.is_file()
            || entry.unix_mode().is_some_and(|mode| mode & 0o170000 != 0 && mode & 0o170000 != 0o100000)
            || entry.size() > MAX_SNAPSHOT_BYTES as u64 {
            return Err(Error::Corrupt);
        }
        let mut data = Vec::new();
        entry.take(MAX_SNAPSHOT_BYTES as u64 + 1).read_to_end(&mut data).map_err(|_| Error::Corrupt)?;
        if data.len() > MAX_SNAPSHOT_BYTES { return Err(Error::TooLarge); }
        parts.push(data);
    }
    let manifest: Manifest = serde_json::from_slice(&parts[0]).map_err(|_| Error::Corrupt)?;
    if manifest.format_version != 1 { return Err(Error::VersionUnsupported); }
    if manifest.kind != DataArtifactKind::PortableExport
        || manifest.data_sha256 != <[u8; 32]>::from(Sha256::digest(&parts[1])) {
        return Err(Error::Corrupt);
    }
    selection(&manifest.categories)?;
    let records: Vec<PortableCategory> = serde_json::from_slice(&parts[1]).map_err(|_| Error::Corrupt)?;
    if records.len() != manifest.categories.len()
        || records.iter().map(|r| r.items.len()).sum::<usize>() > 10_000 {
        return Err(Error::Corrupt);
    }
    for (record, category) in records.iter().zip(&manifest.categories) {
        if record.category != *category { return Err(Error::Corrupt); }
        for item in &record.items {
            if !item.is_object() || !filter_portable_item(item, MigrationLocation::CrossDevice).excluded.is_empty() {
                return Err(Error::Corrupt);
            }
        }
    }
    Ok(records)
}
