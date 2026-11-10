//! Owns the application's loopback listener independently of session relays.

use super::shutdown;
use parking_lot::Mutex;
use std::{
    future::{Future, Ready},
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Weak},
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
    #[error("the routing listener requires an active Tokio runtime")]
    RuntimeUnavailable,
    #[error("the routing listener address must be an IPv4 or IPv6 literal")]
    InvalidAddress,
    #[error("non-loopback routing requires host authorization and data-plane authentication")]
    NonLoopbackDenied,
    #[error("the routing listener address is already in use")]
    AddressInUse,
    #[error("could not bind the routing listener: {0}")]
    Bind(#[from] io::Error),
    #[error("the routing listener timed out waiting for connection tasks to stop")]
    StopTimeout,
}

struct RunningListener {
    address: SocketAddr,
    shutdown: oneshot::Sender<()>,
    exited: oneshot::Receiver<()>,
}

type ListenerLifecycle = Mutex<Lifecycle<RunningListener>>;

/// The runtime owner creates one listener per service and shares that instance
/// among its callers. Dropping a caller does not stop a listener it started.
#[derive(Default)]
pub struct RoutingListener {
    lifecycle: Arc<ListenerLifecycle>,
}

impl RoutingListener {
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
        self.start_at(SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port), refuse_connection)
    }

    /// Bind a loopback IP literal, with optional brackets around IPv6.
    /// Port zero asks the OS to choose an available port.
    pub fn start_on(&self, host: &str, port: u16) -> Result<SocketAddr, ListenerError> {
        let ip = match host.strip_prefix('[').and_then(|host| host.strip_suffix(']')) {
            Some(ipv6) => ipv6.parse().map(IpAddr::V6),
            None => host.parse(),
        }
        .map_err(|_| ListenerError::InvalidAddress)?;
        self.start_at(SocketAddr::new(ip.to_canonical(), port), refuse_connection)
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
        // There is no authenticated application data plane yet.
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
        let socket = bind_listener(requested).map_err(|error| {
            if error.kind() == io::ErrorKind::AddrInUse {
                ListenerError::AddressInUse
            } else {
                ListenerError::Bind(error)
            }
        })?;
        let address = socket.local_addr()?;
        let (shutdown, stop) = oneshot::channel();
        let (completion, exited) = oneshot::channel();
        let task = ListenerTask {
            socket,
            _on_exit: ListenerExit {
                lifecycle: Arc::downgrade(&self.lifecycle),
                _completion: completion,
            },
        };
        self.lifecycle.lock().finish_start(Some(RunningListener { address, shutdown, exited }));
        runtime.spawn(task.serve(stop, serve_connection));
        Ok(address)
    }
}

fn refuse_connection(_: TcpStream) -> Option<Ready<()>> {
    // Application authentication and dispatch are pending.
    None
}

fn bind_listener(requested: SocketAddr) -> io::Result<TcpListener> {
    let socket = if requested.is_ipv4() { TcpSocket::new_v4()? } else { TcpSocket::new_v6()? };
    // Allow restarting on the same Unix port while closed connections are in TIME_WAIT.
    #[cfg(unix)]
    socket.set_reuseaddr(true)?;
    socket.bind(requested)?;
    socket.listen(128)
}

struct ListenerTask {
    // Field order releases the socket before publishing the terminal state.
    socket: TcpListener,
    _on_exit: ListenerExit,
}

impl ListenerTask {
    async fn serve<F, Fut>(self, stop: oneshot::Receiver<()>, serve_connection: F)
    where
        F: Fn(TcpStream) -> Option<Fut>,
        Fut: Future<Output = ()> + Send + 'static,
    {
        shutdown::serve(self.socket, stop, serve_connection).await;
    }
}

struct ListenerExit {
    lifecycle: Weak<ListenerLifecycle>,
    // Closing the channel signals stop after the socket, tasks and state are settled.
    _completion: oneshot::Sender<()>,
}

impl Drop for ListenerExit {
    fn drop(&mut self) {
        if let Some(lifecycle) = self.lifecycle.upgrade() {
            lifecycle.lock().listener_exited();
        }
    }
}
