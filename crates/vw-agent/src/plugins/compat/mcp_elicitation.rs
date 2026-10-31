//! Host-only form interaction. No model tool can provide a user response.
use super::mcp_form_schema::FormSchema;
use crate::session::ui_types::{McpFormAction, McpFormEvent, McpFormRequest, McpFormResponse};
use anyhow::{Result, ensure};
use rmcp::model::{CreateElicitationResult, ElicitationAction, ErrorData, RequestId};
use serde_json::Value;
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex, OnceLock, Weak},
    time::Duration,
};
use tokio::sync::{broadcast, oneshot};
use tokio_util::sync::CancellationToken;

const FORM_TIMEOUT: Duration = Duration::from_secs(120);
pub(super) const MAX_PENDING: usize = 16;

struct Pending {
    request: McpFormRequest,
    schema: FormSchema,
    response: oneshot::Sender<CreateElicitationResult>,
    request_closed: CancellationToken,
    peer_cancelled: CancellationToken,
    operation_closed: CancellationToken,
    connection_closed: CancellationToken,
    deadline: tokio::time::Instant,
}
struct State {
    session: String,
    tenant: vw_api_types::id::TenantScope,
    events: broadcast::Sender<McpFormEvent>,
    pending: Mutex<HashMap<String, Pending>>,
    closed: CancellationToken,
}
struct Lease {
    generation: String,
    state: Arc<State>,
}
impl Drop for Lease {
    fn drop(&mut self) {
        self.state.closed.cancel();
    }
}
fn registry() -> &'static Mutex<HashMap<String, Weak<Lease>>> {
    static HANDLERS: OnceLock<Mutex<HashMap<String, Weak<Lease>>>> = OnceLock::new();
    HANDLERS.get_or_init(Default::default)
}

/// A trusted host's live UI handler lease, scoped to a tenant, workspace and chat.
/// Subscribe before connecting. No UI lease means no advertised form capability.
#[derive(Clone)]
pub struct McpFormSession(Arc<Lease>);
impl McpFormSession {
    pub fn register(session: &str, workspace: &Path) -> Result<Self> {
        ensure!(!session.trim().is_empty(), "MCP form session must not be empty");
        let workspace = workspace.canonicalize()?;
        let key = super::mcp_connection_sessions::key(session, &workspace);
        let mut registry = registry().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        registry.retain(|_, handler| handler.strong_count() != 0);
        ensure!(
            !registry.contains_key(&key),
            "MCP form handler is already installed for this session"
        );
        let (events, _) = broadcast::channel(32);
        let handler = Arc::new(Lease {
            generation: uuid::Uuid::new_v4().to_string(),
            state: Arc::new(State {
                session: session.to_owned(),
                tenant: crate::session::ui_store::current_tenant_scope(),
                events,
                pending: Mutex::new(HashMap::new()),
                closed: CancellationToken::new(),
            }),
        });
        registry.insert(key, Arc::downgrade(&handler));
        Ok(Self(handler))
    }
    pub fn subscribe(&self) -> broadcast::Receiver<McpFormEvent> {
        self.0.state.events.subscribe()
    }
    /// Reconcile a lagged event receiver without persisting server payloads.
    pub fn pending(&self) -> Vec<McpFormRequest> {
        if self.0.state.tenant != crate::session::ui_store::current_tenant_scope() {
            return Vec::new();
        }
        let mut requests = self
            .0
            .state
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .values()
            .map(|pending| pending.request.clone())
            .collect::<Vec<_>>();
        requests.sort_by(|a, b| a.id.cmp(&b.id));
        requests
    }
    /// Only the UI holding this lease may answer; there is deliberately no model tool.
    /// Invalid input keeps the exact pending request available for correction.
    pub fn respond(&self, request: &McpFormRequest, response: McpFormResponse) -> Result<()> {
        let state = &self.0.state;
        ensure!(
            state.tenant == crate::session::ui_store::current_tenant_scope(),
            "MCP form authority changed"
        );
        let mut pending = state.pending.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let entry = pending
            .get(&request.id)
            .ok_or_else(|| anyhow::anyhow!("MCP form request is not pending"))?;
        ensure!(entry.request == *request, "MCP form request authority does not match");
        ensure!(
            !state.closed.is_cancelled()
                && !entry.request_closed.is_cancelled()
                && !entry.peer_cancelled.is_cancelled()
                && !entry.operation_closed.is_cancelled()
                && !entry.connection_closed.is_cancelled()
                && tokio::time::Instant::now() < entry.deadline,
            "MCP form request has expired"
        );
        let action =
            match response.action {
                McpFormAction::Accept => {
                    entry.schema.validate(response.content.as_ref().ok_or_else(|| {
                        anyhow::anyhow!("MCP form acceptance requires content")
                    })?)?;
                    ElicitationAction::Accept
                }
                McpFormAction::Decline | McpFormAction::Cancel => {
                    ensure!(
                        response.content.is_none(),
                        "MCP form decline/cancel cannot contain answers"
                    );
                    if response.action == McpFormAction::Decline {
                        ElicitationAction::Decline
                    } else {
                        ElicitationAction::Cancel
                    }
                }
            };
        let entry = pending.remove(&request.id).expect("pending request checked under lock");
        entry
            .response
            .send(CreateElicitationResult { action, content: response.content })
            .map_err(|_| anyhow::anyhow!("MCP form responder is closed"))
    }
}

