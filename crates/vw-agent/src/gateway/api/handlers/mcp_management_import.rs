//! Import trusted native definitions into the authorized catalog without rewriting native files.
#[cfg(target_os = "macos")]
use super::mcp_management_read::open_database;
use super::{
    agent_apps,
    mcp_management_read::{McpHttpError, PolicyFile, access_error, unavailable},
};
#[cfg(target_os = "macos")]
use crate::agent_apps::mcp::{McpCatalogStore, McpImportSource};
use crate::{
    agent_apps::mcp::{McpImportOutcome, McpImportReport},
    global_database::GlobalDatabase,
};
use axum::{
    Extension, Json,
    extract::rejection::JsonRejection,
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};
use vw_api_types::{
    ProjectScope, TenantScope,
    mcp_management::{McpAppTarget, McpCatalogScope, McpFailureCode, McpRecordId, McpRecordRef},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ImportRequest {
    project_id: Option<String>,
    sources: Vec<ImportSource>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ImportSource {
    target: McpAppTarget,
    account_id: Option<String>,
}
#[derive(Serialize)]
pub(super) struct ImportResponse {
    sources: Vec<SourceResult>,
}
#[derive(Serialize)]
struct SourceResult {
    target: McpAppTarget,
    #[serde(skip_serializing_if = "Option::is_none")]
    entries: Option<Vec<EntryResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<McpFailureCode>,
}
#[derive(Serialize)]
struct EntryResult {
    id: Option<McpRecordId>,
    #[serde(flatten)]
    outcome: EntryOutcome,
}
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum EntryOutcome {
    Added { record: McpRecordRef },
    Merged { record: McpRecordRef },
    Unchanged { record: McpRecordRef },
    Failed { code: McpFailureCode },
}
impl From<McpImportReport> for SourceResult {
    fn from(report: McpImportReport) -> Self {
        match report.entries {
            Err(code) => Self { target: report.target, entries: None, code: Some(code) },
            Ok(entries) => Self {
                target: report.target,
                code: None,
                entries: Some(
                    entries
                        .into_iter()
                        .map(|entry| EntryResult {
                            id: entry.id,
                            outcome: match entry.outcome {
                                Ok(McpImportOutcome::Added(record)) => {
                                    EntryOutcome::Added { record: record.record }
                                }
                                Ok(McpImportOutcome::Merged(record)) => {
                                    EntryOutcome::Merged { record: record.record }
                                }
                                Ok(McpImportOutcome::Unchanged(record)) => {
                                    EntryOutcome::Unchanged { record: record.record }
                                }
                                Err(code) => EntryOutcome::Failed { code },
                            },
                        })
                        .collect(),
                ),
            },
        }
    }
}

pub(super) async fn import(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    database: Option<Extension<GlobalDatabase>>,
    request: Result<Json<ImportRequest>, JsonRejection>,
) -> Result<Json<ImportResponse>, McpHttpError> {
    let Json(request) = request.map_err(|_| access_error(StatusCode::BAD_REQUEST))?;
    if request.sources.is_empty() || request.sources.len() > 64 {
        return Err(access_error(StatusCode::BAD_REQUEST));
    }
    let scope = match request.project_id {
        Some(id) => McpCatalogScope::Project {
            scope: ProjectScope::new(tenant.clone(), id.into())
                .map_err(|_| access_error(StatusCode::BAD_REQUEST))?,
        },
        None => McpCatalogScope::Tenant { scope: tenant.clone() },
    };
    tokio::task::spawn_blocking(move || {
        // Authorize the entire batch before any source can commit catalog changes.
        for source in &request.sources {
            agent_apps::authorize_mcp_catalog_write(
                policy.0.as_deref(),
                &headers,
                &scope,
                &source.target.target_host_id,
            )
            .map_err(access_error)?;
            agent_apps::authorize_provider_config_access(
                policy.0.as_deref(),
                &headers,
                &tenant,
                &source.target.target_host_id,
                false,
            )
            .map_err(access_error)?;
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = database;
            for source in request.sources {
                let _ = source.account_id;
            }
            Err(access_error(StatusCode::NOT_IMPLEMENTED))
        }
        #[cfg(target_os = "macos")]
        {
            let database = open_database(database)?;
            let store = McpCatalogStore::new(database.clone()).map_err(unavailable)?;
            let sources = request
                .sources
                .into_iter()
                .map(|source| {
                    let target = source.target.clone();
                    match import_source(
                        &policy, &tenant, &headers, &scope, &database, &store, source,
                    ) {
                        Ok(report) => report.into(),
                        Err(code) => SourceResult { target, entries: None, code: Some(code) },
                    }
                })
                .collect();
            Ok(Json(ImportResponse { sources }))
        }
    })
    .await
    .map_err(unavailable)?
}

#[cfg(target_os = "macos")]
fn import_source(
    policy: &PolicyFile,
    tenant: &TenantScope,
    headers: &HeaderMap,
    scope: &McpCatalogScope,
    database: &GlobalDatabase,
    store: &McpCatalogStore,
    source: ImportSource,
) -> Result<McpImportReport, McpFailureCode> {
    use crate::agent_apps::auth_accounts::{AuthAccountScope, AuthAccountStore};
    use vw_agent_apps::{
        apps::discovery::read::resolve_cli_installation, mcp::import::NativeMcpSource,
    };
    use vw_api_types::{
        agent_apps::AppId,
        agent_apps_auth::{AuthErrorCode, AuthService},
    };
    let target = source.target;
    if target.installation_id.0.is_empty()
        || (target.app_id == AppId::Claude) != source.account_id.is_some()
    {
        return Err(McpFailureCode::InvalidDefinition);
    }
    if matches!(target.app_id, AppId::ClaudeDesktop | AppId::OpenClaw) {
        return Err(McpFailureCode::TargetUnavailable);
    }
    let host = agent_apps::authorize_mcp_native_read(
        policy.0.as_deref(),
        headers,
        tenant,
        &target.target_host_id,
    )
    .map_err(|_| McpFailureCode::PermissionDenied)?;
    if !resolve_cli_installation(target.app_id, &host, &target.installation_id)
        .map_err(|_| McpFailureCode::TargetUnavailable)?
    {
        return Err(McpFailureCode::NotFound);
    }
    let lease = if let Some(id) = source.account_id {
        let owner = agent_apps::authorize_auth_account_read(
            policy.0.as_deref(),
            headers,
            tenant,
            &target.target_host_id,
            AuthService::ClaudeOfficial,
            false,
        )
        .map_err(|_| McpFailureCode::PermissionDenied)?;
        let accounts = AuthAccountStore::new(database.clone(), &owner.credential_root)
            .map_err(|_| McpFailureCode::StorageUnavailable)?;
        Some(
            accounts
                .managed_official_lease(
                    AuthAccountScope {
                        tenant,
                        host: &owner.id,
                        principal: &owner.principal,
                        service: AuthService::ClaudeOfficial,
                    },
                    &id,
                )
                .map_err(|error| match error {
                    AuthErrorCode::NotFound => McpFailureCode::NotFound,
                    AuthErrorCode::Forbidden => McpFailureCode::PermissionDenied,
                    _ => McpFailureCode::CredentialUnavailable,
                })?,
        )
    } else {
        None
    };
    let native = lease
        .as_ref()
        .map_or(NativeMcpSource::HostHome(&host.home), NativeMcpSource::ClaudeAccount);
    store
        .import_from_apps(scope, &[McpImportSource { target, native }])
        .into_iter()
        .next()
        .ok_or(McpFailureCode::StorageUnavailable)
}
