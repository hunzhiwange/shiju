//! Gateway-owned CLI workbench sessions and request projections.
use std::{collections::HashMap, path::Path as FsPath, sync::LazyLock, time::Duration};

use axum::{
    Extension, Json, Router,
    extract::{Path, Query},
    http::StatusCode,
    middleware::from_fn,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use gateway_session::stream_host::{MessageHost, StreamHost};
use parking_lot::Mutex;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use vw_api_chat::chat::GatewayChatStreamRequest;
use vw_api_transport::{ProjectScope, TenantScope};
use vw_api_types::cli_workbench::{
    CliWorkbenchCapabilities, CliWorkbenchDeliveryCapability, CliWorkbenchError,
    CliWorkbenchErrorCode as ErrorCode, CliWorkbenchMessage, CliWorkbenchMessageId,
    CliWorkbenchMessageState as MessageState, CliWorkbenchProcessState as ProcessState,
    CliWorkbenchSession, CliWorkbenchTarget, SendCliWorkbenchMessage, StartCliWorkbenchSession,
};

use super::session::stream::Host;

#[cfg(not(target_arch = "wasm32"))]
mod child_access;

const SESSIONS: &str = "/projects/{project}/cli-workbench/sessions";
pub(crate) const TURN_OPTION: &str = "cli_workbench_turn_id";
const MAX_SESSIONS_PER_PROJECT: usize = 256;
const MAX_REQUESTS_PER_PROJECT: usize = 4096;
const MAX_TEXT: usize = 64 * 1024;
const MAX_OUTPUT: usize = 16 * 1024;
const TURN_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(15);
const SUBSCRIPTION_WAIT: Duration = Duration::from_secs(20);

static WORKBENCH: LazyLock<Mutex<Workbench>> = LazyLock::new(|| Mutex::new(Workbench::default()));

#[derive(Default)]
struct Workbench {
    sessions: HashMap<String, Session>,
    deliveries: HashMap<(ProjectScope, String), Delivery>,
}

struct Session {
    scope: ProjectScope,
    info: CliWorkbenchSession,
    turn: Option<Turn>,
    #[cfg(not(target_arch = "wasm32"))]
    child_access: Option<child_access::ChildAccess>,
}

struct Turn {
    id: String,
    updates: watch::Sender<CliWorkbenchMessage>,
    cancel: CancellationToken,
    assistant_message_id: Option<String>,
    pending_approvals: Vec<String>,
}

impl Session {
    fn refresh_process_state(&mut self) {
        if self.info.state == ProcessState::Ready
            && llm_runtime::acp::session::logical_session_exited(
                self.scope.tenant_scope(),
                &self.info.id,
                &self.info.directory,
            )
        {
            self.info.state = ProcessState::Exited;
            self.revoke_child_access();
        }
    }

    fn revoke_child_access(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.child_access.take();
        }
    }
}

struct Delivery {
    text_hash: [u8; 32],
    updates: watch::Receiver<CliWorkbenchMessage>,
}

struct WorkbenchError(StatusCode, CliWorkbenchError);

impl IntoResponse for WorkbenchError {
    fn into_response(self) -> Response {
        (self.0, Json(self.1)).into_response()
    }
}

fn error(status: StatusCode, code: ErrorCode, message: &'static str) -> WorkbenchError {
    WorkbenchError(status, CliWorkbenchError { code, message: message.into() })
}

fn not_found() -> WorkbenchError {
    error(StatusCode::NOT_FOUND, ErrorCode::NotFound, "workbench resource not found")
}

fn unavailable() -> WorkbenchError {
    error(
        StatusCode::SERVICE_UNAVAILABLE,
        ErrorCode::Unavailable,
        "workbench target is unavailable",
    )
}

fn project_scope(tenant: TenantScope, project: String) -> Result<ProjectScope, WorkbenchError> {
    gateway_project::project_scope(tenant, project)
        .map_err(|_| error(StatusCode::BAD_REQUEST, ErrorCode::InvalidRequest, "invalid project"))
}

impl Workbench {
    fn session_mut(
        &mut self,
        scope: &ProjectScope,
        session_id: &str,
    ) -> Result<&mut Session, WorkbenchError> {
        self.sessions
            .get_mut(session_id)
            .filter(|session| &session.scope == scope)
            .ok_or_else(not_found)
    }

