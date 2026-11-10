//! Owns the application's loopback listener independently of session relays.

use super::{
    route_auth::{self, RouteAuth},
    shutdown,
};
use axum::response::IntoResponse;
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
    logging_enabled: Arc<AtomicBool>,
}

impl RoutingListener {
    pub(super) async fn shutdown(&self) -> Result<(), ListenerError> {
        let mut closed = self.control_closed.lock().await;
        *closed = true;
        self.stop().await
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
        let logging = Arc::clone(&self.logging_enabled);
        self.start_at(address, move |connection| {
            Some(serve_http(connection, Arc::clone(&auth), Arc::clone(&logging)))
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
            let logging = Arc::clone(&self.logging_enabled);
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
                    Some(serve_http(connection, Arc::clone(&auth), Arc::clone(&logging)))
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
        let running =
            self.lifecycle.lock().begin_stop().map_err(|_| ListenerError::TransitionInProgress)?;
        if let Some(RunningListener { shutdown, exited, .. }) = running {
            drop(shutdown);
            let _ = timeout(shutdown::STOP_TIMEOUT, exited)
                .await
                .map_err(|_| ListenerError::StopTimeout)?;
        }
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

async fn serve_http(connection: TcpStream, auth: Arc<RouteAuth>, logging: Arc<AtomicBool>) {
    route_auth::serve(connection, auth, |_, _| async {
        if logging.load(Ordering::Relaxed) {
            tracing::info!("Authorized application routing request");
        }
        axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response()
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
