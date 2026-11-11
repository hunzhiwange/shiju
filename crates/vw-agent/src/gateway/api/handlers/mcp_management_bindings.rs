//! Authorized single-installation MCP publication, using the catalog's transaction path.
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
use serde::Deserialize;
use vw_api_types::{ProjectScope, TenantScope, mcp_management::*};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct BindingRequest {
    target: McpAppTarget,
    project_id: Option<String>,
    expected_revision: u64,
    enabled: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct BulkRequest {
    target: McpAppTarget,
    project_id: Option<String>,
    records: Vec<McpRecordRef>,
    enabled: bool,
}

pub(super) async fn bind(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Path(id): Path<String>,
    database: Option<Extension<GlobalDatabase>>,
    request: Result<Json<BindingRequest>, JsonRejection>,
) -> Result<Json<McpDistributionResult>, McpHttpError> {
    let Json(request) = request.map_err(|_| access_error(StatusCode::BAD_REQUEST))?;
    execute(
        policy,
        tenant,
        headers,
        database,
        BulkRequest {
            target: request.target,
            project_id: request.project_id,
            records: vec![McpRecordRef {
                id: McpRecordId(id),
                revision: request.expected_revision,
            }],
            enabled: request.enabled,
        },
    )
    .await
}
pub(super) async fn bulk(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    database: Option<Extension<GlobalDatabase>>,
    request: Result<Json<BulkRequest>, JsonRejection>,
) -> Result<Json<McpDistributionResult>, McpHttpError> {
    let Json(request) = request.map_err(|_| access_error(StatusCode::BAD_REQUEST))?;
    execute(policy, tenant, headers, database, request).await
}
async fn execute(
    policy: PolicyFile,
    tenant: TenantScope,
    headers: HeaderMap,
    database: Option<Extension<GlobalDatabase>>,
    request: BulkRequest,
) -> Result<Json<McpDistributionResult>, McpHttpError> {
    let scope = match request.project_id {
        Some(id) => McpCatalogScope::Project {
            scope: ProjectScope::new(tenant.clone(), id)
                .map_err(|_| access_error(StatusCode::BAD_REQUEST))?,
        },
        None => McpCatalogScope::Tenant { scope: tenant.clone() },
    };
    agent_apps::authorize_mcp_catalog_write(
        policy.0.as_deref(),
        &headers,
        &scope,
        &request.target.target_host_id,
    )
    .map_err(access_error)?;
    if request.records.is_empty() || request.records.len() > 256 {
        return Err(access_error(StatusCode::BAD_REQUEST));
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = database;
        Err(failure(StatusCode::NOT_IMPLEMENTED, McpFailureCode::TargetUnavailable))
    }
    #[cfg(target_os = "macos")]
    {
        use crate::agent_apps::{
            auth_accounts::{AuthAccountScope, AuthAccountStore},
            mcp::McpBindingTarget,
        };
        use vw_agent_apps::{
            apps::discovery::read,
            mcp::import::NativeMcpSource,
            providers::live::commit::{ExecutionHostId, ProviderTransactions},
        };
        use vw_api_types::agent_apps::AppId;
        use vw_api_types::agent_apps_auth::AuthService;
        let (env, root) = agent_apps::authorize_mcp_native_write(
            policy.0.as_deref(),
            &headers,
            &tenant,
            &request.target.target_host_id,
        )
        .map_err(access_error)?;
        if request.target.app_id == AppId::Claude {
            return Err(failure(StatusCode::BAD_REQUEST, McpFailureCode::CredentialUnavailable));
        }
        if !read::resolve_cli_installation(
            request.target.app_id,
            &env,
            &request.target.installation_id,
        )
        .map_err(unavailable)?
        {
            return Err(access_error(StatusCode::NOT_FOUND));
        }
        let installation = read::read(request.target.app_id, &env)
            .map_err(unavailable)?
            .installations
            .into_iter()
            .find(|i| i.id == request.target.installation_id)
            .ok_or_else(|| access_error(StatusCode::NOT_FOUND))?;
        let database = open_database(database)?;
        let store = McpCatalogStore::new(database.clone()).map_err(unavailable)?;
        let accounts = AuthAccountStore::new(database, &root).map_err(unavailable)?;
        let transactions = ProviderTransactions::new(
            &root,
            ExecutionHostId::new(&request.target.target_host_id).map_err(unavailable)?,
        )
        .map_err(unavailable)?;
        let mut authorized = Vec::new();
        for service in [
            AuthService::GithubCopilot,
            AuthService::CodexOauth,
            AuthService::XaiOauth,
            AuthService::KimiOauth,
            AuthService::ClaudeOfficial,
            AuthService::AgyOauth,
            AuthService::DshAccount,
        ] {
            if let Ok(owner) = agent_apps::authorize_auth_binding(
                policy.0.as_deref(),
                &headers,
                &tenant,
                &request.target.target_host_id,
                service,
            ) {
                if owner.credential_root == root {
                    authorized.push((service, owner));
                }
            }
        }
        let target = McpBindingTarget {
            target: &request.target,
            installation: &installation,
            source: NativeMcpSource::HostHome(&env.home),
            transactions: &transactions,
        };
        Ok(Json(
            store
                .set_app_bindings(
                    &scope,
                    &request.records,
                    &target,
                    request.enabled,
                    &accounts,
                    |_, reference| {
                        authorized.iter().find(|(service, _)| *service == reference.service).map(
                            |(service, owner)| AuthAccountScope {
                                tenant: &tenant,
                                host: &owner.id,
                                principal: &owner.principal,
                                service: *service,
                            },
                        )
                    },
                )
                .await,
        ))
    }
}
