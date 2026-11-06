//! Authorized fixed Codex save/switch operations. No arbitrary command or file API.
use super::*;
use axum::{
    http::HeaderMap,
    routing::{post, put},
};
use std::path::PathBuf;
use vw_api_types::app_providers::{claude::ClaudeActivation, codex_catalog::*};
#[derive(Clone)]
struct Policy(Option<PathBuf>);
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/app-providers/codex/{id}/config", put(save))
        .route("/app-providers/codex/{id}/switch", post(switch))
        .route("/app-providers/codex/{id}/reapply", post(reapply))
        .layer(DefaultBodyLimit::max(512 * 1024))
        .layer(Extension(Policy(path)))
}
async fn save(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<CodexWrite>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    respond(
        tokio::task::spawn_blocking(move || {
            execute(policy, tenant, db, headers, id, Some(request), None)
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
    body: Result<Json<CodexTarget>, JsonRejection>,
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
                Some((request, ClaudeActivation::Switch)),
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
    body: Result<Json<CodexTarget>, JsonRejection>,
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
                Some((request, ClaudeActivation::IfCurrent)),
            )
        })
        .await,
    )
}
fn respond(result: Result<Result<CodexResult, StatusCode>, tokio::task::JoinError>) -> Response {
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
    _: Option<CodexWrite>,
    _: Option<(CodexTarget, ClaudeActivation)>,
) -> Result<CodexResult, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}
#[cfg(target_os = "macos")]
fn execute(
    policy: Policy,
    tenant: TenantScope,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    id: String,
    write: Option<CodexWrite>,
    operation: Option<(CodexTarget, ClaudeActivation)>,
) -> Result<CodexResult, StatusCode> {
    use vw_agent_apps::providers::{
        codex_catalog::{CodexFiles, identity},
        live::{commit::*, write_guard::WriteError},
    };
    let (host, activation) = match (&write, &operation) {
        (Some(w), _) => (&w.target_host_id, w.activation),
        (_, Some((t, a))) => (&t.target_host_id, *a),
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    let (home, root) = super::super::agent_apps::provider_editor_context(
        policy.0.as_deref(),
        &headers,
        &tenant,
        host,
        true,
    )?;
    let database = match db {
        Some(Extension(db)) => db,
        None => GlobalDatabase::for_home(&crate::global::paths().home)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
    };
    let models = crate::model_catalog::ModelCatalogStore::new(database.clone())
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let store = AppProviderStore::new(database).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let before = store.codex_record(&tenant, &id).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if before.as_ref().is_some_and(managed) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let profile = match &write {
        Some(w) => w.profile.clone(),
        None => serde_json::from_value(
            before.as_ref().and_then(|r| r.get("codex")).cloned().ok_or(StatusCode::NOT_FOUND)?,
        )
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?,
    };
    if profile.name.trim().is_empty()
        || profile.name.len() > 128
        || profile.name.chars().any(char::is_control)
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let identity = identity(host, &id, &profile).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    if let Some(w) = &write {
        vw_agent_apps::models::validate(&identity, w.catalog.as_ref())
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    }
    // Catalog-only requests do not provision host directories or transaction state.
    let mut live = None;
    if activation != ClaudeActivation::CatalogOnly {
        let mut files = CodexFiles::open(&home).map_err(|_| StatusCode::CONFLICT)?;
        let tx = ProviderTransactions::new(
            &root,
            ExecutionHostId::new(host).map_err(|_| StatusCode::BAD_REQUEST)?,
        )
        .map_err(|_| StatusCode::CONFLICT)?;
        if tx.recover(&mut files.guard).map_err(|_| StatusCode::CONFLICT)?.is_some() {
            return Ok(CodexResult {
                catalog_saved: false,
                live: CodexLiveResult::RecoveryRequired,
            });
        }
        let state = tx.state(AppId::Codex).map_err(|_| StatusCode::CONFLICT)?;
        if state.pointer.as_ref().is_some_and(|p| p.tenant != tenant)
            || state.mode.as_ref().is_some_and(|m| m.attached || m.mode != Mode::Direct)
        {
            return Err(StatusCode::CONFLICT);
        }
        let eligible = match activation {
            ClaudeActivation::First => state.pointer.is_none(),
            ClaudeActivation::IfCurrent => state.pointer.as_ref().is_some_and(|p| p.id == id),
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
        .map(|p| store.codex_record(&tenant, &p.id))
        .transpose()
        .map_err(|_| StatusCode::CONFLICT)?
        .flatten();
    if previous.as_ref().is_some_and(managed) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    if live.as_ref().is_some_and(|(_, _, state)| state.pointer.is_some()) && previous.is_none() {
        return Err(StatusCode::CONFLICT);
    }
    let saved = if let Some(w) = &write {
        // Model data remains in the sole model domain owner, scoped to this exact deployment.
        let revision = models
            .save(&tenant, &identity, w.catalog.as_ref())
            .map_err(|_| StatusCode::CONFLICT)?;
        store
            .codex_save(&tenant, &id, &profile, before.as_ref(), revision.as_deref())
            .map_err(|_| StatusCode::CONFLICT)?
    } else {
        before.ok_or(StatusCode::NOT_FOUND)?
    };
    let revision = saved.get("modelCatalogRevision").and_then(serde_json::Value::as_str);
    let catalog =
        models.read(&tenant, &identity, revision).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let Some((mut files, tx, state)) = live else {
        return Ok(CodexResult {
            catalog_saved: write.is_some(),
            live: CodexLiveResult::NotAttempted,
        });
    };
    let unchanged = || {
        store.codex_record(&tenant, &id).ok().flatten().as_ref() == Some(&saved)
            && models.read(&tenant, &identity, revision).ok().as_ref() == Some(&catalog)
            && state
                .pointer
                .as_ref()
                .filter(|p| p.id != id)
                .is_none_or(|p| store.codex_record(&tenant, &p.id).ok().flatten() == previous)
    };
    let result = files.apply(
        &tx,
        &tenant,
        &state,
        &id,
        previous.as_ref().and_then(|v| v.get("settingsConfig")),
        &profile,
        catalog.as_ref(),
        unchanged,
    );
    let live = match result {
        Ok(()) => CodexLiveResult::Applied,
        Err(WriteError::Recovered) => CodexLiveResult::RecoveryRequired,
        Err(_) => {
            if tx.pending(AppId::Codex).ok().flatten().is_some() {
                CodexLiveResult::RecoveryRequired
            } else {
                CodexLiveResult::Failed
            }
        }
    };
    Ok(CodexResult { catalog_saved: write.is_some(), live })
}

#[cfg(target_os = "macos")]
fn managed(row: &serde_json::Value) -> bool {
    row.pointer("/meta/authBinding").is_some()
        || row.pointer("/meta/codexAccountId").is_some()
        || row.pointer("/meta/liveConfigManaged").is_some_and(|v| v == true)
}
