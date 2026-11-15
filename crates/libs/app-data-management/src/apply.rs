//! Selected-category dispatch. Domain owners retain payloads and enforce their
//! preview/CAS boundaries. This layer never opens CC as a runtime database.
use std::collections::BTreeSet;

use super::source::{SourceCategory, SourceScope, SourceSnapshot, SourceSummary};

/// Safe progress retained even when a later owner rejects its commit. A failed
/// owner may have committed before reporting a projection/cleanup error; callers
/// must retain its recovery metadata and must not claim automatic rollback.
pub struct ApplyReport<E> {
    pub completed: Vec<SourceCategory>,
    pub failed: Option<(SourceCategory, E)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyError {
    Forbidden,
    StaleSource,
    InvalidSelection,
}

/// Dispatch only categories bound to the confirmed preparation. Validate the
/// complete selection before invoking any owner; empty selection performs no IO.
/// SourceSummary must come from the server-retained preview, never a request.
/// Each owner rechecks its own target revision in its transaction before writes.
/// Recovery of earlier categories is deliberately separate from dispatch.
pub fn apply_selected<E>(
    scope: &SourceScope,
    source: &SourceSnapshot,
    confirmed_source: &SourceSummary,
    selected: &[SourceCategory],
    prepared_categories: &[SourceCategory],
    mut commit: impl FnMut(SourceCategory) -> Result<(), E>,
) -> Result<ApplyReport<E>, ApplyError> {
    let current = source.summary(scope).map_err(|_| ApplyError::Forbidden)?;
    if current != confirmed_source {
        return Err(ApplyError::StaleSource);
    }
    let selected_set: BTreeSet<_> = selected.iter().copied().collect();
    let prepared_set: BTreeSet<_> = prepared_categories.iter().copied().collect();
    if selected_set.len() != selected.len()
        || prepared_set.len() != prepared_categories.len()
        || selected_set != prepared_set
    {
        return Err(ApplyError::InvalidSelection);
    }
    let mut report = ApplyReport { completed: Vec::new(), failed: None };
    // References are imported before profiles. Usage requires provider bindings.
    for category in [
        SourceCategory::Providers,
        SourceCategory::Mcp,
        SourceCategory::Skills,
        SourceCategory::Rules,
        SourceCategory::Preferences,
        SourceCategory::Usage,
        SourceCategory::Profiles,
    ] {
        if !selected_set.contains(&category) {
            continue;
        }
        match commit(category) {
            Ok(()) => report.completed.push(category),
            Err(error) => {
                report.failed = Some((category, error));
                break;
            }
        }
    }
    Ok(report)
}
