//! Authorized OpenClaw members, declared default models and read-only native import.
use super::*;
use axum::{
    http::{HeaderMap, Method},
    routing::{post, put},
};
use std::path::PathBuf;
use vw_api_types::app_providers::openclaw::*;
#[derive(Clone)]
struct Policy(Option<PathBuf>);
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/app-providers/openclaw/{id}/config", put(save))
        .route("/app-providers/openclaw/{id}/form/read", post(read_form))
        .route("/app-providers/openclaw/{id}/form", post(write_form).patch(write_form))
        .route("/app-providers/openclaw/{id}/add", post(add))
        .route("/app-providers/openclaw/{id}/remove-live", post(remove))
        .route("/app-providers/openclaw/config/default-model", put(model))
        .route("/app-providers/openclaw/config/import", post(import))
        .route("/app-providers/openclaw/config/state", post(state))
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(Extension(Policy(path)))
}
enum Operation {
    Save(OpenClawWrite),
    ReadForm(OpenClawTarget),
    WriteForm(OpenClawFormWrite, bool),
    Add(OpenClawTarget),
    Remove(OpenClawTarget),
    Model(OpenClawDefaultModel),
    Import(OpenClawTarget),
    State(OpenClawTarget),
}
async fn save(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<OpenClawWrite>, JsonRejection>,
) -> Response {
    let Ok(Json(b)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    run(p, t, db, h, id, Operation::Save(b)).await
}
async fn read_form(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<OpenClawTarget>, JsonRejection>,
) -> Response {
    let Ok(Json(b)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    run(p, t, db, h, id, Operation::ReadForm(b)).await
}
async fn write_form(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    Path(id): Path<String>,
    method: Method,
    body: Result<Json<OpenClawFormWrite>, JsonRejection>,
) -> Response {
    let Ok(Json(b)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    run(p, t, db, h, id, Operation::WriteForm(b, method == Method::POST)).await
}
async fn add(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<OpenClawTarget>, JsonRejection>,
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
    body: Result<Json<OpenClawTarget>, JsonRejection>,
) -> Response {
    let Ok(Json(b)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    run(p, t, db, h, id, Operation::Remove(b)).await
}
async fn model(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    body: Result<Json<OpenClawDefaultModel>, JsonRejection>,
) -> Response {
    let Ok(Json(b)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    run(p, t, db, h, String::new(), Operation::Model(b)).await
}
async fn import(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    body: Result<Json<OpenClawTarget>, JsonRejection>,
) -> Response {
    let Ok(Json(b)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    run(p, t, db, h, String::new(), Operation::Import(b)).await
}
async fn state(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    body: Result<Json<OpenClawTarget>, JsonRejection>,
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
    mut id: String,
    op: Operation,
) -> Result<serde_json::Value, StatusCode> {
    use vw_agent_apps::providers::{
        live::{commit::*, write_guard::WriteError},
        openclaw::*,
    };
    let host = match &op {
        Operation::Save(w) => &w.target_host_id,
        Operation::WriteForm(w, _) => &w.target_host_id,
        Operation::Add(t)
        | Operation::Remove(t)
        | Operation::Import(t)
        | Operation::State(t)
        | Operation::ReadForm(t) => &t.target_host_id,
        Operation::Model(m) => &m.target_host_id,
    };
    let (home, root) = super::super::agent_apps::provider_editor_context(
        policy.0.as_deref(),
        &headers,
        &tenant,
        host,
        !matches!(op, Operation::State(_) | Operation::ReadForm(_)),
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
            if store.openclaw_record(&tenant, &id).map_err(|_| StatusCode::CONFLICT)?.is_none() {
                store
                    .openclaw_save(&tenant, &id, &profile, None, None)
                    .map_err(|_| StatusCode::CONFLICT)?;
                imported += 1;
            }
        }
        return encode(&OpenClawImportResult { imported });
    }
    if let Operation::Model(m) = &op {
        id =
            m.model.split_once('/').map(|(id, _)| id.to_string()).ok_or(StatusCode::BAD_REQUEST)?;
    }
    vw_agent_apps::providers::validate_identity(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let before = store.openclaw_record(&tenant, &id).map_err(|_| StatusCode::CONFLICT)?;
    if matches!(op, Operation::ReadForm(_)) {
        let row = before.as_ref().ok_or(StatusCode::NOT_FOUND)?;
        return encode(&form::view(row).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?);
    }
    if before.as_ref().is_some_and(form::managed) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let mut metadata = None;
    let mut activation = None;
    let profile = match &op {
        Operation::Save(w) => {
            validate(&w.profile).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            activation = Some(w.activation);
            Some(w.profile.clone())
        }
        Operation::WriteForm(w, create) => {
            match (*create, before.is_some()) {
                (true, true) => return Err(StatusCode::CONFLICT),
                (false, false) => return Err(StatusCode::NOT_FOUND),
                _ => {}
            }
            if matches!(
                (*create, w.activation),
                (true, OpenClawActivation::IfMember) | (false, OpenClawActivation::Add)
            ) || (*create && w.revision.is_some())
            {
                return Err(StatusCode::BAD_REQUEST);
            }
            let (profile, fields) = form::prepare(w, before.as_ref()).map_err(|error| {
                if matches!(error, WriteError::Conflict) {
                    StatusCode::CONFLICT
                } else {
                    StatusCode::UNPROCESSABLE_ENTITY
                }
            })?;
            activation = Some(w.activation);
            metadata = Some(fields);
            Some(profile)
        }
        Operation::Add(_) => Some(
            serde_json::from_value(
                before
                    .as_ref()
                    .and_then(|v| v.get("openclaw"))
                    .cloned()
                    .ok_or(StatusCode::NOT_FOUND)?,
            )
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?,
        ),
        _ => None,
    };
    let writing = activation.is_some();
    let catalog_only = activation == Some(OpenClawActivation::CatalogOnly);
    let mut live = None;
    if !catalog_only {
        let mut files = OpenClawFiles::open(&home).map_err(|_| StatusCode::CONFLICT)?;
        let tx = ProviderTransactions::new(
            &root,
            ExecutionHostId::new(host).map_err(|_| StatusCode::BAD_REQUEST)?,
        )
        .map_err(|_| StatusCode::CONFLICT)?;
        if tx.recover(&mut files.guard).map_err(|_| StatusCode::CONFLICT)?.is_some() {
            return encode(&OpenClawResult {
                catalog_saved: false,
                live: OpenClawLiveResult::RecoveryRequired,
            });
        }
        let expected = tx.state(AppId::OpenClaw).map_err(|_| StatusCode::CONFLICT)?;
        if expected.pointer.is_some()
            || expected.mode_tenant.as_ref().is_some_and(|t| t != &tenant)
            || expected.mode.as_ref().is_some_and(|m| m.attached || m.mode != Mode::Direct)
        {
            return Err(StatusCode::CONFLICT);
        }
        let (name, pre) = files.snapshot().map_err(|_| StatusCode::CONFLICT)?;
        let eligible = activation != Some(OpenClawActivation::IfMember)
            || declarations(pre.as_deref())
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?
                .contains_key(&id);
        if eligible {
            let model = match &op {
                Operation::Model(m) => Some(m.model.as_str()),
                _ => None,
            };
            let next = patch(pre.as_deref(), &id, profile.as_ref().map(|p| &p.declaration), model)
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            live = Some((files, tx, expected, name, pre, next));
        }
    }
    let saved = if writing {
        Some(
            store
                .openclaw_save(
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
        return encode(&OpenClawResult {
            catalog_saved: writing,
            live: OpenClawLiveResult::NotAttempted,
        });
    };
    let result = files.apply(
        &tx,
        &tenant,
        &expected,
        name,
        pre.as_deref(),
        next,
        || matches!(store.openclaw_record(&tenant, &id), Ok(current) if current == saved),
    );
    let live = match result {
        Ok(()) => OpenClawLiveResult::Applied,
        Err(WriteError::Recovered) => OpenClawLiveResult::RecoveryRequired,
        Err(_) => {
            if tx.pending(AppId::OpenClaw).ok().flatten().is_some() {
                OpenClawLiveResult::RecoveryRequired
            } else {
                OpenClawLiveResult::Failed
            }
        }
    };
    encode(&OpenClawResult { catalog_saved: writing, live })
}
fn encode(value: &impl serde::Serialize) -> Result<serde_json::Value, StatusCode> {
    serde_json::to_value(value).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}
