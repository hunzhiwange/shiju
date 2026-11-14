//! Pi prompts/ CRUD on an executor-authorized, retained directory.
use std::{ffi::{CStr, OsStr}, os::fd::{AsRawFd, FromRawFd}, fs::File};
use sha2::{Digest, Sha256};
use vw_api_types::{TenantScope, agent_apps::AppId};
use crate::providers::live::{
    commit::{PendingTarget, ProviderTransactions},
    stage::{FileChange, WholeFile},
    write_guard::{AppWriteGuard, FilePrivacy, WriteError},
};
use super::{template_slug::validate_template_slug, template_metadata::{get_pi_prompt_template_summary, PiPromptTemplateSummary}};

pub const MISSING_REVISION: &str = "missing";
pub(super) const MAX_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PiPromptTemplate {
    pub slug: String,
    pub content: String,
    pub revision: String,
    pub summary: PiPromptTemplateSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PiTemplateError {
    InvalidSlug,
    ContentTooLarge,
    InvalidUtf8,
    File(WriteError),
}
impl From<WriteError> for PiTemplateError {
    fn from(error: WriteError) -> Self { Self::File(error) }
}

pub struct PiPromptTemplateService;
impl PiPromptTemplateService {
    /// None means the host resolver confirmed prompts/ is absent. Never map
    /// permission failures or unsafe directories to None. This read creates nothing.
    /// A present guard must anchor prompts/ itself, not the parent agent directory.
    pub fn list(guard: Option<&AppWriteGuard>) -> Result<Vec<PiPromptTemplate>, PiTemplateError> {
        let Some(guard) = guard else { return Ok(Vec::new()); };
        check_guard(guard)?;
        let mut templates = Vec::new();
        for name in names(guard)? {
            let Some(slug) = name.strip_suffix(".md") else { continue; };
            if validate_template_slug(slug).is_err() { continue; }
            // Reject unsafe matching entries rather than following symlinks.
            let bytes = guard.read(OsStr::new(&name))?.ok_or(WriteError::Conflict)?;
            templates.push(template(slug, &bytes)?);
        }
        templates.sort_by(|a, b| a.slug.cmp(&b.slug));
        Ok(templates)
    }

    pub fn read(guard: &AppWriteGuard, slug: &str) -> Result<Option<PiPromptTemplate>, PiTemplateError> {
        check_guard(guard)?;
        let name = filename(slug)?;
        guard.read(OsStr::new(&name))?.map(|bytes| template(slug, &bytes)).transpose()
    }

    /// Empty templates are valid. There is deliberately no rename parameter.
    #[allow(clippy::too_many_arguments)]
    pub fn upsert(
        guard: &mut AppWriteGuard, transactions: &ProviderTransactions,
        tenant: &TenantScope, slug: &str, expected_revision: &str, content: &str,
        preflight: impl Fn() -> Result<(), WriteError>,
    ) -> Result<PiPromptTemplate, PiTemplateError> {
        if content.len() > MAX_BYTES { return Err(PiTemplateError::ContentTooLarge); }
        let after = template(slug, content.as_bytes())?;
        publish(guard, transactions, tenant, slug, expected_revision, Some(content.as_bytes().to_vec()), preflight)?;
        Ok(after)
    }

    pub fn delete(
        guard: &mut AppWriteGuard, transactions: &ProviderTransactions,
        tenant: &TenantScope, slug: &str, expected_revision: &str,
        preflight: impl Fn() -> Result<(), WriteError>,
    ) -> Result<bool, PiTemplateError> {
        publish(guard, transactions, tenant, slug, expected_revision, None, preflight)
    }
}

fn check_guard(guard: &AppWriteGuard) -> Result<(), PiTemplateError> {
    if guard.app() != AppId::Pi { return Err(WriteError::InvalidResource.into()); }
    Ok(())
}
pub(super) fn filename(slug: &str) -> Result<String, PiTemplateError> {
    validate_template_slug(slug).map_err(|_| PiTemplateError::InvalidSlug)?;
    Ok(format!("{slug}.md"))
}
pub(super) fn template(slug: &str, bytes: &[u8]) -> Result<PiPromptTemplate, PiTemplateError> {
    filename(slug)?;
    if bytes.len() > MAX_BYTES { return Err(PiTemplateError::ContentTooLarge); }
    let content = std::str::from_utf8(bytes).map_err(|_| PiTemplateError::InvalidUtf8)?;
    Ok(PiPromptTemplate {
        slug: slug.into(), content: content.into(),
        revision: format!("{:x}", Sha256::digest(bytes)),
        summary: get_pi_prompt_template_summary(content),
    })
}
#[allow(clippy::too_many_arguments)]
fn publish(
    guard: &mut AppWriteGuard, transactions: &ProviderTransactions,
    tenant: &TenantScope, slug: &str, expected_revision: &str, after: Option<Vec<u8>>,
    preflight: impl Fn() -> Result<(), WriteError>,
) -> Result<bool, PiTemplateError> {
    let name = filename(slug)?;
    let before = PiPromptTemplateService::read(guard, slug)?;
    let revision = before.as_ref().map_or(MISSING_REVISION, |value| value.revision.as_str());
    if revision != expected_revision { return Err(WriteError::Conflict.into()); }
    let patch = WholeFile::new(before.as_ref().map(|v| v.content.as_bytes()), after);
    transactions.commit_checked(tenant, guard,
        &[FileChange { name: OsStr::new(&name), privacy: FilePrivacy::Private, patch: &patch }],
        PendingTarget::default(), preflight)?;
    Ok(before.is_some())
}

/// Enumerate the retained directory descriptor, never a re-resolved pathname.
fn names(guard: &AppWriteGuard) -> Result<Vec<String>, PiTemplateError> {
    // A fresh open description avoids sharing a directory offset between lists.
    let raw = unsafe { libc::openat(guard.retained_directory().as_raw_fd(), c".".as_ptr(),
        libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW) };
    if raw < 0 { return Err(WriteError::Io.into()); }
    let file = unsafe { File::from_raw_fd(raw) };
    let stream = unsafe { libc::fdopendir(file.as_raw_fd()) };
    if stream.is_null() { return Err(WriteError::Io.into()); }
    std::mem::forget(file); // fdopendir now owns the descriptor.
    struct Directory(*mut libc::DIR);
    impl Drop for Directory {
        fn drop(&mut self) { unsafe { libc::closedir(self.0); } }
    }
    let directory = Directory(stream);
    let mut names = Vec::new();
    loop {
        // macOS errno is thread-local; distinguish EOF from an enumeration error.
        unsafe { *libc::__error() = 0; }
        let entry = unsafe { libc::readdir(directory.0) };
        if entry.is_null() {
            if unsafe { *libc::__error() } != 0 { return Err(WriteError::Io.into()); }
            break;
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
        if let Ok(name) = name.to_str() { names.push(name.to_string()); }
    }
    Ok(names)
}
