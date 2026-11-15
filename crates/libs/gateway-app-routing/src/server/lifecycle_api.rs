//! Host management endpoints for the gateway-owned application listener.
pub use super::ControlError;
use app_access_service::access::{
    RoutingControlScope, authorize_routing_control, authorize_routing_mode, routing_setup,
};
use {
    app_routing_services::{
        ListenerError, RoutingListener,
        config_apply::{ListenerConfig, RoutingConfigStore},
    },
    agent_gateway::agent_apps_diagnostic_jobs::DiagnosticJobs,
    agent_database::GlobalDatabase,
};
use axum::{
    Extension, Json, Router,
    extract::{Path, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use vw_agent_apps::{
    apps::catalog::catalog,
    providers::live::commit::{ExecutionHostId, ProviderTransactions},
    routing::lifecycle_state::LifecycleState,
};
use {
    vw_api_app_identity::AppId,
    vw_api_app_routing::RoutingHealth,
    vw_api_app_routing::RoutingLifecycleRequest,
    vw_api_app_routing::RoutingState,
    vw_api_app_routing::RoutingStatus,
    vw_api_transport::id::ProjectId,
    vw_api_transport::id::ProjectScope,
    vw_api_transport::id::TenantScope,
};

#[derive(Clone)]
pub struct RoutingControl {
    pub database: GlobalDatabase,
    pub routing: Arc<RoutingListener>,
    scope: Arc<RoutingControlScope>,
    pub(super) paths: RoutingPaths,
    pub policy: PathBuf,
    pub policy_default: ListenerConfig,
    pub diagnostic_jobs: Option<DiagnosticJobs>,
}

#[derive(Clone)]
pub struct RoutingPaths {
    pub home: PathBuf,
    pub config: PathBuf,
    pub state: PathBuf,
}

pub async fn configure<H: app_routing_services::lifecycle_host::AggregationLifecycleHost + 'static>(
    app: Router, routing: &Arc<RoutingListener>, paths: RoutingPaths,
    lifecycle: impl FnOnce(GlobalDatabase, PathBuf, TenantScope, String) -> H,
) -> Router {
    let Some(path) = std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from) else {
        return match tokio::task::spawn_blocking({ let state = paths.state.clone(); move || DiagnosticJobs::open_on_execution_host(&state) })
            .await
            .ok()
            .flatten()
        {
            Some(jobs) => app.layer(Extension(jobs)),
            None => app,
        };
    };
    let Ok(database) = GlobalDatabase::for_home(&paths.home) else {
        tracing::warn!("Application aggregation lifecycle storage unavailable");
        return app;
    };
    configure_with_policy(app, routing, path, database, paths, lifecycle).await
}

