//! Retire an authorized catalog revision and attempt each saved native revocation.

use super::{
    agent_apps,
    mcp_management_read::{
        McpHttpError, PolicyFile, access_error, failure, open_database, unavailable,
    },
};
use crate::{agent_apps::mcp::McpCatalogStore, global_database::GlobalDatabase};
use axum::{
    Extension, Json,
    extract::{Path, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
};
use vw_api_types::{
    ProjectScope, TenantScope,
    mcp_management::{
        McpBatchOutcome, McpCatalogScope, McpDeleteRequest, McpDistributionItem,
        McpDistributionOutcome, McpDistributionResult, McpFailureCode, McpNativeStatus,
        McpRecordId, McpRecordRef,
    },
};

pub(super) async fn delete_catalog_record(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Path(id): Path<String>,
    database: Option<Extension<GlobalDatabase>>,
    request: Result<Json<McpDeleteRequest>, JsonRejection>,
) -> Result<Json<McpDistributionResult>, McpHttpError> {
    let Json(request) = request.map_err(|error| {
        let status = if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
            StatusCode::PAYLOAD_TOO_LARGE
        } else {
            StatusCode::BAD_REQUEST
        };
        failure(status, McpFailureCode::InvalidDefinition)
    })?;
    tokio::task::spawn_blocking(move || {
        let scope = match request.project_id {
            Some(project_id) => McpCatalogScope::Project {
                scope: ProjectScope::new(tenant, project_id)
                    .map_err(|_| access_error(StatusCode::BAD_REQUEST))?,
            },
            None => McpCatalogScope::Tenant { scope: tenant },
        };
        agent_apps::authorize_mcp_catalog_write(
            policy.0.as_deref(),
            &headers,
            &scope,
            &request.target_host_id,
        )
        .map_err(access_error)?;
        let store = McpCatalogStore::new(open_database(database)?).map_err(unavailable)?;
        let expected = McpRecordRef { id: McpRecordId(id), revision: request.expected_revision };
        let revocations = store.delete(&scope, &expected).map_err(|code| {
            let status = match code {
                McpFailureCode::InvalidDefinition => StatusCode::BAD_REQUEST,
                McpFailureCode::NotFound => StatusCode::NOT_FOUND,
                McpFailureCode::StaleRevision | McpFailureCode::Conflict => StatusCode::CONFLICT,
                _ => StatusCode::SERVICE_UNAVAILABLE,
            };
            failure(status, code)
        })?;
        let mut items = Vec::with_capacity(revocations.len());
        for revocation in revocations {
            let was_present = revocation.binding.native_status == McpNativeStatus::Present;
            let target = revocation.binding.target;
            let native_outcome = if revocation.outcome == Some(Ok(())) {
                McpDistributionOutcome::Unchanged
            } else {
                let outcome = cleanup(&policy, &headers, &scope, &store, &expected, &target);
                match outcome {
                    Ok(()) if was_present => McpDistributionOutcome::Applied,
                    Ok(()) => McpDistributionOutcome::Unchanged,
                    Err(code) => {
                        // Persist authorization/resolution failures too, without exposing paths.
                        let code = store
                            .record_revocation_result(&scope, &expected, &target, Err(code))
                            .err()
                            .unwrap_or(code);
                        McpDistributionOutcome::Failed { code }
                    }
                }
            };
            items.push(McpDistributionItem {
                record: expected.clone(),
                target,
                enabled: false,
                desired_committed: true,
                native_outcome,
            });
        }
        let failures = items
            .iter()
            .filter(|item| matches!(item.native_outcome, McpDistributionOutcome::Failed { .. }))
            .count();
        let outcome = if failures == 0 {
            McpBatchOutcome::Succeeded
        } else if failures == items.len() {
            McpBatchOutcome::Failed
        } else {
            McpBatchOutcome::PartiallySucceeded
        };
        Ok(Json(McpDistributionResult { outcome, items }))
    })
    .await
    .map_err(unavailable)?
}

#[cfg(target_os = "macos")]
fn cleanup(
    policy: &PolicyFile,
    headers: &HeaderMap,
    scope: &McpCatalogScope,
    store: &McpCatalogStore,
    expected: &McpRecordRef,
    target: &vw_api_types::mcp_management::McpAppTarget,
) -> Result<(), McpFailureCode> {
    use crate::agent_apps::mcp::McpBindingTarget;
    use vw_agent_apps::{
        apps::discovery::read,
        mcp::import::NativeMcpSource,
        providers::live::commit::{ExecutionHostId, ProviderTransactions},
    };
    use vw_api_types::agent_apps::AppId;

    agent_apps::authorize_mcp_catalog_write(
        policy.0.as_deref(),
        headers,
        scope,
        &target.target_host_id,
    )
    .map_err(|_| McpFailureCode::PermissionDenied)?;
    let tenant = match scope {
        McpCatalogScope::Tenant { scope } => scope,
        McpCatalogScope::Project { scope } => scope.tenant_scope(),
    };
    let (env, root) = agent_apps::authorize_mcp_native_write(
        policy.0.as_deref(),
        headers,
        tenant,
        &target.target_host_id,
    )
    .map_err(|status| match status {
        StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED => McpFailureCode::PermissionDenied,
        _ => McpFailureCode::TargetUnavailable,
    })?;
    // Existing bindings carry no Claude account/profile identity. Never guess a default
    // account or use HOME: retain the revocation until its original profile is known.
    if target.app_id == AppId::Claude {
        return Err(McpFailureCode::CredentialUnavailable);
    }
    if !read::resolve_cli_installation(target.app_id, &env, &target.installation_id)
        .map_err(|_| McpFailureCode::TargetUnavailable)?
    {
        return Err(McpFailureCode::TargetUnavailable);
    }
    let installation = read::read(target.app_id, &env)
        .map_err(|_| McpFailureCode::TargetUnavailable)?
        .installations
        .into_iter()
        .find(|installation| installation.id == target.installation_id)
        .ok_or(McpFailureCode::TargetUnavailable)?;
    let host = ExecutionHostId::new(&target.target_host_id)
        .map_err(|_| McpFailureCode::TargetUnavailable)?;
    let transactions =
        ProviderTransactions::new(&root, host).map_err(|_| McpFailureCode::NativeWriteFailed)?;
    store
        .revoke_app_binding(
            scope,
            expected,
            &McpBindingTarget {
                target,
                installation: &installation,
                source: NativeMcpSource::HostHome(&env.home),
                transactions: &transactions,
            },
        )?
        .outcome
        .unwrap_or(Err(McpFailureCode::NativeWriteFailed))
}

#[cfg(not(target_os = "macos"))]
fn cleanup(
    _: &PolicyFile,
    _: &HeaderMap,
    _: &McpCatalogScope,
    _: &McpCatalogStore,
    _: &McpRecordRef,
    _: &vw_api_types::mcp_management::McpAppTarget,
) -> Result<(), McpFailureCode> {
    Err(McpFailureCode::TargetUnavailable)
}
