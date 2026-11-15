//! Authorized selected import orchestration, with concrete owner execution injected.
//! The caller authorizes the exact scope and each selected category before entry.
use anyhow::{Result, anyhow};
use std::path::Path;
use vw_agent_apps::data_management::{
    apply::{self, ApplyReport},
    safety_snapshot::SafetySnapshotReceipt,
    source::{SourceCategory, SourceScope, SourceSnapshot, SourceSummary},
};

/// A prepared owner operation. Implementations retain typed previews and owner
/// handles, and append recovery receipts before returning any post-commit error.
/// This port does not confer authorization or accept client-selected commands.
pub trait PreparedImport {
    fn category(&self) -> SourceCategory;
    fn commit(
        &mut self,
        scope: &SourceScope,
        source: &SourceSnapshot,
        snapshot_root: &Path,
        receipts: &mut Vec<(SourceCategory, SafetySnapshotReceipt)>,
    ) -> Result<()>;
}

/// Preserve successful owner receipts even when a later category fails.
pub struct ImportCommit {
    pub report: ApplyReport<anyhow::Error>,
    pub receipts: Vec<(SourceCategory, SafetySnapshotReceipt)>,
}

/// Validate the source and selection, then stop dispatch at the first failure.
/// This is not a cross-owner transaction; retry requires fresh owner previews.
pub fn commit_selected<O: PreparedImport>(
    scope: &SourceScope,
    source: &SourceSnapshot,
    confirmed_source: &SourceSummary,
    selected: &[SourceCategory],
    operations: &mut [O],
    snapshot_root: &Path,
) -> Result<ImportCommit> {
    let prepared = operations.iter().map(PreparedImport::category).collect::<Vec<_>>();
    let mut receipts = Vec::new();
    let report =
        apply::apply_selected(scope, source, confirmed_source, selected, &prepared, |category| {
            let operation = operations
                .iter_mut()
                .find(|op| op.category() == category)
                .ok_or_else(|| anyhow!("missing prepared migration category"))?;
            operation.commit(scope, source, snapshot_root, &mut receipts)
        })
        .map_err(|_| anyhow!("migration source or selected preparation changed"))?;
    Ok(ImportCommit { report, receipts })
}
