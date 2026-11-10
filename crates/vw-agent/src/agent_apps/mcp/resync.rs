//! Reconcile saved MCP bindings for explicitly selected, authorized applications.

use vw_agent_apps::apps::{catalog::catalog, lifecycle::operations::Operation};
use vw_api_types::{
    agent_apps::AppId,
    mcp_management::{
        McpAppTarget, McpBatchOutcome, McpCatalogScope, McpCliBinding, McpConnectionDefinition,
        McpCredentialRef, McpDistributionItem, McpDistributionOutcome, McpDistributionResult,
        McpFailureCode, McpRecordRef,
    },
};

use super::{
    McpBindingTarget, McpCatalogStore, StoredCatalogEntry, binding::native_definition,
    catalog_create::catalog_location,
};
use crate::{
    agent_apps::auth_accounts::{AuthAccountScope, AuthAccountStore},
    global_database::run_sync_database,
};

#[derive(Debug)]
pub struct McpResyncResult {
    pub outcome: McpBatchOutcome,
    pub targets: Vec<McpResyncTargetResult>,
}

#[derive(Debug)]
pub struct McpResyncTargetResult {
    pub app_id: AppId,
    pub distribution: Result<McpDistributionResult, McpFailureCode>,
}

/// The caller runs the worker inside the host operation and may record per-app
/// progress from the callback, which receives the report, completed count and total.
pub struct McpResyncTask<'a, F> {
    pub operation: &'a Operation,
    pub on_target: F,
}

pub fn resync_targets(requested: &[String]) -> Result<Vec<AppId>, McpFailureCode> {
    let supported = |app| !matches!(app, AppId::ClaudeDesktop | AppId::OpenClaw);
    if requested.is_empty() {
        return Ok(catalog()
            .into_iter()
            .map(|app| app.metadata.id)
            .filter(|app| supported(*app))
            .collect());
    }

    let mut targets = Vec::new();
    for id in requested {
        let app = serde_json::from_value(serde_json::Value::String(id.clone()))
            .map_err(|_| McpFailureCode::InvalidDefinition)?;
        if !supported(app) {
            return Err(McpFailureCode::TargetUnavailable);
        }
        if !targets.contains(&app) {
            targets.push(app);
        }
    }
    Ok(targets)
}

impl McpCatalogStore {
    /// Reconcile saved intent without changing its revision or claiming external entries.
    /// Resolve only selected apps through the caller's trusted execution context. Cancellation
    /// preserves completed results and marks remaining targets as cancelled.
    pub async fn resync_apps<'target, 'account>(
        &self,
        scope: &McpCatalogScope,
        requested: &[String],
        resolve_target: impl FnMut(AppId) -> Result<McpBindingTarget<'target>, McpFailureCode>,
        accounts: &AuthAccountStore,
        authorize_credential_use: impl Fn(
            &McpAppTarget,
            &McpConnectionDefinition,
            &McpCredentialRef,
        ) -> Option<AuthAccountScope<'account>>
        + Send
        + Sync,
        task: McpResyncTask<'_, impl FnMut(&McpResyncTargetResult, usize, usize)>,
    ) -> Result<McpResyncResult, McpFailureCode> {
        let apps = resync_targets(requested)?;
        let McpResyncTask { operation, mut on_target } = task;
        let mut resolve_target = resolve_target;
        let mut targets = Vec::with_capacity(apps.len());
        let bindings = if operation.is_cancelled() {
            Vec::new()
        } else {
            run_sync_database(|| Ok(self.resync_bindings(scope)))
                .map_err(|_| McpFailureCode::StorageUnavailable)??
        };

        let total = apps.len();
        let mut has_success = false;
        let mut has_failure = false;
        let mut cancelled = false;
        for app_id in apps {
            let distribution = if operation.is_cancelled() {
                Ok(McpDistributionResult { outcome: McpBatchOutcome::Cancelled, items: Vec::new() })
            } else {
                match resolve_target(app_id) {
                    Ok(target) if target.target.app_id == app_id => {
                        let authorize =
                            |definition: &McpConnectionDefinition, reference: &McpCredentialRef| {
                                authorize_credential_use(target.target, definition, reference)
                            };
                        self.resync_app(scope, &bindings, &target, accounts, authorize, operation)
                            .await
                    }
                    Ok(_) => Err(McpFailureCode::TargetUnavailable),
                    Err(code) => Err(code),
                }
            };
            match &distribution {
                Ok(report) if report.outcome == McpBatchOutcome::Cancelled => cancelled = true,
                Ok(report) if report.outcome == McpBatchOutcome::PartiallySucceeded => {
                    has_success = true;
                    has_failure = true;
                }
                Ok(report) if report.outcome == McpBatchOutcome::Succeeded => has_success = true,
                _ => has_failure = true,
            }
            let report = McpResyncTargetResult { app_id, distribution };
            if !cancelled {
                on_target(&report, targets.len() + 1, total);
            }
            targets.push(report);
        }

        let outcome = match (cancelled, has_success, has_failure) {
            (true, _, _) => McpBatchOutcome::Cancelled,
            (false, true, true) => McpBatchOutcome::PartiallySucceeded,
            (false, false, true) => McpBatchOutcome::Failed,
            _ => McpBatchOutcome::Succeeded,
        };
        Ok(McpResyncResult { outcome, targets })
    }

