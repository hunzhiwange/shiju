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
        .route("/app-providers/dsh/{id}/remove-live", post(remove))
        .route("/app-providers/dsh/{id}/form/read", post(form_read))
        .route("/app-providers/dsh/{id}/form", post(form_create).patch(form_update))
        .route("/app-providers/dsh/config/state", post(state))
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(Extension(Policy(path)))
}
enum Operation {
    Save(DshWrite),
    FormRead(DshTarget),
    FormCreate(DshFormWrite),
    FormUpdate(DshFormWrite),
    Remove(DshTarget),
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
async fn form_read(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<DshTarget>, JsonRejection>,
) -> Response {
    let Ok(Json(b)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    run(p, t, db, h, id, Operation::FormRead(b)).await
}
async fn form_create(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<DshFormWrite>, JsonRejection>,
) -> Response {
    let Ok(Json(b)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    run(p, t, db, h, id, Operation::FormCreate(b)).await
}
async fn form_update(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<DshFormWrite>, JsonRejection>,
) -> Response {
    let Ok(Json(b)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    run(p, t, db, h, id, Operation::FormUpdate(b)).await
}
async fn remove(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<DshTarget>, JsonRejection>,
) -> Response {
    let Ok(Json(b)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    run(p, t, db, h, id, Operation::Remove(b)).await
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
        Operation::FormCreate(w) | Operation::FormUpdate(w) => &w.target_host_id,
        Operation::FormRead(t) | Operation::Remove(t) | Operation::Add(t) | Operation::State(t) => {
            &t.target_host_id
        }
    };
    let (home, root) = super::super::agent_apps::provider_editor_context(
        policy.0.as_deref(),
        &headers,
        &tenant,
        host,
        !matches!(op, Operation::State(_) | Operation::FormRead(_)),
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
    let observation = read(&home).map_err(form_error)?;
    let observed_member = member(&observation, &id).map_err(form_error)?;
    if matches!(op, Operation::FormRead(_)) {
        if before.is_none() && observed_member.is_none() {
            return Err(StatusCode::NOT_FOUND);
        }
        return encode(
            &form::view(&id, before.as_ref(), observed_member.as_ref()).map_err(form_error)?,
        );
    }
    let form_write = match &op {
        Operation::FormCreate(body) => {
            if before.is_some() || observed_member.is_some() || body.revision.is_some() {
                return Err(StatusCode::CONFLICT);
            }
            if body.activation == DshActivation::IfMember {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            Some(body)
        }
        Operation::FormUpdate(body) => {
            if body.revision.is_none() {
                return Err(StatusCode::CONFLICT);
            }
            if body.activation == DshActivation::Add {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            Some(body)
        }
        _ => None,
    };
    let mut metadata = None;
    let mut profile = match &op {
        Operation::Save(w) => Some(w.profile.clone()),
        Operation::Add(_) => Some(
            serde_json::from_value(
                before.as_ref().and_then(|v| v.get("dsh")).cloned().ok_or(StatusCode::NOT_FOUND)?,
            )
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?,
        ),
        _ => None,
    };
    if let Some(body) = form_write {
        let (prepared, fields) =
            form::prepare(&id, body, before.as_ref(), observed_member.as_ref())
                .map_err(form_error)?;
        profile = Some(prepared);
        metadata = Some(fields);
    }
    if let Some(profile) = &profile {
        validate(&id, profile).map_err(form_error)?;
        if !credential_binding_valid(&observation, profile) {
            return Ok(invalid_binding());
        }
    }
    let writing =
        matches!(op, Operation::Save(_) | Operation::FormCreate(_) | Operation::FormUpdate(_));
    let activation = match &op {
        Operation::Save(w) => w.activation,
        _ => form_write.map_or(DshActivation::Add, |w| w.activation),
    };
    let catalog_only = activation == DshActivation::CatalogOnly;
    if matches!(op, Operation::Save(_)) && before.is_none() && observed_member.is_some() {
        return Err(StatusCode::CONFLICT);
    }
    let mut removed_profile = None;
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
        let locked_member = member(&pre, &id).map_err(form_error)?;
        if let Some(body) = form_write {
            let (prepared, fields) =
                form::prepare(&id, body, before.as_ref(), locked_member.as_ref())
                    .map_err(form_error)?;
            profile = Some(prepared);
            metadata = Some(fields);
        }
        let next = if matches!(op, Operation::Remove(_)) {
            let next = remove(&pre, &id).map_err(form_error)?;
            removed_profile = locked_member.map(|mut p| {
                if let Some(name) = before.as_ref().and_then(|v| v["name"].as_str()) {
                    p.name = name.to_owned();
                }
                p
            });
            next
        } else {
            let profile = profile.as_ref().ok_or(StatusCode::BAD_REQUEST)?;
            if !credential_binding_valid(&pre, profile) {
                return Ok(invalid_binding());
            }
            plan(&pre, &id, profile, activation).map_err(form_error)?
        };
        if next == pre { None } else { Some((files, tx, expected, pre, next)) }
    };
    let catalog_saved = writing || removed_profile.is_some();
    if let Some(removed) = removed_profile {
        profile = Some(removed);
    }
    let saved = if catalog_saved {
        Some(
            store
                .dsh_save(
                    &tenant,
                    &id,
                    profile.as_ref().ok_or(StatusCode::BAD_REQUEST)?,
                    before.as_ref(),
                    metadata,
                )
                .map_err(|_| StatusCode::CONFLICT)?,
        )
    } else {
        before
    };
    let Some((mut files, tx, expected, pre, next)) = live else {
        return encode(&DshResult { catalog_saved, live: DshLiveResult::NotAttempted });
    };
    let result = files.apply(&tx, &tenant, &expected, &pre, &next, || {
        read(&home).ok().as_ref() == Some(&pre)
            && matches!(store.dsh_record(&tenant,&id),Ok(current) if current==saved)
    });
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
    encode(&DshResult { catalog_saved, live })
}
fn encode(value: &impl serde::Serialize) -> Result<serde_json::Value, StatusCode> {
    serde_json::to_value(value).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}
fn invalid_binding() -> serde_json::Value {
    serde_json::json!({"code":"invalid_binding"})
}

#[cfg(target_os = "macos")]
fn form_error(error: vw_agent_apps::providers::live::write_guard::WriteError) -> StatusCode {
    use vw_agent_apps::providers::live::write_guard::WriteError;
    match error {
        WriteError::Conflict => StatusCode::CONFLICT,
        _ => StatusCode::UNPROCESSABLE_ENTITY,
    }
}
