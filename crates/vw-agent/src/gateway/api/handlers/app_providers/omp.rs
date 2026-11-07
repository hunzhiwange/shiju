//! Authorized Omp membership and read-only native import.
use super::*;
use axum::{
    http::HeaderMap,
    routing::{post, put},
};
use std::path::PathBuf;
use vw_api_types::app_providers::omp::*;
#[derive(Clone)]
struct Policy(Option<PathBuf>);
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/app-providers/omp/{id}/config", put(save))
        .route("/app-providers/omp/{id}/form/read", post(form_read))
        .route("/app-providers/omp/{id}/form", post(form_create).patch(form_update))
        .route("/app-providers/omp/{id}/add", post(add))
        .route("/app-providers/omp/{id}/remove-live", post(remove))
        .route("/app-providers/omp/config/import", post(import))
        .route("/app-providers/omp/config/state", post(state))
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(Extension(Policy(path)))
}
enum Operation {
    Save(OmpWrite),
    FormRead(OmpTarget),
    FormCreate(OmpFormWrite),
    FormUpdate(OmpFormWrite),
    Add(OmpTarget),
    Remove(OmpTarget),
    Import(OmpTarget),
    State(OmpTarget),
}
async fn save(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<OmpWrite>, JsonRejection>,
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
    body: Result<Json<OmpTarget>, JsonRejection>,
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
    body: Result<Json<OmpFormWrite>, JsonRejection>,
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
    body: Result<Json<OmpFormWrite>, JsonRejection>,
) -> Response {
    let Ok(Json(b)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    run(p, t, db, h, id, Operation::FormUpdate(b)).await
}
async fn add(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<OmpTarget>, JsonRejection>,
) -> Response {
    let Ok(Json(b)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    run(p, t, db, h, id, Operation::Add(b)).await
}
async fn remove(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<OmpTarget>, JsonRejection>,
) -> Response {
    let Ok(Json(b)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    run(p, t, db, h, id, Operation::Remove(b)).await
}
async fn import(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    body: Result<Json<OmpTarget>, JsonRejection>,
) -> Response {
    let Ok(Json(b)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    run(p, t, db, h, String::new(), Operation::Import(b)).await
}
async fn state(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    body: Result<Json<OmpTarget>, JsonRejection>,
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
        Ok(Ok(v)) => Json(v).into_response(),
        Ok(Err(s)) => s.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
#[cfg(not(target_os = "macos"))]
fn execute(
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
fn execute(
    policy: Policy,
    tenant: TenantScope,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    id: String,
    op: Operation,
) -> Result<serde_json::Value, StatusCode> {
    use vw_agent_apps::providers::{
        live::{commit::*, write_guard::WriteError},
        omp::*,
    };
    let host = match &op {
        Operation::Save(w) => &w.target_host_id,
        Operation::FormCreate(w) | Operation::FormUpdate(w) => &w.target_host_id,
        Operation::FormRead(t) => &t.target_host_id,
        Operation::Add(t) | Operation::Remove(t) | Operation::Import(t) | Operation::State(t) => {
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
            &state(read(&home).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?.as_deref())
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?,
        );
    }
    let database = match db {
        Some(Extension(db)) => db,
        None => GlobalDatabase::for_home(&crate::global::paths().home)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
    };
    let store = AppProviderStore::new(database).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if matches!(op, Operation::Import(_)) {
        let bytes = read(&home).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        let rows = declarations(bytes.as_deref()).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        let mut imported = 0;
        for (id, profile) in rows {
            if store.omp_record(&tenant, &id).map_err(|_| StatusCode::CONFLICT)?.is_none() {
                store
                    .omp_save(&tenant, &id, &profile, None, None)
                    .map_err(|_| StatusCode::CONFLICT)?;
                imported += 1;
            }
        }
        return encode(&OmpImportResult { imported });
    }
    vw_agent_apps::providers::validate_identity(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let before = store.omp_record(&tenant, &id).map_err(|_| StatusCode::CONFLICT)?;
    let read_member = || {
        let bytes = read(&home).map_err(form_error)?;
        Ok::<_, StatusCode>(declarations(bytes.as_deref()).map_err(form_error)?.remove(&id))
    };
    if matches!(op, Operation::FormRead(_)) {
        let member = read_member()?;
        if before.is_none() && member.is_none() {
            return Err(StatusCode::NOT_FOUND);
        }
        return encode(&form::view(before.as_ref(), member.as_ref()).map_err(form_error)?);
    }
    if before.as_ref().is_some_and(form::managed) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let form_write = match &op {
        Operation::FormCreate(body) => {
            if before.is_some() || body.revision.is_some() {
                return Err(StatusCode::CONFLICT);
            }
            if body.activation == OmpActivation::IfMember {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            Some(body)
        }
        Operation::FormUpdate(body) => {
            if body.revision.is_none() {
                return Err(StatusCode::CONFLICT);
            }
            if body.activation == OmpActivation::Add {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            Some(body)
        }
        _ => None,
    };
    if matches!(&op, Operation::Save(_)) && before.is_none() && read_member()?.is_some() {
        return Err(StatusCode::CONFLICT);
    }
    let mut profile = match &op {
        Operation::Save(w) => {
            validate(&w.profile).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            Some(w.profile.clone())
        }
        Operation::Add(_) => Some(
            serde_json::from_value(
                before.as_ref().and_then(|v| v.get("omp")).cloned().ok_or(StatusCode::NOT_FOUND)?,
            )
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?,
        ),
        _ => None,
    };
    let activation = match &op {
        Operation::Save(body) => Some(body.activation),
        _ => form_write.map(|body| body.activation),
    };
    let writing = activation.is_some();
    let catalog_only = activation == Some(OmpActivation::CatalogOnly);
    let mut metadata = None;
    if let Some(body) = form_write {
        let member = read_member()?;
        if matches!(op, Operation::FormCreate(_)) && member.is_some() {
            return Err(StatusCode::CONFLICT);
        }
        let (prepared, fields) =
            form::prepare(body, before.as_ref(), member.as_ref()).map_err(form_error)?;
        profile = Some(prepared);
        metadata = Some(fields);
    }
    let mut live = None;
    let mut removed_profile = None;
    if !catalog_only {
        let mut files = OmpFiles::open(&home).map_err(|_| StatusCode::CONFLICT)?;
        let tx = ProviderTransactions::new(
            &root,
            ExecutionHostId::new(host).map_err(|_| StatusCode::BAD_REQUEST)?,
        )
        .map_err(|_| StatusCode::CONFLICT)?;
        if tx.recover(&mut files.guard).map_err(|_| StatusCode::CONFLICT)?.is_some() {
            return encode(&OmpResult {
                catalog_saved: false,
                live: OmpLiveResult::RecoveryRequired,
            });
        }
        let expected = tx.state(AppId::Omp).map_err(|_| StatusCode::CONFLICT)?;
        if expected.pointer.is_some()
            || expected.mode_tenant.as_ref().is_some_and(|t| t != &tenant)
            || expected.mode.as_ref().is_some_and(|m| m.attached || m.mode != Mode::Direct)
        {
            return Err(StatusCode::CONFLICT);
        }
        let (name, pre) = files.snapshot().map_err(|_| StatusCode::CONFLICT)?;
        let member = declarations(pre.as_deref()).map_err(form_error)?.remove(&id);
        if let Some(body) = form_write {
            if matches!(op, Operation::FormCreate(_)) && member.is_some() {
                return Err(StatusCode::CONFLICT);
            }
            let (prepared, fields) =
                form::prepare(body, before.as_ref(), member.as_ref()).map_err(form_error)?;
            profile = Some(prepared);
            metadata = Some(fields);
        }
        let eligible = activation != Some(OmpActivation::IfMember) || member.is_some();
        if matches!(op, Operation::Remove(_)) {
            removed_profile = member.map(|mut profile| {
                if let Some(name) = before.as_ref().and_then(|row| row["name"].as_str()) {
                    profile.name = name.to_owned();
                }
                profile
            });
        }
        if eligible {
            let activation = activation.unwrap_or(OmpActivation::Add);
            let next =
                patch(pre.as_deref(), &id, profile.as_ref().map(|p| &p.declaration), activation)
                    .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            live = Some((files, tx, expected, name, pre, next));
        }
    }
    let catalog_saved = writing || removed_profile.is_some();
    let saved = if let Some(profile) = removed_profile {
        Some(
            store
                .omp_save(&tenant, &id, &profile, before.as_ref(), None)
                .map_err(|_| StatusCode::CONFLICT)?,
        )
    } else if writing {
        Some(
            store
                .omp_save(
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
    let Some((mut files, tx, expected, name, pre, next)) = live else {
        return encode(&OmpResult { catalog_saved, live: OmpLiveResult::NotAttempted });
    };
    let result = files.apply(
        &tx,
        &tenant,
        &expected,
        name,
        pre.as_deref(),
        next,
        || matches!(store.omp_record(&tenant, &id), Ok(current) if current == saved),
    );
    let live = match result {
        Ok(()) => OmpLiveResult::Applied,
        Err(WriteError::Recovered) => OmpLiveResult::RecoveryRequired,
        Err(_) => {
            if tx.pending(AppId::Omp).ok().flatten().is_some() {
                OmpLiveResult::RecoveryRequired
            } else {
                OmpLiveResult::Failed
            }
        }
    };
    encode(&OmpResult { catalog_saved, live })
}
fn encode(value: &impl serde::Serialize) -> Result<serde_json::Value, StatusCode> {
    serde_json::to_value(value).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}

#[cfg(target_os = "macos")]
fn form_error(error: vw_agent_apps::providers::live::write_guard::WriteError) -> StatusCode {
    use vw_agent_apps::providers::live::write_guard::WriteError;
    match error {
        WriteError::Conflict => StatusCode::CONFLICT,
        _ => StatusCode::UNPROCESSABLE_ENTITY,
    }
}
