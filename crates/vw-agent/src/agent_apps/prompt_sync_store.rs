//! Authorized saved-state projection; never imports the pre-restore live file.
use super::*;
use std::{collections::{BTreeMap, HashMap}, path::Path};
use vw_agent_apps::{
    apps::lifecycle::operations::Operation,
    prompts::{activation, mcode, sync::{self, Outcome, SkipReason, SyncReport, TargetResult}, targets},
    providers::live::{commit::ProviderTransactions, write_guard::{lock_app, WriteError}},
};

impl PromptsStore {
    /// Host inputs and authorization come from the dispatcher. Hold the physical
    /// target lock across catalog loading, expected-file read and transaction.
    /// No catalog mutation or stale-live backup occurs, including after restore.
    #[allow(clippy::too_many_arguments)]
    pub fn sync_to_live(
        &self, scope: PromptScope<'_>, executor_host: &str, home: &Path,
        override_dir: Option<&str>, environment: &BTreeMap<String, String>,
        transactions: &ProviderTransactions,
        authorize: impl Fn() -> Result<(), WriteError>,
    ) -> Result<Outcome> {
        ensure!(!scope.host.is_empty() && scope.host == executor_host, "unauthorized prompt host");
        authorize().map_err(|_| anyhow::anyhow!("prompt authorization denied"))?;
        match scope.app {
            AppId::Pi => return Ok(Outcome::Skipped(SkipReason::NativePiSelection)),
            AppId::ClaudeDesktop => return Ok(Outcome::Skipped(SkipReason::UnsupportedClaudeDesktop)),
            _ => {},
        }
        let key = scope.key()?;
        let target = targets::resolve(home, scope.app, override_dir, environment)
            .map_err(anyhow::Error::msg)?;
        let directory = target.file.parent().ok_or_else(|| anyhow::anyhow!("invalid prompt target"))?;
        let mut guard = lock_app(scope.app, directory)
            .map_err(|_| anyhow::anyhow!("prompt target lock failed"))?;
        let tenant = scope.project.tenant_scope();
        let previous = self.0.get(tenant, NAMESPACE, &key)?;
        let catalog = Self::decode(previous.as_deref())?;
        let ids: Vec<&str> = catalog.active_id.as_deref().into_iter().collect();
        let Some((selected, duplicate_selection)) = sync::selection(&catalog.entries, &ids)
            .map_err(anyhow::Error::msg)? else {
            return Ok(Outcome::Skipped(SkipReason::NoSelection));
        };
        let name = if scope.app == AppId::Mcode {
            std::ffi::OsStr::new("AGENTS.md")
        } else { activation::filename(scope.app)? };
        let live = guard.read(name).map_err(|_| anyhow::anyhow!("prompt file read failed"))?;
        let validate = || {
            authorize()?;
            let current = self.0.get(tenant, NAMESPACE, &key).map_err(|_| WriteError::Io)?;
            if current != previous { return Err(WriteError::Conflict); }
            Ok(())
        };
        if scope.app == AppId::Mcode {
            mcode::write_and_commit(&mut guard, transactions, tenant, live.as_deref(),
                &selected.content, validate, || Ok(()))
                .map_err(|_| anyhow::anyhow!("MCode prompt projection failed"))?;
        } else {
            activation::publish(&mut guard, transactions, scope.project, live.as_deref(),
                &selected.content, validate, || Ok(()))?;
        }
        Ok(Outcome::Published { duplicate_selection })
    }

    /// Target list and override directories are resolved by the host dispatcher.
    /// The operation owner maps report.cancelled to Cancelled, otherwise any
    /// Failed target to Failed, and retains this report for partial-success UI.
    #[allow(clippy::too_many_arguments)]
    pub fn sync_all_to_live(
        &self, scope: PromptScope<'_>, apps: &[AppId], operation: &Operation,
        executor_host: &str, home: &Path, overrides: &HashMap<AppId, String>,
        environment: &BTreeMap<String, String>, transactions: &ProviderTransactions,
        authorize: impl Fn(PromptScope<'_>) -> Result<(), WriteError>,
        emit: impl FnMut(&TargetResult),
    ) -> SyncReport {
        sync::sync_all(apps, operation, |app| {
            let target_scope = PromptScope { app, ..scope };
            self.sync_to_live(target_scope, executor_host, home,
                overrides.get(&app).map(String::as_str), environment, transactions,
                || authorize(target_scope)).map_err(|_| ())
        }, emit)
    }
}