    fn delivery(
        &self,
        scope: ProjectScope,
        session_id: &str,
        request_id: String,
    ) -> Result<&Delivery, WorkbenchError> {
        self.deliveries
            .get(&(scope, request_id))
            .filter(|delivery| delivery.updates.borrow().id.target_session_id == session_id)
            .ok_or_else(not_found)
    }
}

impl Turn {
    fn update(&self, apply: impl FnOnce(&mut CliWorkbenchMessage) -> bool) {
        self.updates.send_if_modified(|message| {
            if !apply(message) {
                return false;
            }
            message.revision += 1;
            true
        });
    }
}

pub(crate) fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    collaboration_routes()
        .route(&format!("{SESSIONS}/{{session}}/stop"), post(stop))
        .layer(from_fn(crate::gateway::user_scope::session_user))
}

fn collaboration_routes<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route(SESSIONS, get(list).post(start))
        .route(&format!("{SESSIONS}/{{session}}/messages"), post(send))
        .route(&format!("{SESSIONS}/{{session}}/messages/{{request}}"), get(receive))
        .route(&format!("{SESSIONS}/{{session}}/messages/{{request}}/events"), get(subscribe))
}

async fn list(
    Extension(tenant): Extension<TenantScope>,
    Path(project): Path<String>,
) -> Result<Json<Vec<CliWorkbenchSession>>, WorkbenchError> {
    let scope = project_scope(tenant, project)?;
    gateway_project::load_project(&scope).await.map_err(|_| not_found())?;
    let mut sessions: Vec<_> = WORKBENCH
        .lock()
        .sessions
        .values_mut()
        .filter(|session| session.scope == scope)
        .map(|session| {
            session.refresh_process_state();
            session.info.clone()
        })
        .collect();
    sessions.sort_unstable_by(|a, b| a.id.cmp(&b.id));
    Ok(Json(sessions))
}

fn capabilities(target: &CliWorkbenchTarget) -> CliWorkbenchCapabilities {
    let acp = matches!(target, CliWorkbenchTarget::Acp { .. });
    CliWorkbenchCapabilities {
        available: acp,
        terminal: false,
        delivery: match target {
            CliWorkbenchTarget::Acp { .. } => CliWorkbenchDeliveryCapability::Structured,
            CliWorkbenchTarget::NativeCli { .. } => CliWorkbenchDeliveryCapability::Draft,
            CliWorkbenchTarget::Shell => CliWorkbenchDeliveryCapability::Unavailable,
        },
    }
}

async fn start(
    Extension(tenant): Extension<TenantScope>,
    Path(project): Path<String>,
    Json(request): Json<StartCliWorkbenchSession>,
) -> Result<(StatusCode, Json<CliWorkbenchSession>), WorkbenchError> {
    let scope = project_scope(tenant.clone(), project)?;
    let project = gateway_project::load_project(&scope).await.map_err(|_| not_found())?;
    let capabilities = capabilities(&request.target);
    let CliWorkbenchTarget::Acp { agent } = &request.target else { return Err(unavailable()) };
    let directory = match request.directory {
        Some(path) if !FsPath::new(&path).is_absolute() => return Err(not_found()),
        Some(path) => path,
        None => gateway_project::project_context_directory(&project).map_err(|_| unavailable())?,
    };
    let canonical = tokio::fs::canonicalize(directory).await.map_err(|_| unavailable())?;
    let mut registered = false;
    for path in std::iter::once(&project.worktree).chain(&project.sandboxes) {
        if tokio::fs::canonicalize(path).await.is_ok_and(|path| path == canonical) {
            registered = true;
            break;
        }
    }
    if !registered || !tokio::fs::metadata(&canonical).await.is_ok_and(|info| info.is_dir()) {
        return Err(not_found());
    }
    let directory = canonical.into_os_string().into_string().map_err(|_| unavailable())?;
    if WORKBENCH.lock().sessions.values().filter(|session| session.scope == scope).count()
        >= MAX_SESSIONS_PER_PROJECT
    {
        return Err(unavailable());
    }
    let id = ordered_id::ascending(ordered_id::Prefix::Session, None).map_err(|_| {
        error(
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::GatewayFailure,
            "could not allocate session ID",
        )
    })?;
    let workspace = directory.clone();
    let expected_project = project.id.clone();
    let selected = agent.clone();
    let session_id = id.clone();
    Host::with_tenant_instance(tenant, directory.clone(), move || {
        Box::pin(async move {
            if agent_project::instance::project().is_none_or(|actual| actual.id != expected_project)
            {
                return Err(gateway_types::ApiError::not_found("project context changed"));
            }
            let profile = Host::agent(&selected)
                .await
                .ok_or_else(|| gateway_types::ApiError::not_found("ACP profile unavailable"))?;
            let config_agent_profile::AgentRuntime::Acp { backend } = profile.runtime else {
                return Err(gateway_types::ApiError::bad_request("ACP profile required"));
            };
            use vw_api_acp_model::acp_model_source::{AcpModelSource, AcpModelSourceCapability};
            if !llm_runtime::acp::model_source::drivers::capabilities(&backend)
                .contains(&AcpModelSourceCapability::Supported { source: AcpModelSource::CliLocal })
            {
                return Err(gateway_types::ApiError::bad_request(
                    "ACP local model source unavailable",
                ));
            }
            Host::prepare_binding(
                &session_id,
                Some(&selected),
                None,
                FsPath::new(&workspace),
                session_engine::admission::EmployeeAdmission::Unverified,
            )
            .await
            .and_then(|(binding, _)| binding.validate_selection(Some(&selected), Some(&backend)))
            .map_err(|_| gateway_types::ApiError::bad_request("ACP profile unavailable"))
        })
    })
    .await
    .map_err(|_| unavailable())?;

    #[cfg(not(target_arch = "wasm32"))]
    let child_access =
        child_access::ChildAccess::start(&scope, &id).await.map_err(|_| unavailable())?;
    let info = CliWorkbenchSession {
        id: id.clone(),
        project_id: project.id,
        directory,
        target: request.target,
        state: ProcessState::Ready,
        capabilities,
    };
    let mut workbench = WORKBENCH.lock();
    if workbench.sessions.values().filter(|session| session.scope == scope).count()
        >= MAX_SESSIONS_PER_PROJECT
    {
        return Err(unavailable());
    }
    workbench.sessions.insert(
        id,
        Session {
            scope,
            info: info.clone(),
            turn: None,
            #[cfg(not(target_arch = "wasm32"))]
            child_access: Some(child_access),
        },
    );
    Ok((StatusCode::CREATED, Json(info)))
}

