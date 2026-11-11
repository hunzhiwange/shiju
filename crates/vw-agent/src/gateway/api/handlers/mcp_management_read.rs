//! Read the managed MCP catalog and inspect native configuration on the authorized host.

use std::path::PathBuf;

use axum::{
    Extension, Json, Router,
    extract::{Query, rejection::QueryRejection},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware,
    response::Response,
    routing::get,
};
use serde::Deserialize;
use vw_agent_apps::{
    apps::catalog,
    mcp::{native_capability, presets},
};
use vw_api_types::{
    TenantScope,
    agent_apps::{AppId, AppInstallationId},
    mcp_management::{
        McpCatalogResponse, McpCatalogScope, McpFailureCode, McpFailureResponse, McpNativePreview,
        McpPresetDraftView, McpPresetsResponse,
    },
};

use super::agent_apps;
use crate::{agent_apps::mcp::McpCatalogStore, global_database::GlobalDatabase};

#[derive(Clone)]
pub(super) struct PolicyFile(pub(super) Option<PathBuf>);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CatalogQuery {
    target_host_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NativeQuery {
    target_host_id: String,
    app_id: AppId,
    installation_id: AppInstallationId,
    account_id: Option<String>,
}

pub(super) fn router() -> Router {
    Router::new()
        .route(
            "/agent-apps/mcp",
            get(read_catalog).post(super::mcp_management_create::create_catalog_record),
        )
        .route("/agent-apps/mcp/presets", get(read_presets))
        .route("/agent-apps/mcp/native", get(read_native))
        .layer(Extension(PolicyFile(
            std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from),
        )))
        .layer(middleware::map_response(async |mut response: Response| {
            response
                .headers_mut()
                .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
            response
        }))
}

pub(super) type McpHttpError = (StatusCode, Json<McpFailureResponse>);

pub(super) fn failure(status: StatusCode, code: McpFailureCode) -> McpHttpError {
    (status, Json(McpFailureResponse { code }))
}

pub(super) fn access_error(status: StatusCode) -> McpHttpError {
    let code = match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => McpFailureCode::PermissionDenied,
        StatusCode::BAD_REQUEST => McpFailureCode::InvalidDefinition,
        StatusCode::NOT_FOUND => McpFailureCode::NotFound,
        StatusCode::CONFLICT => McpFailureCode::Conflict,
        _ => McpFailureCode::TargetUnavailable,
    };
    failure(status, code)
}

pub(super) fn unavailable<T>(_: T) -> McpHttpError {
    failure(StatusCode::SERVICE_UNAVAILABLE, McpFailureCode::StorageUnavailable)
}

pub(super) fn open_database(
    database: Option<Extension<GlobalDatabase>>,
) -> Result<GlobalDatabase, McpHttpError> {
    match database {
        Some(Extension(database)) => Ok(database),
        None => GlobalDatabase::for_home(&crate::global::paths().home).map_err(unavailable),
    }
}

async fn read_catalog(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    query: Result<Query<CatalogQuery>, QueryRejection>,
    database: Option<Extension<GlobalDatabase>>,
) -> Result<Json<McpCatalogResponse>, McpHttpError> {
    let Query(query) = query.map_err(|_| access_error(StatusCode::BAD_REQUEST))?;
    tokio::task::spawn_blocking(move || {
        agent_apps::authorize_mcp_catalog_read(
            policy.0.as_deref(),
            &headers,
            &tenant,
            &query.target_host_id,
        )
        .map_err(access_error)?;
        let scope = McpCatalogScope::Tenant { scope: tenant };
        let store = McpCatalogStore::new(open_database(database)?).map_err(unavailable)?;
        let (records, bindings) =
            store.list_for_host(&scope, &query.target_host_id).map_err(unavailable)?;
        let capabilities = catalog::catalog()
            .into_iter()
            .map(|entry| native_capability(entry.metadata.id))
            .collect();
        Ok(Json(McpCatalogResponse { records, bindings, capabilities }))
    })
    .await
    .map_err(unavailable)?
}

async fn read_presets(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    query: Result<Query<CatalogQuery>, QueryRejection>,
) -> Result<Json<McpPresetsResponse>, McpHttpError> {
    let Query(query) = query.map_err(|_| access_error(StatusCode::BAD_REQUEST))?;
    tokio::task::spawn_blocking(move || {
        agent_apps::authorize_mcp_catalog_read(
            policy.0.as_deref(),
            &headers,
            &tenant,
            &query.target_host_id,
        )
        .map_err(access_error)?;
        let presets = presets::list_presets()
            .iter()
            .map(|preset| {
                let draft = presets::create_preset_draft(preset.id, &[])
                    .map_err(unavailable)?
                    .ok_or_else(|| unavailable(()))?;
                Ok(McpPresetDraftView {
                    id: preset.id.into(),
                    name: preset.name.into(),
                    tags: preset.tags.iter().map(|tag| (*tag).into()).collect(),
                    homepage: preset.homepage.into(),
                    documentation: preset.documentation.into(),
                    connection: draft.connection,
                    enabled: draft.enabled,
                })
            })
            .collect::<Result<_, McpHttpError>>()?;
        Ok(Json(McpPresetsResponse { presets }))
    })
    .await
    .map_err(unavailable)?
}

