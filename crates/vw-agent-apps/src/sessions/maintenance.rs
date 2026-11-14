//! One host-executor entry for history maintenance and provider-switch retries.
//! Authorization, task persistence and native-writer exclusion belong to the
//! existing host owner. This module adds no task store or configuration store.
use super::{
    migration::{provider_refs, restore, unify},
    source::SourceRegistry,
};
use crate::apps::lifecycle::operations::{Operation, Phase, Progress};
use std::{
    collections::BTreeSet,
    io,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use vw_api_types::{agent_apps::AppId, external_session::ExternalSessionIdentity};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaintenancePhase {
    Preparing,
    ThirdParty,
    Unifying,
    Restoring,
    LegacyOfficial,
    Terminal,
}

#[derive(Clone, Debug, Default)]
pub struct MaintenanceOutcome {
    pub changed_rollouts: usize,
    pub changed_state_rows: usize,
    pub skipped_reason: Option<&'static str>,
}

#[derive(Clone, Debug)]
pub struct MaintenanceDetails {
    pub phase: MaintenancePhase,
    /// Fully processed input units, including idempotent no-ops.
    pub completed: usize,
    /// Upper bound: restoration may only visit databases present in its ledger.
    pub total: usize,
    pub outcome: Option<MaintenanceOutcome>,
}

#[derive(Clone, Debug)]
pub struct MaintenanceProgress {
    pub operation: Progress,
    /// A request remains pending until the underlying operation reaches terminal.
    pub cancellation_requested: bool,
    pub details: MaintenanceDetails,
}

/// Retain this handle in the existing authorized task owner. Every query/cancel
/// must repeat its project, principal and host authorization there. Subscribers
/// dropping never abort a worker or release its lifecycle reservation.
#[derive(Clone)]
pub struct MaintenanceOperation {
    operation: Operation,
    details: Arc<Mutex<MaintenanceDetails>>,
}
impl Default for MaintenanceOperation {
    fn default() -> Self {
        Self::new()
    }
}
impl MaintenanceOperation {
    pub fn new() -> Self {
        Self {
            operation: Operation::new(),
            details: Arc::new(Mutex::new(MaintenanceDetails {
                phase: MaintenancePhase::Preparing,
                completed: 0,
                total: 0,
                outcome: None,
            })),
        }
    }

    pub fn snapshot(&self) -> MaintenanceProgress {
        let operation = self.operation.snapshot();
        let mut details = self.details.lock().unwrap_or_else(|e| e.into_inner()).clone();
        // Queued cancellation or coordinator rejection can terminate before work.
        if operation.phase == Phase::Terminal {
            details.phase = MaintenancePhase::Terminal;
        }
        MaintenanceProgress {
            operation,
            cancellation_requested: self.operation.is_cancelled(),
            details,
        }
    }

    pub fn cancel(&self) -> MaintenanceProgress {
        self.operation.cancel();
        self.snapshot()
    }

    pub async fn wait(&self) -> MaintenanceProgress {
        self.operation.wait().await;
        self.snapshot()
    }

    pub async fn cancel_and_wait(&self) -> MaintenanceProgress {
        self.operation.cancel_and_wait().await;
        self.snapshot()
    }

    /// Used by explicit maintenance and provider-switch/startup retries alike.
    /// `work` runs under the existing host-wide Codex lifecycle reservation. It
    /// must refresh authorization, live settings, sources and database candidates
    /// there, retain cross-process/native-writer exclusion, open the same private
    /// ledger on retry, then invoke `execute`. Do not hold an AppWriteGuard across
    /// execute: rollout/state_db acquire that non-reentrant guard themselves.
    pub fn start(
        &self,
        work: impl FnOnce(&MaintenanceContext) -> io::Result<MaintenanceOutcome> + Send + 'static,
    ) -> Result<(), String> {
        let context =
            MaintenanceContext { operation: self.operation.clone(), details: self.details.clone() };
        self.operation.start(vec![AppId::Codex], move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                context.checkpoint()?;
                work(&context)
            }))
            .unwrap_or_else(|_| Err(io::Error::other("history maintenance worker failed")));
            let mut details = context.details.lock().unwrap_or_else(|e| e.into_inner());
            details.phase = MaintenancePhase::Terminal;
            match result {
                Ok(outcome) => {
                    details.outcome = Some(outcome);
                    // A cancellation arriving after the last commit cannot change
                    // its successful terminal outcome.
                    Ok(())
                }
                Err(error)
                    if error.kind() == io::ErrorKind::Interrupted
                        && context.operation.is_cancelled() =>
                {
                    Err("operation cancelled".into())
                }
                Err(_) => Err("history maintenance failed; retained ledger permits retry".into()),
            }
        })
    }
}

/// Fresh authorized host inputs, never a deserializable client-path request.
pub struct HistorySnapshot<'a> {
    pub registry: &'a SourceRegistry,
    pub binding: &'a unify::GenerationBinding,
    pub identities: &'a [ExternalSessionIdentity],
    pub database_paths: &'a [PathBuf],
    pub live_config: &'a str,
}

/// An executor-loaded saved row. Contains native configuration and possibly
/// credentials: never serialize, debug-print or log this snapshot.
pub struct ProviderTemplateSnapshot<'a> {
    pub provider_id: &'a str,
    pub config: &'a str,
    pub official_or_oauth: bool,
}

