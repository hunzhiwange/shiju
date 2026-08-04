//! ACP `Agent` trait 的会话生命周期实现。

use std::cell::OnceCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use agent_client_protocol::{
    Agent, AgentCapabilities, AgentSideConnection, AuthenticateRequest, AuthenticateResponse,
    CancelNotification, Client, ContentBlock, ContentChunk, Error, InitializeRequest,
    InitializeResponse, LoadSessionRequest, LoadSessionResponse, NewSessionRequest,
    McpCapabilities, NewSessionResponse, PermissionOption, PermissionOptionKind, PromptRequest,
    PromptResponse, ProtocolVersion, RequestPermissionOutcome, RequestPermissionRequest, Result,
    SessionNotification, SessionUpdate, StopReason, TextContent, ToolCallUpdate,
    ToolCallUpdateFields,
};
use parking_lot::Mutex;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use super::backend::{
    BackendError, BackendPermissionOptionKind, BackendPermissionRequest, BackendSession,
    SessionBackend,
};
use super::content::prompt_content;
use super::events::{
    BackendEvent, BackendTermination, NoopUpdateSink, SessionUpdateSink, session_update,
};
use super::state::{ConnectionState, SessionBinding};

/// 可注入宿主后端的 ACP Agent。
pub struct VibeWindowAgent<B> {
    backend: B,
    state: Mutex<ConnectionState>,
    updates: Rc<dyn SessionUpdateSink>,
}

impl<B> VibeWindowAgent<B>
where
    B: SessionBackend,
{
    /// 创建一条尚未初始化的 Agent 连接状态。
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            state: Mutex::new(ConnectionState::default()),
            updates: Rc::new(NoopUpdateSink),
        }
    }

    /// 创建带有可注入 `session/update` 输出端的 Agent。
    pub fn with_update_sink(backend: B, updates: impl SessionUpdateSink) -> Self {
        Self {
            backend,
            state: Mutex::new(ConnectionState::default()),
            updates: Rc::new(updates),
        }
    }

    /// 返回当前连接中的会话绑定快照。
    pub fn session_binding(&self, session_id: &str) -> Option<SessionBinding> {
        self.state.lock().binding(session_id)
    }

    fn ensure_initialized(&self) -> Result<()> {
        if self.state.lock().initialized() {
            return Ok(());
        }
        Err(protocol_error(
            Error::invalid_request(),
            "ACP connection must be initialized before session operations",
        ))
    }

    fn register_session(&self, session: BackendSession, expected_cwd: &Path) -> Result<String> {
        let actual_cwd = canonical_directory(&session.cwd)?;
        if actual_cwd != expected_cwd {
            return Err(protocol_error(
                Error::invalid_params(),
                format!(
                    "backend session {} belongs to {}, not {}",
                    session.id,
                    actual_cwd.display(),
                    expected_cwd.display()
                ),
            ));
        }
        if session.id.trim().is_empty() {
            return Err(protocol_error(
                Error::internal_error(),
                "backend returned an empty session id",
            ));
        }

        let id = session.id;
        let binding = SessionBinding::new(id.clone(), actual_cwd, session.history);
        self.state.lock().insert(id.clone(), binding).map_err(|_| {
            protocol_error(
                Error::invalid_request(),
                format!("session is already bound to this connection: {id}"),
            )
        })?;
        Ok(id)
    }

    async fn send_update(&self, session_id: &str, update: SessionUpdate) -> Result<()> {
        self.updates.send(SessionNotification::new(session_id.to_string(), update)).await
    }

    async fn request_permission(
        &self,
        session_id: &str,
        request: BackendPermissionRequest,
    ) {
        let tool_call = ToolCallUpdate::new(
            request.call_id,
            ToolCallUpdateFields::new()
                .title(request.title)
                .raw_input(request.raw_input),
        );
        let options = request
            .options
            .into_iter()
            .map(|option| {
                PermissionOption::new(
                    option.id,
                    option.name,
                    match option.kind {
                        BackendPermissionOptionKind::AllowOnce => PermissionOptionKind::AllowOnce,
                        BackendPermissionOptionKind::RejectOnce => PermissionOptionKind::RejectOnce,
                    },
                )
            })
            .collect();
        let selected = match self
            .updates
            .request_permission(RequestPermissionRequest::new(
                session_id.to_string(),
                tool_call,
                options,
            ))
            .await
        {
            Ok(response) => match response.outcome {
                RequestPermissionOutcome::Selected(selected) => {
                    Some(selected.option_id.0.to_string())
                }
                RequestPermissionOutcome::Cancelled => None,
                _ => None,
            },
            Err(_) => None,
        };
        let _ = request.response.send(selected);
    }
}

