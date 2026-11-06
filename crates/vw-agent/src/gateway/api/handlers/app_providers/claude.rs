//! Authenticated Claude config operations, sharing the established catalog and host transaction.
use super::*;
use axum::{http::HeaderMap, routing::post};
use std::path::PathBuf;
use vw_api_types::app_providers::claude::*;
#[derive(Clone)]
pub(super) struct Policy(pub(super) Option<PathBuf>);
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    Router::new()
        .route("/app-providers/claude/{id}/switch", post(switch))
        .route("/app-providers/claude/{id}/reapply", post(reapply))
        .route("/app-providers/claude/{id}/import", post(import))
        .route("/universal-providers/{id}/sync/claude", post(sync))
        .layer(DefaultBodyLimit::max(32 * 1024))
        .layer(Extension(Policy(policy)))
}
#[derive(Clone, Copy)]
pub(super) enum WriteKind {
    Create,
    Save,
    Update,
}
pub(super) async fn write(
    tenant: TenantScope,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    policy: Option<Extension<Policy>>,
    id: String,
    req: ClaudeWrite,
    kind: WriteKind,
) -> Response {
    let policy = policy
        .map(|Extension(p)| p.0)
        .unwrap_or_else(|| std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from));
    write_with_policy(tenant, db, headers, id, req, kind, policy).await
}
pub(super) async fn write_with_policy(
    tenant: TenantScope,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    id: String,
    req: ClaudeWrite,
    kind: WriteKind,
    policy: Option<PathBuf>,
) -> Response {
    let status =
        if matches!(kind, WriteKind::Create) { StatusCode::CREATED } else { StatusCode::OK };
    match tokio::task::spawn_blocking(move || -> Result<ClaudeResult, StatusCode> {
        write_sync(tenant, db, headers, id, req, kind, policy)
    })
    .await
    {
        Ok(Ok(result)) => (status, Json(result)).into_response(),
        Ok(Err(status)) => status.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
#[cfg(target_os = "macos")]
fn write_sync(
    tenant: TenantScope,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    id: String,
    req: ClaudeWrite,
    kind: WriteKind,
    policy: Option<PathBuf>,
) -> Result<ClaudeResult, StatusCode> {
    let (home, root) = super::super::agent_apps::provider_editor_context(
        policy.as_deref(),
        &headers,
        &tenant,
        &req.target_host_id,
        true,
    )?;
    let store = store(db).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    vw_agent_apps::providers::claude::settings(&req.claude)
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    let before = store.claude_record(&tenant, &id).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if matches!(kind, WriteKind::Update) && before.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }
    if matches!(kind, WriteKind::Create) && before.is_some() {
        return Err(StatusCode::CONFLICT);
    }
    let host = Host::open(&home, &root, &req.target_host_id, &tenant, req.activation, &store, &id)?;
    let row = store
        .claude_save(&tenant, &id, &req.claude, before.as_ref(), matches!(kind, WriteKind::Create))
        .map_err(|_| StatusCode::CONFLICT)?;
    let live = if let Some(host) = host {
        host.apply(&store, &tenant, &id, &row, req.activation)
    } else {
        ClaudeLiveResult::NotAttempted
    };
    Ok(ClaudeResult { catalog_saved: true, live })
}
#[cfg(not(target_os = "macos"))]
fn write_sync(
    _: TenantScope,
    _: Option<Extension<GlobalDatabase>>,
    _: HeaderMap,
    _: String,
    _: ClaudeWrite,
    _: WriteKind,
    _: Option<PathBuf>,
) -> Result<ClaudeResult, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}
#[cfg(target_os = "macos")]
struct Host {
    file: vw_agent_apps::providers::claude::ClaudeFile,
    tx: vw_agent_apps::providers::live::commit::ProviderTransactions,
    state: vw_agent_apps::providers::live::commit::AppState,
    previous: Option<serde_json::Value>,
}
#[cfg(target_os = "macos")]
impl Host {
    fn open(
        home: &std::path::Path,
        root: &std::path::Path,
        host: &str,
        tenant: &TenantScope,
        activation: ClaudeActivation,
        store: &AppProviderStore,
        id: &str,
    ) -> Result<Option<Self>, StatusCode> {
        use vw_agent_apps::providers::live::commit::*;
        if activation == ClaudeActivation::CatalogOnly {
            return Ok(None);
        }
        let observed = ProviderTransactions::open_existing(
            root,
            ExecutionHostId::new(host).map_err(|_| StatusCode::BAD_REQUEST)?,
        )
        .map_err(|_| StatusCode::CONFLICT)?;
        let observed_state = observed
            .as_ref()
            .map(|tx| tx.state(AppId::Claude))
            .transpose()
            .map_err(|_| StatusCode::CONFLICT)?
            .unwrap_or_default();
        if observed_state.mode.as_ref().is_some_and(|m| m.attached)
            || observed_state.pointer.as_ref().is_some_and(|p| p.tenant != *tenant)
            || observed
                .as_ref()
                .is_some_and(|tx| tx.pending(AppId::Claude).map_or(true, |p| p.is_some()))
        {
            return Err(StatusCode::CONFLICT);
        }
        let eligible = |state: &AppState| match activation {
            ClaudeActivation::First => state.pointer.is_none(),
            ClaudeActivation::IfCurrent => {
                state.pointer.as_ref().is_some_and(|p| p.tenant == *tenant && p.id == id)
            }
            _ => true,
        };
        if !eligible(&observed_state) {
            return Ok(None);
        }
        let file = vw_agent_apps::providers::claude::ClaudeFile::open(home, true)
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        let tx = ProviderTransactions::new(
            root,
            ExecutionHostId::new(host).map_err(|_| StatusCode::BAD_REQUEST)?,
        )
        .map_err(|_| StatusCode::CONFLICT)?;
        // commit recovers under its host lock; do not overwrite a pending intent with new catalog data.
        if tx.pending(AppId::Claude).map_err(|_| StatusCode::CONFLICT)?.is_some() {
            return Err(StatusCode::CONFLICT);
        }
        let state = tx.state(AppId::Claude).map_err(|_| StatusCode::CONFLICT)?;
        if state.mode.as_ref().is_some_and(|m| m.attached)
            || state.pointer.as_ref().is_some_and(|p| p.tenant != *tenant)
        {
            return Err(StatusCode::CONFLICT);
        }
        if !eligible(&state) {
            return Err(StatusCode::CONFLICT);
        }
        let previous = state
            .pointer
            .as_ref()
            .map(|p| store.claude_record(tenant, &p.id))
            .transpose()
            .map_err(|_| StatusCode::CONFLICT)?
            .flatten();
        if state.pointer.is_some() && previous.is_none() {
            return Err(StatusCode::CONFLICT);
        }
        Ok(Some(Self { file, tx, state, previous }))
    }
    fn apply(
        mut self,
        store: &AppProviderStore,
        tenant: &TenantScope,
        id: &str,
        row: &serde_json::Value,
        activation: ClaudeActivation,
    ) -> ClaudeLiveResult {
        let current = self.state.pointer.as_ref().map(|p| p.id.as_str());
        let should = match activation {
            ClaudeActivation::CatalogOnly => false,
            ClaudeActivation::First => current.is_none(),
            ClaudeActivation::IfCurrent => current == Some(id),
            ClaudeActivation::Switch => true,
        };
        if !should {
            return ClaudeLiveResult::NotAttempted;
        }
        let previous = self.previous.as_ref().and_then(|r| r.get("settingsConfig"));
        if store.claude_record(tenant, id).ok().flatten().as_ref() != Some(row) {
            return ClaudeLiveResult::Failed;
        }
        let Some(target) = row.get("settingsConfig") else {
            return ClaudeLiveResult::Failed;
        };
        if row.pointer("/meta/authBinding").is_some()
            || row.pointer("/meta/claudeAccountId").is_some()
        {
            return ClaudeLiveResult::Failed;
        }
        let pointer =
            if activation == ClaudeActivation::IfCurrent { None } else { Some(id.to_owned()) };
        let unchanged = || store.claude_record(tenant, id).ok().flatten().as_ref() == Some(row);
        match self.file.apply(&self.tx, tenant, &self.state, previous, target, pointer, unchanged) {
            Ok(()) => ClaudeLiveResult::Applied,
            Err(_) if self.tx.pending(AppId::Claude).ok().flatten().is_some() => {
                ClaudeLiveResult::RecoveryRequired
            }
            Err(_) => ClaudeLiveResult::Failed,
        }
    }
}
async fn switch(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<ClaudeTarget>, JsonRejection>,
) -> Response {
    operation(
        policy,
        tenant,
        db,
        headers,
        id,
        body.map(|Json(r)| r.target_host_id),
        ClaudeActivation::Switch,
    )
    .await
}
async fn reapply(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<ClaudeTarget>, JsonRejection>,
) -> Response {
    operation(
        policy,
        tenant,
        db,
        headers,
        id,
        body.map(|Json(r)| r.target_host_id),
        ClaudeActivation::IfCurrent,
    )
    .await
}
async fn operation(
    policy: Policy,
    tenant: TenantScope,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    id: String,
    host: Result<String, JsonRejection>,
    activation: ClaudeActivation,
) -> Response {
    let Ok(host) = host else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let result = tokio::task::spawn_blocking(move || -> Result<ClaudeResult, StatusCode> {
        #[cfg(target_os = "macos")]
        {
            let (home, root) = super::super::agent_apps::provider_editor_context(
                policy.0.as_deref(),
                &headers,
                &tenant,
                &host,
                true,
            )?;
            let store = store(db).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            let row = store
                .claude_record(&tenant, &id)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
                .ok_or(StatusCode::NOT_FOUND)?;
            let host = Host::open(&home, &root, &host, &tenant, activation, &store, &id)?
                .ok_or(StatusCode::CONFLICT)?;
            Ok(ClaudeResult {
                catalog_saved: false,
                live: host.apply(&store, &tenant, &id, &row, activation),
            })
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (policy, tenant, db, headers, id, host, activation);
            Err::<ClaudeResult, _>(StatusCode::NOT_IMPLEMENTED)
        }
    })
    .await;
    match result {
        Ok(Ok(r)) => Json(r).into_response(),
        Ok(Err(e)) => e.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
async fn import(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<ClaudeImport>, JsonRejection>,
) -> Response {
    let Ok(Json(req)) = body else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let result = tokio::task::spawn_blocking(move || -> Result<ClaudeResult, StatusCode> {
        #[cfg(target_os = "macos")]
        {
            let (home, _) = super::super::agent_apps::provider_editor_context(
                policy.0.as_deref(),
                &headers,
                &tenant,
                &req.target_host_id,
                true,
            )?;
            // A write grant alone must not expose native configuration for import.
            super::super::agent_apps::provider_editor_context(
                policy.0.as_deref(),
                &headers,
                &tenant,
                &req.target_host_id,
                false,
            )?;
            let file = vw_agent_apps::providers::claude::ClaudeFile::open(&home, false)
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            let profile = vw_agent_apps::providers::claude::import(
                &req.name,
                file.bytes.as_deref().ok_or(StatusCode::NOT_FOUND)?,
            )
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            let store = store(db).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            store
                .claude_save(&tenant, &id, &profile, None, true)
                .map_err(|_| StatusCode::CONFLICT)?;
            Ok(ClaudeResult { catalog_saved: true, live: ClaudeLiveResult::NotAttempted })
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (policy, tenant, db, headers, id, req);
            Err::<ClaudeResult, _>(StatusCode::NOT_IMPLEMENTED)
        }
    })
    .await;
    match result {
        Ok(Ok(r)) => Json(r).into_response(),
        Ok(Err(e)) => e.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
async fn sync(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<ClaudeSync>, JsonRejection>,
) -> Response {
    let Ok(Json(req)) = body else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let result = tokio::task::spawn_blocking(move || -> Result<ClaudeResult, StatusCode> {
        #[cfg(target_os = "macos")]
        {
            let (home, root) = super::super::agent_apps::provider_editor_context(
                policy.0.as_deref(),
                &headers,
                &tenant,
                &req.target_host_id,
                true,
            )?;
            let store = store(db).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            let child = vw_agent_apps::providers::universal_sync::child_id(AppId::Claude, &id)
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            let host = Host::open(
                &home,
                &root,
                &req.target_host_id,
                &tenant,
                req.activation,
                &store,
                &child,
            )?;
            let results = store
                .universal_sync(
                    &tenant,
                    &id,
                    &vw_api_types::app_providers::universal_sync::UniversalSyncRequest {
                        apps: vec![AppId::Claude],
                    },
                )
                .map_err(|_| StatusCode::CONFLICT)?;
            if !matches!(
                results[0].catalog,
                vw_api_types::app_providers::universal_sync::UniversalCatalogResult::Updated
            ) {
                return Err(StatusCode::CONFLICT);
            }
            let live = if let Some(host) = host {
                match store.claude_record(&tenant, &child) {
                    Ok(Some(row)) => host.apply(&store, &tenant, &child, &row, req.activation),
                    _ => ClaudeLiveResult::Failed,
                }
            } else {
                ClaudeLiveResult::NotAttempted
            };
            Ok(ClaudeResult { catalog_saved: true, live })
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (policy, tenant, db, headers, id, req);
            Err::<ClaudeResult, _>(StatusCode::NOT_IMPLEMENTED)
        }
    })
    .await;
    match result {
        Ok(Ok(r)) => Json(r).into_response(),
        Ok(Err(e)) => e.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
