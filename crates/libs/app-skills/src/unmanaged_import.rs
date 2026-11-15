//! Import one explicit scan selection; application projections remain untouched.
use std::path::Path;
use super::{
    app_paths::AppSkillsPath,
    local_install::{LocalInstall, LocalInstallError, LocalSkill},
    unmanaged_scan::{SkillLocation, scan_unmanaged},
};

#[derive(Debug, Clone)]
pub struct UnmanagedSelection {
    pub directory: String,
    pub content_hash: String,
    /// Only these application locations are authorized as import sources.
    pub sources: Vec<SkillLocation>,
}

pub enum PreparedImport {
    Existing(LocalSkill),
    New(LocalInstall),
}

/// The caller resolves and authorizes home, SSOT and application paths on the
/// selected execution host. Selection never enables an application binding.
pub fn prepare_import(
    home: &Path, ssot: &Path, saved: &[LocalSkill], targets: &[AppSkillsPath],
    selection: &UnmanagedSelection,
) -> Result<PreparedImport, LocalInstallError> {
    let reject = || LocalInstallError::UnsafeSource;
    super::local_install::validate_directory(&selection.directory)?;
    if selection.sources.is_empty() || selection.content_hash.len() != 64
        || !selection.content_hash.bytes().all(|byte| byte.is_ascii_hexdigit())
    { return Err(reject()); }
    // Do not scan or infer consent for any unselected application.
    let mut selected_targets = Vec::new();
    for source in &selection.sources {
        let target = targets.iter().find(|target| {
            target.app_id == source.app_id
                && target.directory.join(&selection.directory) == source.path
        }).ok_or_else(reject)?;
        if !selected_targets.iter().any(|old: &&AppSkillsPath| {
            old.app_id == target.app_id && old.directory == target.directory
        }) { selected_targets.push(target); }
    }
    let selected_targets: Vec<_> = selected_targets.into_iter().cloned().collect();
    let rows = scan_unmanaged(home, ssot, saved, &selected_targets)?;
    let row = rows.iter().find(|row| row.directory == selection.directory
        && row.content_hash.as_deref() == Some(selection.content_hash.as_str()))
        .ok_or_else(reject)?;
    if row.conflict || row.external_link
        || !selection.sources.iter().all(|source| row.found_in.contains(source))
    { return Err(reject()); }
    if row.managed {
        return saved.iter().find(|skill| skill.directory.eq_ignore_ascii_case(&selection.directory))
            .cloned().map(PreparedImport::Existing).ok_or_else(reject);
    }
    let installation = LocalInstall::prepare(ssot, &selection.sources[0].path, &selection.directory)?;
    if !installation.matches_snapshot(&selection.content_hash)? { return Err(reject()); }
    // A source modified during copying must fail before publication as well.
    let rows = scan_unmanaged(home, ssot, saved, &selected_targets)?;
    if !rows.iter().any(|row| !row.conflict && !row.external_link
        && row.directory == selection.directory
        && row.content_hash.as_deref() == Some(selection.content_hash.as_str())
        && selection.sources.iter().all(|source| row.found_in.contains(source)))
    { return Err(reject()); }
    Ok(PreparedImport::New(installation))
}
