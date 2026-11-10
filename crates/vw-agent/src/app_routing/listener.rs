//! Owns the application's loopback listener independently of session relays.

use parking_lot::Mutex;
use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Weak},
};
use tokio::{net::TcpListener, runtime::Handle, sync::oneshot};
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
        self.start_at(SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port))
    }

    /// Bind a loopback IP literal, with optional brackets around IPv6.
    /// Port zero asks the OS to choose an available port.
    pub fn start_on(&self, host: &str, port: u16) -> Result<SocketAddr, ListenerError> {
        let ip = match host.strip_prefix('[').and_then(|host| host.strip_suffix(']')) {
            Some(ipv6) => ipv6.parse().map(IpAddr::V6),
            None => host.parse(),
        }
        .map_err(|_| ListenerError::InvalidAddress)?;
        self.start_at(SocketAddr::new(ip.to_canonical(), port))
    }

    fn start_at(&self, address: SocketAddr) -> Result<SocketAddr, ListenerError> {
        // There is no authenticated application data plane yet.
        if !address.ip().is_loopback() {
            return Err(ListenerError::NonLoopbackDenied);
        }

        if let Some(running) =
            self.lifecycle.lock().begin_start().map_err(|_| ListenerError::TransitionInProgress)?
        {
            return Ok(running.address);
        }

        let result = self.bind_and_spawn(address);
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
            let _ = exited.await;
        }
        Ok(())
    }

    fn bind_and_spawn(&self, requested: SocketAddr) -> Result<SocketAddr, ListenerError> {
        let runtime = Handle::try_current().map_err(|_| ListenerError::RuntimeUnavailable)?;
        let socket = std::net::TcpListener::bind(requested).map_err(|error| {
            if error.kind() == io::ErrorKind::AddrInUse {
                ListenerError::AddressInUse
            } else {
                ListenerError::Bind(error)
            }
        })?;
        socket.set_nonblocking(true)?;
        let socket = TcpListener::from_std(socket)?;
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
        runtime.spawn(task.serve(stop));
        Ok(address)
    }
}

struct ListenerTask {
    // Field order releases the socket before publishing the terminal state.
    socket: TcpListener,
    _on_exit: ListenerExit,
}

impl ListenerTask {
    async fn serve(self, mut stop: oneshot::Receiver<()>) {
        loop {
            tokio::select! {
                biased;
                _ = &mut stop => return,
                accepted = self.socket.accept() => {
                    let Ok((connection, _)) = accepted else { return };
                    // Application authentication and dispatch are pending. Close
                    // every connection before parsing or forwarding any bytes.
                    drop(connection);
                }
            }
        }
    }
}

struct ListenerExit {
    lifecycle: Weak<ListenerLifecycle>,
    // Closing the channel signals stop after the socket and state are settled.
    _completion: oneshot::Sender<()>,
}

impl Drop for ListenerExit {
    fn drop(&mut self) {
        if let Some(lifecycle) = self.lifecycle.upgrade() {
            lifecycle.lock().listener_exited();
        }
    }
}
