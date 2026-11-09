//! Read-only gateway boundary for authorized usage snapshots.
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::{sync::Arc, time::Instant};

use axum::{
    Extension, Json, Router,
    extract::{Query, rejection::QueryRejection},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use vw_agent_apps::providers::validate_identity;
use vw_api_types::{
    agent_apps_auth::{AuthError, AuthErrorCode},
    apps_usage::{UsageSnapshotQuery, UsageSnapshotResponse},
    id::{ProjectScope, TenantScope},
};

#[cfg(target_os = "macos")]
use super::usage_refresh::UsageRefresh;

#[derive(Clone)]
struct PolicyFile(Option<PathBuf>);

pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new().route("/apps/usage/snapshot", get(read)).layer(Extension(PolicyFile(path)))
}

async fn read(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    query: Result<Query<UsageSnapshotQuery>, QueryRejection>,
    #[cfg(target_os = "macos")] service: Option<Extension<Arc<UsageRefresh>>>,
) -> Result<impl IntoResponse, ReadError> {
    let Query(query) = query.map_err(|_| AuthErrorCode::InvalidInput)?;
    let project = ProjectScope::new(tenant, query.project_id.clone())
        .map_err(|_| AuthErrorCode::InvalidInput)?;
    for id in
        [Some(query.target_host_id.as_str()), Some(&query.provider_id), query.account_id.as_deref()]
            .into_iter()
            .flatten()
    {
        validate_identity(id).map_err(|_| AuthErrorCode::InvalidInput)?;
    }

    let body: UsageSnapshotResponse = tokio::task::spawn_blocking(move || {
        let authorization = super::access::authorize_usage_read(
            policy.0.as_deref(),
            &headers,
            project.tenant_scope(),
            &query,
        )
        .map_err(ReadError::access)?;

        #[cfg(target_os = "macos")]
        {
            let Extension(service) = service.ok_or(AuthErrorCode::StorageUnavailable)?;
            if service
                .projects
                .get(&project)
                .map_err(|_| AuthErrorCode::StorageUnavailable)?
                .is_none()
            {
                return Err(AuthErrorCode::NotFound.into());
            }
            let response = if let Some(account) = &query.account_id {
                account_snapshot(
                    &service,
                    &project,
                    &query,
                    account,
                    authorization,
                    policy.0.is_some(),
                )?
            } else {
                service.provider_snapshot(
                    crate::agent_apps::usage_settings::UsageSettingsScope {
                        project: &project,
                        principal: &authorization.principal,
                        target_host: &authorization.host.id,
                        app: query.app_id,
                        provider: &query.provider_id,
                        account: None,
                    },
                    query.kind,
                    Instant::now(),
                )?
            };
            if query.kind.is_some_and(|kind| !response.capabilities.contains(&kind)) {
                return Err(AuthErrorCode::Unsupported.into());
            }
            Ok::<_, ReadError>(response)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = authorization;
            Err::<UsageSnapshotResponse, _>(ReadError(AuthErrorCode::Unsupported))
        }
    })
    .await
    .map_err(|_| AuthErrorCode::StorageUnavailable)??;
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(body)))
}

#[cfg(target_os = "macos")]
fn account_snapshot(
    service: &UsageRefresh,
    project: &ProjectScope,
    query: &UsageSnapshotQuery,
    account: &str,
    authorization: super::access::Authorization,
    explicit_policy: bool,
) -> Result<UsageSnapshotResponse, AuthErrorCode> {
    use vw_api_types::{agent_apps::AppId, apps_usage::UsageQueryKind};

    let (host, auth_service) = usage_account_host(query.app_id, &authorization, explicit_policy)?;
    if query.app_id == AppId::Codex {
        let scope = super::usage_codex::CodexScope {
            selection: crate::agent_apps::usage_settings::UsageSettingsScope {
                project,
                principal: &host.principal,
                target_host: &host.id,
                app: query.app_id,
                provider: &query.provider_id,
                account: Some(account),
            },
            credential_root: &host.credential_root,
            account,
        };
        let snapshot =
            scope.current_key(service)?.and_then(|key| service.snapshot(&key, Instant::now()));
        return Ok(UsageSnapshotResponse {
            capabilities: vec![UsageQueryKind::OfficialSubscription],
            snapshot,
        });
    }
    let scope = crate::agent_apps::auth_accounts::AuthAccountScope {
        tenant: project.tenant_scope(),
        principal: &host.principal,
        host: &host.id,
        service: auth_service,
    };
    super::super::agent_apps_auth_read::require_provider_account(
        service.database.clone(),
        &host,
        scope,
        query.app_id,
        &query.provider_id,
        account,
    )?;
    Ok(UsageSnapshotResponse {
        capabilities: vec![UsageQueryKind::OfficialSubscription],
        snapshot: None,
    })
}

