//! Authenticated Claude config operations, sharing the established catalog and host transaction.
use super::*;
use axum::{http::HeaderMap, routing::post};
use std::path::PathBuf;
use vw_api_types::app_providers::claude::*;

#[path = "claude_form.rs"]
mod form;
#[derive(Clone)]
pub(super) struct Policy(
    pub(super) Option<PathBuf>,
    #[cfg(target_os = "macos")]
    pub(super)  std::sync::Arc<dyn vw_agent_apps::auth::claude_keychain::ClaudeKeychain>,
);
impl Policy {
    pub(super) fn new(path: Option<PathBuf>) -> Self {
        Self(
            path,
            #[cfg(target_os = "macos")]
            std::sync::Arc::new(vw_agent_apps::auth::claude_keychain::MacOsKeychain),
        )
    }
}
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(in crate::gateway::api::handlers) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    router_with_context(Policy::new(policy))
}
pub(super) fn router_with_context(policy: Policy) -> Router {
    Router::new()
        .route("/app-providers/claude/{id}/form/read", post(form::read))
        .route("/app-providers/claude/{id}/form", post(form::write).patch(form::write))
        .route("/app-providers/claude/{id}/switch", post(switch))
        .route("/app-providers/claude/{id}/reapply", post(reapply))
        .route("/app-providers/claude/{id}/import", post(import))
        .route("/universal-providers/{id}/sync/claude", post(sync))
        .layer(DefaultBodyLimit::max(32 * 1024))
        .layer(Extension(policy))
}
#[derive(Clone, Copy)]
pub(super) enum WriteKind {
    Create,
    Save,
    Update,
}
#[allow(clippy::too_many_arguments)]
pub(super) async fn write(
    tenant: TenantScope,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    policy: Option<Extension<Policy>>,
    id: String,
    req: ClaudeWrite,
    kind: WriteKind,
    publication: PublicationInput,
) -> Response {
    let policy = policy.map(|Extension(p)| p).unwrap_or_else(|| {
        Policy::new(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
    });
    write_input_scoped(tenant, db, headers, id, WriteInput::Profile(req), kind, policy, publication)
        .await
}

enum WriteInput {
    Profile(ClaudeWrite),
    Form(ClaudeFormWrite),
}
impl WriteInput {
    fn target(&self) -> &str {
        match self {
            Self::Profile(req) => &req.target_host_id,
            Self::Form(req) => &req.target_host_id,
        }
    }
}
#[allow(clippy::too_many_arguments)]
async fn write_input_scoped(
    tenant: TenantScope,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    id: String,
    req: WriteInput,
    kind: WriteKind,
    policy: Policy,
    publication: PublicationInput,
) -> Response {
    let status =
        if matches!(kind, WriteKind::Create) { StatusCode::CREATED } else { StatusCode::OK };
    match tokio::task::spawn_blocking(move || -> Result<ClaudeResult, StatusCode> {
        write_sync(tenant, db, headers, id, req, kind, policy, publication)
    })
    .await
    {
        Ok(Ok(result)) => (status, Json(result)).into_response(),
        Ok(Err(status)) => status.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
#[cfg(target_os = "macos")]
#[allow(clippy::too_many_arguments)]
fn write_sync(
    tenant: TenantScope,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    id: String,
    input: WriteInput,
    kind: WriteKind,
    policy: Policy,
    publication: PublicationInput,
) -> Result<ClaudeResult, StatusCode> {
    let headers = publication.headers(headers)?;
    let (home, root, principal, owns_slot) = super::super::agent_apps::claude_editor_authorization(
        policy.0.as_deref(),
        &headers,
        &tenant,
        input.target(),
    )?;
    if let WriteInput::Profile(req) = &input {
        authorize_login(&policy, &headers, &tenant, &req.target_host_id, &req.claude.config)?;
        vw_agent_apps::providers::claude::settings(&req.claude)
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    }
    if matches!(input, WriteInput::Form(_)) {
        vw_agent_apps::providers::validate_identity(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    }
    let store = store(db).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let store = publication_store(
        store,
        policy.0.as_deref(),
        &headers,
        &tenant,
        input.target(),
        AppId::Claude,
        publication,
    )?;
    let scoped = store.resync_configured();
    let resync_published = store.resync_context().is_some_and(|context| {
        context.expected.stack.enabled
            && context.expected.stack.members.contains(&id)
            && context.expected.mode.as_ref().is_some_and(|mode| mode.attached)
    });
    let before = store.claude_record(&tenant, &id).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if matches!(kind, WriteKind::Update) && before.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }
    if matches!(kind, WriteKind::Create) && before.is_some() {
        return Err(StatusCode::CONFLICT);
    }
    let (mut req, metadata) = match input {
        WriteInput::Profile(req) => (req, None),
        WriteInput::Form(req) => {
            let claude = vw_agent_apps::providers::claude::form::profile(&req, before.as_ref())
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            let metadata = vw_agent_apps::providers::basic::apply(
                AppId::Claude,
                before.as_ref().unwrap_or(&serde_json::json!({})),
                &req.fields,
                &vw_api_types::app_providers::create::ProviderSecretWrite::Keep,
            )
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            let activation = match (req.apply, kind) {
                (false, _) => ClaudeActivation::CatalogOnly,
                (true, WriteKind::Create) => ClaudeActivation::First,
                (true, _) => ClaudeActivation::IfCurrent,
            };
            (ClaudeWrite { target_host_id: req.target_host_id, claude, activation }, Some(metadata))
        }
    };
    if scoped {
        req.activation = ClaudeActivation::CatalogOnly;
    }
    let host = Host::open(&home, &root, &req.target_host_id, &tenant, req.activation, &store, &id)?;
    let row = store
        .claude_save_with_metadata(
            &tenant,
            &id,
            &req.claude,
            before.as_ref(),
            matches!(kind, WriteKind::Create),
            metadata,
        )
        .map_err(|_| store.publication_error().unwrap_or(StatusCode::CONFLICT))?;
    let live = if let Some(host) = host {
        host.apply(
            &store,
            &tenant,
            (&principal, owns_slot),
            &id,
            &row,
            req.activation,
            policy.1.as_ref(),
        )
    } else if resync_published {
        ClaudeLiveResult::Applied
    } else {
        ClaudeLiveResult::NotAttempted
    };
    Ok(ClaudeResult { catalog_saved: true, live })
}
#[cfg(not(target_os = "macos"))]
#[allow(clippy::too_many_arguments)]
fn write_sync(
    _: TenantScope,
    _: Option<Extension<GlobalDatabase>>,
    _: HeaderMap,
    _: String,
    req: WriteInput,
    _: WriteKind,
    _: Policy,
    _: PublicationInput,
) -> Result<ClaudeResult, StatusCode> {
    let _ = req.target();
    Err(StatusCode::NOT_IMPLEMENTED)
}
#[cfg(target_os = "macos")]
struct Host {
    file: vw_agent_apps::providers::claude::ClaudeFile,
    tx: vw_agent_apps::providers::live::commit::ProviderTransactions,
    state: vw_agent_apps::providers::live::commit::AppState,
    previous: Option<serde_json::Value>,
    login: Option<vw_agent_apps::providers::claude_login::LoginHost>,
    _directory_lock: std::fs::File,
    root: PathBuf,
    host_id: String,
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
        use vw_agent_apps::providers::live::commit::AppState;
        if activation == ClaudeActivation::CatalogOnly {
            return Ok(None);
        }
        let tx = super::paths::transactions(root, host)?;
        let directory_lock = tx.lock_claude_directory().map_err(super::paths::directory_error)?;
        let directory = tx.claude_directory(tenant).map_err(super::paths::directory_error)?;
        let observed_state = tx.state(AppId::Claude).map_err(|_| StatusCode::CONFLICT)?;
        if observed_state.mode.as_ref().is_some_and(|m| m.attached)
            || observed_state.pointer.as_ref().is_some_and(|p| p.tenant != *tenant)
            || tx.pending(AppId::Claude).map_or(true, |p| p.is_some())
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
        let file = vw_agent_apps::providers::claude::ClaudeFile::open_selected(
            home,
            directory.as_ref(),
            true,
        )
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
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
        let login = if directory.is_none() {
            Some(
                vw_agent_apps::providers::claude_login::LoginHost::open(home)
                    .map_err(|_| StatusCode::CONFLICT)?,
            )
        } else {
            None
        };
        Ok(Some(Self {
            file,
            tx,
            state,
            previous,
            login,
            _directory_lock: directory_lock,
            root: root.into(),
            host_id: host.into(),
        }))
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
            let (home, root, principal, owns_slot) =
                super::super::agent_apps::claude_editor_authorization(
                    policy.0.as_deref(),
                    &headers,
                    &tenant,
                    &host,
                )?;
            let store = store(db).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            let row = store
                .claude_record(&tenant, &id)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
                .ok_or(StatusCode::NOT_FOUND)?;
            if let Some(value) = row.get("claude") {
                let profile: ClaudeProfile = serde_json::from_value(value.clone())
                    .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
                authorize_login(&policy, &headers, &tenant, &host, &profile.config)?;
            }
            let host = Host::open(&home, &root, &host, &tenant, activation, &store, &id)?
                .ok_or(StatusCode::CONFLICT)?;
            Ok(ClaudeResult {
                catalog_saved: false,
                live: host.apply(
                    &store,
                    &tenant,
                    (&principal, owns_slot),
                    &id,
                    &row,
                    activation,
                    policy.1.as_ref(),
                ),
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
            let (home, root) = super::super::agent_apps::provider_editor_context(
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
            let directory =
                super::paths::saved_claude_directory(Some(&root), &req.target_host_id, &tenant)?;
            let file = vw_agent_apps::providers::claude::ClaudeFile::open_selected(
                &home,
                directory.as_ref(),
                false,
            )
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
            let (home, root, principal, owns_slot) =
                super::super::agent_apps::claude_editor_authorization(
                    policy.0.as_deref(),
                    &headers,
                    &tenant,
                    &req.target_host_id,
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
                    Ok(Some(row)) => host.apply(
                        &store,
                        &tenant,
                        (&principal, owns_slot),
                        &child,
                        &row,
                        req.activation,
                        policy.1.as_ref(),
                    ),
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

#[cfg(target_os = "macos")]
fn authorize_login(
    policy: &Policy,
    headers: &HeaderMap,
    tenant: &TenantScope,
    host: &str,
    config: &ClaudeConfig,
) -> Result<(), StatusCode> {
    if let ClaudeConfig::Official { account: Some(binding) } = config {
        super::super::agent_apps::authorize_provider_login(
            policy.0.as_deref(),
            headers,
            tenant,
            host,
            &binding.account_id,
        )?;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
#[path = "claude_login.rs"]
mod login;
