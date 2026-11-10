//! Host management endpoints for the gateway-owned application listener.
use super::super::agent_apps::{RoutingControlScope, authorize_routing_control, routing_setup};
use crate::{
    app_routing::{
        ListenerError, RoutingListener,
        config_apply::{ListenerConfig, RoutingConfigStore},
    },
    global_database::GlobalDatabase,
};
use axum::{
    Extension, Json, Router,
    extract::{Path, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use std::{path::PathBuf, sync::Arc};
use vw_agent_apps::{
    apps::catalog::catalog,
    providers::live::commit::{ExecutionHostId, ProviderTransactions},
    routing::lifecycle_state::LifecycleState,
};
use vw_api_types::{
    agent_apps::AppId,
    app_routing::{RoutingHealth, RoutingLifecycleRequest, RoutingState, RoutingStatus},
    error::{ApiErrorBody, ApiErrorDetail},
    id::{ProjectId, TenantScope},
};

#[derive(Clone)]
pub(crate) struct RoutingControl {
    pub(super) routing: Arc<RoutingListener>,
    scope: Arc<RoutingControlScope>,
    policy: PathBuf,
    pub(super) policy_default: ListenerConfig,
}

pub(crate) async fn configure(app: Router, routing: &Arc<RoutingListener>) -> Router {
    let Some(path) = std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from) else {
        return app;
    };
    let Ok((policy, Ok(setup))) = tokio::task::spawn_blocking(move || {
        let setup = routing_setup(&path);
        (path, setup)
    })
    .await
    else {
        tracing::warn!("Application routing policy rejected");
        return app;
    };
    let Some(scope) = setup.control else {
        if let Some(bind) = setup.startup_address
            && let Err(error) = routing.start_on(&bind.ip().to_string(), bind.port())
        {
            tracing::warn!(%error, "Application routing startup failed");
        }
        return app;
    };
    let policy_default = ListenerConfig {
        enabled: setup.startup_address.is_some(),
        bind: scope.bind,
        logging_enabled: false,
    };
    let control = RoutingControl {
        routing: Arc::clone(routing),
        scope: Arc::new(scope),
        policy,
        policy_default,
    };
    if policy_default.enabled {
        match control.config().await {
            Ok(config) if config.enabled => {
                if let Err(error) = routing.start_configured(config) {
                    tracing::warn!(%error, "Application routing startup failed");
                }
            }
            Ok(_) => {}
            Err(_) => tracing::warn!("Application routing settings unavailable"),
        }
    }
    app.layer(Extension(control))
}

impl RoutingControl {
    pub(super) async fn with_config_store<T: Send + 'static>(
        &self,
        operation: impl FnOnce(RoutingConfigStore) -> anyhow::Result<T> + Send + 'static,
    ) -> Result<T, ControlError> {
        let scope = Arc::clone(&self.scope);
        tokio::task::spawn_blocking(move || {
            let db = GlobalDatabase::for_home(&crate::global::paths().home)?;
            let settings = RoutingConfigStore::new(db, scope.tenant.clone(), scope.host.clone())?;
            operation(settings)
        })
        .await
        .map_err(|_| ControlError::Unavailable)?
        .map_err(|_| ControlError::Unavailable)
    }

    pub(super) async fn config(&self) -> Result<ListenerConfig, ControlError> {
        let fallback = self.policy_default;
        self.with_config_store(move |store| Ok(store.listener()?.unwrap_or(fallback))).await
    }

    pub(super) async fn authorize(
        control: Option<Extension<Self>>,
        headers: HeaderMap,
        tenant: TenantScope,
        host: String,
        app: Option<(ProjectId, AppId)>,
        manage: bool,
    ) -> Result<(Self, Option<PathBuf>), ControlError> {
        let Extension(control) = control.ok_or(ControlError::Unavailable)?;
        let policy = control.policy.clone();
        let scope = Arc::clone(&control.scope);
        let root = tokio::task::spawn_blocking(move || {
            authorize_routing_control(
                &policy,
                &headers,
                &tenant,
                &host,
                &scope,
                app.as_ref().map(|(project, app)| (project, *app)),
                manage,
            )
            .map_err(ControlError::from)
        })
        .await
        .map_err(|_| ControlError::Unavailable)??;
        Ok((control, root))
    }

    fn status(&self) -> RoutingStatus {
        let (state, binding) = self.routing.snapshot();
        let state = match state {
            LifecycleState::Stopped => RoutingState::Stopped,
            LifecycleState::Starting => RoutingState::Starting,
            LifecycleState::Running => RoutingState::Running,
            LifecycleState::Stopping => RoutingState::Stopping,
            LifecycleState::Failed => RoutingState::Failed,
        };
        RoutingStatus {
            state,
            bound_address: binding.map(|(address, _)| address.ip()),
            bound_port: binding.map(|(address, _)| address.port()),
            started_at: binding.map(|(_, started_at)| started_at.to_rfc3339()),
        }
    }
}

pub(crate) async fn status(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    Path(host): Path<String>,
    headers: HeaderMap,
) -> Result<Json<RoutingStatus>, ControlError> {
    let (control, _) =
        RoutingControl::authorize(control, headers, tenant, host, None, false).await?;
    Ok(Json(control.status()))
}

pub(crate) async fn health(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    Path(host): Path<String>,
    headers: HeaderMap,
) -> Result<Json<RoutingHealth>, ControlError> {
    let (control, _) =
        RoutingControl::authorize(control, headers, tenant, host, None, false).await?;
    if control.routing.state() != LifecycleState::Running {
        return Err(ControlError::NotReady);
    }
    Ok(Json(RoutingHealth { ready: true }))
}