pub async fn configure_with_policy<H: app_routing_services::lifecycle_host::AggregationLifecycleHost + 'static>(
    app: Router,
    routing: &Arc<RoutingListener>,
    policy: PathBuf,
    database: GlobalDatabase,
    paths: RoutingPaths,
    lifecycle: impl FnOnce(GlobalDatabase, PathBuf, TenantScope, String) -> H,
) -> Router {
    // One host registry is shared by diagnostics and outbound routing checks, even
    // when this deployment does not enable listener management.
    let diagnostic_jobs =
        tokio::task::spawn_blocking({ let state = paths.state.clone(); move || DiagnosticJobs::open_on_execution_host(&state) }).await.ok().flatten();
    let app = match diagnostic_jobs.as_ref() {
        Some(jobs) => app.layer(Extension(jobs.clone())),
        None => app,
    };
    let Ok((policy, Ok(setup))) = tokio::task::spawn_blocking(move || {
        let setup = routing_setup(&policy);
        (policy, setup)
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
        paths: paths.clone(),
        database: database.clone(),
        routing: Arc::clone(routing),
        scope: Arc::new(scope),
        policy,
        policy_default,
        diagnostic_jobs,
    };
    let tenant = control.scope.tenant.clone();
    let host = control.scope.host.clone();
    routing.set_egress(tenant.clone(), host.clone(), None);
    let secrets = super::outbound_api::secrets(&paths.config);
    match control
        .with_config_store(move |store| store.load_egress(&secrets))
        .await
    {
        Ok(egress) => routing.set_egress(tenant, host, Some(egress)),
        Err(_) => tracing::warn!("Application routing egress unavailable"),
    }
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
    routing.set_aggregation_lifecycle(
        lifecycle(database, control.policy.clone(), control.scope.tenant.clone(), control.scope.host.clone()),
    );
    if routing.bound_address().is_some()
        && let Err(error) = routing.resume_aggregations().await
    {
        tracing::warn!(%error, "Application aggregation startup failed");
    }
    app.layer(Extension(control))
}

impl RoutingControl {
    pub fn authorize_mode(
        &self,
        headers: &HeaderMap,
        project: &ProjectScope,
        host: &str,
        app: AppId,
    ) -> Result<(PathBuf, String), StatusCode> {
        authorize_routing_mode(&self.policy, headers, &self.scope, project, host, app)
    }

    /// Entry for the native configuration transaction. Check the current policy
    /// before resolving saved members, and use the same executor root throughout.
    pub fn publish_aggregation(
        &self,
        headers: &HeaderMap,
        project: &ProjectScope,
        host: &str,
        expected_revision: Option<&str>,
        prepare: impl FnOnce(
            &std::path::Path,
            &str,
        ) -> Result<
            app_routing_services::aggregation_routing::AggregationPublication,
            StatusCode,
        >,
        commit: impl FnOnce(
            &std::path::Path,
            SocketAddr,
            &str,
        ) -> Result<(), app_routing_services::RouteCommitError>,
    ) -> Result<app_routing_services::RouteRevocation, StatusCode> {
        self.publish_aggregation_app(
            headers,
            project,
            host,
            AppId::Codex,
            expected_revision,
            prepare,
            commit,
        )
    }

    /// Application-specific publication retains the same policy and unique listener.
    #[allow(clippy::too_many_arguments)]
    pub fn publish_aggregation_app(
        &self,
        headers: &HeaderMap,
        project: &ProjectScope,
        host: &str,
        app: AppId,
        expected_revision: Option<&str>,
        prepare: impl FnOnce(
            &std::path::Path,
            &str,
        ) -> Result<
            app_routing_services::aggregation_routing::AggregationPublication,
            StatusCode,
        >,
        commit: impl FnOnce(
            &std::path::Path,
            SocketAddr,
            &str,
        ) -> Result<(), app_routing_services::RouteCommitError>,
    ) -> Result<app_routing_services::RouteRevocation, StatusCode> {
        let (root, principal) = self.authorize_mode(headers, project, host, app)?;
        let publication = prepare(&root, &principal)?;
        if !publication.binding.has_scope(project, host, app) {
            return Err(StatusCode::FORBIDDEN);
        }
        self.routing.publish_aggregation(publication, expected_revision, |address, token| {
            commit(&root, address, token)
        })
    }

    /// Ordinary routing and aggregation share the authorization and token owner.
    pub fn publish_failover_route(
        &self,
        headers: &HeaderMap,
        project: &ProjectScope,
        host: &str,
        expected_revision: Option<&str>,
        prepare: impl FnOnce(
            &std::path::Path,
            &str,
        ) -> Result<
            app_routing_services::failover_routing::FailoverPublication,
            StatusCode,
        >,
        commit: impl FnOnce(
            &std::path::Path,
            SocketAddr,
            &str,
            &app_routing_services::failover_store::FailoverQueueState,
            &str,
        ) -> Result<(), app_routing_services::RouteCommitError>,
    ) -> Result<app_routing_services::RouteRevocation, StatusCode> {
        let app = AppId::Codex;
        let (root, principal) = self.authorize_mode(headers, project, host, app)?;
        let publication = prepare(&root, &principal)?;
        if !publication.binding.has_scope(project, host, app) {
            return Err(StatusCode::FORBIDDEN);
        }
        let database = GlobalDatabase::for_home(&self.paths.home)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let store =
            app_routing_services::failover_store::FailoverQueueStore::new(database)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        self.routing.publish_failover_route(
            publication,
            &store,
            expected_revision,
            |address, token, queue, provider| commit(&root, address, token, queue, provider),
        )
    }

    pub async fn with_config_store<T: Send + 'static>(
        &self,
        operation: impl FnOnce(RoutingConfigStore) -> anyhow::Result<T> + Send + 'static,
    ) -> Result<T, ControlError> {
        let scope = Arc::clone(&self.scope);
        let database = self.database.clone();
        tokio::task::spawn_blocking(move || {
            let settings =
                RoutingConfigStore::new(database, scope.tenant.clone(), scope.host.clone())?;
            operation(settings)
        })
        .await
        .map_err(|_| ControlError::Unavailable)?
        .map_err(|_| ControlError::Unavailable)
    }

    pub async fn config(&self) -> Result<ListenerConfig, ControlError> {
        let fallback = self.policy_default;
        self.with_config_store(move |store| Ok(store.listener()?.unwrap_or(fallback))).await
    }

    pub async fn authorize(
        control: Option<Extension<Self>>,
        headers: HeaderMap,
        tenant: TenantScope,
        host: String,
        app: Option<(ProjectId, AppId)>,
        manage: bool,
    ) -> Result<(Self, Option<PathBuf>), ControlError> {
        let (control, root, _) =
            Self::authorize_member(control, headers, tenant, host, app, manage).await?;
        Ok((control, root))
    }

    pub async fn authorize_member(
        control: Option<Extension<Self>>,
        headers: HeaderMap,
        tenant: TenantScope,
        host: String,
        app: Option<(ProjectId, AppId)>,
        manage: bool,
    ) -> Result<(Self, Option<PathBuf>, String), ControlError> {
        let Extension(control) = control.ok_or(ControlError::Unavailable)?;
        let policy = control.policy.clone();
        let scope = Arc::clone(&control.scope);
        let (root, principal) = tokio::task::spawn_blocking(move || {
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
        Ok((control, root, principal))
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

pub async fn status(
    control: Option<Extension<RoutingControl>>,
    tenant: Extension<TenantScope>,
    host: Path<String>,
    headers: HeaderMap,
) -> Result<Json<RoutingStatus>, ControlError> {
    super::status(control, tenant, host, headers).await
}

pub async fn health(
    control: Option<Extension<RoutingControl>>,
    tenant: Extension<TenantScope>,
    host: Path<String>,
    headers: HeaderMap,
) -> Result<Json<RoutingHealth>, ControlError> {
    super::health(control, tenant, host, headers).await
}

pub async fn start(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    Path(host): Path<String>,
    headers: HeaderMap,
    body: Result<Json<RoutingLifecycleRequest>, JsonRejection>,
) -> Result<Json<RoutingStatus>, ControlError> {
    let (control, _) =
        RoutingControl::authorize(control, headers, tenant, host, None, true).await?;
    let Json(_) = body.map_err(|_| ControlError::InvalidInput)?;
    let permit = control.routing.try_control().map_err(listener_error)?;
    let config = control.config().await?;
    control.routing.start_configured(config).map_err(listener_error)?;
    drop(permit);
    control.routing.resume_aggregations().await.map_err(listener_error)?;
    Ok(Json(control.status()))
}

pub async fn stop(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    Path(host): Path<String>,
    headers: HeaderMap,
    body: Result<Json<RoutingLifecycleRequest>, JsonRejection>,
) -> Result<Json<RoutingStatus>, ControlError> {
    let (control, root) =
        RoutingControl::authorize(control, headers, tenant, host, None, true).await?;
    let Json(_) = body.map_err(|_| ControlError::InvalidInput)?;
    let permit = control.routing.try_control().map_err(listener_error)?;
    // Keep both file restoration and draining alive if the HTTP caller leaves.
    tokio::spawn(async move {
        let _permit = permit;
        let detached = control.routing.detach_aggregations().await.map_err(listener_error)?;
        if !matches!(control.routing.state(), LifecycleState::Stopped | LifecycleState::Failed) {
            let host = control.scope.host.clone();
            let attached =
                tokio::task::spawn_blocking(move || apps_attached_except(root, &host, &detached))
                    .await
                    .map_err(|_| ControlError::Unavailable)??;
            if attached || control.routing.has_active_routes() {
                return Err(ControlError::AppsAttached);
            }
        }
        control.routing.stop_controlled().await.map_err(listener_error)?;
        Ok(Json(control.status()))
    })
    .await
    .map_err(|_| ControlError::Unavailable)?
}

pub fn apps_attached(root: Option<PathBuf>, host: &str) -> Result<bool, ControlError> {
    apps_attached_except(root, host, &[])
}

fn apps_attached_except(
    root: Option<PathBuf>,
    host: &str,
    detached: &[vw_api_app_failover::FailoverQueueScope],
) -> Result<bool, ControlError> {
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
        // Desktop retains its projection by design. Only a scope actually
        // authorized and retired by this stop may bypass the attachment check.
        if app == AppId::ClaudeDesktop
            && state.stack.enabled
            && pending.is_none()
            && detached.iter().any(|scope| {
                scope.app_id == app && state.project_scope.as_ref() == Some(&scope.project)
            })
        {
            continue;
        }
        if state.mode.is_some_and(|mode| mode.attached)
            || pending.and_then(|pending| pending.target.mode).is_some_and(|mode| mode.attached)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn listener_error(error: ListenerError) -> ControlError {
        match error {
            ListenerError::AddressInUse => ControlError::AddressInUse,
            ListenerError::AppsAttached => ControlError::AppsAttached,
            ListenerError::AggregationRecovery(status) => match status {
                StatusCode::CONFLICT => ControlError::AggregationConflict,
                StatusCode::UNPROCESSABLE_ENTITY => ControlError::AggregationUnsupported,
                StatusCode::BAD_GATEWAY => ControlError::AggregationTargetFailed,
                _ => ControlError::from(status),
            },
            ListenerError::TransitionInProgress => ControlError::TransitionInProgress,
            _ => ControlError::Unavailable,
        }
    }

impl super::RoutingObservation for RoutingControl {
    fn routing_status(&self) -> RoutingStatus {
        self.status()
    }

    fn ready(&self) -> bool {
        self.routing.state() == LifecycleState::Running
    }

    async fn authorize(
        control: Option<Extension<Self>>,
        headers: HeaderMap,
        tenant: TenantScope,
        host: String,
        app: Option<(ProjectId, AppId)>,
        manage: bool,
    ) -> Result<(Self, Option<PathBuf>), ControlError> {
        RoutingControl::authorize(control, headers, tenant, host, app, manage).await
    }

    async fn config_bind(&self) -> Result<std::net::IpAddr, ControlError> {
        Ok(self.config().await?.bind.ip())
    }

    fn request_status(
        &self,
        project: &ProjectScope,
        host: &str,
        app: AppId,
    ) -> vw_api_app_routing::RoutingRequestsStatus {
        self.routing.request_status(project, host, app)
    }
}

impl super::RoutingDiagnostics for RoutingControl {
    async fn authorize_member(
        control: Option<Extension<Self>>, headers: HeaderMap, tenant: TenantScope,
        host: String, app: Option<(ProjectId, AppId)>, manage: bool,
    ) -> Result<(Self, Option<PathBuf>, String), ControlError> {
        RoutingControl::authorize_member(control, headers, tenant, host, app, manage).await
    }

    fn diagnostic_jobs(&self) -> Option<DiagnosticJobs> {
        self.diagnostic_jobs.clone()
    }
}

pub struct OutboundProbeEgress(Arc<app_routing_services::outbound_config::RoutingEgress>);
impl super::ProbeEgress for OutboundProbeEgress {
    fn probe_client(&self, target: &reqwest::Url, listener: Option<SocketAddr>)
        -> Result<Option<reqwest::Client>, agent_gateway::agent_apps_diagnostic_jobs::DiagnosticProbeError>
    {
        use vw_api_app_diagnostics::{DiagnosticFailure, DiagnosticFailureCode, DiagnosticPhase};
        self.0.probe_client(target, listener).map_err(|_| {
            agent_gateway::agent_apps_diagnostic_jobs::DiagnosticProbeError::Failed(DiagnosticFailure {
                phase: DiagnosticPhase::ResponseHeaders, code: DiagnosticFailureCode::Transport,
            })
        })
    }
}
impl super::RoutingProbe for RoutingControl {
    type Egress = OutboundProbeEgress;
    fn configured_egress(&self, tenant: &TenantScope, host: &str) -> Option<Self::Egress> {
        self.routing.configured_egress(tenant, host).map(OutboundProbeEgress)
    }
    fn bound_address(&self) -> Option<SocketAddr> {
        self.routing.bound_address()
    }
}
