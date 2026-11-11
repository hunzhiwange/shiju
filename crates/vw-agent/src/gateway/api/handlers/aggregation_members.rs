//! Scoped membership and default-provider writes through the native owner.
#[cfg(target_os = "macos")]
use super::app_routing::RoutingControl;
use crate::global_database::GlobalDatabase;
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::put,
};
use std::path::PathBuf;
use vw_api_types::{
    agent_apps::{AppId, aggregation::*},
    id::TenantScope,
};
#[cfg(not(target_os = "macos"))]
#[derive(Clone)]
struct RoutingControl;
#[derive(Clone)]
struct Policy(Option<PathBuf>);
pub(super) fn router() -> Router {
    let router =
        router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from));
    match GlobalDatabase::for_home(&crate::global::paths().home) {
        Ok(db) => router.layer(Extension(db)),
        Err(_) => router,
    }
}
pub(super) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    Router::new()
        .route("/agent-apps/{app}/aggregation/members/{providerId}", put(member))
        .route("/agent-apps/{app}/aggregation/default", put(default_provider))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(Extension(Policy(policy)))
}
#[derive(Clone)]
enum Intent {
    Member { id: String, enabled: bool },
    Default { id: String },
}
impl Intent {
    fn id(&self) -> &str {
        match self {
            Self::Member { id, .. } | Self::Default { id } => id,
        }
    }
}
async fn member(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    control: Option<Extension<RoutingControl>>,
    headers: HeaderMap,
    path: Result<Path<(AppId, String)>, axum::extract::rejection::PathRejection>,
    request: Result<Json<AggregationMemberRequest>, JsonRejection>,
) -> Response {
    let Ok(Path((app, id))) = path else {
        return failure(StatusCode::BAD_REQUEST, "invalid_request");
    };
    let Ok(Json(request)) = request else {
        return failure(StatusCode::BAD_REQUEST, "invalid_request");
    };
    respond(
        tokio::task::spawn_blocking(move || {
            execute(
                policy,
                tenant,
                db,
                control,
                headers,
                app,
                AggregationQuery {
                    target_host_id: request.target_host_id,
                    project_id: request.project_id,
                },
                request.expected_revision,
                Intent::Member { id, enabled: request.enabled },
            )
        })
        .await,
    )
}
async fn default_provider(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    control: Option<Extension<RoutingControl>>,
    headers: HeaderMap,
    app: Result<Path<AppId>, axum::extract::rejection::PathRejection>,
    request: Result<Json<AggregationDefaultRequest>, JsonRejection>,
) -> Response {
    let Ok(Path(app)) = app else { return failure(StatusCode::BAD_REQUEST, "invalid_request") };
    let Ok(Json(request)) = request else {
        return failure(StatusCode::BAD_REQUEST, "invalid_request");
    };
    respond(
        tokio::task::spawn_blocking(move || {
            execute(
                policy,
                tenant,
                db,
                control,
                headers,
                app,
                AggregationQuery {
                    target_host_id: request.target_host_id,
                    project_id: request.project_id,
                },
                request.expected_revision,
                Intent::Default { id: request.provider_id },
            )
        })
        .await,
    )
}
#[derive(Debug)]
struct Failure(StatusCode, &'static str);
fn failure(status: StatusCode, code: &str) -> Response {
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
                StatusCode::UNPROCESSABLE_ENTITY => "invalid_configuration",
                StatusCode::BAD_GATEWAY => "target_failed",
                StatusCode::GATEWAY_TIMEOUT => "target_timeout",
                _ => "target_unavailable",
            },
        )
    }
}
fn respond(result: Result<Result<AggregationState, Failure>, tokio::task::JoinError>) -> Response {
    let mut response = match result {
        Ok(Ok(state)) => Json(state).into_response(),
        Ok(Err(Failure(status, code))) => failure(status, code),
        Err(_) => failure(StatusCode::SERVICE_UNAVAILABLE, "target_unavailable"),
    };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, axum::http::HeaderValue::from_static("no-store"));
    response
}
#[cfg(not(target_os = "macos"))]
#[allow(clippy::too_many_arguments)]
fn execute(
    _: Policy,
    _: TenantScope,
    _: Option<Extension<GlobalDatabase>>,
    _: Option<Extension<RoutingControl>>,
    _: HeaderMap,
    _: AppId,
    _: AggregationQuery,
    _: String,
    _: Intent,
) -> Result<AggregationState, Failure> {
    Err(Failure(StatusCode::UNPROCESSABLE_ENTITY, "unsupported_target"))
}
#[cfg(target_os = "macos")]
#[allow(clippy::too_many_arguments)]
fn execute(
    policy: Policy,
    tenant: TenantScope,
    db: Option<Extension<GlobalDatabase>>,
    control: Option<Extension<RoutingControl>>,
    headers: HeaderMap,
    app: AppId,
    query: AggregationQuery,
    expected_revision: String,
    intent: Intent,
) -> Result<AggregationState, Failure> {
    use super::aggregation_read_snapshot as snapshot;
    use vw_agent_apps::{
        aggregation::{
            member_validation::{ProviderRole, validate_member_removal, validate_provider},
            membership::{self, MembershipChange, SavedMember},
        },
        providers::{
            live::{
                commit::{ExecutionHostId, ProviderTransactions},
                write_guard::lock_app,
            },
            mode_state,
        },
    };
    use vw_api_types::{agent_apps_auth::AuthService, id::ProjectScope};
    let invalid = || Failure(StatusCode::BAD_REQUEST, "invalid_request");
    let unavailable = || Failure(StatusCode::SERVICE_UNAVAILABLE, "target_unavailable");
    let project = ProjectScope::new(tenant.clone(), query.project_id).map_err(|_| invalid())?;
    vw_agent_apps::providers::validate_identity(&query.target_host_id).map_err(|_| invalid())?;
    let auth = |attached| {
        super::agent_apps::authorize_aggregation_write(
            policy.0.as_deref(),
            &headers,
            &project,
            &query.target_host_id,
            app,
            attached,
        )
        .map_err(Failure::from)
    };
    let (home, root, principal) = auth(false)?;
    let provider_headers = super::aggregation_resync::provider_headers(headers.clone())?;
    if !matches!(app, AppId::Claude | AppId::Codex | AppId::ClaudeDesktop) {
        return Err(Failure(StatusCode::UNPROCESSABLE_ENTITY, "unsupported_app"));
    }
    vw_agent_apps::providers::validate_identity(intent.id()).map_err(|_| invalid())?;
    if expected_revision.trim().is_empty() || expected_revision.len() > 256 {
        return Err(invalid());
    }
    let root = root.ok_or_else(unavailable)?;
    let mut guard = lock_app(app, &home).map_err(|_| unavailable())?;
    let tx = ProviderTransactions::open_existing(
        &root,
        ExecutionHostId::new(&query.target_host_id).map_err(|_| invalid())?,
    )
    .map_err(|_| unavailable())?
    .ok_or(Failure(StatusCode::CONFLICT, "revision_conflict"))?;
    let state = tx.state(app).map_err(|_| unavailable())?;
    if state.project_scope.as_ref() != Some(&project) {
        return Err(Failure(
            if state.project_scope.is_some() {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::CONFLICT
            },
            if state.project_scope.is_some() { "forbidden" } else { "revision_conflict" },
        ));
    }
    let observed = mode_state::observe(&state, &tenant)
        .map_err(|_| Failure(StatusCode::FORBIDDEN, "forbidden"))?;
    state
        .require_project(&project)
        .map_err(|_| Failure(StatusCode::CONFLICT, "revision_conflict"))?;
    if observed.attached {
        let again = auth(true)?;
        if again != (home.clone(), Some(root.clone()), principal.clone()) {
            return Err(Failure(StatusCode::CONFLICT, "revision_conflict"));
        }
    }
    if let Some(pending) = tx.pending(app).map_err(|_| unavailable())? {
        if pending.tenant != tenant
            || pending.target.project_scope.as_ref().is_some_and(|scope| scope != &project)
        {
            return Err(Failure(StatusCode::FORBIDDEN, "forbidden"));
        }
        return Err(Failure(StatusCode::CONFLICT, "recovery_pending"));
    }
    let db = db.ok_or_else(unavailable)?.0;
    let capture = |candidate| {
        snapshot::load(&db, &project, &query.target_host_id, app, &home, &state, candidate)
    };
    let sign = |input: &snapshot::Snapshot| {
        snapshot::revision(&project, &query.target_host_id, app, &principal, &state, input)
    };
    let baseline = capture(None)?;
    if sign(&baseline)? != expected_revision {
        return Err(Failure(StatusCode::CONFLICT, "revision_conflict"));
    }
    let row = if matches!(intent, Intent::Member { enabled: false, .. }) {
        None
    } else {
        Some(
            crate::app_providers::AppProviderStore::preview_record(&db, &tenant, app, intent.id())
                .map_err(|error| {
                    if error == "application provider not found" {
                        Failure(StatusCode::NOT_FOUND, "not_found")
                    } else {
                        unavailable()
                    }
                })?,
        )
    };
    let official = row.as_ref().filter(|r| r["category"] == "official").map(|_| match app {
        AppId::Codex => AuthService::CodexOauth,
        _ => AuthService::ClaudeOfficial,
    });
    let role = if matches!(intent, Intent::Default { .. }) {
        ProviderRole::Default
    } else {
        ProviderRole::Member
    };
    if matches!(intent, Intent::Member { enabled: false, .. }) {
        validate_member_removal(app, &state, intent.id()).map_err(validation_error)?;
    } else {
        validate_provider(app, official, role).map_err(validation_error)?;
    }
    // Account transitions require the native account owner; never infer login
    // or recovery capability from an official category alone.
    if official.is_some() {
        return Err(Failure(StatusCode::UNPROCESSABLE_ENTITY, "unsupported_target"));
    }
    let inputs = capture(row.as_ref().map(|_| intent.id()))?;
    let input_revision = sign(&inputs)?;
    let row_unchanged = || {
        row.as_ref().is_none_or(|expected| {
            crate::app_providers::AppProviderStore::preview_record(&db, &tenant, app, intent.id())
                .ok()
                .as_ref()
                == Some(expected)
        })
    };
    if !row_unchanged() {
        return Err(Failure(StatusCode::CONFLICT, "revision_conflict"));
    }

    let saved = || {
        row.as_ref().map(|r| SavedMember {
            name: r["name"].as_str().unwrap_or(intent.id()),
            icon: r["icon"].as_str(),
            official_service: official,
        })
    };
    let change = || match &intent {
        Intent::Member { id, enabled: true } => {
            MembershipChange::Add { provider_id: id, saved: saved() }
        }
        Intent::Member { id, enabled: false } => MembershipChange::Remove { provider_id: id },
        Intent::Default { id } => MembershipChange::SetDefault { provider_id: id, saved: saved() },
    };
    let target = membership::plan_member(app, &state, change()).map_err(membership_error)?;
    let mut candidate = state.clone();
    if let Some(stack) = target.stack {
        candidate.stack = stack;
    }
    if let Some(mode) = target.mode {
        candidate.mode = Some(mode);
    }
    // Validate the response projection before changing durable intent. Invalid
    // saved client declarations must not turn a committed write into an error.
    let projected = snapshot::project(app, &tenant, &candidate, &inputs, None)?;
    if observed.attached
        && candidate != state
        && (observed.stack.active || candidate.mode != state.mode)
    {
        if let Some(issue) =
            projected.issues.iter().find(|issue| issue.blocking && observed.stack.active)
        {
            let (status, code) = match issue.code {
                PreviewIssueCode::FailoverEnabled => (StatusCode::CONFLICT, "failover_conflict"),
                PreviewIssueCode::InvalidMember => {
                    (StatusCode::UNPROCESSABLE_ENTITY, "invalid_member")
                }
                PreviewIssueCode::UnrestorableDirect => {
                    (StatusCode::UNPROCESSABLE_ENTITY, "unsupported_target")
                }
                _ => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_configuration"),
            };
            return Err(Failure(status, code));
        }
        let Extension(control) = control.ok_or_else(unavailable)?;
        let (control_root, control_principal) =
            control.authorize_mode(&provider_headers, &project, &query.target_host_id, app)?;
        if control_root != root || control_principal != principal {
            return Err(Failure(StatusCode::FORBIDDEN, "forbidden"));
        }
        // Native publication obtains the same app/physical guards itself. It
        // rechecks this preimage and scoped revision before its durable intent.
        if !row_unchanged() {
            return Err(Failure(StatusCode::CONFLICT, "revision_conflict"));
        }
        drop(guard);
        let changed = crate::gateway::aggregation_resync::RecordPublicationContext {
            database: db.clone(),
            project: project.clone(),
            host: query.target_host_id.clone(),
            app,
            home: home.clone(),
            root: root.clone(),
            routing: Some(control.routing),
            expected: state.clone(),
            principal: principal.clone(),
            revision: expected_revision.clone(),
            failure: Default::default(),
        }
        .publish_candidate(&candidate)?;
        if !changed {
            return Err(Failure(StatusCode::CONFLICT, "revision_conflict"));
        }
        guard = lock_app(app, &home).map_err(|_| unavailable())?;
    } else {
        membership::set_member_project(&tx, &mut guard, &project, &state, change(), || {
            row_unchanged()
                && auth(observed.attached).ok()
                    == Some((home.clone(), Some(root.clone()), principal.clone()))
                && capture(None).and_then(|s| sign(&s)).ok().as_deref() == Some(&expected_revision)
                && capture(row.as_ref().map(|_| intent.id())).and_then(|s| sign(&s)).ok().as_deref()
                    == Some(&input_revision)
        })
        .map_err(membership_error)?;
    }
    // Retain the reacquired guard until the returned scoped snapshot is assembled.
    let _guard = guard;
    let settled = tx.state(app).map_err(|_| unavailable())?;
    let settled_snapshot =
        snapshot::load(&db, &project, &query.target_host_id, app, &home, &settled, None)?;
    let revision = snapshot::revision(
        &project,
        &query.target_host_id,
        app,
        &principal,
        &settled,
        &settled_snapshot,
    )?;
    let projected = snapshot::project(app, &tenant, &settled, &settled_snapshot, None)?;
    let observed = mode_state::observe(&settled, &tenant).map_err(|_| unavailable())?;
    let mut members = projected.members;
    if !observed.attached {
        for member in &mut members {
            member.models.clear();
        }
    }
    Ok(AggregationState {
        revision,
        mode: if observed.stack.active {
            AggregationMode::Aggregation
        } else if observed.mode == vw_api_types::app_providers::mode_state::ProviderMode::Proxy {
            AggregationMode::Routing
        } else {
            AggregationMode::Direct
        },
        attached: observed.attached,
        default_provider_id: observed.proxy_route,
        direct_provider_id: observed.direct_provider,
        members,
        notices: projected.notices,
        recovery: RecoveryState::None,
    })
}
#[cfg(target_os = "macos")]
fn validation_error(
    error: vw_agent_apps::aggregation::member_validation::MemberValidationError,
) -> Failure {
    use vw_agent_apps::aggregation::member_validation::MemberValidationError::*;
    match error {
        UnsupportedApp => Failure(StatusCode::UNPROCESSABLE_ENTITY, "unsupported_app"),
        InvalidMember => Failure(StatusCode::UNPROCESSABLE_ENTITY, "invalid_member"),
        DefaultMemberRequired => Failure(StatusCode::CONFLICT, "default_member_required"),
    }
}
#[cfg(target_os = "macos")]
fn membership_error(error: vw_agent_apps::aggregation::membership::MembershipError) -> Failure {
    use vw_agent_apps::aggregation::membership::MembershipError::*;
    match error {
        Validation(error) => validation_error(error),
        Forbidden => Failure(StatusCode::FORBIDDEN, "forbidden"),
        NotFound => Failure(StatusCode::NOT_FOUND, "not_found"),
        UnsupportedTarget => Failure(StatusCode::UNPROCESSABLE_ENTITY, "unsupported_target"),
        RevisionConflict => Failure(StatusCode::CONFLICT, "revision_conflict"),
        RecoveryPending => Failure(StatusCode::CONFLICT, "recovery_pending"),
        TargetFailed => Failure(StatusCode::BAD_GATEWAY, "target_failed"),
    }
}
