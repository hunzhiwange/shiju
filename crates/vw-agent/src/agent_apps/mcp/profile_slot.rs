//! Project composition delegates to the MCP catalog owner, never native writes.
use std::collections::BTreeMap;

use vw_agent_apps::{
    mcp::import::read,
    project_profiles::{
        apply_mcp::{McpSlotState, apply_mcp},
        snapshot::SnapshotScope,
    },
};
use vw_api_types::{
    mcp_management::{
        McpCatalogScope, McpConnectionDefinition, McpCredentialRef, McpFailureCode, McpNativeStatus,
    },
    project_profiles::{ProjectProfileItemResult, ProjectProfilePayload},
};

use super::{McpBindingTarget, McpCatalogStore};
use crate::{
    agent_apps::auth_accounts::{AuthAccountScope, AuthAccountStore},
    global_database::run_sync_database,
};

/// Trusted execution context, assembled after project/host/installation and
/// catalog write authorization. No field supplied by a request grants access.
pub(crate) struct ProjectProfileMcpSlot<'a> {
    pub scope: SnapshotScope<'a>,
    pub catalog: &'a McpCatalogScope,
    pub target: &'a McpBindingTarget<'a>,
    pub accounts: &'a AuthAccountStore,
}

impl McpCatalogStore {
    /// Call on the authorized execution host under the outer composition
    /// operation boundary. Do not pre-acquire the catalog/native write guards.
    /// Each toggle keeps the existing credential policy and revision checks.
    pub(crate) async fn apply_project_profile_mcp<'a>(
        &self,
        payload: &ProjectProfilePayload,
        slot: &ProjectProfileMcpSlot<'_>,
        authorize_credential_use: impl Fn(
            &McpConnectionDefinition,
            &McpCredentialRef,
        ) -> Option<AuthAccountScope<'a>>
        + Send
        + Sync,
    ) -> Vec<ProjectProfileItemResult> {
        apply_mcp(
            payload,
            slot.scope.group,
            slot.target.target,
            || {
                let allowed = match slot.catalog {
                    McpCatalogScope::Tenant { scope } => scope == slot.scope.project.tenant_scope(),
                    McpCatalogScope::Project { scope } => scope == slot.scope.project,
                };
                if !allowed || slot.target.target.target_host_id != slot.scope.target_host_id {
                    return Err(McpFailureCode::PermissionDenied);
                }
                slot.target.validate(slot.catalog)?;
                run_sync_database(|| Ok(self.read_profile_mcp_state(slot)))
                    .map_err(|_| McpFailureCode::StorageUnavailable)?
            },
            |record, enabled| {
                let authorize = &authorize_credential_use;
                async move {
                    self.set_app_binding(
                        slot.catalog,
                        &record,
                        slot.target,
                        enabled,
                        slot.accounts,
                        authorize,
                    )
                    .await
                }
            },
        )
        .await
    }

    fn read_profile_mcp_state(
        &self,
        slot: &ProjectProfileMcpSlot<'_>,
    ) -> Result<McpSlotState, McpFailureCode> {
        let target = slot.target.target;
        let (records, bindings) = self.list_for_host(slot.catalog, &target.target_host_id)?;
        let native = read(slot.scope.project.tenant_scope(), target, slot.target.source)?;
        let mut observed = BTreeMap::new();
        for record in records {
            let id = &record.record.id.0;
            let enabled = match native.get(id) {
                Some(Ok(candidate)) => candidate.enabled,
                Some(Err(code)) => return Err(*code),
                None => false,
            };
            let binding = bindings
                .iter()
                .find(|binding| binding.target == *target && binding.record.id == record.record.id);
            match binding {
                Some(binding)
                    if binding.enabled == enabled
                        && (if native.contains_key(id) {
                            binding.native_status == McpNativeStatus::Present
                        } else {
                            matches!(
                                binding.native_status,
                                McpNativeStatus::Missing | McpNativeStatus::NotVerified
                            )
                        }) => {}
                None if !native.contains_key(id) => {}
                _ => return Err(McpFailureCode::Conflict),
            }
            observed.insert(id.clone(), (record.record, enabled));
        }
        // A profile must not claim to restore an enablement set containing
        // native entries it cannot resolve and manage through this catalog.
        if native.keys().any(|id| !observed.contains_key(id)) {
            return Err(McpFailureCode::Conflict);
        }
        Ok(McpSlotState { target: target.clone(), records: observed })
    }
}