    fn resync_bindings(
        &self,
        scope: &McpCatalogScope,
    ) -> Result<Vec<(McpRecordRef, Vec<McpCliBinding>)>, McpFailureCode> {
        let (tenant, namespace) = catalog_location(scope)?;
        let rows =
            self.0.list(tenant, &namespace).map_err(|_| McpFailureCode::StorageUnavailable)?;
        let mut bindings = Vec::new();
        for row in rows {
            let entry = serde_json::from_str(&row.payload_json)
                .map_err(|_| McpFailureCode::StorageUnavailable)?;
            if let StoredCatalogEntry::Active(stored) = entry
                && !stored.bindings.is_empty()
            {
                bindings.push((stored.record.record, stored.bindings));
            }
        }
        Ok(bindings)
    }

    async fn resync_app<'a>(
        &self,
        scope: &McpCatalogScope,
        bindings: &[(McpRecordRef, Vec<McpCliBinding>)],
        target: &McpBindingTarget<'_>,
        accounts: &AuthAccountStore,
        authorize: impl Fn(&McpConnectionDefinition, &McpCredentialRef) -> Option<AuthAccountScope<'a>>
        + Send
        + Sync,
        operation: &Operation,
    ) -> Result<McpDistributionResult, McpFailureCode> {
        target.validate(scope)?;
        let mut items = Vec::new();
        let mut succeeded = 0;
        for (record, bindings) in bindings {
            let Some(binding) = bindings.iter().find(|binding| binding.target == *target.target)
            else {
                continue;
            };
            if operation.is_cancelled() {
                return Ok(McpDistributionResult { outcome: McpBatchOutcome::Cancelled, items });
            }
            let connection = if binding.enabled {
                self.resolve_for_execution(
                    scope,
                    record,
                    &target.target.target_host_id,
                    accounts,
                    &authorize,
                )
                .await
                .and_then(native_definition)
                .map(Some)
            } else {
                Ok(None)
            };
            if operation.is_cancelled() {
                return Ok(McpDistributionResult { outcome: McpBatchOutcome::Cancelled, items });
            }
            let result = connection.and_then(|connection| {
                run_sync_database(|| {
                    Ok(self.publish_binding(scope, record, target, connection.as_ref(), true))
                })
                .map_err(|_| McpFailureCode::StorageUnavailable)?
            });
            let native_outcome = match result {
                Ok(outcome) => {
                    succeeded += 1;
                    outcome
                }
                Err(code) => McpDistributionOutcome::Failed { code },
            };
            items.push(McpDistributionItem {
                record: record.clone(),
                target: binding.target.clone(),
                enabled: binding.enabled,
                desired_committed: true,
                native_outcome,
            });
        }

        let outcome = if succeeded == items.len() {
            McpBatchOutcome::Succeeded
        } else if succeeded == 0 {
            McpBatchOutcome::Failed
        } else {
            McpBatchOutcome::PartiallySucceeded
        };
        Ok(McpDistributionResult { outcome, items })
    }
}