async fn stop(
    Extension(tenant): Extension<TenantScope>,
    Path((project, session_id)): Path<(String, String)>,
) -> Result<Json<CliWorkbenchSession>, WorkbenchError> {
    let scope = project_scope(tenant.clone(), project)?;
    let mut workbench = WORKBENCH.lock();
    let session = workbench.session_mut(&scope, &session_id)?;
    if !matches!(session.info.state, ProcessState::Exited | ProcessState::Stopping) {
        session.info.state = ProcessState::Stopping;
        session.revoke_child_access();
        if let Some(turn) = &session.turn {
            turn.cancel.cancel();
        } else {
            let directory = session.info.directory.clone();
            tokio::spawn(close_session(tenant, session_id, directory));
        }
    }
    Ok(Json(session.info.clone()))
}

async fn close_session(tenant: TenantScope, session_id: String, directory: String) {
    let closed = llm_runtime::acp::session::close_logical_session(&tenant, &session_id, &directory)
        .await
        .is_ok();
    if let Some(session) = WORKBENCH.lock().sessions.get_mut(&session_id) {
        session.info.state = if closed { ProcessState::Exited } else { ProcessState::Disconnected };
    }
}

async fn send(
    Extension(tenant): Extension<TenantScope>,
    Path((project, session_id)): Path<(String, String)>,
    Json(request): Json<SendCliWorkbenchMessage>,
) -> Result<Json<CliWorkbenchMessage>, WorkbenchError> {
    if request.request_id.is_empty()
        || request.request_id.len() > 128
        || !request
            .request_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err(error(
            StatusCode::BAD_REQUEST,
            ErrorCode::InvalidRequest,
            "invalid workbench message",
        ));
    }
    let scope = project_scope(tenant.clone(), project)?;
    let key = (scope.clone(), request.request_id.clone());
    let text_hash: [u8; 32] = Sha256::digest(request.text.as_bytes()).into();
    let mut workbench = WORKBENCH.lock();
    workbench.session_mut(&scope, &session_id)?;
    if let Some(delivery) = workbench.deliveries.get(&key) {
        let previous = delivery.updates.borrow();
        if delivery.text_hash != text_hash || previous.id.target_session_id != session_id {
            return Err(error(
                StatusCode::CONFLICT,
                ErrorCode::RequestConflict,
                "request ID already used",
            ));
        }
        return Ok(Json(previous.clone()));
    }
    if request.text.trim().is_empty() || request.text.len() > MAX_TEXT {
        return Err(error(
            StatusCode::BAD_REQUEST,
            ErrorCode::InvalidRequest,
            "invalid workbench message text",
        ));
    }
    if workbench.deliveries.keys().filter(|(project, _)| project == &scope).count()
        >= MAX_REQUESTS_PER_PROJECT
    {
        return Err(unavailable());
    }
    let session = workbench.session_mut(&scope, &session_id)?;
    session.refresh_process_state();
    match session.info.state {
        ProcessState::Ready => {}
        ProcessState::WaitingForApproval => {
            return Err(error(
                StatusCode::CONFLICT,
                ErrorCode::AwaitingApproval,
                "workbench target is waiting for approval",
            ));
        }
        ProcessState::Running => {
            return Err(error(StatusCode::CONFLICT, ErrorCode::Busy, "workbench target is busy"));
        }
        ProcessState::Exited | ProcessState::Stopping => {
            return Err(error(StatusCode::GONE, ErrorCode::Closed, "workbench target is closed"));
        }
        ProcessState::Disconnected => {
            return Err(error(
                StatusCode::SERVICE_UNAVAILABLE,
                ErrorCode::Disconnected,
                "workbench target disconnected",
            ));
        }
    }
    let CliWorkbenchTarget::Acp { agent } = &session.info.target else { return Err(unavailable()) };
    let directory = session.info.directory.clone();
    let agent = agent.clone();
    let message = CliWorkbenchMessage {
        id: CliWorkbenchMessageId {
            request_id: request.request_id,
            sender_user_id: tenant.user_id().to_string(),
            target_session_id: session_id.clone(),
        },
        turn_id: uuid::Uuid::new_v4().to_string(),
        state: MessageState::Accepted,
        revision: 1,
        approval_request_id: None,
        approval_history: Vec::new(),
        assistant_message_id: None,
        output: String::new(),
        output_truncated: false,
        error: None,
    };
    let (updates, receiver) = watch::channel(message.clone());
    let cancel = CancellationToken::new();
    session.info.state = ProcessState::Running;
    session.turn = Some(Turn {
        id: message.turn_id.clone(),
        updates,
        cancel: cancel.clone(),
        assistant_message_id: None,
        pending_approvals: Vec::new(),
    });
    workbench.deliveries.insert(key, Delivery { text_hash, updates: receiver });
    tokio::spawn(run_turn(
        scope,
        session_id,
        directory,
        agent,
        message.turn_id.clone(),
        request.text,
        cancel,
    ));
    Ok(Json(message))
}

