//! Rename through the existing Pi journal, publishing the destination first.
use std::ffi::OsStr;

use vw_api_types::TenantScope;

use crate::providers::live::{
    commit::{PendingTarget, ProviderTransactions},
    stage::{FileChange, WholeFile},
    write_guard::{AppWriteGuard, FilePrivacy, WriteError},
};

use super::templates::{
    MAX_BYTES, PiPromptTemplate, PiPromptTemplateService, PiTemplateError, filename, template,
};

impl PiPromptTemplateService {
    /// Same-name saves use ordinary upsert semantics. A changed name requires an
    /// existing source revision and a missing destination, including on volumes
    /// where differently spelled names resolve to the same file.
    #[allow(clippy::too_many_arguments)]
    pub fn upsert_with_original(
        guard: &mut AppWriteGuard,
        transactions: &ProviderTransactions,
        tenant: &TenantScope,
        slug: &str,
        original_slug: Option<&str>,
        expected_revision: &str,
        content: &str,
        preflight: impl Fn() -> Result<(), WriteError>,
    ) -> Result<PiPromptTemplate, PiTemplateError> {
        let destination = filename(slug)?;
        if let Some(original) = original_slug {
            filename(original)?;
        }
        let Some(original) = original_slug.filter(|original| *original != slug) else {
            return Self::upsert(
                guard, transactions, tenant, slug, expected_revision, content, preflight,
            );
        };
        if content.len() > MAX_BYTES {
            return Err(PiTemplateError::ContentTooLarge);
        }
        let after = template(slug, content.as_bytes())?;
        let source = filename(original)?;
        let before = Self::read(guard, original)?.ok_or(WriteError::Conflict)?;
        if before.revision != expected_revision {
            return Err(WriteError::Conflict.into());
        }
        // Use the actual volume lookup rather than ASCII lowercasing. This also
        // rejects Unicode/canonical aliases on a case-insensitive macOS volume.
        if guard.read(OsStr::new(&destination))?.is_some() {
            return Err(WriteError::Conflict.into());
        }
        let create = WholeFile::new(None, Some(content.as_bytes().to_vec()));
        let remove = WholeFile::new(Some(before.content.as_bytes()), None);
        // Both snapshots are checked during staging. Do not remove the source
        // until the destination has been written and its directory synced.
        // Before publication failures discard staged writes; interrupted partial
        // publication retains the journal for the shared recovery owner.
        transactions.commit_checked(
            tenant,
            guard,
            &[
                FileChange {
                    name: OsStr::new(&destination),
                    privacy: FilePrivacy::Private,
                    patch: &create,
                },
                FileChange {
                    name: OsStr::new(&source),
                    privacy: FilePrivacy::Private,
                    patch: &remove,
                },
            ],
            PendingTarget::default(),
            preflight,
        )?;
        Ok(after)
    }
}