async fn read_native(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    query: Result<Query<NativeQuery>, QueryRejection>,
    database: Option<Extension<GlobalDatabase>>,
) -> Result<Json<McpNativePreview>, McpHttpError> {
    let Query(query) = query.map_err(|_| access_error(StatusCode::BAD_REQUEST))?;
    tokio::task::spawn_blocking(move || {
        #[cfg(not(target_os = "macos"))]
        {
            agent_apps::authorize_provider_config_access(
                policy.0.as_deref(),
                &headers,
                &tenant,
                &query.target_host_id,
                false,
            )
            .map_err(access_error)?;
            let _ = (query.app_id, query.installation_id, query.account_id, database);
            Err(access_error(StatusCode::NOT_IMPLEMENTED))
        }
        #[cfg(target_os = "macos")]
        {
            let host = agent_apps::authorize_mcp_native_read(
                policy.0.as_deref(),
                &headers,
                &tenant,
                &query.target_host_id,
            )
            .map_err(access_error)?;
            native_preview(policy, tenant, headers, query, host, database).map(Json)
        }
    })
    .await
    .map_err(unavailable)?
}

#[cfg(target_os = "macos")]
fn native_preview(
    policy: PolicyFile,
    tenant: TenantScope,
    headers: HeaderMap,
    query: NativeQuery,
    host: vw_agent_apps::apps::discovery::read::LocalReadEnvironment,
    database: Option<Extension<GlobalDatabase>>,
) -> Result<McpNativePreview, McpHttpError> {
    use crate::agent_apps::auth_accounts::{AuthAccountScope, AuthAccountStore};
    use vw_agent_apps::{
        apps::discovery::read::resolve_cli_installation,
        mcp::import::{NativeMcpSource, read},
    };
    use vw_api_types::{
        agent_apps_auth::{AuthErrorCode, AuthService},
        mcp_management::{
            McpAppTarget, McpConnectionDefinition, McpNativePreviewEntry, McpNativeStatus,
            McpTransport,
        },
    };

    let target = McpAppTarget {
        app_id: query.app_id,
        installation_id: query.installation_id,
        target_host_id: query.target_host_id,
    };
    if target.installation_id.0.is_empty()
        || (target.app_id == AppId::Claude) != query.account_id.is_some()
    {
        return Err(access_error(StatusCode::BAD_REQUEST));
    }
    if matches!(target.app_id, AppId::ClaudeDesktop | AppId::OpenClaw) {
        return Ok(McpNativePreview {
            target,
            status: McpNativeStatus::Unsupported,
            entries: vec![],
        });
    }
    let installed = resolve_cli_installation(target.app_id, &host, &target.installation_id)
        .map_err(|_| access_error(StatusCode::SERVICE_UNAVAILABLE))?;
    if !installed {
        return Err(access_error(StatusCode::NOT_FOUND));
    }
    let database = open_database(database)?;
    let lease = if let Some(id) = query.account_id {
        let account_host = agent_apps::authorize_auth_account_read(
            policy.0.as_deref(),
            &headers,
            &tenant,
            &target.target_host_id,
            AuthService::ClaudeOfficial,
            false,
        )
        .map_err(access_error)?;
        let accounts = AuthAccountStore::new(database.clone(), &account_host.credential_root)
            .map_err(unavailable)?;
        let owner = AuthAccountScope {
            tenant: &tenant,
            host: &account_host.id,
            principal: &account_host.principal,
            service: AuthService::ClaudeOfficial,
        };
        Some(accounts.managed_official_lease(owner, &id).map_err(|error| match error {
            AuthErrorCode::NotFound => access_error(StatusCode::NOT_FOUND),
            AuthErrorCode::Forbidden => access_error(StatusCode::FORBIDDEN),
            AuthErrorCode::InvalidInput => access_error(StatusCode::BAD_REQUEST),
            AuthErrorCode::AccountInUse => access_error(StatusCode::CONFLICT),
            _ => failure(StatusCode::SERVICE_UNAVAILABLE, McpFailureCode::CredentialUnavailable),
        })?)
    } else {
        None
    };

    let source = lease
        .as_ref()
        .map_or(NativeMcpSource::HostHome(&host.home), NativeMcpSource::ClaudeAccount);
    let candidates = match read(&tenant, &target, source) {
        Ok(candidates) => candidates,
        Err(McpFailureCode::PermissionDenied) => return Err(access_error(StatusCode::FORBIDDEN)),
        Err(McpFailureCode::CredentialUnavailable) => {
            return Err(failure(
                StatusCode::SERVICE_UNAVAILABLE,
                McpFailureCode::CredentialUnavailable,
            ));
        }
        Err(_) => {
            return Ok(McpNativePreview {
                target,
                status: McpNativeStatus::Unreadable,
                entries: vec![],
            });
        }
    };
    let status =
        if candidates.is_empty() { McpNativeStatus::Missing } else { McpNativeStatus::Present };
    let records = McpCatalogStore::new(database)
        .and_then(|store| store.list(&McpCatalogScope::Tenant { scope: tenant }))
        .map_err(unavailable)?;
    let entries = records
        .into_iter()
        .map(|record| {
            let id = record.record.id;
            let (status, enabled, transport) = match candidates.get(&id.0) {
                Some(Ok(candidate)) => {
                    let transport = match candidate.connection {
                        McpConnectionDefinition::Stdio { .. } => McpTransport::Stdio,
                        McpConnectionDefinition::StreamableHttp { .. } => {
                            McpTransport::StreamableHttp
                        }
                        McpConnectionDefinition::Sse { .. } => McpTransport::Sse,
                    };
                    (McpNativeStatus::Present, Some(candidate.enabled), Some(transport))
                }
                Some(Err(_)) => (McpNativeStatus::Unreadable, None, None),
                None => (McpNativeStatus::Missing, None, None),
            };
            McpNativePreviewEntry { id, status, enabled, transport }
        })
        .collect();
    Ok(McpNativePreview { target, status, entries })
}
