//! Managed Codex catalog publication through the existing native transaction owner.
use super::codex_catalog::{
    CodexCatalogDefault, CodexCatalogError, CodexCatalogMember, plan_codex_stack_catalog,
};
use crate::providers::{
    codex_catalog::{CATALOG_FILENAME, CodexFiles},
    live::{
        commit::{AppState, PendingTarget, ProviderTransactions},
        stage::{FileChange, WholeFile},
        write_guard::{FilePrivacy, WriteError},
    },
    mode_state::observe,
};
use std::{ffi::OsStr, path::Path};
use toml_edit::DocumentMut;
use vw_api_types::{agent_apps::AppId, id::ProjectScope};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatalogPolicy {
    Preserve,
    UseManaged,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodexPublishError {
    Forbidden,
    RevisionConflict,
    RecoveryPending,
    InvalidConfiguration,
    UnsupportedTarget,
    TargetFailed,
    RecoveryFailed,
}
#[derive(Debug, PartialEq, Eq)]
pub struct PublishFailure<'a> {
    pub code: CodexPublishError,
    /// The caller's accepted revision, never a claim that partial publication settled.
    pub previous_revision: &'a str,
    pub recovery_pending: bool,
}
pub struct Publication<'a> {
    pub scope: &'a ProjectScope,
    pub expected: &'a AppState,
    pub expected_revision: &'a str,
    pub policy: CatalogPolicy,
    pub default: CodexCatalogDefault<'a>,
    pub members: &'a [CodexCatalogMember<'a>],
}

/// Publish catalog and pointer only; routing, login and full client contracts stay
/// with their existing owners. No file path or credentials come from HTTP input.
/// The executor authorizes host/project/app and captures this snapshot under the
/// physical guard. `unchanged` rechecks its full scoped revision, including native
/// login and saved provider inputs, before any publication starts.
pub fn publish<'a>(
    files: &mut CodexFiles,
    transactions: &ProviderTransactions,
    publication: Publication<'a>,
    unchanged: impl Fn() -> bool,
) -> Result<bool, PublishFailure<'a>> {
    let previous_revision = publication.expected_revision;
    publish_inner(files, transactions, publication, unchanged).map_err(|code| PublishFailure {
        code,
        previous_revision,
        recovery_pending: transactions.pending(AppId::Codex).map_or(true, |p| p.is_some()),
    })
}

