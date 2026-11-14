//! Profile adapter to the existing ordinary prompt transaction owner.
use super::*;
use std::{collections::BTreeMap, path::Path};
use vw_agent_apps::{
    project_profiles::{apply_prompt::{apply_prompt, PromptApplyFailure, PromptObservation}, snapshot::SnapshotScope},
    prompts::{activation, targets},
    providers::live::{commit::ProviderTransactions, write_guard::{lock_app, WriteError}},
};
use vw_api_types::project_profiles::ProjectProfilePayload;

impl PromptsStore {
    /// Executor-owned paths and authorization only. The outer profile operation
    /// serializes prepare/apply/select; the domain rechecks revision and preimage.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_profile_prompt(
        &self, scope: &SnapshotScope<'_>, payload: &ProjectProfilePayload,
        executor_host: &str, home: &Path, override_dir: Option<&str>,
        environment: &BTreeMap<String, String>, transactions: &ProviderTransactions,
        authorize: impl Fn() -> Result<(), WriteError>,
    ) -> Option<Result<(), PromptApplyFailure>> {
        let prompt_scope = PromptScope {
            project: scope.project, host: scope.target_host_id, app: scope.group.app_id(),
        };
        apply_prompt(scope, payload, |_| {
            if scope.target_host_id != executor_host { return Err(PromptApplyFailure::Denied); }
            authorize().map_err(|_| PromptApplyFailure::Denied)?;
            let name = activation::filename(prompt_scope.app).map_err(|_| PromptApplyFailure::Unsupported)?;
            let target = targets::resolve(home, prompt_scope.app, override_dir, environment)
                .map_err(|_| PromptApplyFailure::Failed)?;
            let directory = target.file.parent().ok_or(PromptApplyFailure::Failed)?;
            let guard = lock_app(prompt_scope.app, directory).map_err(|_| PromptApplyFailure::Conflict)?;
            transactions.state(prompt_scope.app).map_err(|_| PromptApplyFailure::Failed)?
                .require_project(scope.project).map_err(|_| PromptApplyFailure::Denied)?;
            let catalog = self.list(prompt_scope).map_err(|_| PromptApplyFailure::Failed)?;
            let live = guard.read(name).map_err(|_| PromptApplyFailure::Failed)?;
            Ok(PromptObservation { revision: catalog.revision, active_id: catalog.active_id,
                ids: catalog.entries.into_iter().map(|entry| entry.id).collect(), live })
        }, |_, desired, before| {
            let result = match desired {
                Some(id) => self.activate_ordinary(prompt_scope, before.revision, id, None,
                    executor_host, home, override_dir, environment, transactions, &authorize).map(|_| ()),
                None => self.reconcile_ordinary(prompt_scope, before.revision,
                    OrdinaryPromptReconciliation::Deactivate {
                        id: before.active_id.as_deref().ok_or(PromptApplyFailure::Conflict)?,
                        expected: before.live.as_deref(),
                    }, executor_host, home, override_dir, environment, transactions, &authorize).map(|_| ()),
            };
            result.map_err(|_| PromptApplyFailure::Failed)
        })
    }
}
