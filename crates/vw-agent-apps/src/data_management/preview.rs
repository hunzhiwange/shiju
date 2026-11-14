//! Read-only preview core. Domain owners normalize records and validate references;
//! this module never opens target storage or grants write authority.
use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;
use uuid::Uuid;

use super::{
    portable_policy::{MigrationLocation, filter_portable_item},
    source::{SourceCategory, SourceRecord, SourceScope, SourceSnapshot, SourceSummary},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewError {
    Forbidden,
    InvalidSelection,
    UnsupportedCategory,
    InvalidCandidate,
    Stale,
    InvalidDecision,
    UnresolvedConflict,
}

/// Business-only normalized item. Never serialize or log these values. A domain
/// owner must parse embedded configuration and match target identity/name before
/// constructing this item; arbitrary source rows are not normalized domain items.
pub struct PreviewCandidate {
    pub incoming: Value,
    pub existing: Option<Value>,
    pub references_valid: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Difference {
    Added,
    Different,
    Identical,
    InvalidReference,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision { Skip, Replace }

/// No source labels, paths, field pointers or values reach display output.
#[derive(Debug)]
pub struct PreviewEntry {
    pub id: Uuid,
    pub category: SourceCategory,
    pub difference: Difference,
    pub excluded_fields: usize,
}

pub struct ImportPreview {
    scope: SourceScope,
    source: SourceSummary,
    target_revision: String,
    entries: Vec<PreviewEntry>,
}

impl ImportPreview {
    pub fn entries(&self) -> &[PreviewEntry] { &self.entries }

    /// Call under the target owner's lock/CAS boundary immediately before writes.
    /// Returned decisions are preparation only, not authorization or a transaction.
    /// Every differing item needs a decision; invalid references permit skip only.
    pub fn resolve(
        &self,
        scope: &SourceScope,
        source: &SourceSnapshot,
        target_revision: &str,
        decisions: &[(Uuid, Decision)],
    ) -> Result<BTreeMap<Uuid, Decision>, PreviewError> {
        if scope != &self.scope { return Err(PreviewError::Forbidden); }
        let summary = source.summary(scope).map_err(|_| PreviewError::Forbidden)?;
        if summary != &self.source || target_revision != self.target_revision {
            return Err(PreviewError::Stale);
        }
        let mut resolved = BTreeMap::new();
        for &(id, decision) in decisions {
            let entry = self.entries.iter().find(|entry| entry.id == id)
                .ok_or(PreviewError::InvalidDecision)?;
            if resolved.insert(id, decision).is_some()
                || !matches!(entry.difference, Difference::Different | Difference::InvalidReference)
                || (entry.difference == Difference::InvalidReference && decision != Decision::Skip)
            {
                return Err(PreviewError::InvalidDecision);
            }
        }
        for entry in &self.entries {
            let default = match entry.difference {
                Difference::Added => Decision::Replace,
                Difference::Identical => Decision::Skip,
                Difference::Different | Difference::InvalidReference => {
                    if !resolved.contains_key(&entry.id) {
                        return Err(PreviewError::UnresolvedConflict);
                    }
                    continue;
                }
            };
            resolved.insert(entry.id, default);
        }
        Ok(resolved)
    }
}

/// Only selected categories are handed to the read-only domain adapter. Missing
/// domain support must return UnsupportedCategory, never an empty success. The
/// adapter must capture target_revision consistently with its target reads, using
/// the same authenticated scope; execution must recheck it under a write lock.
/// Cross-device policy is deliberately fixed: host-path reuse needs a separate
/// owner-authorized flow, not a caller-provided boolean on this preview.
pub fn build_preview(
    scope: &SourceScope,
    source: &SourceSnapshot,
    selected: &[SourceCategory],
    target_revision: String,
    mut normalize: impl FnMut(SourceCategory, &[&SourceRecord]) -> Result<Vec<PreviewCandidate>, PreviewError>,
) -> Result<ImportPreview, PreviewError> {
    let summary = source.summary(scope).map_err(|_| PreviewError::Forbidden)?.clone();
    let records = source.records(scope).map_err(|_| PreviewError::Forbidden)?;
    let categories: BTreeSet<_> = selected.iter().copied().collect();
    if categories.is_empty() || categories.len() != selected.len() || target_revision.trim().is_empty() {
        return Err(PreviewError::InvalidSelection);
    }
    let mut entries = Vec::new();
    for category in categories {
        let records: Vec<_> = records.iter().filter(|record| record.category == category).collect();
        for candidate in normalize(category, &records)? {
            if !candidate.incoming.is_object()
                || candidate.existing.as_ref().is_some_and(|value| !value.is_object())
            {
                return Err(PreviewError::InvalidCandidate);
            }
            let incoming = filter_portable_item(&candidate.incoming, MigrationLocation::CrossDevice);
            let existing = candidate.existing.as_ref().map(|value| {
                filter_portable_item(value, MigrationLocation::CrossDevice).value
            });
            let difference = if !candidate.references_valid {
                Difference::InvalidReference
            } else {
                match existing {
                    None => Difference::Added,
                    Some(value) if value == incoming.value => Difference::Identical,
                    Some(_) => Difference::Different,
                }
            };
            entries.push(PreviewEntry {
                id: Uuid::new_v4(), category, difference,
                excluded_fields: incoming.excluded.len(),
            });
        }
    }
    Ok(ImportPreview { scope: scope.clone(), source: summary, target_revision, entries })
}
