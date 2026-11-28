//! Project-scoped Claude Desktop mapping resources.
use agent_database::GlobalDatabase;
#[cfg(target_os = "macos")]
use super::publication_host::Host;
use axum::{
    Extension, Json,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use std::path::Path;
use vw_api_app_aggregation::*;
use vw_api_transport::id::TenantScope;
pub fn finish(
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
pub fn failure(status: StatusCode, code: &str) -> Response {
    use {
    vw_api_transport::error::ApiErrorBody,
    vw_api_transport::error::ApiErrorDetail,
};
    (
        status,
        [(header::CACHE_CONTROL, "no-store")],
        Json(ApiErrorBody {
            error: ApiErrorDetail { code: code.into(), message: code.into(), details: None },
        }),
    )
        .into_response()
}
#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use std::path::PathBuf;
    use vw_agent_apps::providers::{
        claude_desktop::publication as desktop,
        live::commit::{AppState, ExecutionHostId, ProviderTransactions},
    };
    use {
    vw_api_app_identity::AppId,
    vw_api_transport::id::ProjectScope,
};
    pub(super) const APP: AppId = AppId::ClaudeDesktop;
    pub(super) struct Context<H> {
        pub host: H,
        pub project: ProjectScope,
        pub home: PathBuf,
        pub root: PathBuf,
        pub principal: String,
        pub tx: ProviderTransactions,
        pub state: AppState,
        pub db: GlobalDatabase,
    }
    impl<H: Host> Context<H> {
        pub fn read(
            host: H,
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
            let (home, root, principal) = app_access_service::access::authorize_aggregation_read(
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
            if state.stack.enabled {
                return Err(StatusCode::CONFLICT);
            }
            if tx.pending(APP).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.is_some_and(
                |pending| {
                    pending.tenant != *project.tenant_scope()
                        || pending.target.project_scope.as_ref() != Some(&project)
                },
            ) {
                return Err(StatusCode::FORBIDDEN);
            }
            let db = db.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
            Ok(Self { host, project, home, root, principal, tx, state, db })
        }
        pub fn revision(&self, host: &str, id: Option<&str>) -> Result<String, StatusCode> {
            let snapshot = self.host.load(
                &self.db,
                &self.project,
                host,
                APP,
                &self.home,
                &self.state,
                id,
            )?;
            self.host.revision(
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
    pub(super) fn installation<H: Host>(
        policy: Option<&Path>,
        headers: &HeaderMap,
        context: &Context<H>,
        host: &str,
    ) -> Result<desktop::DesktopInstallation, StatusCode> {
        // The strict aggregation authorization has already checked the one Bearer
        // value. Discovery uses the existing host resolver's internal header form.
        let discovery = context.host.provider_headers(headers.clone())?;
        let (environment, principal) = context.host.discovery(
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
    pub fn read_form<H: Host>(
    host: H,
        policy: Option<&Path>,
        tenant: TenantScope,
        db: Option<Extension<GlobalDatabase>>,
        headers: HeaderMap,
        id: String,
        query: AggregationQuery,
    ) -> Result<Response, StatusCode> {
        vw_agent_apps::providers::validate_identity(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
        let context = Context::read(host.clone(), policy, tenant, db, &headers, &query)?;
        if context.tx.pending(APP).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.is_some() {
            return Ok(failure(StatusCode::CONFLICT, "recovery_pending"));
        }
        let revision = context.revision(&query.target_host_id, Some(&id))?;
        let row = app_provider_services::AppProviderStore::preview_record(
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
pub use native::read_form;
#[cfg(not(target_os = "macos"))]
pub fn read_form<H: super::publication_host::SnapshotHost>(
    _host: H,
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
pub use write::save as save_mapping;
