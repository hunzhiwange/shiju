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
        .route("/app-providers/hermes/{id}/form/read", post(form_read))
        .route("/app-providers/hermes/{id}/form", post(form_create).patch(form_update))
        .route("/app-providers/hermes/{id}/add", post(add))
        .route("/app-providers/hermes/{id}/remove-live", post(remove))
        .route("/app-providers/hermes/config/import", post(import))
        .route("/app-providers/hermes/config/state", post(state))
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(Extension(Policy(path)))
}
enum Operation {
    Save(HermesWrite),
    FormRead(HermesTarget),
    FormCreate(HermesFormWrite),
    FormUpdate(HermesFormWrite),
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
async fn form_read(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<HermesTarget>, JsonRejection>,
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
    body: Result<Json<HermesFormWrite>, JsonRejection>,
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
    body: Result<Json<HermesFormWrite>, JsonRejection>,
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
            if store.hermes_record(&tenant, &id).map_err(|_| StatusCode::CONFLICT)?.is_none() {
                store
                    .hermes_save(&tenant, &id, &profile, None, None)
                    .map_err(|_| StatusCode::CONFLICT)?;
                imported += 1;
            }
        }
        return encode(&HermesImportResult { imported });
    }
    vw_agent_apps::providers::validate_identity(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let before = store.hermes_record(&tenant, &id).map_err(|_| StatusCode::CONFLICT)?;
    if matches!(op, Operation::FormRead(_)) {
        let bytes = read(&home).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        let mut members = declarations(bytes.as_deref()).map_err(form_error)?;
        let member = members.remove(&id);
        let read_only = member.as_ref().is_some_and(read_only_member);
        let row = match before {
            Some(row) => row,
            None => native_row(member.as_ref().filter(|_| !read_only).ok_or_else(|| {
                if read_only { StatusCode::UNPROCESSABLE_ENTITY } else { StatusCode::NOT_FOUND }
            })?),
        };
        let mut view = form::view(&row).map_err(form_error)?;
        if read_only {
            view.config = None;
        }
        return encode(&view);
    }
    let form_write = match &op {
        Operation::FormCreate(body) => {
            if before.is_some() || body.revision.is_some() {
                return Err(StatusCode::CONFLICT);
            }
            if body.activation == HermesActivation::IfMember {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            Some(body)
        }
        Operation::FormUpdate(body) => {
            if before.is_none() && body.activation != HermesActivation::IfMember {
                return Err(StatusCode::NOT_FOUND);
            }
            if body.activation == HermesActivation::Add {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            Some(body)
        }
        _ => None,
    };
    let mut metadata = None;
    if before.as_ref().is_some_and(|v| {
        v.pointer("/meta/authBinding").is_some()
            || v.pointer("/meta/liveConfigManaged").is_some_and(|v| v == true)
    }) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let mut profile = match &op {
        Operation::FormUpdate(_) if before.is_none() => None,
        Operation::FormCreate(body) | Operation::FormUpdate(body) => {
            let (profile, fields) = form::prepare(body, before.as_ref()).map_err(form_error)?;
            metadata = Some(fields);
            Some(profile)
        }
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
    let activation = match &op {
        Operation::Save(body) => Some(body.activation),
        _ => form_write.map(|body| body.activation),
    };
    let writing = activation.is_some();
    let catalog_only = activation == Some(HermesActivation::CatalogOnly);
    if catalog_only && form_write.is_some() {
        let bytes = read(&home).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        if let Some(member) = declarations(bytes.as_deref()).map_err(form_error)?.get(&id) {
            if matches!(op, Operation::FormCreate(_)) {
                return Err(StatusCode::CONFLICT);
            }
            if read_only_member(member) {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
        }
    }
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
        let members = if form_write.is_some() || activation == Some(HermesActivation::IfMember) {
            declarations(pre.as_deref()).map_err(form_error)?
        } else {
            Default::default()
        };
        let member = members.get(&id);
        if let Operation::FormUpdate(body) = &op
            && before.is_none()
        {
            let member = member.ok_or(StatusCode::CONFLICT)?;
            let (native_profile, fields) =
                form::prepare(body, Some(&native_row(member))).map_err(form_error)?;
            profile = Some(native_profile);
            metadata = Some(fields);
        }
        if let (Some(body), Some(previous), Some(next)) = (form_write, member, profile.as_ref()) {
            if body.activation == HermesActivation::Add {
                return Err(StatusCode::CONFLICT);
            }
            form::check_live_credentials(
                body,
                &normalize_declaration(&previous.declaration).map_err(form_error)?,
                &next.declaration,
            )
            .map_err(form_error)?;
        }
        if activation != Some(HermesActivation::IfMember) || member.is_some() {
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
                    metadata,
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

#[cfg(target_os = "macos")]
fn read_only_member(profile: &HermesProfile) -> bool {
    profile.declaration["_cc_source"] == "providers_dict"
}

#[cfg(target_os = "macos")]
fn native_row(profile: &HermesProfile) -> serde_json::Value {
    serde_json::json!({"name":profile.name, "hermes":profile})
}

#[cfg(target_os = "macos")]
fn form_error(error: vw_agent_apps::providers::live::write_guard::WriteError) -> StatusCode {
    use vw_agent_apps::providers::live::write_guard::WriteError;
    if matches!(error, WriteError::Conflict) {
        StatusCode::CONFLICT
    } else {
        StatusCode::UNPROCESSABLE_ENTITY
    }
}
