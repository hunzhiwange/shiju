//! Revision-checked edits of one provider in the authorized Pi directory.
use super::{PiFiles, WriteError, form::directory_declaration, provider_entries};
use crate::providers::live::{
    commit::{Mode, ProviderTransactions},
    patch_json::JsonPatch,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};
use std::{os::unix::ffi::OsStrExt, path::Path};
use vw_api_types::{agent_apps::AppId, app_providers::pi::PiProfile, id::TenantScope};

pub enum PiDirectoryChange<'a> {
    Insert(&'a PiProfile),
    Replace(&'a PiProfile),
    Remove,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PiDirectoryWriteError {
    Forbidden,
    InvalidConfiguration,
    RevisionConflict,
    AlreadyExists,
    NotFound,
    RecoveryPending,
    TargetFailed,
}

impl From<WriteError> for PiDirectoryWriteError {
    fn from(error: WriteError) -> Self {
        match error {
            WriteError::Conflict => Self::RevisionConflict,
            WriteError::Recovered => Self::RecoveryPending,
            WriteError::InvalidResource | WriteError::Io => Self::TargetFailed,
        }
    }
}

// Settings are observational. The write revision guards the chosen models file,
// including exact bytes and the distinction between missing and present.
pub(super) fn source_revision(directory: &Path, models: Option<&[u8]>) -> String {
    let mut hash = Sha256::new();
    hash.update(b"pi-models-v1");
    let path = directory.as_os_str().as_bytes();
    hash.update((path.len() as u64).to_be_bytes());
    hash.update(path);
    hash.update([u8::from(models.is_some())]);
    hash.update(models.unwrap_or_default());
    URL_SAFE_NO_PAD.encode(hash.finalize())
}

impl PiFiles {
    /// The caller authorizes the project and host, then verifies any scope-bound
    /// revision against this native source revision. Recheck upstream inputs
    /// through `unchanged` before the existing transaction publishes.
    pub fn apply_directory_change(
        &mut self,
        tx: &ProviderTransactions,
        tenant: &TenantScope,
        expected_revision: &str,
        id: &str,
        change: PiDirectoryChange<'_>,
        unchanged: impl Fn() -> bool,
    ) -> Result<(), PiDirectoryWriteError> {
        use PiDirectoryWriteError::*;

        let expected = tx.state(AppId::Pi)?;
        if expected.mode_tenant.as_ref().is_some_and(|owner| owner != tenant)
            || expected.pointer.as_ref().is_some_and(|pointer| &pointer.tenant != tenant)
        {
            return Err(Forbidden);
        }
        if tx.pending(AppId::Pi)?.is_some() {
            return Err(RecoveryPending);
        }
        let (name, pre) = self.snapshot()?;
        if source_revision(&self.directory, pre.as_deref()) != expected_revision || !unchanged() {
            return Err(RevisionConflict);
        }
        if expected.pointer.is_some()
            || expected.mode.as_ref().is_some_and(|mode| mode.attached || mode.mode != Mode::Direct)
        {
            return Err(RevisionConflict);
        }
        let next = patch_entry(pre.as_deref(), id, change)?;
        if next == pre.as_deref().unwrap_or_default() {
            return Ok(());
        }
        self.apply(tx, tenant, &expected, name, pre.as_deref(), next, unchanged)?;
        Ok(())
    }
}

fn patch_entry(
    before: Option<&[u8]>,
    id: &str,
    change: PiDirectoryChange<'_>,
) -> Result<Vec<u8>, PiDirectoryWriteError> {
    use PiDirectoryWriteError::*;

    crate::providers::validate_identity(id).map_err(|_| InvalidConfiguration)?;
    let providers = provider_entries(before).map_err(|_| InvalidConfiguration)?;
    let current = providers.get(id);
    let path = vec!["providers".into(), id.into()];
    let mut patch = JsonPatch::default();
    match change {
        PiDirectoryChange::Insert(profile) => {
            if current.is_some() {
                return Err(AlreadyExists);
            }
            let node = directory_declaration(profile, None).map_err(|_| InvalidConfiguration)?;
            patch.set.push((path, node));
        }
        PiDirectoryChange::Replace(profile) => {
            let current = current.ok_or(NotFound)?;
            if !current.is_object() {
                return Err(InvalidConfiguration);
            }
            let node =
                directory_declaration(profile, Some(current)).map_err(|_| InvalidConfiguration)?;
            if *current == node {
                return Ok(before.unwrap_or_default().to_vec());
            }
            patch.set.push((path, node));
        }
        PiDirectoryChange::Remove => {
            let Some(current) = current else { return Ok(before.unwrap_or_default().to_vec()) };
            if !current.is_object() {
                return Err(InvalidConfiguration);
            }
            patch.remove.push(path);
        }
    }
    patch.apply(before).map_err(|_| InvalidConfiguration)
}
