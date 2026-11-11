//! Project-scoped Claude Desktop status and mapping resources.
use crate::global_database::GlobalDatabase;
use axum::{
    Extension, Json, Router,
    extract::{Query, rejection::QueryRejection},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use std::path::{Path, PathBuf};
use vw_api_types::{agent_apps::aggregation::*, id::TenantScope};
#[derive(Clone)]
struct Policy(Option<PathBuf>);
pub(super) fn router() -> Router {
    let router =
        router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from));
    match GlobalDatabase::for_home(&crate::global::paths().home) {
        Ok(database) => router.layer(Extension(database)),
        Err(_) => router,
    }
}
pub(super) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    Router::new()
        .route("/agent-apps/claude-desktop/profile/status", get(status))
        .layer(Extension(Policy(policy)))
}
async fn status(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    query: Result<Query<AggregationQuery>, QueryRejection>,
) -> Response {
    let Ok(Query(query)) = query else {
        return failure(StatusCode::BAD_REQUEST, "invalid_request");
    };
    finish(
        tokio::task::spawn_blocking(move || {
            read_status(policy.0.as_deref(), tenant, db, headers, query)
        })
        .await,
    )
}
pub(super) fn finish(
    result: Result<Result<Response, StatusCode>, tokio::task::JoinError>,
) -> Response {
    let mut response = match result {
        Ok(Ok(response)) => response,
        Ok(Err(status)) => failure(
            status,
            match status {
                StatusCode::BAD_REQUEST => "invalid_request",
                StatusCode::UNAUTHORIZED => "unauthenticated",
                StatusCode::FORBIDDEN => "forbidden",
                StatusCode::NOT_FOUND => "not_found",
                StatusCode::CONFLICT => "revision_conflict",
                StatusCode::UNPROCESSABLE_ENTITY => "invalid_configuration",
                StatusCode::BAD_GATEWAY => "target_failed",
                StatusCode::GATEWAY_TIMEOUT => "target_timeout",
                _ => "target_unavailable",
            },
        ),
        Err(_) => failure(StatusCode::SERVICE_UNAVAILABLE, "target_unavailable"),
    };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, axum::http::HeaderValue::from_static("no-store"));
    response
}
pub(super) fn failure(status: StatusCode, code: &str) -> Response {
    use vw_api_types::error::{ApiErrorBody, ApiErrorDetail};
    (
        status,
        [(header::CACHE_CONTROL, "no-store")],
        Json(ApiErrorBody {
            error: ApiErrorDetail { code: code.into(), message: code.into(), details: None },
        }),
    )
        .into_response()
}
#[cfg(not(target_os = "macos"))]
fn read_status(
    _: Option<&Path>,
    _: TenantScope,
    _: Option<Extension<GlobalDatabase>>,
    _: HeaderMap,
    _: AggregationQuery,
) -> Result<Response, StatusCode> {
    Ok(failure(StatusCode::UNPROCESSABLE_ENTITY, "unsupported_target"))
}
#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use vw_agent_apps::providers::{
        claude_desktop::publication as desktop,
        live::commit::{AppState, ExecutionHostId, ProviderTransactions},
    };
    use vw_api_types::{agent_apps::AppId, id::ProjectScope};
    pub(super) const APP: AppId = AppId::ClaudeDesktop;
    pub(super) struct Context {
        pub project: ProjectScope,
        pub home: PathBuf,
        pub root: PathBuf,
        pub principal: String,
        pub tx: ProviderTransactions,
        pub state: AppState,
        pub db: GlobalDatabase,
    }
    impl Context {
        pub fn read(
            policy: Option<&Path>,
            tenant: TenantScope,
            db: Option<Extension<GlobalDatabase>>,
            headers: &HeaderMap,
            query: &AggregationQuery,
        ) -> Result<Self, StatusCode> {
            let project = ProjectScope::new(tenant, query.project_id.clone())
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            vw_agent_apps::providers::validate_identity(&query.target_host_id)
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            let (home, root, principal) = super::super::agent_apps::authorize_aggregation_read(
                policy,
                headers,
                &project,
                &query.target_host_id,
                APP,
            )?;
            let root = root.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
            let tx = ProviderTransactions::open_existing(
                &root,
                ExecutionHostId::new(&query.target_host_id).map_err(|_| StatusCode::BAD_REQUEST)?,
            )
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
            .ok_or(StatusCode::CONFLICT)?;
            let state = tx.state(APP).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            if state.project_scope.as_ref() != Some(&project) {
                return Err(if state.project_scope.is_some() {
                    StatusCode::FORBIDDEN
                } else {
                    StatusCode::CONFLICT
                });
            }
            state.require_project(&project).map_err(|_| StatusCode::FORBIDDEN)?;
            if tx.pending(APP).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.is_some_and(
                |pending| {
                    pending.tenant != *project.tenant_scope()
                        || pending.target.project_scope.as_ref() != Some(&project)
                },
            ) {
                return Err(StatusCode::FORBIDDEN);
            }
            let db = db.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
            Ok(Self { project, home, root, principal, tx, state, db })
        }
        pub fn revision(&self, host: &str, id: Option<&str>) -> Result<String, StatusCode> {
            let snapshot = super::super::aggregation_read_snapshot::load(
                &self.db,
                &self.project,
                host,
                APP,
                &self.home,
                &self.state,
                id,
            )?;
            super::super::aggregation_read_snapshot::revision(
                &self.project,
                host,
                APP,
                &self.principal,
                &self.state,
                &snapshot,
            )
        }
        pub fn unchanged(
            &self,
            host: &str,
            id: Option<&str>,
            revision: &str,
        ) -> Result<(), StatusCode> {
            if self.tx.state(APP).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)? != self.state
                || self.revision(host, id)? != revision
            {
                return Err(StatusCode::CONFLICT);
            }
            Ok(())
        }
    }
    pub(super) fn installation(
        policy: Option<&Path>,
        headers: &HeaderMap,
        context: &Context,
        host: &str,
    ) -> Result<desktop::DesktopInstallation, StatusCode> {
        // The strict aggregation authorization has already checked the one Bearer
        // value. Discovery uses the existing host resolver's internal header form.
        let discovery = super::super::aggregation_resync::provider_headers(headers.clone())?;
        let (environment, principal) = super::super::agent_apps::provider_discovery_host(
            policy,
            &discovery,
            context.project.tenant_scope(),
            host,
        )?;
        if environment.home != context.home || principal != context.principal {
            return Err(StatusCode::FORBIDDEN);
        }
        let observation = vw_agent_apps::apps::discovery::read::read(APP, &environment)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
            .desktop
            .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
        Ok(desktop::DesktopInstallation::from(&observation))
    }
    pub(super) fn read_status(
        policy: Option<&Path>,
        tenant: TenantScope,
        db: Option<Extension<GlobalDatabase>>,
        headers: HeaderMap,
        query: AggregationQuery,
    ) -> Result<Response, StatusCode> {
        let context = Context::read(policy, tenant, db, &headers, &query)?;
        let revision = context.revision(&query.target_host_id, None)?;
        let observed = desktop::get_status(
            &context.home,
            installation(policy, &headers, &context, &query.target_host_id)?,
            Some(&context.tx),
            context.project.tenant_scope(),
        )
        .map_err(|error| match error {
            desktop::DesktopPublicationError::Forbidden => StatusCode::FORBIDDEN,
            desktop::DesktopPublicationError::InvalidConfiguration => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            _ => StatusCode::SERVICE_UNAVAILABLE,
        })?;
        context.unchanged(&query.target_host_id, None, &revision)?;
        let mut view = DesktopProfileStatus {
            revision: String::new(),
            installation: match observed.installation {
                desktop::DesktopInstallation::Installed => DesktopInstallation::Installed,
                desktop::DesktopInstallation::NotInstalled => DesktopInstallation::NotInstalled,
                desktop::DesktopInstallation::Unsupported => DesktopInstallation::Unsupported,
            },
            configured: observed.configured,
            mode: match observed.mode {
                desktop::DesktopMode::Official => DesktopProfileMode::Official,
                desktop::DesktopMode::DirectGateway => DesktopProfileMode::DirectGateway,
                desktop::DesktopMode::Routing => DesktopProfileMode::Routing,
                desktop::DesktopMode::Aggregation => DesktopProfileMode::Aggregation,
            },
            applied_provider_id: observed.applied_provider_id,
            notices: vec![],
            recovery: if observed.recovery_pending {
                RecoveryState::Pending
            } else {
                RecoveryState::None
            },
        };
        view.revision = super::super::aggregation_read_snapshot::sign(
            &serde_json::to_vec(&(revision, &view)).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        )?;
        Ok(Json(view).into_response())
    }
    pub(in super::super) fn read_form(
        policy: Option<&Path>,
        tenant: TenantScope,
        db: Option<Extension<GlobalDatabase>>,
        headers: HeaderMap,
        id: String,
        query: AggregationQuery,
    ) -> Result<Response, StatusCode> {
        vw_agent_apps::providers::validate_identity(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
        let context = Context::read(policy, tenant, db, &headers, &query)?;
        if context.tx.pending(APP).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.is_some() {
            return Ok(failure(StatusCode::CONFLICT, "recovery_pending"));
        }
        let revision = context.revision(&query.target_host_id, Some(&id))?;
        let row = crate::app_providers::AppProviderStore::preview_record(
            &context.db,
            context.project.tenant_scope(),
            APP,
            &id,
        )
        .map_err(|error| {
            if error == "application provider not found" {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            }
        })?;
        let form = vw_agent_apps::providers::desktop_mapping::form_view(&row)
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        context.unchanged(&query.target_host_id, Some(&id), &revision)?;
        Ok(Json(DesktopMappingView { revision, form }).into_response())
    }
}
#[cfg(target_os = "macos")]
pub(super) use native::read_form;
#[cfg(target_os = "macos")]
use native::read_status;
#[cfg(not(target_os = "macos"))]
pub(super) fn read_form(
    _: Option<&Path>,
    _: TenantScope,
    _: Option<Extension<GlobalDatabase>>,
    _: HeaderMap,
    _: String,
    _: AggregationQuery,
) -> Result<Response, StatusCode> {
    Ok(failure(StatusCode::UNPROCESSABLE_ENTITY, "unsupported_target"))
}

#[cfg(target_os = "macos")]
#[path = "aggregation_desktop_write.rs"]
mod write;
#[cfg(target_os = "macos")]
pub(super) use write::save as save_mapping;
