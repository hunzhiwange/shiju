//! Authorized fixed Grok save/switch operations. No arbitrary command or file API.
use super::*;
use axum::{
    http::{HeaderMap, Method},
    routing::{post, put},
};
use std::path::PathBuf;
use vw_api_types::app_providers::grok::*;
#[derive(Clone)]
struct Policy(Option<PathBuf>);

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
enum Write {
    Config(GrokWrite),
    Form(GrokFormWrite, bool),
}

#[cfg(target_os = "macos")]
impl Write {
    fn target(&self) -> (&str, GrokActivation) {
        match self {
            Self::Config(body) => (&body.target_host_id, body.activation),
            Self::Form(body, create) => {
                let activation = match (body.apply, create) {
                    (true, true) => GrokActivation::First,
                    (true, false) => GrokActivation::IfCurrent,
                    (false, _) => GrokActivation::CatalogOnly,
                };
                (&body.target_host_id, activation)
            }
        }
    }
}
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/app-providers/grokbuild/{id}/config", put(save))
        .route("/app-providers/grokbuild/{id}/switch", post(switch))
        .route("/app-providers/grokbuild/{id}/reapply", post(reapply))
        .route("/app-providers/grokbuild/{id}/form/read", post(read_form))
        .route("/app-providers/grokbuild/{id}/form", post(write_form).patch(write_form))
        .layer(DefaultBodyLimit::max(32 * 1024))
        .layer(Extension(Policy(path)))
}
async fn save(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<GrokWrite>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    respond(
        tokio::task::spawn_blocking(move || {
            execute(policy, tenant, db, headers, id, Some(Write::Config(request)), None)
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
    body: Result<Json<GrokTarget>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    respond(
        tokio::task::spawn_blocking(move || {
            execute(policy, tenant, db, headers, id, None, Some((request, GrokActivation::Switch)))
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
    body: Result<Json<GrokTarget>, JsonRejection>,
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
                None,
                Some((request, GrokActivation::IfCurrent)),
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
    _: Option<Write>,
    _: Option<(GrokTarget, GrokActivation)>,
) -> Result<GrokResult, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}
#[cfg(target_os = "macos")]
fn execute(
    policy: Policy,
    tenant: TenantScope,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    id: String,
    write: Option<Write>,
    operation: Option<(GrokTarget, GrokActivation)>,
) -> Result<GrokResult, StatusCode> {
    use vw_agent_apps::providers::{
        grok::{GrokFiles, form},
        live::{commit::*, write_guard::WriteError},
    };
    let (host, activation) = match (&write, &operation) {
        (Some(w), _) => w.target(),
        (_, Some((t, a))) => (t.target_host_id.as_str(), *a),
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    let (home, root) = super::super::agent_apps::provider_editor_context(
        policy.0.as_deref(),
        &headers,
        &tenant,
        host,
        true,
    )?;
    let store = open_store(db, &id)?;
    let before = store.grok_record(&tenant, &id).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if before.as_ref().is_some_and(managed) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let mut metadata = None;
    let profile = match &write {
        Some(Write::Config(body)) => body.profile.clone(),
        Some(Write::Form(body, create)) => {
            match (*create, before.is_some()) {
                (true, true) => return Err(StatusCode::CONFLICT),
                (false, false) => return Err(StatusCode::NOT_FOUND),
                _ => {}
            }
            metadata = Some(
                vw_agent_apps::providers::basic::apply(
                    AppId::GrokBuild,
                    before.as_ref().unwrap_or(&serde_json::json!({})),
                    &body.fields,
                    &vw_api_types::app_providers::create::ProviderSecretWrite::Keep,
                )
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?,
            );
            form::prepare(&id, body, before.as_ref())
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?
        }
        None => serde_json::from_value(
            before.as_ref().and_then(|r| r.get("grok")).cloned().ok_or(StatusCode::NOT_FOUND)?,
        )
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?,
    };
    if profile.name.trim().is_empty()
        || profile.name.len() > 128
        || profile.name.chars().any(char::is_control)
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    vw_agent_apps::providers::grok::settings(&profile)
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    // Catalog-only requests do not provision host directories or transaction state.
    let mut live = None;
    if activation != GrokActivation::CatalogOnly {
        let mut files = GrokFiles::open(&home).map_err(|_| StatusCode::CONFLICT)?;
        let tx = ProviderTransactions::new(
            &root,
            ExecutionHostId::new(host).map_err(|_| StatusCode::BAD_REQUEST)?,
        )
        .map_err(|_| StatusCode::CONFLICT)?;
        if tx.recover(&mut files.guard).map_err(|_| StatusCode::CONFLICT)?.is_some() {
            return Ok(GrokResult { catalog_saved: false, live: GrokLiveResult::RecoveryRequired });
        }
        let state = tx.state(AppId::GrokBuild).map_err(|_| StatusCode::CONFLICT)?;
        if state.mode_tenant.as_ref().is_some_and(|t| t != &tenant)
            || state.pointer.as_ref().is_some_and(|p| p.tenant != tenant)
            || state.mode.as_ref().is_some_and(|m| m.attached || m.mode != Mode::Direct)
        {
            return Err(StatusCode::CONFLICT);
        }
        let eligible = match activation {
            GrokActivation::First => state.pointer.is_none(),
            GrokActivation::IfCurrent => state.pointer.as_ref().is_some_and(|p| p.id == id),
            _ => true,
        };
        if eligible {
            files.preflight(&profile, &state).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            live = Some((files, tx, state));
        }
    }
    let previous = live
        .as_ref()
        .and_then(|(_, _, state)| state.pointer.as_ref())
        .map(|p| store.grok_record(&tenant, &p.id))
        .transpose()
        .map_err(|_| StatusCode::CONFLICT)?
        .flatten();
    if previous.as_ref().is_some_and(managed) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    if live.as_ref().is_some_and(|(_, _, state)| state.pointer.is_some()) && previous.is_none() {
        return Err(StatusCode::CONFLICT);
    }
    let saved = if write.is_some() {
        store
            .grok_save(&tenant, &id, &profile, before.as_ref(), metadata)
            .map_err(|_| StatusCode::CONFLICT)?
    } else {
        before.ok_or(StatusCode::NOT_FOUND)?
    };
    let Some((mut files, tx, state)) = live else {
        return Ok(GrokResult {
            catalog_saved: write.is_some(),
            live: GrokLiveResult::NotAttempted,
        });
    };
    let unchanged = || {
        store.grok_record(&tenant, &id).ok().flatten().as_ref() == Some(&saved)
            && state
                .pointer
                .as_ref()
                .filter(|p| p.id != id)
                .is_none_or(|p| store.grok_record(&tenant, &p.id).ok().flatten() == previous)
    };
    let result = files.apply(&tx, &tenant, &state, &id, &profile, unchanged);
    let live = match result {
        Ok(()) => GrokLiveResult::Applied,
        Err(WriteError::Recovered) => GrokLiveResult::RecoveryRequired,
        Err(_) => {
            if tx.pending(AppId::GrokBuild).ok().flatten().is_some() {
                GrokLiveResult::RecoveryRequired
            } else {
                GrokLiveResult::Failed
            }
        }
    };
    Ok(GrokResult { catalog_saved: write.is_some(), live })
}

#[cfg(target_os = "macos")]
fn managed(row: &serde_json::Value) -> bool {
    row.pointer("/meta/authBinding").is_some()
        || row.pointer("/meta/liveConfigManaged").is_some_and(|v| v == true)
}

#[cfg(target_os = "macos")]
fn open_store(
    db: Option<Extension<GlobalDatabase>>,
    id: &str,
) -> Result<AppProviderStore, StatusCode> {
    vw_agent_apps::providers::validate_identity(id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let database = match db {
        Some(Extension(db)) => db,
        None => GlobalDatabase::for_home(&crate::global::paths().home)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
    };
    AppProviderStore::new(database).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}

async fn read_form(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<GrokTarget>, JsonRejection>,
) -> Response {
    let Ok(Json(target)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    respond(
        tokio::task::spawn_blocking(move || -> Result<GrokFormView, StatusCode> {
            #[cfg(target_os = "macos")]
            {
                super::super::agent_apps::provider_editor_context(
                    policy.0.as_deref(),
                    &headers,
                    &tenant,
                    &target.target_host_id,
                    false,
                )?;
                let row = open_store(db, &id)?
                    .grok_record(&tenant, &id)
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
                    .ok_or(StatusCode::NOT_FOUND)?;
                vw_agent_apps::providers::grok::form::view(&id, &row, managed(&row))
                    .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = (policy, tenant, db, headers, id, target);
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
    body: Result<Json<GrokFormWrite>, JsonRejection>,
) -> Response {
    let Ok(Json(body)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    respond(
        tokio::task::spawn_blocking(move || {
            execute(
                policy,
                tenant,
                db,
                headers,
                id,
                Some(Write::Form(body, method == Method::POST)),
                None,
            )
        })
        .await,
    )
}
