//! Agy catalog writes do not activate a global provider or touch native settings.
use super::*;
use axum::{
    http::HeaderMap,
    routing::{post, put},
};
use std::path::PathBuf;
use vw_api_types::app_providers::agy::*;
#[derive(Clone)]
struct Policy(Option<PathBuf>);
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(in crate::gateway::api::handlers) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/app-providers/agy/{id}/config", put(save))
        .route("/app-providers/agy/{id}/form/read", post(form_read))
        .route("/app-providers/agy/{id}/form/delete", post(form_delete))
        .route("/app-providers/agy/{id}/form", post(form_create).patch(form_update))
        .route("/app-providers/agy/{id}/switch", post(switch))
        .layer(DefaultBodyLimit::max(32768))
        .layer(Extension(Policy(path)))
}
async fn save(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    Path(id): Path<String>,
    Json(b): Json<AgyWrite>,
) -> Response {
    match tokio::task::spawn_blocking(move || -> Result<AgySaved, StatusCode> {
        super::super::agent_apps::provider_editor_context(
            p.0.as_deref(),
            &h,
            &t,
            &b.target_host_id,
            true,
        )?;
        // Validation is pure; persistence cannot be interpreted as a running terminal.
        if !cfg!(target_os = "macos") {
            return Err(StatusCode::NOT_IMPLEMENTED);
        }
        #[cfg(target_os = "macos")]
        vw_agent_apps::providers::agy::validate(&b.profile)
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        store(db)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
            .agy_save(&t, &id, &b.profile)
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        Ok(AgySaved { catalog_saved: true })
    })
    .await
    {
        Ok(Ok(v)) => Json(v).into_response(),
        Ok(Err(s)) => s.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
async fn switch(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    h: HeaderMap,
    Path(_id): Path<String>,
    Json(b): Json<AgyTarget>,
) -> Response {
    match tokio::task::spawn_blocking(move || {
        super::super::agent_apps::provider_editor_context(
            p.0.as_deref(),
            &h,
            &t,
            &b.target_host_id,
            true,
        )
    })
    .await
    {
        Ok(Ok(_)) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({"code":"agy_switch_unsupported"})),
        )
            .into_response(),
        Ok(Err(s)) => s.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

async fn form_read(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<AgyTarget>, JsonRejection>,
) -> Response {
    let Ok(Json(b)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    match tokio::task::spawn_blocking(move || form_execute(p, t, db, h, id, FormOperation::Read(b)))
        .await
    {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(status)) => status.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

async fn form_create(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<AgyFormWrite>, JsonRejection>,
) -> Response {
    let Ok(Json(b)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    match tokio::task::spawn_blocking(move || {
        form_execute(p, t, db, h, id, FormOperation::Write(b, true))
    })
    .await
    {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(status)) => status.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

async fn form_update(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<AgyFormWrite>, JsonRejection>,
) -> Response {
    let Ok(Json(b)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    match tokio::task::spawn_blocking(move || {
        form_execute(p, t, db, h, id, FormOperation::Write(b, false))
    })
    .await
    {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(status)) => status.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

enum FormOperation {
    Delete(AgyTarget),
    Read(AgyTarget),
    Write(AgyFormWrite, bool),
}
fn form_execute(
    p: Policy,
    t: TenantScope,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    id: String,
    op: FormOperation,
) -> Result<serde_json::Value, StatusCode> {
    let host = match &op {
        FormOperation::Read(b) | FormOperation::Delete(b) => &b.target_host_id,
        FormOperation::Write(b, _) => &b.target_host_id,
    };
    let (home, root) = super::super::agent_apps::provider_editor_context(
        p.0.as_deref(),
        &h,
        &t,
        host,
        !matches!(op, FormOperation::Read(_)),
    )?;
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (db, id, op, home, root);
        Err(StatusCode::NOT_IMPLEMENTED)
    }
    #[cfg(target_os = "macos")]
    {
        let _lease = if matches!(op, FormOperation::Read(_)) {
            None
        } else {
            Some(
                vw_agent_apps::providers::agy::catalog_lease(
                    &vw_agent_apps::providers::agy::Scope {
                        home: &home,
                        private_root: &root,
                        host,
                        tenant: &t,
                        provider: &id,
                    },
                    matches!(op, FormOperation::Delete(_)),
                )
                .map_err(|_| StatusCode::CONFLICT)?,
            )
        };
        let store = store(db).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let result = match op {
            FormOperation::Delete(_) => {
                store.agy_form_delete(&t, &id).map(|()| serde_json::json!({"deleted":true}))
            }
            FormOperation::Read(_) => store
                .agy_form_read(&t, &id)
                .and_then(|v| serde_json::to_value(v).map_err(|_| "invalid profile".into())),
            FormOperation::Write(b, create) => store
                .agy_form_write(&t, &id, &b, create)
                .and_then(|v| serde_json::to_value(v).map_err(|_| "invalid profile".into())),
        };
        result.map_err(|e| match e.as_str() {
            "provider changed" | "Agy provider references unavailable" => StatusCode::CONFLICT,
            "application provider not found" => StatusCode::NOT_FOUND,
            _ => StatusCode::UNPROCESSABLE_ENTITY,
        })
    }
}

async fn form_delete(
    Extension(p): Extension<Policy>,
    Extension(t): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    h: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<AgyTarget>, JsonRejection>,
) -> Response {
    let Ok(Json(b)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    match tokio::task::spawn_blocking(move || {
        form_execute(p, t, db, h, id, FormOperation::Delete(b))
    })
    .await
    {
        Ok(Ok(v)) => Json(v).into_response(),
        Ok(Err(s)) => s.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
