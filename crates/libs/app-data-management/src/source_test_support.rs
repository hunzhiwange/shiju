//! Helpers for existing domain fixtures; production staging remains immutable.
use super::source::{SourceRecord, SourceScope, SourceSnapshot, SourceSummary};

pub(super) fn edit_source(
    source: &mut SourceSnapshot,
    scope: &SourceScope,
    edit: impl FnOnce(&mut SourceSummary, &mut Vec<SourceRecord>),
) {
    let mut summary = source.summary(scope).unwrap().clone();
    let mut records = source.records(scope).unwrap().iter().map(|record| SourceRecord {
        category: record.category, origin: record.origin.clone(), fields: record.fields.clone(),
    }).collect();
    edit(&mut summary, &mut records);
    *source = SourceSnapshot::from_staged_parts(scope.clone(), summary, records);
}