pub enum MaintenanceAction<'a> {
    Unify {
        generation: &'a mut unify::BackupGeneration,
        enabled: bool,
        migrate_existing_requested: bool,
    },
    Restore {
        generations: &'a [unify::BackupGeneration],
        undo: &'a mut unify::BackupGeneration,
        unify_enabled: bool,
    },
    /// Sources are derived from authorized saved templates by provider_refs.
    /// Template publication stays in ProviderTransactions and finishes
    /// before this action; its AppWriteGuard must already have been dropped.
    ThirdParty {
        generation: &'a mut unify::BackupGeneration,
        templates: &'a [ProviderTemplateSnapshot<'a>],
    },
    LegacyOfficial {
        backup: &'a mut unify::BackupGeneration,
    },
}

/// Constructed only by start, so the unique entry owns lifecycle exclusion.
/// A provider switch submits its history follow-up through that same entry after
/// its original configuration transaction releases its guards, never nested in it.
pub struct MaintenanceContext {
    operation: Operation,
    details: Arc<Mutex<MaintenanceDetails>>,
}
impl MaintenanceContext {
    pub fn checkpoint(&self) -> io::Result<()> {
        if self.operation.is_cancelled() {
            Err(io::Error::new(io::ErrorKind::Interrupted, "history maintenance cancelled"))
        } else {
            Ok(())
        }
    }

    /// `still_current` rechecks the trusted live setting/configuration revision
    /// at each safe boundary. It must fail closed on revocation or host errors.
    /// Returning false preserves an incomplete, retryable ledger. No cancellation
    /// is observed inside a rollout publication or SQLite transaction.
    pub fn execute(
        &self,
        history: &HistorySnapshot<'_>,
        action: MaintenanceAction<'_>,
        mut still_current: impl FnMut() -> io::Result<bool>,
    ) -> io::Result<MaintenanceOutcome> {
        self.checkpoint()?;
        let phase = match &action {
            MaintenanceAction::Unify { .. } => MaintenancePhase::Unifying,
            MaintenanceAction::Restore { .. } => MaintenancePhase::Restoring,
            MaintenanceAction::ThirdParty { .. } => MaintenancePhase::ThirdParty,
            MaintenanceAction::LegacyOfficial { .. } => MaintenancePhase::LegacyOfficial,
        };
        {
            let mut details = self.details.lock().unwrap_or_else(|e| e.into_inner());
            details.phase = phase;
            details.completed = 0;
            details.total = history.identities.len() + history.database_paths.len();
            details.outcome = None;
        }
        let mut allowed = || {
            self.checkpoint()?;
            still_current()
        };
        let advanced = || {
            self.details.lock().unwrap_or_else(|e| e.into_inner()).completed += 1;
        };
        if !allowed()? {
            return Ok(MaintenanceOutcome {
                skipped_reason: Some("maintenance_state_changed"),
                ..Default::default()
            });
        }
        match action {
            MaintenanceAction::Unify { generation, enabled, migrate_existing_requested } => {
                check_binding(history, generation)?;
                let outcome = unify::migrate(
                    history.registry,
                    generation,
                    enabled,
                    migrate_existing_requested,
                    history.live_config,
                    history.identities,
                    history.database_paths,
                    allowed,
                    advanced,
                )?;
                Ok(MaintenanceOutcome {
                    changed_rollouts: outcome.migrated_rollouts,
                    changed_state_rows: outcome.migrated_state_rows,
                    skipped_reason: outcome.skipped_reason,
                })
            }
            MaintenanceAction::ThirdParty { generation, templates } => {
                check_binding(history, generation)?;
                let mut source_ids = BTreeSet::new();
                for template in templates {
                    self.checkpoint()?;
                    source_ids.extend(
                        provider_refs::collect_source_model_provider_ids(
                            template.provider_id,
                            template.config,
                            template.official_or_oauth,
                        )
                        .map_err(|_| {
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                "invalid trusted provider template",
                            )
                        })?,
                    );
                }
                if source_ids.is_empty() {
                    return Ok(MaintenanceOutcome {
                        skipped_reason: Some("no_third_party_provider_ids"),
                        ..Default::default()
                    });
                }
                let outcome = unify::migrate_buckets(
                    history.registry,
                    generation,
                    history.identities,
                    history.database_paths,
                    &source_ids,
                    allowed,
                    advanced,
                )?;
                Ok(MaintenanceOutcome {
                    changed_rollouts: outcome.migrated_rollouts,
                    changed_state_rows: outcome.migrated_state_rows,
                    skipped_reason: outcome.skipped_reason,
                })
            }
            MaintenanceAction::Restore { generations, undo, unify_enabled } => {
                let outcome = restore::restore(
                    history.registry,
                    history.binding,
                    generations,
                    undo,
                    unify_enabled,
                    history.live_config,
                    history.identities,
                    history.database_paths,
                    allowed,
                    advanced,
                )?;
                Ok(restored(outcome))
            }
            MaintenanceAction::LegacyOfficial { backup } => {
                let outcome = restore::migrate_legacy(
                    history.registry,
                    history.binding,
                    backup,
                    history.live_config,
                    history.identities,
                    history.database_paths,
                    allowed,
                    advanced,
                )?;
                Ok(restored(outcome))
            }
        }
    }
}

fn check_binding(
    history: &HistorySnapshot<'_>,
    generation: &unify::BackupGeneration,
) -> io::Result<()> {
    if generation.metadata().binding != *history.binding {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "history generation mismatch"));
    }
    Ok(())
}
fn restored(outcome: restore::RestoreOutcome) -> MaintenanceOutcome {
    MaintenanceOutcome {
        changed_rollouts: outcome.restored_rollouts,
        changed_state_rows: outcome.restored_state_rows,
        skipped_reason: outcome.skipped_reason,
    }
}