#[cfg(target_os = "macos")]
pub(super) fn usage_account_host(
    app: vw_api_types::agent_apps::AppId,
    authorization: &super::access::Authorization,
    explicit_policy: bool,
) -> Result<(super::AuthAccountHost, vw_api_types::agent_apps_auth::AuthService), AuthErrorCode> {
    use vw_api_types::{agent_apps::AppId, agent_apps_auth::AuthService};

    let service = match app {
        AppId::Claude => AuthService::ClaudeOfficial,
        AppId::Codex => AuthService::CodexOauth,
        AppId::Agy => AuthService::AgyOauth,
        AppId::Kimi => AuthService::KimiOauth,
        _ => return Err(AuthErrorCode::Unsupported),
    };
    let owns_native_slot =
        explicit_policy && authorization.native_login_owner(service) == Some(true);
    if service == AuthService::AgyOauth && !owns_native_slot {
        return Err(AuthErrorCode::Forbidden);
    }
    let host = super::account_host(authorization, owns_native_slot)
        .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
    Ok((host, service))
}

pub(super) struct ReadError(AuthErrorCode);

impl ReadError {
    #[cfg(target_os = "macos")]
    pub(super) fn operation_response(self) -> Response {
        if self.0 == AuthErrorCode::NotFound {
            StatusCode::NOT_FOUND.into_response()
        } else {
            self.into_response()
        }
    }

    pub(super) fn access(status: StatusCode) -> Self {
        Self(match status {
            StatusCode::UNAUTHORIZED => AuthErrorCode::Unauthenticated,
            StatusCode::FORBIDDEN => AuthErrorCode::Forbidden,
            StatusCode::NOT_FOUND => AuthErrorCode::NotFound,
            StatusCode::NOT_IMPLEMENTED => AuthErrorCode::ExecutionChannelUnavailable,
            _ => AuthErrorCode::ExecutionHostUnavailable,
        })
    }
}

impl From<AuthErrorCode> for ReadError {
    fn from(code: AuthErrorCode) -> Self {
        Self(code)
    }
}

impl IntoResponse for ReadError {
    fn into_response(self) -> Response {
        use AuthErrorCode::*;
        let (status, message) = match self.0 {
            InvalidInput => (StatusCode::BAD_REQUEST, "Invalid usage selector"),
            Unauthenticated => (StatusCode::UNAUTHORIZED, "Authentication required"),
            Forbidden => (StatusCode::FORBIDDEN, "Usage action forbidden"),
            NotFound => (StatusCode::NOT_FOUND, "Usage resource not found"),
            Unsupported | ExecutionChannelUnavailable => {
                (StatusCode::NOT_IMPLEMENTED, "Usage query unavailable on this host")
            }
            StorageUnavailable | ExecutionHostUnavailable | KeychainUnavailable => {
                (StatusCode::SERVICE_UNAVAILABLE, "Usage state unavailable")
            }
            _ => (StatusCode::CONFLICT, "Usage resource state changed or is unavailable"),
        };
        (
            status,
            [(header::CACHE_CONTROL, "no-store")],
            Json(AuthError {
                code: self.0,
                message: message.into(),
                correlation_id: uuid::Uuid::new_v4().to_string(),
            }),
        )
            .into_response()
    }
}
