//! Confirmed aggregation publication through the existing host operation registry.
use super::app_providers::operations::{State, aggregation::Owner};
#[cfg(target_os = "macos")]
use super::app_routing::RoutingControl;
use crate::global_database::GlobalDatabase;
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use sha2::{Digest, Sha256};
use vw_api_types::{
    agent_apps::{AppId, aggregation::*},
    id::{ProjectScope, TenantScope},
};
#[cfg(not(target_os = "macos"))]
#[derive(Clone)]
struct RoutingControl;
pub(super) fn router() -> Router {
    Router::new()
        .route("/agent-apps/{app}/aggregation/publish", post(submit))
        .layer(DefaultBodyLimit::max(24 * 1024))
}

#[derive(Debug)]
struct Failure(StatusCode, &'static str);
impl From<StatusCode> for Failure {
    fn from(status: StatusCode) -> Self {
        Self(
            status,
            match status {
                StatusCode::BAD_REQUEST => "invalid_request",
                StatusCode::UNAUTHORIZED => "unauthenticated",
                StatusCode::FORBIDDEN => "forbidden",
                StatusCode::NOT_FOUND => "not_found",
                StatusCode::CONFLICT => "revision_conflict",
                StatusCode::UNPROCESSABLE_ENTITY => "unsupported_target",
                StatusCode::BAD_GATEWAY => "target_failed",
                StatusCode::GATEWAY_TIMEOUT => "target_timeout",
                _ => "target_unavailable",
            },
        )
    }
}
impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        use vw_api_types::error::{ApiErrorBody, ApiErrorDetail};
        (
            self.0,
            [(header::CACHE_CONTROL, "no-store")],
            Json(ApiErrorBody {
                error: ApiErrorDetail {
                    code: self.1.into(),
                    message: self.1.into(),
                    details: None,
                },
            }),
        )
            .into_response()
    }
}
async fn submit(
    Extension(state): Extension<State>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    control: Option<Extension<RoutingControl>>,
    headers: HeaderMap,
    app: Result<Path<AppId>, axum::extract::rejection::PathRejection>,
    input: Result<Json<AggregationPublishRequest>, JsonRejection>,
) -> Response {
    let result = async {
        let Path(app) = app.map_err(|_| Failure::from(StatusCode::BAD_REQUEST))?;
        let Json(request) = input.map_err(|_| Failure::from(StatusCode::BAD_REQUEST))?;
        tokio::task::spawn_blocking(move || {
            prepare(state, tenant, db, control, headers, app, request)
        })
        .await
        .map_err(|_| Failure::from(StatusCode::SERVICE_UNAVAILABLE))?
    }
    .await;
    match result {
        Ok(progress) => {
            (StatusCode::ACCEPTED, [(header::CACHE_CONTROL, "no-store")], Json(progress))
                .into_response()
        }
        Err(error) => error.into_response(),
    }
}
#[cfg(not(target_os = "macos"))]
#[allow(clippy::too_many_arguments)]
fn prepare(
    _: State,
    _: TenantScope,
    _: Option<Extension<GlobalDatabase>>,
    _: Option<Extension<RoutingControl>>,
    _: HeaderMap,
    _: AppId,
    _: AggregationPublishRequest,
) -> Result<serde_json::Value, Failure> {
    Err(Failure(StatusCode::UNPROCESSABLE_ENTITY, "unsupported_target"))
}
#[cfg(target_os = "macos")]
#[allow(clippy::too_many_arguments)]
fn prepare(
    state: State,
    tenant: TenantScope,
    db: Option<Extension<GlobalDatabase>>,
    control: Option<Extension<RoutingControl>>,
    headers: HeaderMap,
    app: AppId,
    request: AggregationPublishRequest,
) -> Result<serde_json::Value, Failure> {
    use super::aggregation_read_snapshot as snapshot;
    use vw_agent_apps::providers::live::{
        commit::{ExecutionHostId, ProviderTransactions},
        write_guard::lock_app,
    };
    let project = ProjectScope::new(tenant.clone(), request.project_id.clone())
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    for id in [&request.target_host_id, &request.request_id] {
        vw_agent_apps::providers::validate_identity(id).map_err(|_| StatusCode::BAD_REQUEST)?;
    }
    let (home, root, principal) = super::agent_apps::authorize_aggregation_write(
        state.aggregation_policy(),
        &headers,
        &project,
        &request.target_host_id,
        app,
        true,
    )?;
    if !matches!(app, AppId::Claude | AppId::Codex | AppId::ClaudeDesktop) {
        return Err(Failure(StatusCode::UNPROCESSABLE_ENTITY, "unsupported_app"));
    }
    if !request.confirmed {
        return Err(Failure(StatusCode::UNPROCESSABLE_ENTITY, "confirmation_required"));
    }
    if request.expected_revision.is_empty() || request.expected_revision.len() > 256 {
        return Err(StatusCode::BAD_REQUEST.into());
    }
    let owner = Owner {
        project: project.clone(),
        host: request.target_host_id.clone(),
        app,
        principal: principal.clone(),
    };
    let fingerprint: [u8; 32] =
        Sha256::digest(serde_json::to_vec(&request).map_err(|_| StatusCode::BAD_REQUEST)?).into();
    if let Some(progress) =
        state.aggregation_replay(&owner, &request.request_id, &fingerprint).map_err(|s| {
            if s == StatusCode::CONFLICT { Failure(s, "request_conflict") } else { s.into() }
        })?
    {
        return Ok(progress);
    }
    let preview = snapshot::verify_token(
        &request.preview_token,
        &principal,
        &project,
        &request.target_host_id,
        app,
        &request.expected_revision,
    )?;
    if app == AppId::ClaudeDesktop {
        require_desktop(
            state.aggregation_policy(),
            &headers,
            &project,
            &request.target_host_id,
            &home,
            &principal,
        )?;
    }
    let root = root.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let guard = lock_app(app, &home).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if let Some(progress) =
        state.aggregation_replay(&owner, &request.request_id, &fingerprint).map_err(|s| {
            if s == StatusCode::CONFLICT { Failure(s, "request_conflict") } else { s.into() }
        })?
    {
        return Ok(progress);
    }
    let tx = ProviderTransactions::open_existing(
        &root,
        ExecutionHostId::new(&request.target_host_id).map_err(|_| StatusCode::BAD_REQUEST)?,
    )
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    .ok_or(StatusCode::CONFLICT)?;
    let expected = tx.state(app).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if expected.project_scope.as_ref() != Some(&project) {
        return Err(StatusCode::FORBIDDEN.into());
    }
    expected.require_project(&project).map_err(|_| StatusCode::CONFLICT)?;
    if let Some(pending) = tx.pending(app).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)? {
        if pending.tenant != tenant
            || pending.target.project_scope.as_ref().is_some_and(|p| p != &project)
        {
            return Err(StatusCode::FORBIDDEN.into());
        }
        return Err(Failure(StatusCode::CONFLICT, "recovery_pending"));
    }
    let db = match db {
        Some(Extension(db)) => db,
        None => GlobalDatabase::for_home(&crate::global::paths().home)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
    };
    let input = snapshot::load(
        &db,
        &project,
        &request.target_host_id,
        app,
        &home,
        &expected,
        preview.default_provider_id.as_deref(),
    )?;
    if snapshot::revision(&project, &request.target_host_id, app, &principal, &expected, &input)?
        != request.expected_revision
    {
        return Err(StatusCode::CONFLICT.into());
    }
    let projected = snapshot::project(app, &tenant, &expected, &input, Some(&preview))?;
    if let Some(issue) = projected.issues.iter().find(|issue| issue.blocking) {
        return Err(match issue.code {
            PreviewIssueCode::FailoverEnabled => Failure(StatusCode::CONFLICT, "failover_conflict"),
            PreviewIssueCode::InvalidMember => {
                Failure(StatusCode::UNPROCESSABLE_ENTITY, "invalid_member")
            }
            PreviewIssueCode::UnrestorableDirect => {
                Failure(StatusCode::UNPROCESSABLE_ENTITY, "unsupported_target")
            }
            _ => Failure(StatusCode::UNPROCESSABLE_ENTITY, "invalid_configuration"),
        });
    }
    let default = projected.default_provider_id.ok_or(StatusCode::CONFLICT)?;
    let default_record =
        crate::app_providers::AppProviderStore::preview_record(&db, &tenant, app, &default)
            .map_err(|_| StatusCode::NOT_FOUND)?;
    let again = snapshot::load(
        &db,
        &project,
        &request.target_host_id,
        app,
        &home,
        &expected,
        preview.default_provider_id.as_deref(),
    )?;
    if snapshot::revision(&project, &request.target_host_id, app, &principal, &expected, &again)?
        != request.expected_revision
    {
        return Err(StatusCode::CONFLICT.into());
    }
    let Extension(control) = control.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let provider_headers = super::aggregation_resync::provider_headers(headers.clone())?;
    let (route_root, route_principal) =
        control.authorize_mode(&provider_headers, &project, &request.target_host_id, app)?;
    if route_root != root || route_principal != principal {
        return Err(StatusCode::FORBIDDEN.into());
    }
    // The journal's input validator signs its original baseline; a preview may
    // include an additional default candidate, which is verified above.
    let baseline =
        snapshot::load(&db, &project, &request.target_host_id, app, &home, &expected, None)?;
    let revision = snapshot::revision(
        &project,
        &request.target_host_id,
        app,
        &principal,
        &expected,
        &baseline,
    )?;
    let context = crate::gateway::aggregation_resync::RecordPublicationContext {
        database: db,
        project: project.clone(),
        host: request.target_host_id.clone(),
        app,
        home: home.clone(),
        root: root.clone(),
        routing: Some(control.routing),
        expected,
        principal: principal.clone(),
        revision,
        failure: Default::default(),
    };
    let policy = state.aggregation_policy().map(std::path::Path::to_path_buf);
    let catalog_policy = match preview.catalog_policy {
        CatalogPolicy::Preserve => {
            vw_agent_apps::aggregation::codex_publish::CatalogPolicy::Preserve
        }
        CatalogPolicy::UseManaged => {
            vw_agent_apps::aggregation::codex_publish::CatalogPolicy::UseManaged
        }
    };
    drop(guard);
    state
        .aggregation_start(owner, request.request_id, fingerprint, move || {
            let result = (|| -> Result<AggregationState, Failure> {
                let again = super::agent_apps::authorize_aggregation_write(
                    policy.as_deref(),
                    &headers,
                    &project,
                    &request.target_host_id,
                    app,
                    true,
                )
                .map_err(Failure::from)?;
                if again != (home, Some(root.clone()), principal) {
                    return Err(StatusCode::CONFLICT.into());
                }
                if app == AppId::ClaudeDesktop {
                    require_desktop(
                        policy.as_deref(),
                        &headers,
                        &project,
                        &request.target_host_id,
                        &context.home,
                        &context.principal,
                    )?;
                }
                context.publish_aggregation(&default, catalog_policy, &default_record)?;
                context.published_state().map_err(Failure::from)
            })();
            match result {
                Ok(state) => AggregationOutcome::Published { state },
                Err(error) => {
                    let recovery = match ProviderTransactions::open_existing(
                        &root,
                        ExecutionHostId::new(&request.target_host_id).expect("validated host"),
                    )
                    .and_then(|tx| tx.map(|tx| tx.pending(app)).transpose())
                    {
                        Ok(Some(Some(_))) => RecoveryState::Pending,
                        Ok(Some(None)) => RecoveryState::None,
                        Ok(None) | Err(_) => RecoveryState::Failed,
                    };
                    let code = serde_json::from_value(serde_json::json!(error.1))
                        .unwrap_or(AggregationErrorCode::TargetFailed);
                    AggregationOutcome::Failed { status: error.0.as_u16(), code, recovery }
                }
            }
        })
        .map_err(
            |s| if s == StatusCode::CONFLICT { Failure(s, "request_conflict") } else { s.into() },
        )
}

pub(super) fn operation_error(status: StatusCode) -> Response {
    Failure::from(status).into_response()
}

#[cfg(target_os = "macos")]
fn require_desktop(
    policy: Option<&std::path::Path>,
    headers: &HeaderMap,
    project: &ProjectScope,
    host: &str,
    home: &std::path::Path,
    principal: &str,
) -> Result<(), StatusCode> {
    let internal = super::aggregation_resync::provider_headers(headers.clone())?;
    let (environment, subject) = super::agent_apps::provider_discovery_host(
        policy,
        &internal,
        project.tenant_scope(),
        host,
    )?;
    if environment.home != home || subject != principal {
        return Err(StatusCode::FORBIDDEN);
    }
    let observation =
        vw_agent_apps::apps::discovery::read::read(AppId::ClaudeDesktop, &environment)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
            .desktop
            .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    if vw_agent_apps::providers::claude_desktop::publication::DesktopInstallation::from(
        &observation,
    ) != vw_agent_apps::providers::claude_desktop::publication::DesktopInstallation::Installed
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    Ok(())
}
