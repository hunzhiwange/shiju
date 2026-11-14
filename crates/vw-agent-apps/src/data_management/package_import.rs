//! Native portable packages enter the same scoped, read-only preview staging as
//! other sources. This module neither extracts files nor writes domain storage.
use std::collections::BTreeMap;
use vw_api_types::agent_apps::data_management::DataCategory;
use super::{
    export::verify_portable_package,
    safety_snapshot::SafetySnapshotError,
    source::{SourceCategory, SourceFormat, SourceRecord, SourceScope, SourceSnapshot, SourceSummary},
};

/// The executor must authorize both the upload read and target scope before this
/// call. `digest` comes from the retained transfer receipt, not package metadata.
/// Bytes must be captured with the transfer owner's bounded, no-follow reader.
/// Returned normalized records still require domain adapters, conflict decisions,
/// target revision checks and the existing apply transaction before any writes.
pub fn stage_portable_package(
    scope: &SourceScope,
    bytes: &[u8],
    digest: &[u8; 32],
) -> Result<SourceSnapshot, SafetySnapshotError> {
    let categories = verify_portable_package(bytes, digest)?;
    let mut records = Vec::new();
    let mut counts = BTreeMap::new();
    for group in categories {
        let category = match group.category {
            DataCategory::Providers => SourceCategory::Providers,
            DataCategory::Mcp => SourceCategory::Mcp,
            DataCategory::Skills => SourceCategory::Skills,
            DataCategory::Rules => SourceCategory::Rules,
            DataCategory::Profiles => SourceCategory::Profiles,
            DataCategory::Preferences => SourceCategory::Preferences,
            DataCategory::Usage => SourceCategory::Usage,
        };
        counts.insert(category, group.items.len());
        for fields in group.items {
            records.push(SourceRecord { category, origin: "native_package".into(), fields });
        }
    }
    Ok(SourceSnapshot {
        scope: scope.clone(),
        summary: SourceSummary {
            format: SourceFormat::PortablePackage, version: 1,
            main_sha256: *digest, wal_sha256: None, counts,
        },
        records,
    })
}
