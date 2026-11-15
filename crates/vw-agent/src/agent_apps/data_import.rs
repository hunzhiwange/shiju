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
use vw_api_project_profiles::{ProjectProfileCategory, ProjectProfileScope};

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

// Concrete host adapters retain their existing prepared inputs. The library owns
// source validation, selection dispatch, failure ordering and receipt retention.
impl agent_app_services::data_import::PreparedImport for PreparedCategory<'_> {
    fn category(&self) -> SourceCategory {
        PreparedCategory::category(self)
    }

    fn commit(
        &mut self,
        scope: &SourceScope,
        source: &SourceSnapshot,
        snapshot_root: &Path,
        receipts: &mut Vec<(SourceCategory, SafetySnapshotReceipt)>,
    ) -> Result<()> {
        let category = self.category();
        match self {
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
                agent_app_services::preferences_import::apply(
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
    }
}

pub(crate) use agent_app_services::data_import::{ImportCommit, commit_selected};
