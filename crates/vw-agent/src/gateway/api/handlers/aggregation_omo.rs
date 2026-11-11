//! OMO variants share the authorized OpenCode slot and native transaction owner.
use crate::global_database::GlobalDatabase;
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Query},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use std::path::PathBuf;
use vw_api_types::{agent_apps::aggregation::*, id::TenantScope};
#[derive(Clone)]
struct Policy(Option<PathBuf>);
#[derive(Clone, Copy)]
enum Variant {
    Standard,
    Slim,
}
enum Action {
    Read,
    Apply(OmoApplyRequest),
    Disable(OmoDisableRequest),
}
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
        .route("/agent-apps/opencode/omo", get(standard_read))
        .route("/agent-apps/opencode/omo/apply", post(standard_apply))
        .route("/agent-apps/opencode/omo/disable", post(standard_disable))
        .route("/agent-apps/opencode/omo-slim", get(slim_read))
        .route("/agent-apps/opencode/omo-slim/apply", post(slim_apply))
        .route("/agent-apps/opencode/omo-slim/disable", post(slim_disable))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(Extension(Policy(policy)))
}
macro_rules! read_endpoint {
    ($name:ident,$variant:ident) => {
        async fn $name(
            Extension(policy): Extension<Policy>,
            Extension(tenant): Extension<TenantScope>,
            db: Option<Extension<GlobalDatabase>>,
            headers: HeaderMap,
            query: Result<Query<AggregationQuery>, axum::extract::rejection::QueryRejection>,
        ) -> Response {
            let Ok(Query(query)) = query else {
                return failure(StatusCode::BAD_REQUEST, "invalid_input");
            };
            run(policy, tenant, db, headers, Variant::$variant, query, Action::Read).await
        }
    };
}
macro_rules! write_endpoint {
    ($name:ident,$variant:ident,$action:ident,$request:ty) => {
        async fn $name(
            Extension(policy): Extension<Policy>,
            Extension(tenant): Extension<TenantScope>,
            db: Option<Extension<GlobalDatabase>>,
            headers: HeaderMap,
            body: Result<Json<$request>, axum::extract::rejection::JsonRejection>,
        ) -> Response {
            let Ok(Json(body)) = body else {
                return failure(StatusCode::BAD_REQUEST, "invalid_input");
            };
            let query = AggregationQuery {
                target_host_id: body.target_host_id.clone(),
                project_id: body.project_id.clone(),
            };
            run(policy, tenant, db, headers, Variant::$variant, query, Action::$action(body)).await
        }
    };
}
read_endpoint!(standard_read, Standard);
read_endpoint!(slim_read, Slim);
write_endpoint!(standard_apply, Standard, Apply, OmoApplyRequest);
write_endpoint!(slim_apply, Slim, Apply, OmoApplyRequest);
write_endpoint!(standard_disable, Standard, Disable, OmoDisableRequest);
write_endpoint!(slim_disable, Slim, Disable, OmoDisableRequest);
#[allow(clippy::too_many_arguments)]
async fn run(
    policy: Policy,
    tenant: TenantScope,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    variant: Variant,
    query: AggregationQuery,
    action: Action,
) -> Response {
    let result = tokio::task::spawn_blocking(move || {
        execute(policy, tenant, db, headers, variant, query, action)
    })
    .await;
    let mut response = match result {
        Ok(Ok(response)) => response,
        Ok(Err(status)) => error(status),
        Err(_) => error(StatusCode::SERVICE_UNAVAILABLE),
    };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, axum::http::HeaderValue::from_static("no-store"));
    response
}
fn error(status: StatusCode) -> Response {
    failure(
        status,
        match status {
            StatusCode::BAD_REQUEST => "invalid_input",
            StatusCode::UNAUTHORIZED => "unauthenticated",
            StatusCode::FORBIDDEN => "forbidden",
            StatusCode::NOT_FOUND => "not_found",
            StatusCode::CONFLICT => "revision_conflict",
            StatusCode::UNPROCESSABLE_ENTITY => "invalid_configuration",
            _ => "target_unavailable",
        },
    )
}
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
#[cfg(not(target_os = "macos"))]
fn execute(
    _: Policy,
    _: TenantScope,
    _: Option<Extension<GlobalDatabase>>,
    _: HeaderMap,
    _: Variant,
    _: AggregationQuery,
    _: Action,
) -> Result<Response, StatusCode> {
    Ok(failure(StatusCode::UNPROCESSABLE_ENTITY, "unsupported_target"))
}
#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use serde::Serialize;
    use serde_json::Value;
    use std::path::Path;
    use vw_agent_apps::{
        aggregation::{omo_read, omo_slim, omo_write},
        providers::live::commit::{AppState, OmoSelection},
    };
    use vw_api_types::id::ProjectScope;
    #[derive(Serialize)]
    pub(super) struct Snapshot {
        pub standard: Option<omo_write::OmoSnapshot>,
        pub standard_present: bool,
        pub slim: omo_slim::OmoSlimSnapshot,
        rows: Vec<(String, Value)>,
        // Standard fallback inputs are included even when no unified file exists.
        standard_view: omo_read::OmoRead,
    }
    pub(super) fn capture(
        db: &GlobalDatabase,
        scope: &ProjectScope,
        home: &Path,
    ) -> Result<Snapshot, StatusCode> {
        let standard = match omo_write::snapshot(home) {
            Ok(snapshot) => Some(snapshot),
            Err(omo_write::OmoWriteError::UnsupportedTarget) => None,
            Err(error) => return Err(write_error(error)),
        };
        let slim = omo_slim::snapshot(home, None).map_err(slim_error)?;
        let read = omo_read::read(home, None, None).map_err(|error| match error {
            omo_read::OmoReadError::Forbidden => StatusCode::FORBIDDEN,
            omo_read::OmoReadError::InvalidConfiguration => StatusCode::UNPROCESSABLE_ENTITY,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        })?;
        let standard_present =
            omo_read::configuration_present(home, None).map_err(|error| match error {
                omo_read::OmoReadError::Forbidden => StatusCode::FORBIDDEN,
                omo_read::OmoReadError::InvalidConfiguration => StatusCode::UNPROCESSABLE_ENTITY,
                _ => StatusCode::SERVICE_UNAVAILABLE,
            })?;
        let rows = crate::app_providers::AppProviderStore::opencode_revision_records(
            db,
            scope.tenant_scope(),
        )
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        Ok(Snapshot { standard, standard_present, slim, rows, standard_view: read })
    }
    pub(super) fn revision(
        scope: &ProjectScope,
        host: &str,
        principal: &str,
        state: &AppState,
        snapshot: &Snapshot,
        variant: Variant,
    ) -> Result<String, StatusCode> {
        let variant = match variant {
            Variant::Standard => "standard",
            Variant::Slim => "slim",
        };
        super::super::aggregation_read_snapshot::sign(
            &serde_json::to_vec(&(
                "omo_revision_v1",
                scope,
                host,
                principal,
                variant,
                state,
                snapshot,
            ))
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        )
    }
    impl Snapshot {
        pub(super) fn same_other_inputs(&self, other: &Self, variant: Option<Variant>) -> bool {
            self.standard_present == other.standard_present
                && self.standard == other.standard && self.rows == other.rows
                && self.standard_view == other.standard_view
                // The Slim owner validates its own directory identity and exact
                // files, including a directory it provisions before staging.
                && (matches!(variant, Some(Variant::Slim)) || self.slim == other.slim)
        }
    }
    pub(super) fn fields(
        snapshot: &Snapshot,
        id: &str,
        variant: Variant,
    ) -> Result<serde_json::Map<String, Value>, StatusCode> {
        let row = &snapshot.rows.iter().find(|(key, _)| key == id).ok_or(StatusCode::NOT_FOUND)?.1;
        let category = match variant {
            Variant::Standard => "omo",
            Variant::Slim => "omo-slim",
        };
        if row["category"] != category {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let fields =
            row["settingsConfig"].as_object().ok_or(StatusCode::UNPROCESSABLE_ENTITY)?.clone();
        // Provider references must not turn this endpoint into a credential writer.
        let mut sanitized = fields.clone();
        omo_read::remove_credentials(&mut sanitized);
        if sanitized != fields {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        Ok(fields)
    }
    pub(super) fn response(
        variant: Variant,
        state: &AppState,
        snapshot: &Snapshot,
        revision: String,
    ) -> Result<Response, StatusCode> {
        match variant {
            Variant::Standard => {
                let current = match &state.omo {
                    Some(OmoSelection::Standard(id)) => Some(id.clone()),
                    _ => None,
                };
                let value = OmoState {
                    revision,
                    current_provider_id: current,
                    agents: snapshot.standard_view.agents.clone(),
                    categories: snapshot.standard_view.categories.clone(),
                    other_fields: snapshot.standard_view.other_fields.clone(),
                };
                Ok(Json(value).into_response())
            }
            Variant::Slim => {
                let current = match &state.omo {
                    Some(OmoSelection::Slim(id)) => Some(id.clone()),
                    _ => None,
                };
                let read = snapshot.slim.read(current.as_deref()).map_err(slim_error)?;
                Ok(Json(OmoSlimState {
                    revision,
                    current_provider_id: current,
                    agents: read.agents,
                    other_fields: read.other_fields,
                })
                .into_response())
            }
        }
    }
    pub(super) fn write_error(error: omo_write::OmoWriteError) -> StatusCode {
        match error {
            omo_write::OmoWriteError::Forbidden => StatusCode::FORBIDDEN,
            omo_write::OmoWriteError::InvalidConfiguration
            | omo_write::OmoWriteError::UnsupportedTarget => StatusCode::UNPROCESSABLE_ENTITY,
            omo_write::OmoWriteError::RevisionConflict
            | omo_write::OmoWriteError::RecoveryPending => StatusCode::CONFLICT,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
    pub(super) fn slim_error(error: omo_slim::OmoSlimError) -> StatusCode {
        match error {
            omo_slim::OmoSlimError::Forbidden => StatusCode::FORBIDDEN,
            omo_slim::OmoSlimError::InvalidConfiguration
            | omo_slim::OmoSlimError::UnsupportedTarget => StatusCode::UNPROCESSABLE_ENTITY,
            omo_slim::OmoSlimError::RevisionConflict
            | omo_slim::OmoSlimError::VariantConflict
            | omo_slim::OmoSlimError::RecoveryPending => StatusCode::CONFLICT,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}
#[cfg(target_os = "macos")]
fn execute(
    policy: Policy,
    tenant: TenantScope,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    variant: Variant,
    query: AggregationQuery,
    action: Action,
) -> Result<Response, StatusCode> {
    use native::*;
    use vw_agent_apps::{
        aggregation::{omo_scope::ProjectOmoEdit, omo_slim, omo_write},
        providers::{
            live::{
                commit::{ExecutionHostId, OmoSelection, ProviderTransactions},
                write_guard::lock_app,
            },
            validate_identity,
        },
    };
    use vw_api_types::{agent_apps::AppId, id::ProjectScope};
    let scope =
        ProjectScope::new(tenant.clone(), query.project_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    validate_identity(&query.target_host_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let authorize = || match &action {
        Action::Read => super::agent_apps::authorize_aggregation_read(
            policy.0.as_deref(),
            &headers,
            &scope,
            &query.target_host_id,
            AppId::OpenCode,
        ),
        _ => super::agent_apps::authorize_aggregation_write(
            policy.0.as_deref(),
            &headers,
            &scope,
            &query.target_host_id,
            AppId::OpenCode,
            false,
        ),
    };
    let binding = match authorize() {
        Ok(binding) => binding,
        Err(StatusCode::UNPROCESSABLE_ENTITY) => {
            return Ok(failure(StatusCode::UNPROCESSABLE_ENTITY, "unsupported_target"));
        }
        Err(status) => return Err(status),
    };
    let (home, root, principal) = &binding;
    let root = root.as_deref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let _guard = if matches!(action, Action::Read) {
        Some(lock_app(AppId::OpenCode, home).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?)
    } else {
        None
    };
    let tx = ProviderTransactions::open_existing(
        root,
        ExecutionHostId::new(&query.target_host_id).map_err(|_| StatusCode::BAD_REQUEST)?,
    )
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    .ok_or(StatusCode::CONFLICT)?;
    let state = tx.state(AppId::OpenCode).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if state.project_scope.as_ref() != Some(&scope) {
        return Err(if state.project_scope.is_some() {
            StatusCode::FORBIDDEN
        } else {
            StatusCode::CONFLICT
        });
    }
    state.require_project(&scope).map_err(|_| StatusCode::FORBIDDEN)?;
    if let Some(pending) =
        tx.pending(AppId::OpenCode).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    {
        if pending.tenant != tenant || pending.target.project_scope.as_ref() != Some(&scope) {
            return Err(StatusCode::FORBIDDEN);
        }
        return Ok(failure(StatusCode::CONFLICT, "recovery_pending"));
    }
    match &state.omo {
        Some(OmoSelection::Standard(id) | OmoSelection::Slim(id)) => {
            validate_identity(id).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?
        }
        _ => {}
    }
    let db = db.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
    let snapshot = capture(&db, &scope, home)?;
    let observed = revision(&scope, &query.target_host_id, principal, &state, &snapshot, variant)?;
    let unchanged = || {
        authorize().is_ok_and(|current| current == binding)
            && tx.state(AppId::OpenCode).is_ok_and(|current| current == state)
            && capture(&db, &scope, home).is_ok_and(|current| {
                snapshot.same_other_inputs(
                    &current,
                    if matches!(action, Action::Read) { None } else { Some(variant) },
                )
            })
    };
    let (expected, provider) = match &action {
        Action::Read => {
            if !unchanged() {
                return Err(StatusCode::CONFLICT);
            }
            return response(variant, &state, &snapshot, observed);
        }
        Action::Apply(request) => (&request.expected_revision, Some(request.provider_id.as_str())),
        Action::Disable(request) => (&request.expected_revision, None),
    };
    if expected.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    if *expected != observed {
        return Err(StatusCode::CONFLICT);
    }
    if let Some(id) = provider {
        validate_identity(id).map_err(|_| StatusCode::BAD_REQUEST)?;
    }
    let opposite = matches!(
        (&variant, &state.omo),
        (Variant::Standard, Some(OmoSelection::Slim(_)))
            | (Variant::Slim, Some(OmoSelection::Standard(_)))
    );
    let opposite_configuration = match variant {
        Variant::Standard => snapshot.slim.configuration_present(),
        Variant::Slim => snapshot.standard_present,
    };
    if (opposite || opposite_configuration) && provider.is_some() {
        return Ok(failure(StatusCode::CONFLICT, "variant_conflict"));
    }
    let selection = match (variant, provider) {
        (Variant::Standard, Some(id)) => OmoSelection::Standard(id.into()),
        (Variant::Slim, Some(id)) => OmoSelection::Slim(id.into()),
        (_, None) => match (variant, &state.omo) {
            (Variant::Standard, Some(OmoSelection::Slim(id))) => OmoSelection::Slim(id.clone()),
            (Variant::Slim, Some(OmoSelection::Standard(id))) => OmoSelection::Standard(id.clone()),
            _ => OmoSelection::Disabled,
        },
    };
    let fields = provider.map(|id| native::fields(&snapshot, id, variant)).transpose()?;
    let edit = ProjectOmoEdit { scope: scope.clone(), expected: state.clone(), selection };
    match variant {
        Variant::Standard => {
            match &snapshot.standard {
                Some(expected) => {
                    let change = match &fields {
                        Some(fields) => omo_write::OmoChange::Apply(fields),
                        None => omo_write::OmoChange::Disable,
                    };
                    omo_write::write_project(
                        home, &tx, &tenant, expected, change, unchanged, &edit,
                    )
                    .map_err(write_error)?;
                }
                None => {
                    if fields.is_some() || snapshot.standard_present {
                        return Ok(failure(StatusCode::UNPROCESSABLE_ENTITY, "unsupported_target"));
                    }
                    if matches!(state.omo, Some(OmoSelection::Standard(_))) {
                        vw_agent_apps::aggregation::omo_scope::disable_absent(home, &tx, &edit, unchanged).map_err(|error| match error {
                        vw_agent_apps::providers::live::write_guard::WriteError::Conflict | vw_agent_apps::providers::live::write_guard::WriteError::Recovered => StatusCode::CONFLICT,
                        _ => StatusCode::SERVICE_UNAVAILABLE,
                    })?;
                    } else if !unchanged() {
                        return Err(StatusCode::CONFLICT);
                    }
                }
            }
        }
        Variant::Slim => {
            let change = match &fields {
                Some(fields) => {
                    omo_slim::OmoSlimChange::Apply { fields, standard_provider_id: None }
                }
                None => omo_slim::OmoSlimChange::Disable,
            };
            omo_slim::write_project(home, &tx, &tenant, &snapshot.slim, change, unchanged, &edit)
                .map_err(slim_error)?;
        }
    }
    let after = tx.state(AppId::OpenCode).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let snapshot = capture(&db, &scope, home)?;
    let revision = revision(&scope, &query.target_host_id, principal, &after, &snapshot, variant)?;
    response(variant, &after, &snapshot, revision)
}
