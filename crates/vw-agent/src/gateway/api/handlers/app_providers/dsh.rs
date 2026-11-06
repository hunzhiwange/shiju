//! Authorized Dsh provider configuration; paths and commands belong to the executor.
use super::*;
use axum::{
    http::HeaderMap,
    routing::{post, put},
};
use std::path::PathBuf;
use vw_api_types::app_providers::dsh::*;
#[derive(Clone)]
struct Policy(Option<PathBuf>);
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/app-providers/dsh/{id}/config", put(save))
        .route("/app-providers/dsh/{id}/add", post(add))
        .route("/app-providers/dsh/config/state", post(state))
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(Extension(Policy(path)))
}
enum Operation {
    Save(DshWrite),
    Add(DshTarget),
    State(DshTarget),
}
async fn save(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<DshWrite>, JsonRejection>,
) -> Response {
    let Ok(Json(b)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    run(p, t, db, h, id, Operation::Save(b)).await
}
async fn add(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<DshTarget>, JsonRejection>,
) -> Response {
    let Ok(Json(b)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    run(p, t, db, h, id, Operation::Add(b)).await
}
async fn state(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    body: Result<Json<DshTarget>, JsonRejection>,
) -> Response {
    let Ok(Json(b)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    run(p, t, db, h, String::new(), Operation::State(b)).await
}
async fn run(
    p: Policy,
    t: TenantScope,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    id: String,
    op: Operation,
) -> Response {
    match tokio::task::spawn_blocking(move || execute(p, t, db, h, id, op)).await {
        Ok(r) => r,
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
fn execute(
    p: Policy,
    t: TenantScope,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    id: String,
    op: Operation,
) -> Response {
    match execute_inner(p, t, db, h, id, op) {
        Ok(v) if serde_json::from_value::<DshError>(v.clone()).is_ok() => {
            (StatusCode::UNPROCESSABLE_ENTITY, Json(v)).into_response()
        }
        Ok(v) => Json(v).into_response(),
        Err(s) => s.into_response(),
    }
}
#[cfg(not(target_os = "macos"))]
fn execute_inner(
    _: Policy,
    _: TenantScope,
    _: Option<Extension<GlobalDatabase>>,
    _: HeaderMap,
    _: String,
    _: Operation,
) -> Result<serde_json::Value, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}
#[cfg(target_os = "macos")]
fn execute_inner(
    policy: Policy,
    tenant: TenantScope,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    id: String,
    op: Operation,
) -> Result<serde_json::Value, StatusCode> {
    use vw_agent_apps::providers::{
        dsh::*,
        live::{commit::*, write_guard::WriteError},
    };
    let host = match &op {
        Operation::Save(w) => &w.target_host_id,
        Operation::Add(t) | Operation::State(t) => &t.target_host_id,
    };
    let (home, root) = super::super::agent_apps::provider_editor_context(
        policy.0.as_deref(),
        &headers,
        &tenant,
        host,
        !matches!(op, Operation::State(_)),
    )?;
    if matches!(op, Operation::State(_)) {
        return encode(
            &state(&read(&home).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?)
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?,
        );
    }
    vw_agent_apps::providers::validate_identity(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let database = match db {
        Some(Extension(db)) => db,
        None => GlobalDatabase::for_home(&crate::global::paths().home)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
    };
    let store = AppProviderStore::new(database).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let before = store.dsh_record(&tenant, &id).map_err(|_| StatusCode::CONFLICT)?;
    if !valid_binding(before.as_ref()) {
        return Ok(invalid_binding());
    }
    let profile = match &op {
        Operation::Save(w) => w.profile.clone(),
        Operation::Add(_) => serde_json::from_value(
            before.as_ref().and_then(|v| v.get("dsh")).cloned().ok_or(StatusCode::NOT_FOUND)?,
        )
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?,
        Operation::State(_) => unreachable!(),
    };
    validate(&id, &profile).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    let writing = matches!(op, Operation::Save(_));
    let activation = match &op {
        Operation::Save(w) => w.activation,
        _ => DshActivation::Add,
    };
    let catalog_only = activation == DshActivation::CatalogOnly;
    // All checks that could reject a profile happen before saving its catalog row.
    let observation = read(&home).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    if !credential_binding_valid(&observation, &profile) {
        return Ok(invalid_binding());
    }
    if writing
        && before.is_none()
        && state(&observation).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?.members.contains(&id)
    {
        return Err(StatusCode::CONFLICT);
    }
    let live = if catalog_only {
        None
    } else {
        let mut files = DshFiles::open(&home).map_err(|_| StatusCode::CONFLICT)?;
        let tx = ProviderTransactions::new(
            &root,
            ExecutionHostId::new(host).map_err(|_| StatusCode::BAD_REQUEST)?,
        )
        .map_err(|_| StatusCode::CONFLICT)?;
        if tx.recover(&mut files.guard).map_err(|_| StatusCode::CONFLICT)?.is_some() {
            return encode(&DshResult {
                catalog_saved: false,
                live: DshLiveResult::RecoveryRequired,
            });
        }
        let expected = tx.state(AppId::Dsh).map_err(|_| StatusCode::CONFLICT)?;
        if expected.pointer.is_some()
            || expected.mode_tenant.as_ref().is_some_and(|t| t != &tenant)
            || expected.mode.as_ref().is_some_and(|m| m.attached || m.mode != Mode::Direct)
        {
            return Err(StatusCode::CONFLICT);
        }
        let pre = files.snapshot().map_err(|_| StatusCode::CONFLICT)?;
        if !credential_binding_valid(&pre, &profile) {
            return Ok(invalid_binding());
        }
        let next = plan(&pre, &id, &profile, activation).map_err(|e| {
            if e == WriteError::Conflict {
                StatusCode::CONFLICT
            } else {
                StatusCode::UNPROCESSABLE_ENTITY
            }
        })?;
        if next == pre { None } else { Some((files, tx, expected, pre, next)) }
    };
    let saved = if writing {
        Some(
            store
                .dsh_save(&tenant, &id, &profile, before.as_ref())
                .map_err(|_| StatusCode::CONFLICT)?,
        )
    } else {
        before
    };
    let Some((mut files, tx, expected, pre, next)) = live else {
        return encode(&DshResult { catalog_saved: writing, live: DshLiveResult::NotAttempted });
    };
    let result = files.apply(
        &tx,
        &tenant,
        &expected,
        &pre,
        &next,
        || read(&home).ok().as_ref() == Some(&pre) && matches!(store.dsh_record(&tenant,&id),Ok(current) if current==saved),
    );
    let live = match result {
        Ok(()) => DshLiveResult::Applied,
        Err(WriteError::Recovered) => DshLiveResult::RecoveryRequired,
        Err(_) => {
            if tx.pending(AppId::Dsh).ok().flatten().is_some() {
                DshLiveResult::RecoveryRequired
            } else {
                DshLiveResult::Failed
            }
        }
    };
    encode(&DshResult { catalog_saved: writing, live })
}
fn encode(value: &impl serde::Serialize) -> Result<serde_json::Value, StatusCode> {
    serde_json::to_value(value).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}
fn invalid_binding() -> serde_json::Value {
    serde_json::json!({"code":"invalid_binding"})
}
