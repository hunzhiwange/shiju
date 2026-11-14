//! Owns the application's loopback listener independently of session relays.

#[cfg(target_os = "macos")]
use super::outbound_config::{EgressState, RoutingEgress};
use super::{
    body_limits::BodyLimits,
    dispatch,
    protocol_errors::Protocol,
    route_auth::{self, RouteAuth},
    shutdown,
};
use parking_lot::Mutex;
use std::{
    future::Future,
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::{
    net::{TcpListener, TcpSocket, TcpStream},
    runtime::Handle,
    sync::oneshot,
    time::timeout,
};
use vw_agent_apps::routing::lifecycle_state::{Lifecycle, LifecycleState};
#[cfg(target_os = "macos")]
use vw_api_types::{app_routing::RoutingEgressStatus, id::TenantScope};

#[derive(Debug, thiserror::Error)]
pub enum ListenerError {
    #[error("a routing listener transition is already in progress")]
    TransitionInProgress,
    #[cfg(target_os = "macos")]
    #[error("the gateway is shutting down")]
    ShuttingDown,
    #[error("the routing listener requires an active Tokio runtime")]
    RuntimeUnavailable,
    #[error("the routing listener address must be an IPv4 or IPv6 literal")]
    InvalidAddress,
    #[error("non-loopback routing requires host authorization and data-plane authentication")]
    NonLoopbackDenied,
    #[error("the routing listener address is already in use")]
    AddressInUse,
    #[cfg(target_os = "macos")]
    #[error("application routes are attached")]
    AppsAttached,
    #[cfg(target_os = "macos")]
    #[error("aggregation lifecycle recovery failed ({0})")]
    AggregationRecovery(axum::http::StatusCode),
    #[error("could not bind the routing listener: {0}")]
    Bind(#[from] io::Error),
    #[error("the routing listener timed out waiting for connection tasks to stop")]
    StopTimeout,
}

struct RunningListener {
    address: SocketAddr,
    #[cfg(target_os = "macos")]
    started_at: chrono::DateTime<chrono::Utc>,
    shutdown: oneshot::Sender<()>,
    exited: oneshot::Receiver<()>,
    #[cfg(target_os = "macos")]
    retired: Arc<AtomicBool>,
}

type ListenerLifecycle = Mutex<Lifecycle<RunningListener>>;

/// The runtime owner creates one listener per service and shares that instance
/// among its callers. Dropping a caller does not stop a listener it started.
#[derive(Default)]
pub struct RoutingListener {
    lifecycle: Arc<ListenerLifecycle>,
    // Shared with management requests so shutdown closes the control plane
    // before draining the single listener.
    control_closed: Arc<tokio::sync::Mutex<bool>>,
    pub(super) auth: Arc<RouteAuth>,
    #[cfg(target_os = "macos")]
    aggregation_lifecycle:
        Mutex<Option<crate::gateway::aggregation_lifecycle::AggregationLifecycle>>,
    #[cfg(target_os = "macos")]
    aggregation_resume: Arc<tokio::sync::Mutex<()>>,
    #[cfg(target_os = "macos")]
    egress: Arc<EgressState>,
    body_limits: BodyLimits,
    logging_enabled: Arc<AtomicBool>,
}

impl RoutingListener {
    #[cfg(target_os = "macos")]
    pub(crate) fn set_failover_events(
        &self,
        events: tokio::sync::broadcast::Sender<serde_json::Value>,
    ) {
        self.auth.set_failover_events(events);
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn set_aggregation_lifecycle(
        &self,
        context: crate::gateway::aggregation_lifecycle::AggregationLifecycle,
    ) {
        *self.aggregation_lifecycle.lock() = Some(context);
    }

    /// Each publisher obtains the existing control permit itself. Do not hold
    /// that permit while resuming. Closing the gateway prevents late publication.
    #[cfg(target_os = "macos")]
    pub(crate) async fn resume_aggregations(self: &Arc<Self>) -> Result<(), ListenerError> {
        let context = self.aggregation_lifecycle.lock().clone();
        let Some(context) = context else {
            return Ok(());
        };
        let permit = Arc::clone(&self.aggregation_resume).lock_owned().await;
        let routing = Arc::clone(self);
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            context.resume(routing)
        })
        .await
        .map_err(|_| {
            ListenerError::AggregationRecovery(axum::http::StatusCode::SERVICE_UNAVAILABLE)
        })?
        .map_err(ListenerError::AggregationRecovery)
    }

    /// The management caller holds its control permit throughout detach/drain;
    /// gateway shutdown closes that permit before reaching this hook.
    #[cfg(target_os = "macos")]
    pub(crate) async fn detach_aggregations(
        &self,
    ) -> Result<Vec<vw_api_types::agent_apps_high_availability::FailoverQueueScope>, ListenerError>
    {
        let context = self.aggregation_lifecycle.lock().clone();
        let Some(context) = context else {
            return Ok(vec![]);
        };
        let result = tokio::task::spawn_blocking(move || context.detach()).await.map_err(|_| {
            ListenerError::AggregationRecovery(axum::http::StatusCode::SERVICE_UNAVAILABLE)
        })?;
        for scope in &result.retired {
            self.auth
                .restore_provider(scope, || Ok(()))
                .map_err(ListenerError::AggregationRecovery)?;
        }
        if let Some(status) = result.failure {
            return Err(ListenerError::AggregationRecovery(status));
        }
        Ok(result.retired)
    }

    /// Check the current listener and its credential owner without issuing or starting anything.
    #[cfg(target_os = "macos")]
    pub(crate) fn prepare_route(
        &self,
        binding: &route_auth::VerifiedRouteBinding,
    ) -> anyhow::Result<()> {
        let _permit = self.try_control()?;
        anyhow::ensure!(self.bound_address().is_some(), "application routing listener is stopped");
        self.auth.prepare(binding)
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn publish_provider(
        &self,
        binding: super::VerifiedRouteBinding,
        target: super::dispatch::JsonTarget,
        commit: impl FnOnce(
            SocketAddr,
            &str,
        ) -> Result<
            super::provider_publication::PublishedProvider,
            super::RouteCommitError,
        >,
    ) -> Result<(), axum::http::StatusCode> {
        let _permit = self.try_control().map_err(|_| axum::http::StatusCode::CONFLICT)?;
        let address = self.bound_address().ok_or(axum::http::StatusCode::SERVICE_UNAVAILABLE)?;
        if !address.ip().is_loopback() {
            return Err(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        }
        self.auth.publish_provider(binding, target, |token| commit(address, token))
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn with_provider<T>(
        &self,
        scope: &vw_api_types::agent_apps_high_availability::FailoverQueueScope,
        operation: impl FnOnce(
            &super::provider_publication::PublishedProvider,
            u64,
        ) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        let _permit = self.try_control()?;
        let address = self
            .bound_address()
            .ok_or_else(|| anyhow::anyhow!("application listener is stopped"))?;
        self.auth.with_provider(scope, |provider, generation| {
            anyhow::ensure!(provider.address == address, "application route listener changed");
            operation(provider, generation)
        })
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn restore_provider(
        &self,
        scope: &vw_api_types::agent_apps_high_availability::FailoverQueueScope,
        commit: impl FnOnce() -> Result<(), super::RouteCommitError>,
    ) -> Result<(), axum::http::StatusCode> {
        let _permit = self.try_control().map_err(|_| axum::http::StatusCode::CONFLICT)?;
        self.auth.restore_provider(scope, commit)
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn aggregation_revision(
        &self,
        project: &vw_api_types::id::ProjectScope,
        host: &str,
        app: vw_api_types::agent_apps::AppId,
    ) -> Option<String> {
        self.auth.aggregation_revision(project, host, app)
    }

    /// Commit the private client token before making this snapshot reachable.
    /// The configuration owner must validate the same revision and use its existing
    /// atomic write transaction in the callback. It must not reenter the listener.
    #[cfg(target_os = "macos")]
    pub(crate) fn publish_aggregation(
        &self,
        publication: super::aggregation_routing::AggregationPublication,
        expected_revision: Option<&str>,
        commit: impl FnOnce(SocketAddr, &str) -> Result<(), super::RouteCommitError>,
    ) -> Result<super::RouteRevocation, axum::http::StatusCode> {
        use axum::http::StatusCode;
        let _permit = self.try_control().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let address = self.bound_address().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
        let binding = publication.binding.clone();
        let snapshot = publication.validate()?;
        self.auth.publish_aggregation(binding, snapshot, expected_revision, |token| {
            commit(address, token)
        })
    }

    /// Publish an ordinary queue-selected route through the same credential owner.
    #[cfg(target_os = "macos")]
    pub(crate) fn publish_failover_route(
        &self,
        publication: super::failover_routing::FailoverPublication,
        queue_store: &crate::gateway::agent_apps_high_availability_store::FailoverQueueStore,
        expected_revision: Option<&str>,
        commit: impl FnOnce(
            SocketAddr,
            &str,
            &crate::gateway::agent_apps_high_availability_store::FailoverQueueState,
            &str,
        ) -> Result<(), super::RouteCommitError>,
    ) -> Result<super::RouteRevocation, axum::http::StatusCode> {
        use axum::http::StatusCode;
        let _permit = self.try_control().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let address = self.bound_address().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
        let binding = publication.binding.clone();
        let scope = vw_api_types::agent_apps_high_availability::FailoverQueueScope {
            project: binding.project().clone(),
            target_host_id: binding.host().into(),
            app_id: binding.app(),
        };
        let queue = queue_store.get(&scope).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let (provider, target) = publication.select(&queue)?;
        self.auth.publish_plain(binding, target, expected_revision, |token| {
            if queue_store.get(&scope).ok().as_ref() != Some(&queue) {
                return Err(super::RouteCommitError {
                    status: StatusCode::CONFLICT,
                    recovery_pending: false,
                });
            }
            commit(address, token, &queue, &provider)
        })
    }

    #[cfg(target_os = "macos")]
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "awaits the aggregation configuration caller")
    )]
    pub(crate) fn revoke_aggregation(
        &self,
        handle: &super::RouteRevocation,
    ) -> Result<(), ListenerError> {
        let _permit = self.try_control()?;
        self.auth.revoke(handle);
        Ok(())
    }

    pub(super) async fn shutdown(&self) -> Result<(), ListenerError> {
        let mut closed = self.control_closed.lock().await;
        *closed = true;
        self.stop_controlled().await
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn try_control(&self) -> Result<tokio::sync::OwnedMutexGuard<bool>, ListenerError> {
        let guard = Arc::clone(&self.control_closed)
            .try_lock_owned()
            .map_err(|_| ListenerError::TransitionInProgress)?;
        if *guard {
            return Err(ListenerError::ShuttingDown);
        }
        Ok(guard)
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn has_active_routes(&self) -> bool {
        self.auth.has_active_routes()
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn snapshot(
        &self,
    ) -> (LifecycleState, Option<(SocketAddr, chrono::DateTime<chrono::Utc>)>) {
        let lifecycle = self.lifecycle.lock();
        let binding = match &*lifecycle {
            Lifecycle::Running(running) => Some((running.address, running.started_at)),
            _ => None,
        };
        (lifecycle.state(), binding)
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn set_egress(
        &self,
        tenant: TenantScope,
        host: String,
        egress: Option<RoutingEgress>,
    ) {
        self.egress.set(tenant, host, egress);
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn egress_status(
        &self,
        tenant: &TenantScope,
        host: &str,
    ) -> Option<RoutingEgressStatus> {
        self.configured_egress(tenant, host).map(|egress| egress.status.clone())
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn configured_egress(
        &self,
        tenant: &TenantScope,
        host: &str,
    ) -> Option<Arc<RoutingEgress>> {
        self.egress.for_scope(tenant, host).ok().flatten()
    }

    pub fn state(&self) -> LifecycleState {
        self.lifecycle.lock().state()
    }

    pub fn bound_address(&self) -> Option<SocketAddr> {
        match &*self.lifecycle.lock() {
            Lifecycle::Running(running) => Some(running.address),
            _ => None,
        }
    }

    /// Start from an I/O-enabled Tokio runtime. Repeated starts return the
    /// address of the current listener.
    pub fn start(&self, port: u16) -> Result<SocketAddr, ListenerError> {
        self.start_http(SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port))
    }

    /// Bind a loopback IP literal, with optional brackets around IPv6.
    /// Port zero asks the OS to choose an available port.
    pub fn start_on(&self, host: &str, port: u16) -> Result<SocketAddr, ListenerError> {
        let ip = match host.strip_prefix('[').and_then(|host| host.strip_suffix(']')) {
            Some(ipv6) => ipv6.parse().map(IpAddr::V6),
            None => host.parse(),
        }
        .map_err(|_| ListenerError::InvalidAddress)?;
        self.start_http(SocketAddr::new(ip.to_canonical(), port))
    }

    fn start_http(&self, address: SocketAddr) -> Result<SocketAddr, ListenerError> {
        let auth = Arc::clone(&self.auth);
        #[cfg(target_os = "macos")]
        let egress = Arc::clone(&self.egress);
        let logging = Arc::clone(&self.logging_enabled);
        let body_limits = self.body_limits.clone();
        self.start_at(address, move |connection| {
            Some(serve_http(
                connection,
                Arc::clone(&auth),
                #[cfg(target_os = "macos")]
                Arc::clone(&egress),
                Arc::clone(&logging),
                body_limits.clone(),
            ))
        })
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn start_configured(
        &self,
        config: super::config_apply::ListenerConfig,
    ) -> Result<SocketAddr, ListenerError> {
        let address = self.start_http(config.bind)?;
        self.logging_enabled.store(config.logging_enabled, Ordering::Relaxed);
        Ok(address)
    }

    #[cfg(all(test, target_os = "macos"))]
    pub(super) fn logging_enabled(&self) -> bool {
        self.logging_enabled.load(Ordering::Relaxed)
    }

    #[cfg(all(test, target_os = "macos"))]
    pub(super) fn available_body_permits(&self) -> usize {
        self.body_limits.available()
    }

    /// The gateway control permit must cover the entire write and publication.
    #[cfg(target_os = "macos")]
    pub(super) fn apply_config(
        &self,
        current: super::config_apply::ListenerConfig,
        config: super::config_apply::ListenerConfig,
        commit: impl FnOnce() -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        let replacement = match self.bound_address() {
            Some(bound) if current.bind != config.bind && bound != config.bind => {
                if self.has_active_routes() {
                    return Err(ListenerError::AppsAttached.into());
                }
                let runtime =
                    Handle::try_current().map_err(|_| ListenerError::RuntimeUnavailable)?;
                let socket = bind_listener(config.bind)?;
                let address = socket.local_addr()?;
                Some((runtime, socket, address))
            }
            _ => None,
        };
        // A failed bind or write leaves the original task and its configuration intact.
        commit()?;
        self.logging_enabled.store(config.logging_enabled, Ordering::Relaxed);
        if let Some((runtime, socket, address)) = replacement {
            let auth = Arc::clone(&self.auth);
            let egress = Arc::clone(&self.egress);
            let logging = Arc::clone(&self.logging_enabled);
            let body_limits = self.body_limits.clone();
            let mut lifecycle = self.lifecycle.lock();
            // The listener may have exited on its own while the database write ran.
            let previous = match std::mem::replace(&mut *lifecycle, Lifecycle::Starting) {
                Lifecycle::Running(running) => {
                    running.retired.store(true, Ordering::Relaxed);
                    drop(running.shutdown);
                    Some(running.exited)
                }
                Lifecycle::Failed => None,
                _ => unreachable!("the control permit serializes listener transitions"),
            };
            let (running, task) = self.prepare_task(socket, address);
            lifecycle.finish_start(Some(running));
            drop(lifecycle);
            runtime.spawn(async move {
                if let Some(exited) = previous {
                    let _ = exited.await;
                }
                task.serve(move |connection| {
                    Some(serve_http(
                        connection,
                        Arc::clone(&auth),
                        Arc::clone(&egress),
                        Arc::clone(&logging),
                        body_limits.clone(),
                    ))
                })
                .await;
            });
        }
        Ok(())
    }

    /// Connection futures own request handlers and response bodies so stopping
    /// the listener can cancel and join all work they perform.
    pub(super) fn start_at<F, Fut>(
        &self,
        address: SocketAddr,
        serve_connection: F,
    ) -> Result<SocketAddr, ListenerError>
    where
        F: Fn(TcpStream) -> Option<Fut> + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        // Exposing routing beyond loopback requires explicit host authorization.
        if !address.ip().is_loopback() {
            return Err(ListenerError::NonLoopbackDenied);
        }

        if let Some(running) =
            self.lifecycle.lock().begin_start().map_err(|_| ListenerError::TransitionInProgress)?
        {
            return Ok(running.address);
        }

        let result = self.bind_and_spawn(address, serve_connection);
        if result.is_err() {
            self.lifecycle.lock().finish_start(None);
        }
        result
    }

    pub async fn stop(&self) -> Result<(), ListenerError> {
        let _permit = self.control_closed.lock().await;
        self.stop_controlled().await
    }

    /// Only callers already holding the gateway control permit use this entry.
    pub(crate) async fn stop_controlled(&self) -> Result<(), ListenerError> {
        #[cfg(target_os = "macos")]
        let restore = self.detach_aggregations().await;
        let running =
            self.lifecycle.lock().begin_stop().map_err(|_| ListenerError::TransitionInProgress)?;
        #[cfg(target_os = "macos")]
        self.auth.revoke_providers();
        if let Some(RunningListener { shutdown, exited, .. }) = running {
            drop(shutdown);
            let _ = timeout(shutdown::STOP_TIMEOUT, exited)
                .await
                .map_err(|_| ListenerError::StopTimeout)?;
        }
        #[cfg(target_os = "macos")]
        restore?;
        Ok(())
    }

    fn bind_and_spawn<F, Fut>(
        &self,
        requested: SocketAddr,
        serve_connection: F,
    ) -> Result<SocketAddr, ListenerError>
    where
        F: Fn(TcpStream) -> Option<Fut> + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let runtime = Handle::try_current().map_err(|_| ListenerError::RuntimeUnavailable)?;
        let socket = bind_listener(requested)?;
        let address = socket.local_addr()?;
        let (running, task) = self.prepare_task(socket, address);
        self.lifecycle.lock().finish_start(Some(running));
        runtime.spawn(task.serve(serve_connection));
        Ok(address)
    }

    fn prepare_task(
        &self,
        socket: TcpListener,
        address: SocketAddr,
    ) -> (RunningListener, ListenerTask) {
        let (shutdown, stop) = oneshot::channel();
        let (completion, exited) = oneshot::channel();
        #[cfg(target_os = "macos")]
        let retired = Arc::new(AtomicBool::new(false));
        let task = ListenerTask {
            socket,
            stop,
            _on_exit: ListenerExit {
                lifecycle: Arc::downgrade(&self.lifecycle),
                _completion: completion,
                #[cfg(target_os = "macos")]
                retired: Arc::clone(&retired),
            },
        };
        let running = RunningListener {
            address,
            #[cfg(target_os = "macos")]
            started_at: chrono::Utc::now(),
            shutdown,
            exited,
            #[cfg(target_os = "macos")]
            retired,
        };
        (running, task)
    }
}

async fn serve_http(
    connection: TcpStream,
    auth: Arc<RouteAuth>,
    #[cfg(target_os = "macos")] egress: Arc<EgressState>,
    logging: Arc<AtomicBool>,
    body_limits: BodyLimits,
) {
    let Ok(local_addr) = connection.local_addr() else {
        return;
    };
    route_auth::serve(connection, auth, |request, route| {
        #[cfg(target_os = "macos")]
        let egress = Arc::clone(&egress);
        let logging = logging.load(Ordering::Relaxed);
        let body_limits = body_limits.clone();
        async move {
            if logging {
                tracing::info!("Authorized application routing request");
            }
            let protocol = Protocol::for_path(request.uri().path());
            dispatch::forward_json(
                request,
                &route,
                &body_limits,
                local_addr,
                #[cfg(target_os = "macos")]
                &egress,
            )
            .await
            .unwrap_or_else(|error| error.response(protocol))
        }
    })
    .await;
}

fn bind_listener(requested: SocketAddr) -> Result<TcpListener, ListenerError> {
    let socket = if requested.is_ipv4() { TcpSocket::new_v4()? } else { TcpSocket::new_v6()? };
    // Allow restarting on the same Unix port while closed connections are in TIME_WAIT.
    #[cfg(unix)]
    socket.set_reuseaddr(true)?;
    socket.bind(requested).and_then(|()| socket.listen(128)).map_err(|error| {
        if error.kind() == io::ErrorKind::AddrInUse {
            ListenerError::AddressInUse
        } else {
            ListenerError::Bind(error)
        }
    })
}

struct ListenerTask {
    // Field order releases the socket before publishing the terminal state.
    socket: TcpListener,
    stop: oneshot::Receiver<()>,
    _on_exit: ListenerExit,
}

impl ListenerTask {
    async fn serve<F, Fut>(self, serve_connection: F)
    where
        F: Fn(TcpStream) -> Option<Fut>,
        Fut: Future<Output = ()> + Send + 'static,
    {
        shutdown::serve(self.socket, self.stop, serve_connection).await;
    }
}

struct ListenerExit {
    lifecycle: Weak<ListenerLifecycle>,
    // Closing the channel signals stop after the socket, tasks and state are settled.
    _completion: oneshot::Sender<()>,
    #[cfg(target_os = "macos")]
    retired: Arc<AtomicBool>,
}

impl Drop for ListenerExit {
    fn drop(&mut self) {
        if let Some(lifecycle) = self.lifecycle.upgrade() {
            let mut lifecycle = lifecycle.lock();
            #[cfg(target_os = "macos")]
            if self.retired.load(Ordering::Relaxed) {
                return;
            }
            lifecycle.listener_exited();
        }
    }
}