struct PromptScope<'a> {
    state: &'a Mutex<ConnectionState>,
    session_id: String,
    completed: bool,
}

impl Drop for PromptScope<'_> {
    fn drop(&mut self) {
        if !self.completed {
            self.state.lock().cancel_prompt(&self.session_id);
        }
        self.state.lock().finish_prompt(&self.session_id);
    }
}

fn protocol_error(error: Error, message: impl Into<String>) -> Error {
    error.data(serde_json::json!({ "reason": message.into() }))
}

fn canonical_directory(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() {
        return Err(protocol_error(
            Error::invalid_params(),
            format!("session cwd must be absolute: {}", path.display()),
        ));
    }
    let canonical = std::fs::canonicalize(path).map_err(|error| {
        protocol_error(
            Error::invalid_params(),
            format!("session cwd is unavailable ({}): {error}", path.display()),
        )
    })?;
    if !canonical.is_dir() {
        return Err(protocol_error(
            Error::invalid_params(),
            format!("session cwd is not a directory: {}", canonical.display()),
        ));
    }
    Ok(canonical)
}

fn map_backend_error(error: BackendError) -> Error {
    match error {
        BackendError::NotFound(session_id) => Error::resource_not_found(Some(session_id)),
        BackendError::OwnershipMismatch(message) | BackendError::InvalidRequest(message) => {
            protocol_error(Error::invalid_params(), message)
        }
        BackendError::Internal(message) => protocol_error(Error::internal_error(), message),
    }
}

fn stop_reason(termination: BackendTermination) -> StopReason {
    match termination {
        BackendTermination::EndTurn => StopReason::EndTurn,
        BackendTermination::MaxTokens => StopReason::MaxTokens,
        BackendTermination::MaxTurnRequests => StopReason::MaxTurnRequests,
        BackendTermination::Refusal => StopReason::Refusal,
        BackendTermination::Cancelled => StopReason::Cancelled,
    }
}