fn publish_inner(
    files: &mut CodexFiles,
    transactions: &ProviderTransactions,
    publication: Publication<'_>,
    unchanged: impl Fn() -> bool,
) -> Result<bool, CodexPublishError> {
    use CodexPublishError::*;
    let Publication { scope, expected, expected_revision, policy, default, members } = publication;
    let tenant = scope.tenant_scope();
    if files.guard.app() != AppId::Codex {
        return Err(UnsupportedTarget);
    }
    if expected_revision.trim().is_empty() {
        return Err(InvalidConfiguration);
    }
    let state = transactions.state(AppId::Codex).map_err(write_error)?;
    observe(&state, tenant).map_err(|_| Forbidden)?;
    state.require_project(scope).map_err(|_| Forbidden)?;
    if let Some(pending) = transactions.pending(AppId::Codex).map_err(write_error)? {
        return Err(
            if &pending.tenant == tenant && pending.target.project_scope.as_ref() == Some(scope) {
                RecoveryPending
            } else {
                Forbidden
            },
        );
    }
    let check = || {
        if transactions.state(AppId::Codex)? != *expected || !unchanged() {
            return Err(WriteError::Conflict);
        }
        Ok(())
    };
    check().map_err(write_error)?;
    let before = files.guard.read(OsStr::new("config.toml")).map_err(write_error)?;
    let live = std::str::from_utf8(before.as_deref().unwrap_or_default())
        .map_err(|_| InvalidConfiguration)?;
    let mut doc: DocumentMut = live.parse().map_err(|_| InvalidConfiguration)?;
    let default_config = match &default {
        CodexCatalogDefault::ThirdParty(row) => row.config,
        CodexCatalogDefault::Official { config, .. } => config,
    };
    let foreign = foreign_pointer(default_config)?.or(foreign_pointer(&doc)?).map(str::to_owned);
    let preserve = policy == CatalogPolicy::Preserve && foreign.is_some();
    let catalog = if preserve {
        // Never read the external path or replace its catalog as a side effect.
        doc.insert("model_catalog_json", toml_edit::value(foreign.expect("checked pointer")));
        None
    } else {
        let catalog = plan_codex_stack_catalog(default, members).map_err(|error| match error {
            CodexCatalogError::NativeCatalogUnavailable => UnsupportedTarget,
            CodexCatalogError::MissingDefaultModel | CodexCatalogError::InvalidCatalog => {
                InvalidConfiguration
            }
        })?;
        // Global window overrides would replace every member's per-row windows.
        doc.remove("model_context_window");
        doc.remove("model_auto_compact_token_limit");
        doc.insert("model_catalog_json", toml_edit::value(CATALOG_FILENAME));
        Some(serde_json::to_vec_pretty(&catalog).map_err(|_| InvalidConfiguration)?)
    };
    let config_bytes = doc.to_string().into_bytes();
    let old_catalog = if catalog.is_some() {
        files.guard.read(OsStr::new(CATALOG_FILENAME)).map_err(write_error)?
    } else {
        None
    };
    let changed = before.as_deref() != Some(config_bytes.as_slice())
        || catalog.as_ref().is_some_and(|bytes| old_catalog.as_ref() != Some(bytes));
    if !changed {
        check().map_err(write_error)?;
        return Ok(false);
    }
    let config_patch = WholeFile::new(before.as_deref(), Some(config_bytes));
    let catalog_patch = WholeFile::new(old_catalog.as_deref(), catalog.clone());
    // Publish the complete catalog before exposing its pointer to a native reader.
    let mut changes = Vec::new();
    if catalog.is_some() {
        changes.push(FileChange {
            name: OsStr::new(CATALOG_FILENAME),
            privacy: FilePrivacy::Private,
            patch: &catalog_patch,
        });
    }
    changes.push(FileChange {
        name: OsStr::new("config.toml"),
        privacy: FilePrivacy::Private,
        patch: &config_patch,
    });
    let result = transactions.commit_project_checked(
        scope,
        &mut files.guard,
        expected,
        false,
        &changes,
        PendingTarget::default(),
        check,
    );
    if let Err(error) = result {
        if transactions
            .pending(AppId::Codex)
            .map_err(|_| RecoveryFailed)?
            .is_some_and(|pending| !pending.published)
        {
            // Only our unpublished intent can be discarded. Once publication has
            // started, leave the frozen intent with the normal recovery owner.
            transactions.recover_project(scope, &mut files.guard).map_err(|_| RecoveryFailed)?;
        }
        return Err(write_error(error));
    }
    Ok(true)
}

fn foreign_pointer(doc: &DocumentMut) -> Result<Option<&str>, CodexPublishError> {
    let Some(item) = doc.get("model_catalog_json") else { return Ok(None) };
    let pointer = item
        .as_str()
        .filter(|p| !p.trim().is_empty() && !p.chars().any(char::is_control))
        .ok_or(CodexPublishError::InvalidConfiguration)?;
    Ok((!Path::new(pointer)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| [CATALOG_FILENAME, "cc-switch-model-catalog.json"].contains(&name)))
    .then_some(pointer))
}

fn write_error(error: WriteError) -> CodexPublishError {
    match error {
        WriteError::Conflict => CodexPublishError::RevisionConflict,
        WriteError::Recovered => CodexPublishError::RecoveryPending,
        WriteError::InvalidResource => CodexPublishError::InvalidConfiguration,
        WriteError::Io => CodexPublishError::TargetFailed,
    }
}
