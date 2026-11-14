//! Executor-local selected migration dispatch; no HTTP authorization is implied.
//! Call only after authorizing the exact scope and every selected domain. Keep
//! previews server-side. Task recovery/projection layers consume owner receipts.
use super::{
    mcp::{McpCatalogStore, McpMigration},
    prompts_store::{PromptMigration, PromptsStore},
};
use crate::{
    app_providers::AppProviderStore,
    gateway::{
        agent_apps::profile_store::ProfileOperation, agent_apps_skills_store::SkillInstallsStore,
    },
    global_database::GlobalDatabase,
};
use anyhow::{Result, anyhow};
use std::{collections::BTreeMap, path::Path};
use uuid::Uuid;
use vw_agent_apps::{
    data_management::{
        apply::{self, ApplyReport},
        preferences::PreferencesPreview,
        preview::Decision,
        profiles::ProfilePreview,
        providers::ProviderPreview,
        safety_snapshot::SafetySnapshotReceipt,
        skills::SkillsPreview,
        source::*,
        usage::UsagePreview,
    },
    skills::bulk_sync::SkillSyncTarget,
};
use vw_api_types::project_profiles::{ProjectProfileCategory, ProjectProfileScope};

type Decisions<'a> = &'a [(Uuid, Decision)];

/// Closed domain operations, not arbitrary namespaces, commands or SQL. The
/// caller retains durable recovery receipts for owners requiring an outer
/// preimage (profiles, preferences, usage) before constructing those variants.
pub(crate) enum PreparedCategory<'a> {
    Providers(&'a AppProviderStore, &'a ProviderPreview, Decisions<'a>),
    Mcp(&'a McpCatalogStore, &'a McpMigration, Decisions<'a>),
    Rules(&'a PromptsStore, &'a PromptMigration, Decisions<'a>),
    Skills {
        store: &'a SkillInstallsStore,
        preview: &'a SkillsPreview,
        decisions: Decisions<'a>,
        root: &'a Path,
        targets: &'a BTreeMap<String, SkillSyncTarget>,
    },
    Profiles {
        operation: &'a ProfileOperation<'a>,
        preview: &'a ProfilePreview,
        decisions: Decisions<'a>,
        recovery: SafetySnapshotReceipt,
        exists: &'a mut dyn FnMut(ProjectProfileCategory, ProjectProfileScope, &str) -> bool,
    },
    Preferences {
        database: &'a GlobalDatabase,
        preview: &'a PreferencesPreview,
        source: &'a [u8],
        decision: Option<Decision>,
        recovery: SafetySnapshotReceipt,
    },
    Usage {
        connection: &'a mut rusqlite::Connection,
        preview: &'a UsagePreview,
        decisions: Decisions<'a>,
        recovery: SafetySnapshotReceipt,
        binding: &'a mut dyn FnMut(&str, &str) -> bool,
    },
}
impl PreparedCategory<'_> {
    fn category(&self) -> SourceCategory {
        match self {
            Self::Providers(..) => SourceCategory::Providers,
            Self::Mcp(..) => SourceCategory::Mcp,
            Self::Rules(..) => SourceCategory::Rules,
            Self::Skills { .. } => SourceCategory::Skills,
            Self::Profiles { .. } => SourceCategory::Profiles,
            Self::Preferences { .. } => SourceCategory::Preferences,
            Self::Usage { .. } => SourceCategory::Usage,
        }
    }
}

/// Receipts are retained on failure as well as success. Owner errors are not
/// serialized to clients: they may contain private storage details.
pub(crate) struct ImportCommit {
    pub report: ApplyReport<anyhow::Error>,
    pub receipts: Vec<(SourceCategory, SafetySnapshotReceipt)>,
}

/// This is category dispatch, not a cross-owner transaction. Stop on the first
/// failure and return earlier commit facts to the recovery task. Re-import uses
/// fresh domain previews, whose identical entries resolve to no writes.
pub(crate) fn commit_selected(
    scope: &SourceScope,
    source: &SourceSnapshot,
    confirmed_source: &SourceSummary,
    selected: &[SourceCategory],
    operations: &mut [PreparedCategory<'_>],
    snapshot_root: &Path,
) -> Result<ImportCommit> {
    let prepared = operations.iter().map(PreparedCategory::category).collect::<Vec<_>>();
    let mut receipts = Vec::new();
    let report =
        apply::apply_selected(scope, source, confirmed_source, selected, &prepared, |category| {
            let operation = operations
                .iter_mut()
                .find(|op| op.category() == category)
                .ok_or_else(|| anyhow!("missing prepared migration category"))?;
            match operation {
                PreparedCategory::Providers(store, preview, decisions) => {
                    let (_, receipt) = store
                        .import_cc_providers(scope, source, preview, decisions, snapshot_root)
                        .map_err(|_| anyhow!("provider migration failed"))?;
                    if let Some(receipt) = receipt {
                        receipts.push((category, receipt));
                    }
                }
                PreparedCategory::Mcp(store, preview, decisions) => {
                    let result = store
                        .commit_data_migration(scope, source, preview, decisions, snapshot_root)
                        .map_err(|_| anyhow!("MCP migration failed"))?;
                    if let Some(receipt) = result.snapshot {
                        receipts.push((category, receipt));
                    }
                    result.outcome.map_err(|_| anyhow!("MCP migration commit failed"))?;
                }
                PreparedCategory::Rules(store, preview, decisions) => {
                    let result = store.commit_data_migration(
                        scope,
                        source,
                        preview,
                        decisions,
                        snapshot_root,
                    )?;
                    if let Some(receipt) = result.snapshot {
                        receipts.push((category, receipt));
                    }
                    result.outcome?;
                }
                PreparedCategory::Skills { store, preview, decisions, root, targets } => {
                    let state = store.migrate_skills(
                        scope,
                        source,
                        root,
                        snapshot_root,
                        preview,
                        decisions,
                        targets,
                    )?;
                    if let Some(receipt) = state.migration_snapshot {
                        receipts.push((category, receipt));
                    }
                }
                PreparedCategory::Profiles { operation, preview, decisions, recovery, exists } => {
                    receipts.push((category, recovery.clone()));
                    operation.import_profiles(scope, source, preview, decisions, &mut **exists)?;
                }
                PreparedCategory::Preferences {
                    database,
                    preview,
                    source: settings,
                    decision,
                    recovery,
                } => {
                    receipts.push((category, recovery.clone()));
                    super::preferences_import::apply(
                        database, scope, preview, settings, *decision,
                    )?;
                }
                PreparedCategory::Usage { connection, preview, decisions, recovery, binding } => {
                    receipts.push((category, recovery.clone()));
                    preview
                        .execute(connection, scope, source, decisions, &mut **binding)
                        .map_err(|_| anyhow!("usage migration failed"))?;
                }
            }
            Ok(())
        })
        .map_err(|_| anyhow!("migration source or selected preparation changed"))?;
    Ok(ImportCommit { report, receipts })
}
