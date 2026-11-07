//! Authorized fixed Gemini save/switch operations. No arbitrary command or file API.
use super::*;
use axum::{
    http::{HeaderMap, Method},
    routing::{post, put},
};
use std::path::PathBuf;
use vw_api_types::app_providers::gemini::*;
#[derive(Clone)]
struct Policy(Option<PathBuf>);

enum Request {
    Save(GeminiWrite),
    Form { body: GeminiFormWrite, create: bool },
    Apply(GeminiTarget, GeminiActivation),
}
impl Request {
    fn target(&self) -> &str {
        match self {
            Self::Save(body) => &body.target_host_id,
            Self::Form { body, create } => {
                let _ = create;
                &body.target_host_id
            }
            Self::Apply(target, activation) => {
                let _ = activation;
                &target.target_host_id
            }
        }
    }
}

pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/app-providers/gemini/{id}/config", put(save))
        .route("/app-providers/gemini/{id}/form/read", post(read_form))
        .route("/app-providers/gemini/{id}/form", post(write_form).patch(write_form))
        .route("/app-providers/gemini/{id}/switch", post(switch))
        .route("/app-providers/gemini/{id}/reapply", post(reapply))
        .layer(DefaultBodyLimit::max(32 * 1024))
        .layer(Extension(Policy(path)))
}
async fn save(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<GeminiWrite>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    respond(
        tokio::task::spawn_blocking(move || {
            execute(policy, tenant, db, headers, id, Request::Save(request))
        })
        .await,
    )
}
async fn switch(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<GeminiTarget>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    respond(
        tokio::task::spawn_blocking(move || {
            execute(
                policy,
                tenant,
                db,
                headers,
                id,
                Request::Apply(request, GeminiActivation::Switch),
            )
        })
        .await,
    )
}
async fn reapply(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<GeminiTarget>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    respond(
        tokio::task::spawn_blocking(move || {
            execute(
                policy,
                tenant,
                db,
                headers,
                id,
                Request::Apply(request, GeminiActivation::IfCurrent),
            )
        })
        .await,
    )
}
fn respond<T: serde::Serialize>(
    result: Result<Result<T, StatusCode>, tokio::task::JoinError>,
) -> Response {
    match result {
        Ok(Ok(r)) => Json(r).into_response(),
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
    request: Request,
) -> Result<GeminiResult, StatusCode> {
    let _ = request.target();
    Err(StatusCode::NOT_IMPLEMENTED)
}
#[cfg(target_os = "macos")]
fn execute(
    policy: Policy,
    tenant: TenantScope,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    id: String,
    request: Request,
) -> Result<GeminiResult, StatusCode> {
    use vw_agent_apps::providers::{
        gemini::GeminiFiles,
        live::{commit::*, write_guard::WriteError},
    };
    let host = request.target();
    let (home, root) = super::super::agent_apps::provider_editor_context(
        policy.0.as_deref(),
        &headers,
        &tenant,
        host,
        true,
    )?;
    vw_agent_apps::providers::validate_identity(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let store = store(db).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let before = store.gemini_record(&tenant, &id).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if before.as_ref().is_some_and(managed) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let catalog_write = !matches!(request, Request::Apply(..));
    let (profile, activation, metadata) = match &request {
        Request::Save(body) => (body.profile.clone(), body.activation, None),
        Request::Form { body, create } => {
            if *create && before.is_some() {
                return Err(StatusCode::CONFLICT);
            }
            if !create && before.is_none() {
                return Err(StatusCode::NOT_FOUND);
            }
            let profile = vw_agent_apps::providers::gemini::form::profile(body, before.as_ref())
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            let metadata = vw_agent_apps::providers::basic::apply(
                AppId::Gemini,
                before.as_ref().unwrap_or(&serde_json::json!({})),
                &body.fields,
                &vw_api_types::app_providers::create::ProviderSecretWrite::Keep,
            )
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            let activation = match (body.apply, create) {
                (false, _) => GeminiActivation::CatalogOnly,
                (true, true) => GeminiActivation::First,
                (true, false) => GeminiActivation::IfCurrent,
            };
            (profile, activation, Some(metadata))
        }
        Request::Apply(_, activation) => {
            let profile = serde_json::from_value(
                before
                    .as_ref()
                    .and_then(|r| r.get("gemini"))
                    .cloned()
                    .ok_or(StatusCode::NOT_FOUND)?,
            )
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            (profile, *activation, None)
        }
    };
    if profile.name.trim().is_empty()
        || profile.name.len() > 128
        || profile.name.chars().any(char::is_control)
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    vw_agent_apps::providers::gemini::settings(&profile)
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    // Catalog-only requests do not provision host directories or transaction state.
    let mut live = None;
    if activation != GeminiActivation::CatalogOnly {
        let mut files = GeminiFiles::open(&home).map_err(|_| StatusCode::CONFLICT)?;
        let tx = ProviderTransactions::new(
            &root,
            ExecutionHostId::new(host).map_err(|_| StatusCode::BAD_REQUEST)?,
        )
        .map_err(|_| StatusCode::CONFLICT)?;
        if tx.recover(&mut files.guard).map_err(|_| StatusCode::CONFLICT)?.is_some() {
            return Ok(GeminiResult {
                catalog_saved: false,
                live: GeminiLiveResult::RecoveryRequired,
            });
        }
        let state = tx.state(AppId::Gemini).map_err(|_| StatusCode::CONFLICT)?;
        if state.pointer.as_ref().is_some_and(|p| p.tenant != tenant)
            || state.mode.as_ref().is_some_and(|m| m.attached || m.mode != Mode::Direct)
        {
            return Err(StatusCode::CONFLICT);
        }
        let eligible = match activation {
            GeminiActivation::First => state.pointer.is_none(),
            GeminiActivation::IfCurrent => state.pointer.as_ref().is_some_and(|p| p.id == id),
            _ => true,
        };
        if eligible {
            files.preflight(&profile).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            live = Some((files, tx, state));
        }
    }
    let previous = live
        .as_ref()
        .and_then(|(_, _, state)| state.pointer.as_ref())
        .map(|p| store.gemini_record(&tenant, &p.id))
        .transpose()
        .map_err(|_| StatusCode::CONFLICT)?
        .flatten();
    if previous.as_ref().is_some_and(managed) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    if live.as_ref().is_some_and(|(_, _, state)| state.pointer.is_some()) && previous.is_none() {
        return Err(StatusCode::CONFLICT);
    }
    let saved = if catalog_write {
        store
            .gemini_save_with_metadata(&tenant, &id, &profile, before.as_ref(), metadata)
            .map_err(|_| StatusCode::CONFLICT)?
    } else {
        before.ok_or(StatusCode::NOT_FOUND)?
    };
    let Some((mut files, tx, state)) = live else {
        return Ok(GeminiResult {
            catalog_saved: catalog_write,
            live: GeminiLiveResult::NotAttempted,
        });
    };
    let unchanged = || {
        store.gemini_record(&tenant, &id).ok().flatten().as_ref() == Some(&saved)
            && state
                .pointer
                .as_ref()
                .filter(|p| p.id != id)
                .is_none_or(|p| store.gemini_record(&tenant, &p.id).ok().flatten() == previous)
    };
    let result = files.apply(&tx, &tenant, &state, &id, &profile, unchanged);
    let live = match result {
        Ok(()) => GeminiLiveResult::Applied,
        Err(WriteError::Recovered) => GeminiLiveResult::RecoveryRequired,
        Err(_) => {
            if tx.pending(AppId::Gemini).ok().flatten().is_some() {
                GeminiLiveResult::RecoveryRequired
            } else {
                GeminiLiveResult::Failed
            }
        }
    };
    Ok(GeminiResult { catalog_saved: catalog_write, live })
}

#[cfg(target_os = "macos")]
fn managed(row: &serde_json::Value) -> bool {
    row.pointer("/meta/authBinding").is_some()
        || row.pointer("/meta/liveConfigManaged").is_some_and(|v| v == true)
}

async fn read_form(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<GeminiTarget>, JsonRejection>,
) -> Response {
    let Ok(Json(req)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    respond(
        tokio::task::spawn_blocking(move || -> Result<GeminiFormView, StatusCode> {
            #[cfg(target_os = "macos")]
            {
                super::super::agent_apps::provider_editor_context(
                    policy.0.as_deref(),
                    &headers,
                    &tenant,
                    &req.target_host_id,
                    false,
                )?;
                vw_agent_apps::providers::validate_identity(&id)
                    .map_err(|_| StatusCode::BAD_REQUEST)?;
                let store = store(db).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
                let row = store
                    .gemini_record(&tenant, &id)
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
                    .ok_or(StatusCode::NOT_FOUND)?;
                vw_agent_apps::providers::gemini::form::view(&row, managed(&row))
                    .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = (policy, tenant, db, headers, id, req);
                Err(StatusCode::NOT_IMPLEMENTED)
            }
        })
        .await,
    )
}

async fn write_form(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    method: Method,
    body: Result<Json<GeminiFormWrite>, JsonRejection>,
) -> Response {
    let Ok(Json(body)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    let create = method == Method::POST;
    respond(
        tokio::task::spawn_blocking(move || {
            execute(policy, tenant, db, headers, id, Request::Form { body, create })
        })
        .await,
    )
}
