//! Apply a batch of managed MCP bindings to one authorized installation.

use std::collections::HashMap;

use vw_api_types::mcp_management::{
    McpBatchOutcome, McpCatalogScope, McpConnectionDefinition, McpCredentialRef,
    McpDistributionItem, McpDistributionOutcome, McpDistributionResult, McpRecordRef,
};

use super::{McpBindingTarget, McpCatalogStore};
use crate::agent_apps::auth_accounts::{AuthAccountScope, AuthAccountStore};

impl McpCatalogStore {
    /// Apply requested revisions in order, preserving independent failures and commits.
    /// Exact duplicates share one attempt and keep their positions in the report.
    pub async fn set_app_bindings<'a>(
        &self,
        scope: &McpCatalogScope,
        records: &[McpRecordRef],
        target: &McpBindingTarget<'_>,
        enabled: bool,
        accounts: &AuthAccountStore,
        authorize_credential_use: impl Fn(
            &McpConnectionDefinition,
            &McpCredentialRef,
        ) -> Option<AuthAccountScope<'a>>
        + Send
        + Sync,
    ) -> McpDistributionResult {
        let mut items: Vec<McpDistributionItem> = Vec::with_capacity(records.len());
        let mut attempts = HashMap::<_, usize>::new();
        let mut succeeded = 0;
        for record in records {
            let key = (&record.id, record.revision);
            let item = if let Some(&index) = attempts.get(&key) {
                items[index].clone()
            } else {
                let item = self
                    .set_app_binding(
                        scope,
                        record,
                        target,
                        enabled,
                        accounts,
                        &authorize_credential_use,
                    )
                    .await
                    .unwrap_or_else(|code| McpDistributionItem {
                        record: record.clone(),
                        target: target.target.clone(),
                        enabled,
                        desired_committed: false,
                        native_outcome: McpDistributionOutcome::Failed { code },
                    });
                attempts.insert(key, items.len());
                item
            };
            if matches!(
                item.native_outcome,
                McpDistributionOutcome::Applied | McpDistributionOutcome::Unchanged
            ) {
                succeeded += 1;
            }
            items.push(item);
        }
        let outcome = if succeeded == items.len() {
            McpBatchOutcome::Succeeded
        } else if succeeded == 0 {
            McpBatchOutcome::Failed
        } else {
            McpBatchOutcome::PartiallySucceeded
        };
        McpDistributionResult { outcome, items }
    }
}