pub(crate) async fn start(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    Path(host): Path<String>,
    headers: HeaderMap,
    body: Result<Json<RoutingLifecycleRequest>, JsonRejection>,
) -> Result<Json<RoutingStatus>, ControlError> {
    let (control, _) =
        RoutingControl::authorize(control, headers, tenant, host, None, true).await?;
    let Json(_) = body.map_err(|_| ControlError::InvalidInput)?;
    let _permit = control.routing.try_control()?;
    let config = control.config().await?;
    control.routing.start_configured(config)?;
    Ok(Json(control.status()))
}

pub(crate) async fn stop(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    Path(host): Path<String>,
    headers: HeaderMap,
    body: Result<Json<RoutingLifecycleRequest>, JsonRejection>,
) -> Result<Json<RoutingStatus>, ControlError> {
    let (control, root) =
        RoutingControl::authorize(control, headers, tenant, host, None, true).await?;
    let Json(_) = body.map_err(|_| ControlError::InvalidInput)?;
    let permit = control.routing.try_control()?;
    if !matches!(control.routing.state(), LifecycleState::Stopped | LifecycleState::Failed) {
        let host = control.scope.host.clone();
        let attached = tokio::task::spawn_blocking(move || apps_attached(root, &host))
            .await
            .map_err(|_| ControlError::Unavailable)??;
        if attached || control.routing.has_active_routes() {
            return Err(ControlError::AppsAttached);
        }
    }
    // A disconnected HTTP caller must still finish stopping and release the
    // control permit only after the listener has drained.
    tokio::spawn(async move {
        let _permit = permit;
        control.routing.stop().await?;
        Ok(Json(control.status()))
    })
    .await
    .map_err(|_| ControlError::Unavailable)?
}

pub(super) fn apps_attached(root: Option<PathBuf>, host: &str) -> Result<bool, ControlError> {
    let Some(root) = root else { return Ok(false) };
    let host = ExecutionHostId::new(host).map_err(|_| ControlError::Unavailable)?;
    let Some(store) =
        ProviderTransactions::open_existing(&root, host).map_err(|_| ControlError::Unavailable)?
    else {
        return Ok(false);
    };
    for app in catalog() {
        let app = app.metadata.id;
        let state = store.state(app).map_err(|_| ControlError::Unavailable)?;
        let pending = store.pending(app).map_err(|_| ControlError::Unavailable)?;
        if state.mode.is_some_and(|mode| mode.attached)
            || pending.and_then(|pending| pending.target.mode).is_some_and(|mode| mode.attached)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) enum ControlError {
    InvalidInput,
    InvalidConfig,
    UnsupportedApp,
    ConfigNotApplied,
    Unauthenticated,
    Forbidden,
    NotFound,
    UnsupportedHost,
    AppsAttached,
    AddressInUse,
    TransitionInProgress,
    NotReady,
    Unavailable,
}

impl From<StatusCode> for ControlError {
    fn from(status: StatusCode) -> Self {
        match status {
            StatusCode::UNAUTHORIZED => Self::Unauthenticated,
            StatusCode::FORBIDDEN => Self::Forbidden,
            StatusCode::NOT_FOUND => Self::NotFound,
            StatusCode::NOT_IMPLEMENTED => Self::UnsupportedHost,
            _ => Self::Unavailable,
        }
    }
}

impl From<ListenerError> for ControlError {
    fn from(error: ListenerError) -> Self {
        match error {
            ListenerError::AddressInUse => Self::AddressInUse,
            ListenerError::AppsAttached => Self::AppsAttached,
            ListenerError::TransitionInProgress => Self::TransitionInProgress,
            _ => Self::Unavailable,
        }
    }
}

impl IntoResponse for ControlError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::InvalidInput => {
                (StatusCode::BAD_REQUEST, "invalid_input", "Expected an empty JSON object")
            }
            Self::InvalidConfig => {
                (StatusCode::BAD_REQUEST, "invalid_input", "Invalid routing configuration")
            }
            Self::UnsupportedApp => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "unsupported_app",
                "Application routing settings are unsupported",
            ),
            Self::ConfigNotApplied => (
                StatusCode::CONFLICT,
                "config_not_applied",
                "Routing address change could not be applied",
            ),
            Self::Unauthenticated => {
                (StatusCode::UNAUTHORIZED, "unauthenticated", "Application membership is required")
            }
            Self::Forbidden => (StatusCode::FORBIDDEN, "forbidden", "Routing permission denied"),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found", "Execution host not found"),
            Self::UnsupportedHost => {
                (StatusCode::NOT_IMPLEMENTED, "unsupported_host", "Execution host is unsupported")
            }
            Self::AppsAttached => {
                (StatusCode::CONFLICT, "apps_attached", "Applications are still attached")
            }
            Self::AddressInUse => {
                (StatusCode::CONFLICT, "address_in_use", "Routing address is in use")
            }
            Self::TransitionInProgress => (
                StatusCode::CONFLICT,
                "transition_in_progress",
                "Routing transition is in progress",
            ),
            Self::NotReady => {
                (StatusCode::SERVICE_UNAVAILABLE, "not_ready", "Routing listener is not running")
            }
            Self::Unavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "unavailable",
                "Application routing control is unavailable",
            ),
        };
        (
            status,
            Json(ApiErrorBody {
                error: ApiErrorDetail { code: code.into(), message: message.into(), details: None },
            }),
        )
            .into_response()
    }
}