#[derive(Clone)]
pub(crate) struct FormHost {
    lease: Weak<Lease>,
    generation: String,
}
pub(crate) fn current_host() -> Option<FormHost> {
    let context = crate::tools::current_tool_use_context()?;
    let root = context.workspace_root()?;
    let key = super::mcp_connection_sessions::key(context.session(), &root);
    let lease = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(&key)?
        .upgrade()?;
    if lease.state.closed.is_cancelled() || lease.state.events.receiver_count() == 0 {
        return None;
    }
    Some(FormHost { lease: Arc::downgrade(&lease), generation: lease.generation.clone() })
}
impl FormHost {
    pub(crate) fn generation(&self) -> &str {
        &self.generation
    }
    pub(crate) fn bind(&self, server: &str, closed: CancellationToken) -> FormBinding {
        FormBinding {
            host: self.clone(),
            server: server.to_owned(),
            connection: uuid::Uuid::new_v4().to_string(),
            closed,
            operation: Arc::new(Mutex::new(None)),
        }
    }
}
#[derive(Clone)]
pub(crate) struct FormBinding {
    host: FormHost,
    server: String,
    connection: String,
    closed: CancellationToken,
    operation: Arc<Mutex<Option<CancellationToken>>>,
}
pub(crate) struct FormOperation(CancellationToken);
impl Drop for FormOperation {
    fn drop(&mut self) {
        self.0.cancel();
    }
}
impl FormBinding {
    pub(crate) fn available(&self) -> bool {
        self.host.lease.upgrade().is_some_and(|lease| {
            !lease.state.closed.is_cancelled() && lease.state.events.receiver_count() != 0
        })
    }
    pub(crate) fn activate(&self) -> FormOperation {
        let token = self.closed.child_token();
        if let Some(previous) = self
            .operation
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .replace(token.clone())
        {
            previous.cancel();
        }
        FormOperation(token)
    }
    pub(crate) async fn request(
        &self,
        raw: Value,
        id: RequestId,
        cancelled: CancellationToken,
        peer_cancelled: CancellationToken,
        wire: &super::mcp_resources::RawResponse,
    ) -> Result<CreateElicitationResult, ErrorData> {
        let invalid = || ErrorData::invalid_params("unsupported or invalid MCP form request", None);
        let lease = self.host.lease.upgrade().ok_or_else(|| {
            ErrorData::invalid_request("MCP form UI handler is unavailable", None)
        })?;
        let state = lease.state.clone();
        // Do not let a pending request keep the UI lease alive after the host drops it.
        drop(lease);
        if state.closed.is_cancelled() || state.events.receiver_count() == 0 {
            return Err(ErrorData::invalid_request("MCP form UI handler is unavailable", None));
        }
        let operation = self
            .operation
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
            .filter(|token| !token.is_cancelled())
            .ok_or_else(|| {
                ErrorData::invalid_request("MCP form has no active session operation", None)
            })?;
        let params = raw.as_object().ok_or_else(invalid)?;
        if params
            .keys()
            .any(|key| !matches!(key.as_str(), "message" | "requestedSchema" | "mode" | "_meta"))
            || params.get("mode").is_some_and(|mode| mode != "form")
            || params.get("_meta").is_some_and(|meta| !meta.is_object())
        {
            return Err(invalid());
        }
        let message = params
            .get("message")
            .and_then(Value::as_str)
            .filter(|m| m.len() <= 8192)
            .ok_or_else(invalid)?;
        let schema = params.get("requestedSchema").ok_or_else(invalid)?;
        let parsed = FormSchema::parse(schema).map_err(|_| invalid())?;
        let request = McpFormRequest {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: state.session.clone(),
            server_name: self.server.clone(),
            connection_id: self.connection.clone(),
            server_request_id: serde_json::to_value(&id).map_err(|_| invalid())?,
            message: message.to_owned(),
            requested_schema: schema.clone(),
            meta: params.get("_meta").cloned(),
        };
        let deadline = tokio::time::Instant::now() + FORM_TIMEOUT;
        let (tx, rx) = oneshot::channel();
        let _guard;
        {
            let mut pending =
                state.pending.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if pending.len() >= MAX_PENDING
                || pending.values().any(|entry| {
                    entry.request.connection_id == self.connection
                        && entry.request.server_request_id == request.server_request_id
                })
            {
                return Err(ErrorData::invalid_request(
                    "MCP form request limit or duplicate request ID",
                    None,
                ));
            }
            pending.insert(
                request.id.clone(),
                Pending {
                    request: request.clone(),
                    schema: parsed,
                    response: tx,
                    request_closed: cancelled.clone(),
                    peer_cancelled: peer_cancelled.clone(),
                    operation_closed: operation.clone(),
                    connection_closed: self.closed.clone(),
                    deadline,
                },
            );
            _guard = PendingGuard { state: state.clone(), id: request.id.clone() };
            if state.events.send(McpFormEvent::Requested { request }).is_err() {
                return Err(ErrorData::invalid_request("MCP form UI handler is unavailable", None));
            }
        }
        let cancel =
            || CreateElicitationResult { action: ElicitationAction::Cancel, content: None };
        let mut response = tokio::select! {
            biased;
            _ = cancelled.cancelled() => cancel(),
            _ = peer_cancelled.cancelled() => cancel(),
            _ = self.closed.cancelled() => cancel(),
            _ = operation.cancelled() => cancel(),
            _ = state.closed.cancelled() => cancel(),
            _ = tokio::time::sleep_until(deadline) => cancel(),
            response = rx => response.unwrap_or_else(|_| cancel()),
        };
        if response.content.is_some() {
            // rmcp logs typed handler results at debug/trace level. Keep user data outside
            // that model and restore it only at the owned transport's last send boundary.
            let answer = super::mcp_form_wire::FormWireAnswer {
                result: serde_json::to_value(&response).map_err(|_| {
                    ErrorData::internal_error("MCP form response encoding failed", None)
                })?,
                request_closed: peer_cancelled,
                operation_closed: operation,
                connection_closed: self.closed.clone(),
                ui_closed: state.closed.clone(),
                deadline,
            };
            wire.store_form_answer(&id, answer).map_err(|_| {
                ErrorData::internal_error("MCP form response could not be queued", None)
            })?;
            response.content = None;
        }
        Ok(response)
    }
}
struct PendingGuard {
    state: Arc<State>,
    id: String,
}
impl Drop for PendingGuard {
    fn drop(&mut self) {
        self.state
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&self.id);
        let _ = self.state.events.send(McpFormEvent::Finished { request_id: self.id.clone() });
    }
}
