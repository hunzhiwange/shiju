//! Bounded, expiring executor-only baselines. Restart invalidates contexts; no secret baseline on wire.
use crate::global_database::GlobalDatabase;
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    routing::post,
};
use serde_json::Value;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
#[cfg(target_os = "macos")]
use vw_agent_apps::providers::{
    editor_merge as business,
    live::commit::{AppState, ExecutionHostId, ProviderTransactions},
};
use vw_api_types::{agent_apps::AppId, app_providers::editor_merge::*, id::TenantScope};
#[derive(Clone)]
pub(super) struct Editor {
    policy: Option<PathBuf>,
    contexts: Arc<Mutex<HashMap<String, Context>>>,
}
struct Context {
    tenant: TenantScope,
    app: AppId,
    id: String,
    host: String,
    home: PathBuf,
    root: PathBuf,
    record: Value,
    base: Value,
    created: Instant,
    #[cfg(target_os = "macos")]
    state: AppState,
    current: bool,
    identity: Option<(u64, u64)>,
    resolution: Option<(String, Vec<u8>, Vec<u8>)>,
}
impl Editor {
    pub(super) fn new(policy: Option<PathBuf>) -> Self {
        Self { policy, contexts: Default::default() }
    }

    pub(super) fn router(self) -> Router {
        Router::new()
            .route("/app-providers/{app}/{id}/editor/open", post(open))
            .route("/app-providers/{app}/{id}/editor/save", post(save))
            .layer(DefaultBodyLimit::max(128 * 1024))
            .layer(Extension(self))
    }
}
#[cfg(test)]
pub(super) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    Editor::new(policy).router()
}
async fn open(
    Extension(editor): Extension<Editor>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path((app, id)): Path<(AppId, String)>,
    body: Result<Json<EditorOpen>, JsonRejection>,
) -> Result<Json<EditorView>, StatusCode> {
    let Json(req) = body.map_err(|_| StatusCode::BAD_REQUEST)?;
    tokio::task::spawn_blocking(move || open_sync(editor, tenant, db, headers, app, id, req))
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
async fn save(
    Extension(editor): Extension<Editor>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path((app, id)): Path<(AppId, String)>,
    body: Result<Json<EditorSave>, JsonRejection>,
) -> Result<Json<EditorResult>, StatusCode> {
    let Json(req) = body.map_err(|_| StatusCode::BAD_REQUEST)?;
    tokio::task::spawn_blocking(move || save_sync(editor, tenant, db, headers, app, id, req))
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
#[cfg(target_os = "macos")]
fn observation(
    root: &std::path::Path,
    host: &str,
    app: AppId,
) -> Result<(Option<ProviderTransactions>, AppState), StatusCode> {
    let tx = ProviderTransactions::open_existing(
        root,
        ExecutionHostId::new(host).map_err(|_| StatusCode::BAD_REQUEST)?,
    )
    .map_err(|_| StatusCode::CONFLICT)?;
    let state = if let Some(tx) = &tx {
        if tx.pending(app).map_err(|_| StatusCode::CONFLICT)?.is_some() {
            return Err(StatusCode::CONFLICT);
        }
        tx.state(app).map_err(|_| StatusCode::CONFLICT)?
    } else {
        AppState::default()
    };
    Ok((tx, state))
}
#[cfg(target_os = "macos")]
fn database(db: Option<Extension<GlobalDatabase>>) -> Result<GlobalDatabase, StatusCode> {
    match db {
        Some(Extension(db)) => Ok(db),
        None => GlobalDatabase::for_home(&crate::global::paths().home)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE),
    }
}
#[cfg(target_os = "macos")]
fn open_sync(
    editor: Editor,
    tenant: TenantScope,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    app: AppId,
    id: String,
    req: EditorOpen,
) -> Result<Json<EditorView>, StatusCode> {
    let (home, root) = super::super::agent_apps::provider_editor_context(
        editor.policy.as_deref(),
        &headers,
        &tenant,
        &req.target_host_id,
        false,
    )?;
    let db = database(db)?;
    if app != AppId::Claude {
        return Err(StatusCode::NOT_IMPLEMENTED);
    }
    let record = crate::app_providers::AppProviderStore::preview_record(&db, &tenant, app, &id)
        .map_err(|_| StatusCode::NOT_FOUND)?;
    if record.get("category").and_then(Value::as_str) == Some("official")
        || record
            .get("meta")
            .is_some_and(|v| !v.is_null() && v.as_object().is_none_or(|m| !m.is_empty()))
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let row = record
        .get("settingsConfig")
        .filter(|v| v.is_object())
        .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
    let (_, state) = observation(&root, &req.target_host_id, app)?;
    if state.mode.as_ref().is_some_and(|m| m.attached)
        || state.pointer.as_ref().is_some_and(|p| p.tenant != tenant)
    {
        return Err(StatusCode::CONFLICT);
    }
    let current = state.pointer.as_ref().is_some_and(|p| p.id == id);
    let (base, identity) = if current {
        let file =
            business::ClaudeFile::open(&home).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        (
            business::project(&file.bytes, row).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?,
            Some(file.identity),
        )
    } else {
        (row.clone(), None)
    };
    let token = uuid::Uuid::new_v4().to_string();
    let view =
        EditorView { context: token.clone(), settings: business::display(base.clone()), current };
    let mut contexts = editor.contexts.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    contexts.retain(|_, v| v.created.elapsed() < Duration::from_secs(900));
    if contexts.len() >= 128 {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    contexts.insert(
        token,
        Context {
            tenant,
            app,
            id,
            host: req.target_host_id,
            home,
            root,
            record,
            base,
            created: Instant::now(),
            state,
            current,
            identity,
            resolution: None,
        },
    );
    Ok(Json(view))
}
#[cfg(target_os = "macos")]
pub(super) fn save_sync(
    editor: Editor,
    tenant: TenantScope,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    app: AppId,
    id: String,
    req: EditorSave,
) -> Result<Json<EditorResult>, StatusCode> {
    let (home, root) = super::super::agent_apps::provider_editor_context(
        editor.policy.as_deref(),
        &headers,
        &tenant,
        &req.target_host_id,
        true,
    )?;
    let db = database(db)?;
    let mut contexts = editor.contexts.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let ctx = contexts.get_mut(&req.context).ok_or(StatusCode::CONFLICT)?;
    if ctx.created.elapsed() >= Duration::from_secs(900)
        || ctx.tenant != tenant
        || ctx.app != app
        || ctx.id != id
        || ctx.host != req.target_host_id
        || ctx.home != home
        || ctx.root != root
    {
        return Err(StatusCode::CONFLICT);
    }
    let (tx, state) = observation(&root, &req.target_host_id, app)?;
    if state != ctx.state {
        return Err(StatusCode::CONFLICT);
    }
    let record = crate::app_providers::AppProviderStore::preview_record(&db, &tenant, app, &id)
        .map_err(|_| StatusCode::CONFLICT)?;
    if record != ctx.record {
        return Err(StatusCode::CONFLICT);
    }
    let mut file = if ctx.current {
        Some(business::ClaudeFile::open(&home).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?)
    } else {
        None
    };
    if file.as_ref().map(|f| f.identity) != ctx.identity {
        return Err(StatusCode::CONFLICT);
    }
    let bytes = if let Some(f) = &file {
        f.bytes.clone()
    } else {
        serde_json::to_vec(&ctx.base).map_err(|_| StatusCode::BAD_REQUEST)?
    };
    let draft = serde_json::to_vec(&req.edits).map_err(|_| StatusCode::BAD_REQUEST)?;
    let approved = ctx.resolution.as_ref().is_some_and(|(token, old, edits)| {
        Some(token) == req.resolution.as_ref() && *old == bytes && *edits == draft
    });
    let choice = if approved { req.choice } else { EditorChoice::Refuse };
    let merged = match business::merge(&ctx.base, &bytes, &req.edits, choice, ctx.current) {
        Ok(v) => v,
        Err(business::MergeError::Invalid) => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        Err(business::MergeError::Conflict(conflicts)) => {
            let token = uuid::Uuid::new_v4().to_string();
            ctx.resolution = Some((token.clone(), bytes, draft));
            return Ok(Json(EditorResult { saved: false, conflicts, resolution: Some(token) }));
        }
    };
    if req.choice != EditorChoice::Refuse && !approved {
        return Err(StatusCode::CONFLICT);
    }
    if let Some(f) = &mut file {
        if merged != bytes {
            f.commit(
                tx.as_ref().ok_or(StatusCode::CONFLICT)?,
                &tenant,
                &ctx.state,
                &ctx.base,
                &req.edits,
                choice,
            )
            .map_err(|_| StatusCode::CONFLICT)?;
        }
    } else {
        let settings = serde_json::from_slice(&merged).map_err(|_| StatusCode::BAD_REQUEST)?;
        business::project(&merged, &settings).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        let store = crate::app_providers::AppProviderStore::new(db)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        vw_agent_apps::providers::operations::publication().map_err(|_| StatusCode::CONFLICT)?;
        store
            .editor_save_record(&tenant, app, &id, &ctx.record, settings)
            .map_err(|_| StatusCode::CONFLICT)?;
    }
    contexts.remove(&req.context);
    Ok(Json(EditorResult { saved: true, conflicts: vec![], resolution: None }))
}
#[cfg(not(target_os = "macos"))]
fn open_sync(
    _: Editor,
    _: TenantScope,
    _: Option<Extension<GlobalDatabase>>,
    _: HeaderMap,
    _: AppId,
    _: String,
    _: EditorOpen,
) -> Result<Json<EditorView>, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}
#[cfg(not(target_os = "macos"))]
pub(super) fn save_sync(
    _: Editor,
    _: TenantScope,
    _: Option<Extension<GlobalDatabase>>,
    _: HeaderMap,
    _: AppId,
    _: String,
    _: EditorSave,
) -> Result<Json<EditorResult>, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}
