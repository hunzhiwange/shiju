//! Server-owned client connections and bounded, single-use callback rendezvous.

use super::SessionScope;
use crate::tools::ToolSpec;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::Instant;
use vw_api_types::tools::{DynamicToolCallRequest, DynamicToolResponse};

pub const MAX_PENDING_CLIENT_CALLS: usize = 32;
pub const MAX_CLIENT_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const MAX_CLIENT_ARGUMENT_BYTES: usize = 1024 * 1024;
const CLIENT_CALL_TIMEOUT: Duration = Duration::from_secs(60);

/// Constructed by the authenticated server, never from callback payload fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicCallOwner {
    pub scope: SessionScope,
    pub session: String,
    pub host: String,
}

impl DynamicCallOwner {
    pub fn new(scope: SessionScope, session: &str, host: &str) -> anyhow::Result<Self> {
        if session.trim().is_empty() || host.trim().is_empty() {
            anyhow::bail!("client tool owner requires a session and host");
        }
        Ok(Self { scope, session: session.into(), host: host.into() })
    }
}

struct PendingCall {
    deadline: Instant,
    response: oneshot::Sender<DynamicToolResponse>,
}

enum ConnectionState {
    Open(HashMap<String, PendingCall>),
    Closed,
}

/// One authenticated host connection. Reconnecting creates a new instance.
pub struct ClientToolHost {
    owner: DynamicCallOwner,
    state: Mutex<ConnectionState>,
    requests: mpsc::Sender<DynamicToolCallRequest>,
}

/// Dropping the transport receiver disconnects the host and wakes all waiters.
pub struct ClientToolRequests {
    host: Weak<ClientToolHost>,
    receiver: mpsc::Receiver<DynamicToolCallRequest>,
}

impl ClientToolRequests {
    pub async fn recv(&mut self) -> Option<DynamicToolCallRequest> {
        self.receiver.recv().await
    }
}

impl Drop for ClientToolRequests {
    fn drop(&mut self) {
        if let Some(host) = self.host.upgrade() {
            host.disconnect();
        }
    }
}

impl ClientToolHost {
    pub fn register(owner: DynamicCallOwner) -> (Arc<Self>, ClientToolRequests) {
        let (requests, receiver) = mpsc::channel(MAX_PENDING_CLIENT_CALLS);
        let host = Arc::new(Self {
            owner,
            state: Mutex::new(ConnectionState::Open(HashMap::new())),
            requests,
        });
        let receiver = ClientToolRequests { host: Arc::downgrade(&host), receiver };
        (host, receiver)
    }

    pub fn owner(&self) -> &DynamicCallOwner {
        &self.owner
    }

    pub fn disconnect(&self) {
        *self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner) =
            ConnectionState::Closed;
    }

    /// The caller supplies the owner derived from its authenticated connection.
    pub fn respond(
        &self,
        owner: &DynamicCallOwner,
        call_id: &str,
        response: DynamicToolResponse,
    ) -> anyhow::Result<()> {
        if owner != &self.owner {
            anyhow::bail!("client tool response owner mismatch");
        }
        if serde_json::to_vec(&response)?.len() > MAX_CLIENT_RESPONSE_BYTES {
            anyhow::bail!("client tool response exceeds byte limit");
        }
        let mut state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let ConnectionState::Open(pending) = &mut *state else {
            anyhow::bail!("client tool host disconnected");
        };
        let Some(call) = pending.remove(call_id) else {
            anyhow::bail!("unknown or completed client tool call");
        };
        if Instant::now() >= call.deadline {
            anyhow::bail!("client tool call expired");
        }
        call.response.send(response).map_err(|_| anyhow::anyhow!("client tool call cancelled"))
    }

    pub(crate) async fn request(
        self: &Arc<Self>,
        spec: &ToolSpec,
        arguments: Value,
        turn_id: String,
    ) -> anyhow::Result<DynamicToolResponse> {
        self.request_with_timeout(spec, arguments, turn_id, CLIENT_CALL_TIMEOUT).await
    }

    pub(super) async fn request_with_timeout(
        self: &Arc<Self>,
        spec: &ToolSpec,
        arguments: Value,
        turn_id: String,
        timeout: Duration,
    ) -> anyhow::Result<DynamicToolResponse> {
        if serde_json::to_vec(&arguments)?.len() > MAX_CLIENT_ARGUMENT_BYTES {
            anyhow::bail!("client tool arguments exceed byte limit");
        }
        let call_id = uuid::Uuid::new_v4().to_string();
        let (response, receiver) = oneshot::channel();
        let deadline = Instant::now() + timeout;
        {
            let mut state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let ConnectionState::Open(pending) = &mut *state else {
                anyhow::bail!("client tool host disconnected");
            };
            if pending.len() >= MAX_PENDING_CLIENT_CALLS {
                anyhow::bail!("client tool in-flight limit reached");
            }
            pending.insert(call_id.clone(), PendingCall { deadline, response });
            if self
                .requests
                .try_send(DynamicToolCallRequest {
                    call_id: call_id.clone(),
                    turn_id,
                    tool_id: spec.id.clone().into(),
                    identity: spec.identity.clone(),
                    arguments,
                })
                .is_err()
            {
                pending.remove(&call_id);
                anyhow::bail!("client tool host disconnected or queue full");
            }
        }
        let _guard = PendingGuard { host: self.clone(), call_id };
        tokio::time::timeout_at(deadline, receiver)
            .await
            .map_err(|_| anyhow::anyhow!("client tool response timed out"))?
            .map_err(|_| anyhow::anyhow!("client tool host disconnected"))
    }

    #[cfg(test)]
    pub(super) fn pending_count(&self) -> usize {
        match &*self.state.lock().unwrap() {
            ConnectionState::Open(pending) => pending.len(),
            ConnectionState::Closed => 0,
        }
    }
}

struct PendingGuard {
    host: Arc<ClientToolHost>,
    call_id: String,
}

impl Drop for PendingGuard {
    fn drop(&mut self) {
        let mut state = self.host.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let ConnectionState::Open(pending) = &mut *state {
            pending.remove(&self.call_id);
        }
    }
}
