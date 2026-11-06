//! Authorized Hermes membership and read-only native import.
use super::*;
use axum::{
    http::HeaderMap,
    routing::{post, put},
};
use std::path::PathBuf;
use vw_api_types::app_providers::hermes::*;
#[derive(Clone)]
struct Policy(Option<PathBuf>);
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/app-providers/hermes/{id}/config", put(save))
        .route("/app-providers/hermes/{id}/add", post(add))
        .route("/app-providers/hermes/{id}/remove-live", post(remove))
        .route("/app-providers/hermes/config/import", post(import))
        .route("/app-providers/hermes/config/state", post(state))
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(Extension(Policy(path)))
}
enum Operation {
    Save(HermesWrite),
    Add(HermesTarget),
    Remove(HermesTarget),
    Import(HermesTarget),
    State(HermesTarget),
}
async fn save(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<HermesWrite>, JsonRejection>,
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
    body: Result<Json<HermesTarget>, JsonRejection>,
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
    body: Result<Json<HermesTarget>, JsonRejection>,
) -> Response {
    let Ok(Json(b)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    run(p, t, db, h, id, Operation::Remove(b)).await
}
async fn import(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    body: Result<Json<HermesTarget>, JsonRejection>,
) -> Response {
    let Ok(Json(b)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    run(p, t, db, h, String::new(), Operation::Import(b)).await
}
async fn state(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    body: Result<Json<HermesTarget>, JsonRejection>,
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
        hermes::*,
        live::{commit::*, write_guard::WriteError},
    };
    let host = match &op {
        Operation::Save(w) => &w.target_host_id,
        Operation::Add(t) | Operation::Remove(t) | Operation::Import(t) | Operation::State(t) => {
            &t.target_host_id
        }
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
            if store.hermes_record(&tenant, &id).map_err(|_| StatusCode::CONFLICT)?.is_none() {
                store
                    .hermes_save(&tenant, &id, &profile, None)
                    .map_err(|_| StatusCode::CONFLICT)?;
                imported += 1;
            }
        }
        return encode(&HermesImportResult { imported });
    }
    vw_agent_apps::providers::validate_identity(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let before = store.hermes_record(&tenant, &id).map_err(|_| StatusCode::CONFLICT)?;
    if before.as_ref().is_some_and(|v| {
        v.pointer("/meta/authBinding").is_some()
            || v.pointer("/meta/liveConfigManaged").is_some_and(|v| v == true)
    }) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let profile = match &op {
        Operation::Save(w) => {
            validate(&w.profile).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            let mut profile = w.profile.clone();
            profile.declaration = normalize_declaration(&profile.declaration)
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            if let Some(previous) = before.as_ref().and_then(|v| v.pointer("/hermes/declaration")) {
                let mut merged =
                    previous.as_object().cloned().ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
                for (key, value) in
                    profile.declaration.as_object().ok_or(StatusCode::UNPROCESSABLE_ENTITY)?
                {
                    merged.insert(key.clone(), value.clone());
                }
                profile.declaration = serde_json::Value::Object(merged);
            }
            validate(&profile).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            Some(profile)
        }
        Operation::Add(_) => Some(
            serde_json::from_value(
                before
                    .as_ref()
                    .and_then(|v| v.get("hermes"))
                    .cloned()
                    .ok_or(StatusCode::NOT_FOUND)?,
            )
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?,
        ),
        _ => None,
    };
    let writing = matches!(op, Operation::Save(_));
    let catalog_only =
        matches!(&op,Operation::Save(w) if w.activation==HermesActivation::CatalogOnly);
    let mut live = None;
    if !catalog_only {
        let mut files = HermesFiles::open(&home).map_err(|_| StatusCode::CONFLICT)?;
        let tx = ProviderTransactions::new(
            &root,
            ExecutionHostId::new(host).map_err(|_| StatusCode::BAD_REQUEST)?,
        )
        .map_err(|_| StatusCode::CONFLICT)?;
        if tx.recover(&mut files.guard).map_err(|_| StatusCode::CONFLICT)?.is_some() {
            return encode(&HermesResult {
                catalog_saved: false,
                live: HermesLiveResult::RecoveryRequired,
            });
        }
        let expected = tx.state(AppId::Hermes).map_err(|_| StatusCode::CONFLICT)?;
        if expected.pointer.is_some()
            || expected.mode_tenant.as_ref().is_some_and(|t| t != &tenant)
            || expected.mode.as_ref().is_some_and(|m| m.attached || m.mode != Mode::Direct)
        {
            return Err(StatusCode::CONFLICT);
        }
        let (name, pre) = files.snapshot().map_err(|_| StatusCode::CONFLICT)?;
        let eligible = !matches!(&op,Operation::Save(w) if w.activation==HermesActivation::IfMember && !declarations(pre.as_deref()).map_err(|_|StatusCode::UNPROCESSABLE_ENTITY)?.contains_key(&id));
        if eligible {
            let next = patch(pre.as_deref(), &id, profile.as_ref().map(|p| &p.declaration))
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            live = Some((files, tx, expected, name, pre, next));
        }
    }
    let saved = if writing {
        Some(
            store
                .hermes_save(
                    &tenant,
                    &id,
                    profile.as_ref().ok_or(StatusCode::BAD_REQUEST)?,
                    before.as_ref(),
                )
                .map_err(|_| StatusCode::CONFLICT)?,
        )
    } else {
        before
    };
    let Some((mut files, tx, expected, name, pre, next)) = live else {
        return encode(&HermesResult {
            catalog_saved: writing,
            live: HermesLiveResult::NotAttempted,
        });
    };
    let result = files.apply(
        &tx,
        &tenant,
        &expected,
        name,
        pre.as_deref(),
        next,
        || matches!(store.hermes_record(&tenant, &id), Ok(current) if current == saved),
    );
    let live = match result {
        Ok(()) => HermesLiveResult::Applied,
        Err(WriteError::Recovered) => HermesLiveResult::RecoveryRequired,
        Err(_) => {
            if tx.pending(AppId::Hermes).ok().flatten().is_some() {
                HermesLiveResult::RecoveryRequired
            } else {
                HermesLiveResult::Failed
            }
        }
    };
    encode(&HermesResult { catalog_saved: writing, live })
}
fn encode(value: &impl serde::Serialize) -> Result<serde_json::Value, StatusCode> {
    serde_json::to_value(value).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}