async fn receive(
    Extension(tenant): Extension<TenantScope>,
    Path((project, session, request)): Path<(String, String, String)>,
) -> Result<Json<CliWorkbenchMessage>, WorkbenchError> {
    let scope = project_scope(tenant, project)?;
    let workbench = WORKBENCH.lock();
    let result = workbench.delivery(scope, &session, request)?.updates.borrow().clone();
    Ok(Json(result))
}

#[derive(Deserialize)]
struct SubscriptionQuery {
    #[serde(default)]
    after_revision: u64,
}

async fn subscribe(
    Extension(tenant): Extension<TenantScope>,
    Path((project, session, request)): Path<(String, String, String)>,
    Query(query): Query<SubscriptionQuery>,
) -> Result<Json<CliWorkbenchMessage>, WorkbenchError> {
    let scope = project_scope(tenant, project)?;
    let mut updates = WORKBENCH.lock().delivery(scope, &session, request)?.updates.clone();
    let _ = tokio::time::timeout(
        SUBSCRIPTION_WAIT,
        updates.wait_for(|message| {
            message.revision > query.after_revision || message.state.is_terminal()
        }),
    )
    .await;
    let result = updates.borrow().clone();
    Ok(Json(result))
}

fn with_turn(session_id: &str, turn_id: &str, update: impl FnOnce(&Turn)) {
    if let Some(turn) =
        WORKBENCH.lock().sessions.get(session_id).and_then(|session| session.turn.as_ref())
        && turn.id == turn_id
    {
        update(turn);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn acp_child_context(
    tenant: &TenantScope,
    session_id: &str,
    assistant_id: &str,
    directory: &FsPath,
) -> Option<String> {
    let workbench = WORKBENCH.lock();
    let session = workbench.sessions.get(session_id)?;
    if session.scope.tenant_scope() != tenant
        || FsPath::new(&session.info.directory) != directory
        || session.turn.as_ref()?.assistant_message_id.as_deref() != Some(assistant_id)
    {
        return None;
    }

    session.child_access.as_ref().map(|access| access.context().to_owned())
}

pub(crate) fn bind_assistant_message(
    tenant: &TenantScope,
    session_id: &str,
    turn_id: &str,
    assistant_id: &str,
) {
    let mut workbench = WORKBENCH.lock();
    if let Some(session) = workbench.sessions.get_mut(session_id)
        && session.scope.tenant_scope() == tenant
        && let Some(turn) = &mut session.turn
        && turn.id == turn_id
        && turn.assistant_message_id.is_none()
    {
        turn.assistant_message_id = Some(assistant_id.into());
    }
}

pub(crate) fn approval_changed(
    tenant: &TenantScope,
    session_id: &str,
    assistant_id: Option<&str>,
    request_id: &str,
    pending: bool,
) {
    let mut workbench = WORKBENCH.lock();
    let Some(session) = workbench.sessions.get_mut(session_id) else { return };
    if session.scope.tenant_scope() != tenant || session.info.state == ProcessState::Stopping {
        return;
    }
    let Some(turn) = session.turn.as_mut() else { return };
    if assistant_id.is_none() || turn.assistant_message_id.as_deref() != assistant_id {
        return;
    }
    if pending {
        turn.pending_approvals.push(request_id.into());
    } else {
        turn.pending_approvals.retain(|id| id != request_id);
    }
    let current = turn.pending_approvals.first();
    let (process, message_state) = if current.is_some() {
        (ProcessState::WaitingForApproval, MessageState::WaitingForApproval)
    } else {
        (ProcessState::Running, MessageState::Submitted)
    };
    session.info.state = process;
    turn.update(|message| {
        message.state = message_state;
        message.approval_request_id = current.cloned();
        if pending && message.approval_history.len() < 64 {
            message.approval_history.push(request_id.into());
        }
        true
    });
}

#[allow(clippy::too_many_arguments)]
async fn run_turn(
    scope: ProjectScope,
    session_id: String,
    directory: String,
    agent: String,
    turn_id: String,
    text: String,
    cancel: CancellationToken,
) {
    let tenant = scope.tenant_scope().clone();
    let body = GatewayChatStreamRequest {
        session_id: Some(session_id.clone().into()),
        agent: Some(agent),
        acp_model_source: vw_api_acp_model::acp_model_source::AcpModelSource::CliLocal,
        messages: vec![json!({ "role": "user", "content": text })],
        options: Some(json!({ (TURN_OPTION): turn_id.as_str(), "cwd": directory.as_str() })),
        ..Default::default()
    };
    let root = directory.clone();
    let producer_cancel = cancel.clone();
    let deadline = tokio::time::Instant::now() + TURN_TIMEOUT;
    let start = Host::with_tenant_instance(tenant.clone(), root.clone(), move || {
        Box::pin(async move {
            if agent_project::instance::project()
                .is_none_or(|actual| actual.id != scope.project_id().as_ref())
                || !tokio::fs::canonicalize(&root)
                    .await
                    .is_ok_and(|actual| actual.as_path() == FsPath::new(&root))
            {
                return Err(gateway_types::ApiError::not_found("project context changed"));
            }
            gateway_session::stream::start_gateway_chat_stream::<Host>(
                tenant,
                root,
                body,
                None,
                Some(producer_cancel),
            )
            .await
        })
    });
    let started = tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(ErrorCode::Cancelled),
        result = tokio::time::timeout_at(deadline, start) => match result {
            Ok(result) => result.map_err(|_| ErrorCode::Unavailable),
            Err(_) => Err(ErrorCode::TimedOut),
        },
    };
    let result = match started {
        Ok(mut events) => {
            with_turn(&session_id, &turn_id, |turn| {
                turn.update(|message| {
                    if message.state != MessageState::Accepted {
                        return false;
                    }
                    message.state = MessageState::Submitted;
                    true
                })
            });
            let result = tokio::select! {
                biased;
                _ = cancel.cancelled() => Err(ErrorCode::Cancelled),
                result = tokio::time::timeout_at(deadline, collect_turn(&session_id, &turn_id, &mut events)) => {
                    result.unwrap_or(Err(ErrorCode::TimedOut))
                }
            };
            if matches!(result, Err(ErrorCode::Cancelled | ErrorCode::TimedOut)) {
                cancel.cancel();
                let _ = tokio::time::timeout(SHUTDOWN_TIMEOUT, async {
                    while events.recv().await.is_some() {}
                })
                .await;
            }
            result
        }
        Err(code) => {
            cancel.cancel();
            Err(code)
        }
    };
    if let Some(tenant) = finish_turn(&session_id, &turn_id, result) {
        close_session(tenant, session_id, directory).await;
    }
}

async fn collect_turn(
    session_id: &str,
    turn_id: &str,
    events: &mut tokio::sync::mpsc::UnboundedReceiver<Value>,
) -> Result<String, ErrorCode> {
    let mut reason = None;
    let mut result = None;
    while let Some(payload) = events.recv().await {
        let kind = payload["type"].as_str();
        if payload["session_id"].as_str().is_some_and(|id| id != session_id) {
            continue;
        }
        if result.is_some()
            && !(kind == Some("chat.error")
                && payload["error"]
                    .as_str()
                    .is_some_and(|error| error.contains("execution shutdown unconfirmed")))
        {
            continue;
        }
        match kind {
            Some("chat.step_start") => reason = None,
            Some("chat.step_finish") => {
                reason = payload["finish_reason"].as_str().map(str::to_owned)
            }
            Some("chat.delta") if let Some(delta) = payload["delta"].as_str() => {
                with_turn(session_id, turn_id, |turn| {
                    turn.update(|message| append_output(message, delta))
                });
            }
            Some("chat.done") => {
                if let Some(assistant_id) = payload["message_id"].as_str()
                    && !owns_assistant_message(session_id, turn_id, assistant_id)
                {
                    continue;
                }
                result = Some(match (reason.as_deref(), payload["message_id"].as_str()) {
                    (Some("cancelled"), _) => Err(ErrorCode::Cancelled),
                    (Some(reason), Some(message))
                        if crate::session::runtime::is_successful_finish_reason(Some(reason)) =>
                    {
                        Ok(message.into())
                    }
                    _ => Err(ErrorCode::TurnFailed),
                });
            }
            Some("chat.error") => {
                let error = payload["error"].as_str().unwrap_or_default();
                result = Some(Err(
                    if error.contains("execution shutdown unconfirmed")
                        || error.contains("stream 异常结束")
                        || error.contains("connection closed")
                        || error.contains("ACP client actor is unavailable")
                    {
                        ErrorCode::Disconnected
                    } else {
                        ErrorCode::TurnFailed
                    },
                ));
            }
            _ => {}
        }
    }
    result.unwrap_or(Err(ErrorCode::Disconnected))
}

fn owns_assistant_message(session_id: &str, turn_id: &str, assistant_id: &str) -> bool {
    let mut current = false;
    with_turn(session_id, turn_id, |turn| {
        current = turn.assistant_message_id.as_deref() == Some(assistant_id);
    });
    current
}

fn append_output(message: &mut CliWorkbenchMessage, delta: &str) -> bool {
    if message.output_truncated || delta.is_empty() {
        return false;
    }
    let available = MAX_OUTPUT - message.output.len();
    let end = delta.floor_char_boundary(available);
    message.output.push_str(&delta[..end]);
    message.output_truncated = end < delta.len();
    true
}

fn finish_turn(
    session_id: &str,
    turn_id: &str,
    result: Result<String, ErrorCode>,
) -> Option<TenantScope> {
    let mut workbench = WORKBENCH.lock();
    let session = workbench.sessions.get_mut(session_id)?;
    let turn = session.turn.as_ref()?;
    if turn.id != turn_id {
        return None;
    }
    let stopping = session.info.state == ProcessState::Stopping;
    let result = if stopping { Err(ErrorCode::Cancelled) } else { result };
    turn.update(|message| {
        message.approval_request_id = None;
        match result {
            Ok(assistant) if turn.assistant_message_id.as_ref() == Some(&assistant) => {
                message.state = MessageState::Completed;
                message.assistant_message_id = Some(assistant);
            }
            result => {
                message.state = MessageState::Failed;
                message.error = Some(result.err().unwrap_or(ErrorCode::Disconnected));
            }
        }
        true
    });
    let disconnected = matches!(
        turn.updates.borrow().error,
        Some(ErrorCode::Disconnected | ErrorCode::TimedOut | ErrorCode::Unavailable)
    );
    session.turn = None;
    if stopping {
        return Some(session.scope.tenant_scope().clone());
    }
    if disconnected {
        session.info.state = ProcessState::Disconnected;
        session.revoke_child_access();
    } else {
        session.info.state = ProcessState::Ready;
    }
    None
}
