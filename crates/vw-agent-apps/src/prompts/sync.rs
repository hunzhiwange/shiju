//! Saved-state projection, independent of activation and live-file backfill.
use crate::apps::lifecycle::operations::Operation;
use super::listing::{PromptEntry, list_entries};
use vw_api_types::agent_apps::AppId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    NativePiSelection,
    UnsupportedClaudeDesktop,
    NoSelection,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Published { duplicate_selection: bool },
    Skipped(SkipReason),
    Failed,
    Cancelled,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetResult {
    pub app: AppId,
    pub outcome: Outcome,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncReport {
    pub targets: Vec<TargetResult>,
    pub cancelled: bool,
}

/// Also usable with source snapshots containing multiple enabled rows. Never
/// invent a selection or fall back to live content; invalid IDs reject the target.
pub fn selection<'a>(
    entries: &'a [PromptEntry], selected: &[&str],
) -> Result<Option<(&'a PromptEntry, bool)>, &'static str> {
    if selected.iter().any(|id| !entries.iter().any(|entry| entry.id == *id)) {
        return Err("invalid stored prompt selection");
    }
    let ordered = list_entries(entries.to_vec(), "");
    let Some(first) = ordered.iter().find(|entry| selected.contains(&entry.id.as_str())) else {
        return Ok(None);
    };
    let duplicate = ordered.iter().filter(|entry| selected.contains(&entry.id.as_str())).count() > 1;
    Ok(Some((entries.iter().find(|entry| entry.id == first.id).unwrap(), duplicate)))
}

/// The existing host operation owns cancellation and terminal events. Invoke in
/// its worker; emit persists/streams each completed target through that owner's
/// event sink. Do not cancel a transaction midway or roll back completed siblings.
/// Errors intentionally contain no catalog text, host paths or sensitive payload.
pub fn sync_all(
    apps: &[AppId], operation: &Operation,
    mut project: impl FnMut(AppId) -> Result<Outcome, ()>,
    mut emit: impl FnMut(&TargetResult),
) -> SyncReport {
    let mut report = SyncReport { targets: Vec::with_capacity(apps.len()), cancelled: false };
    for &app in apps {
        let outcome = if operation.is_cancelled() {
            report.cancelled = true;
            Outcome::Cancelled
        } else {
            match app {
                AppId::Pi => Outcome::Skipped(SkipReason::NativePiSelection),
                AppId::ClaudeDesktop => Outcome::Skipped(SkipReason::UnsupportedClaudeDesktop),
                _ => project(app).unwrap_or(Outcome::Failed),
            }
        };
        let result = TargetResult { app, outcome };
        emit(&result);
        report.targets.push(result);
    }
    report
}