#[async_trait::async_trait(?Send)]
impl<B> Agent for VibeWindowAgent<B>
where
    B: SessionBackend,
{
    async fn initialize(&self, _args: InitializeRequest) -> Result<InitializeResponse> {
        if !self.state.lock().initialize() {
            return Err(protocol_error(
                Error::invalid_request(),
                "ACP connection is already initialized",
            ));
        }
        Ok(InitializeResponse::new(ProtocolVersion::V1)
            .agent_capabilities(
                AgentCapabilities::new()
                    .load_session(true)
                    .mcp_capabilities(McpCapabilities::new().http(true).sse(true)),
            ))
    }

    async fn authenticate(&self, _args: AuthenticateRequest) -> Result<AuthenticateResponse> {
        Err(Error::method_not_found())
    }

    async fn new_session(&self, args: NewSessionRequest) -> Result<NewSessionResponse> {
        self.ensure_initialized()?;
        let cwd = canonical_directory(&args.cwd)?;
        let session =
            self.backend
                .create_session_with_mcp(cwd.clone(), args.mcp_servers)
                .await
                .map_err(map_backend_error)?;
        let session_id = self.register_session(session, &cwd)?;
        Ok(NewSessionResponse::new(session_id))
    }

    async fn load_session(&self, args: LoadSessionRequest) -> Result<LoadSessionResponse> {
        self.ensure_initialized()?;
        let cwd = canonical_directory(&args.cwd)?;
        let session_id = args.session_id.0.as_ref();
        let session = self
            .backend
            .load_session_with_mcp(session_id, cwd.clone(), args.mcp_servers)
            .await
            .map_err(map_backend_error)?;
        let history = session.history.clone();
        self.register_session(session, &cwd)?;
        for entry in history {
            let chunk = ContentChunk::new(ContentBlock::Text(TextContent::new(entry.text)));
            let update = match entry.role {
                super::backend::BackendRole::User => SessionUpdate::UserMessageChunk(chunk),
                super::backend::BackendRole::Assistant => SessionUpdate::AgentMessageChunk(chunk),
            };
            self.send_update(session_id, update).await?;
        }
        Ok(LoadSessionResponse::new())
    }

    async fn prompt(&self, args: PromptRequest) -> Result<PromptResponse> {
        self.ensure_initialized()?;
        let session_id = args.session_id.0.to_string();
        if self.state.lock().binding(&session_id).is_none() {
            return Err(Error::resource_not_found(Some(session_id)));
        }
        let prompt = prompt_content(args.prompt)?;
        if !self.state.lock().reserve_prompt(&session_id) {
            return Err(protocol_error(
                Error::invalid_request(),
                format!("a prompt is already active for session {session_id}"),
            ));
        }
        let mut scope = PromptScope {
            state: &self.state,
            session_id: session_id.clone(),
            completed: false,
        };
        let mut prompt = self
            .backend
            .start_prompt(&session_id, prompt)
            .await
            .map_err(map_backend_error)?;
        self.state.lock().attach_cancel(&session_id, prompt.cancel.clone());

        let mut permissions_open = true;
        loop {
            tokio::select! {
                event = prompt.events.recv() => {
                    let Some(event) = event else {
                        return Err(protocol_error(
                            Error::internal_error(),
                            "backend prompt stream closed before a terminal event",
                        ));
                    };
                    match event {
                        BackendEvent::Terminal(termination) => {
                            scope.completed = true;
                            return Ok(PromptResponse::new(stop_reason(termination)));
                        }
                        BackendEvent::Error(error) => {
                            scope.completed = true;
                            return Err(protocol_error(Error::internal_error(), error));
                        }
                        event => {
                            if let Some(update) = session_update(event)? {
                                self.send_update(&session_id, update).await?;
                            }
                        }
                    }
                }
                permission = prompt.permissions.recv(), if permissions_open => {
                    match permission {
                        Some(request) => self.request_permission(&session_id, request).await,
                        None => permissions_open = false,
                    }
                }
            }
        }
    }

    async fn cancel(&self, args: CancelNotification) -> Result<()> {
        self.ensure_initialized()?;
        self.state.lock().cancel_prompt(args.session_id.0.as_ref());
        Ok(())
    }
}

struct ConnectionUpdateSink {
    connection: Rc<OnceCell<AgentSideConnection>>,
}

#[async_trait::async_trait(?Send)]
impl SessionUpdateSink for ConnectionUpdateSink {
    async fn send(&self, notification: SessionNotification) -> Result<()> {
        let connection = self.connection.get().ok_or_else(|| {
            protocol_error(Error::internal_error(), "ACP connection is not ready")
        })?;
        Client::session_notification(connection, notification).await
    }

    async fn request_permission(
        &self,
        request: RequestPermissionRequest,
    ) -> Result<agent_client_protocol::RequestPermissionResponse> {
        let connection = self.connection.get().ok_or_else(|| {
            protocol_error(Error::internal_error(), "ACP connection is not ready")
        })?;
        Client::request_permission(connection, request).await
    }
}

/// 使用可注入 Tokio reader/writer 运行纯 JSON-RPC Agent 传输。
pub async fn serve_with_io<B, R, W>(backend: B, reader: R, writer: W) -> Result<()>
where
    B: SessionBackend,
    R: AsyncRead + Unpin + 'static,
    W: AsyncWrite + Unpin + 'static,
{
    tokio::task::LocalSet::new()
        .run_until(async move {
            let connection = Rc::new(OnceCell::new());
            let agent = VibeWindowAgent::with_update_sink(
                backend,
                ConnectionUpdateSink { connection: Rc::clone(&connection) },
            );
            let (client, io_task) = AgentSideConnection::new(
                agent,
                writer.compat_write(),
                reader.compat(),
                |future| {
                    tokio::task::spawn_local(future);
                },
            );
            connection.set(client).map_err(|_| {
                protocol_error(Error::internal_error(), "ACP connection was initialized twice")
            })?;
            io_task.await
        })
        .await
}
