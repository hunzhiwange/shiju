//! ACP stdio Agent for a single employee and authenticated account.

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
};

use agent_client_protocol::{
    Agent, AgentCapabilities, AuthenticateRequest, AuthenticateResponse, CancelNotification,
    ContentBlock, ContentChunk, Error, InitializeRequest, InitializeResponse, ListSessionsRequest,
    ListSessionsResponse, LoadSessionRequest, LoadSessionResponse, NewSessionRequest,
    NewSessionResponse, PromptRequest, PromptResponse, ProtocolVersion, Result,
    SessionCapabilities, SessionInfo, SessionListCapabilities, SessionNotification, SessionUpdate,
    StopReason,
};
use shiju_client::employee_conversations::{
    Conversation, ConversationHistory, ConversationKind, ConversationMessage,
    ConversationSubmission, ConversationTurn, EmployeeConversationError as CloudError,
    MessageSender, TurnStatus,
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Notify,
    time::{self, Instant},
};
use uuid::Uuid;

pub use shiju_client::employee_conversations::{
    EmployeeConversationClient, EmployeeKind, EmployeeTarget, ShijuCredential,
};

use crate::{
    employee_state::{
        ConfirmedSend, PendingSend, SessionState, StateStore, cloud_id, digest, missing_session,
    },
    server::{
        SessionUpdateSink,
        agent::{canonical_directory, serve_agent_with_io},
    },
};

const MAX_MESSAGE_BYTES: usize = 32_768;
const PAGE_SIZE: usize = 16;
const TURN_TIMEOUT: Duration = Duration::from_secs(300);
const CANCEL_TIMEOUT: Duration = Duration::from_secs(20);
const POLL_MIN: Duration = Duration::from_millis(200);
const POLL_MAX: Duration = Duration::from_secs(2);

pub struct EmployeeAgent {
    client: EmployeeConversationClient,
    target: EmployeeTarget,
    store: StateStore,
    cwd: PathBuf,
}

impl EmployeeAgent {
    pub fn new(
        client: EmployeeConversationClient,
        target: EmployeeTarget,
        state_dir: &Path,
        account_scope: &str,
    ) -> Result<Self> {
        let store = StateStore::open(state_dir, account_scope, &target)?;
        let cwd = std::env::current_dir()
            .map_err(|_| Error::new(-32603, "Employee ACP working directory is unavailable"))?;
        Ok(Self { client, target, store, cwd: canonical_directory(&cwd)? })
    }

    pub async fn serve_with_io<R, W>(self, reader: R, writer: W) -> Result<()>
    where
        R: AsyncRead + Unpin + 'static,
        W: AsyncWrite + Unpin + 'static,
    {
        serve_agent_with_io(reader, writer, |updates| Connection {
            agent: self,
            updates,
            initialized: Cell::new(false),
            active: RefCell::new(HashMap::new()),
        })
        .await
    }

    fn matches(&self, conversation: &Conversation) -> bool {
        let kind = match self.target.kind {
            EmployeeKind::Private => ConversationKind::PrivateDirect,
            EmployeeKind::Shared => ConversationKind::SharedDirect,
        };
        conversation.assistant_id == self.target.employee_id
            && conversation.kind == kind
            && cloud_id(&conversation.conversation_id)
    }

    async fn visible_conversations(&self) -> Result<Vec<Conversation>> {
        let mut items = self.client.list_conversations(&self.target).await.map_err(cloud_error)?;
        items.retain(|item| self.matches(item));
        // Agent Cloud returns updated_at descending, with the conversation ID as a tiebreaker.
        Ok(items)
    }
}

struct Connection<U> {
    agent: EmployeeAgent,
    updates: U,
    initialized: Cell<bool>,
    active: RefCell<HashMap<String, Rc<ActivePrompt>>>,
}

struct InitializeProbe;

#[async_trait::async_trait(?Send)]
impl Agent for InitializeProbe {
    async fn initialize(&self, _: InitializeRequest) -> Result<InitializeResponse> {
        Ok(employee_initialize_response())
    }

    async fn authenticate(&self, _: AuthenticateRequest) -> Result<AuthenticateResponse> {
        Err(Error::method_not_found())
    }

    async fn new_session(&self, _: NewSessionRequest) -> Result<NewSessionResponse> {
        Err(Error::method_not_found())
    }

    async fn prompt(&self, _: PromptRequest) -> Result<PromptResponse> {
        Err(Error::method_not_found())
    }

    async fn cancel(&self, _: CancelNotification) -> Result<()> {
        Err(Error::method_not_found())
    }
}

pub async fn serve_initialize_probe<R, W>(reader: R, writer: W) -> Result<()>
where
    R: AsyncRead + Unpin + 'static,
    W: AsyncWrite + Unpin + 'static,
{
    serve_agent_with_io(reader, writer, |_| InitializeProbe).await
}

fn employee_initialize_response() -> InitializeResponse {
    InitializeResponse::new(ProtocolVersion::V1).agent_capabilities(
        AgentCapabilities::new().load_session(true).session_capabilities(
            SessionCapabilities::new().list(SessionListCapabilities::new()),
        ),
    )
}

#[derive(Default)]
struct ActivePrompt {
    armed: Cell<bool>,
    cancelled: Cell<bool>,
    changed: Notify,
}

struct PromptScope<'a> {
    active: &'a RefCell<HashMap<String, Rc<ActivePrompt>>>,
    id: &'a str,
}

impl Drop for PromptScope<'_> {
    fn drop(&mut self) {
        self.active.borrow_mut().remove(self.id);
    }
}

#[async_trait::async_trait(?Send)]
impl<U: SessionUpdateSink> Agent for Connection<U> {
    async fn initialize(&self, _: InitializeRequest) -> Result<InitializeResponse> {
        if self.initialized.replace(true) {
            return Err(Error::new(-32600, "ACP connection is already initialized"));
        }
        Ok(employee_initialize_response())
    }

    async fn authenticate(&self, _: AuthenticateRequest) -> Result<AuthenticateResponse> {
        Err(Error::method_not_found())
    }

    async fn new_session(&self, args: NewSessionRequest) -> Result<NewSessionResponse> {
        self.ensure_initialized()?;
        let cwd = session_directory(&args.cwd, args.mcp_servers.is_empty())?;
        let id = format!("local-{}", Uuid::new_v4());
        self.agent.store.write(&id, &SessionState::new(cwd, None))?;
        Ok(NewSessionResponse::new(id))
    }

    async fn load_session(&self, args: LoadSessionRequest) -> Result<LoadSessionResponse> {
        self.ensure_initialized()?;
        let cwd = session_directory(&args.cwd, args.mcp_servers.is_empty())?;
        let id = args.session_id.0.as_ref();
        let _lease = self.agent.store.lease(id)?;
        let existing = self.agent.store.read(id)?;
        if existing.as_ref().is_some_and(|state| state.cwd != cwd)
            || (existing.is_none() && (!cloud_id(id) || cwd != self.agent.cwd))
        {
            return Err(missing_session());
        }

        let visible = self.agent.visible_conversations().await?;
        let imported = existing.is_none();
        let state = existing.unwrap_or_else(|| SessionState::new(cwd, Some(id.to_owned())));
        if let Some(cloud) = &state.cloud_id {
            if !visible.iter().any(|item| item.conversation_id == *cloud) {
                return Err(missing_session());
            }
            let history = self.agent.client.history(cloud).await.map_err(cloud_error)?;
            if imported {
                self.agent.store.write(id, &state)?;
            }
            if history
                .messages
                .iter()
                .any(|message| message.sender != MessageSender::System && !valid_message(message))
            {
                return Err(cloud_error(CloudError::InvalidResponse));
            }
            for message in history.messages {
                let update = match message.sender {
                    MessageSender::User => SessionUpdate::UserMessageChunk(message_chunk(message)),
                    MessageSender::Employee => {
                        SessionUpdate::AgentMessageChunk(message_chunk(message))
                    }
                    MessageSender::System => continue,
                };
                self.updates.send(SessionNotification::new(id.to_owned(), update)).await?;
            }
        }
        Ok(LoadSessionResponse::new())
    }

    async fn list_sessions(&self, args: ListSessionsRequest) -> Result<ListSessionsResponse> {
        self.ensure_initialized()?;
        let cwd = args.cwd.as_deref().map(canonical_directory).transpose()?;
        let scope = self.agent.store.cursor_scope(cwd.as_deref());
        let offset = match args.cursor.as_deref() {
            None => 0,
            Some(cursor) => cursor
                .split_once(':')
                .filter(|(key, _)| *key == scope)
                .and_then(|(_, index)| index.parse::<usize>().ok())
                .filter(|index| *index > 0 && index % PAGE_SIZE == 0)
                .ok_or_else(|| Error::new(-32602, "Invalid employee ACP list cursor"))?,
        };
        let mut known = HashMap::new();
        for (id, state) in self.agent.store.sessions()? {
            if let Some(cloud) = state.cloud_id {
                known.insert(cloud, (id, state.cwd));
            }
        }
        let mut items: Vec<_> = self
            .agent
            .visible_conversations()
            .await?
            .into_iter()
            .filter_map(|item| {
                let (id, directory) = known
                    .remove(&item.conversation_id)
                    .unwrap_or_else(|| (item.conversation_id, self.agent.cwd.clone()));
                if cwd.as_ref().is_some_and(|cwd| *cwd != directory) {
                    return None;
                }
                Some(SessionInfo::new(id, directory).title(item.title).updated_at(item.updated_at))
            })
            .skip(offset)
            .take(PAGE_SIZE + 1)
            .collect();
        let next = (items.len() > PAGE_SIZE).then(|| format!("{scope}:{}", offset + PAGE_SIZE));
        items.truncate(PAGE_SIZE);
        Ok(ListSessionsResponse::new(items).next_cursor(next))
    }

    async fn prompt(&self, args: PromptRequest) -> Result<PromptResponse> {
        self.ensure_initialized()?;
        let text = prompt_text(&args.prompt)?;
        let id = args.session_id.0.as_ref();
        let _lease = self.agent.store.lease(id)?;
        let state = self.agent.store.read(id)?.ok_or_else(missing_session)?;
        let active = Rc::new(ActivePrompt::default());
        self.active.borrow_mut().insert(id.to_owned(), Rc::clone(&active));
        let _scope = PromptScope { active: &self.active, id };
        time::timeout(TURN_TIMEOUT, self.run_prompt(id, &text, state, &active)).await.map_err(
            |_| {
                Error::new(-32603, "Employee reply timed out; the pending message can be recovered")
            },
        )?
    }

    async fn cancel(&self, args: CancelNotification) -> Result<()> {
        self.ensure_initialized()?;
        let id = args.session_id.0.as_ref();
        let active = self.active.borrow().get(id).cloned();
        if let Some(active) = active {
            active.cancelled.set(true);
            active.changed.notify_one();
            // The prompt owns the file lease. Persist the intent immediately even if
            // the POST is still in flight or the host disconnects before it resolves.
            if active.armed.get()
                && let Some(mut state) = self.agent.store.read(id)?
                && let Some(pending) = &mut state.pending
            {
                pending.cancel_requested = true;
                self.agent.store.write(id, &state)?;
            }
        }
        Ok(())
    }
}

impl<U: SessionUpdateSink> Connection<U> {
    fn ensure_initialized(&self) -> Result<()> {
        if self.initialized.get() {
            Ok(())
        } else {
            Err(Error::new(-32600, "Initialize ACP before opening an employee session"))
        }
    }

    async fn run_prompt(
        &self,
        id: &str,
        text: &str,
        mut state: SessionState,
        active: &ActivePrompt,
    ) -> Result<PromptResponse> {
        let message_digest = digest(text.as_bytes());
        if let Some(pending) = &state.pending
            && pending.digest != message_digest
        {
            let (Some(cloud), Some(confirmed)) = (&state.cloud_id, &pending.confirmed) else {
                return Err(pending_error());
            };
            let history = self.agent.client.history(cloud).await.map_err(cloud_error)?;
            if !find_turn(&history, confirmed).is_some_and(|turn| terminal(turn.status)) {
                return Err(pending_error());
            }
            state.pending = None;
        }

        let fresh = state.pending.is_none();
        if fresh {
            if active.cancelled.get() {
                return Ok(PromptResponse::new(StopReason::Cancelled));
            }
            state.pending = Some(PendingSend {
                key: Uuid::new_v4().to_string(),
                digest: message_digest,
                confirmed: None,
                cancel_requested: false,
            });
            self.agent.store.write(id, &state)?;
        }
        let pending = state.pending.as_ref().expect("prompt intent is saved");
        if pending.cancel_requested {
            active.cancelled.set(true);
        }
        active.armed.set(true);

        let mut status = None;
        if pending.confirmed.is_none() {
            let key = pending.key.clone();
            let submission = self.submit(state.cloud_id.as_deref(), text, &key).await;
            let lost_response = submission.as_ref().is_err_and(|error| uncertain(*error));
            // An interrupted response may already have committed the turn.
            // Use the original request identity to obtain the ID for cancel.
            let submission = if lost_response && active.cancelled.get() {
                self.submit(state.cloud_id.as_deref(), text, &key).await
            } else {
                submission
            };
            let submission = match submission {
                Ok(submission) => submission,
                Err(error) => {
                    if uncertain(error) || !fresh || lost_response {
                        return Err(pending_error());
                    }
                    state.pending = None;
                    self.agent.store.write(id, &state)?;
                    return Err(cloud_error(error));
                }
            };
            if !self.agent.matches(&submission.conversation)
                || state
                    .cloud_id
                    .as_ref()
                    .is_some_and(|cloud| *cloud != submission.conversation.conversation_id)
                || submission.message.sender != MessageSender::User
                || submission.message.message_id != submission.turn.user_message_id
                || submission.message.body != text
                || !cloud_id(&submission.turn.turn_id)
                || !cloud_id(&submission.turn.user_message_id)
            {
                return Err(pending_error());
            }
            status = Some(submission.turn.status);
            state.cloud_id = Some(submission.conversation.conversation_id);
            let pending = state.pending.as_mut().expect("prompt intent is saved");
            pending.confirmed = Some(ConfirmedSend {
                turn_id: submission.turn.turn_id,
                user_message_id: submission.turn.user_message_id,
                turn_events: submission.turn_events,
            });
            pending.cancel_requested = active.cancelled.get();
            self.agent.store.write(id, &state)?;
        }

        let cloud = state.cloud_id.as_deref().ok_or_else(pending_error)?;
        let confirmed = state
            .pending
            .as_ref()
            .and_then(|pending| pending.confirmed.as_ref())
            .ok_or_else(pending_error)?;
        let outcome = self.wait_for_turn(id, cloud, confirmed, status, active).await?;
        let result = match outcome {
            TurnOutcome::Completed => Ok(PromptResponse::new(StopReason::EndTurn)),
            TurnOutcome::Cancelled => Ok(PromptResponse::new(StopReason::Cancelled)),
            TurnOutcome::Failed => Err(Error::new(-32603, "Employee could not complete this turn")),
        };
        active.armed.set(false);
        state.pending = None;
        self.agent.store.write(id, &state)?;

        result
    }

    async fn submit(
        &self,
        cloud: Option<&str>,
        text: &str,
        key: &str,
    ) -> std::result::Result<ConversationSubmission, CloudError> {
        match cloud {
            Some(cloud) => self.agent.client.send_message(cloud, text, key).await,
            None => {
                self.agent.client.create_conversation(&self.agent.target, text, None, key).await
            }
        }
    }

    async fn wait_for_turn(
        &self,
        id: &str,
        cloud: &str,
        confirmed: &ConfirmedSend,
        mut status: Option<TurnStatus>,
        active: &ActivePrompt,
    ) -> Result<TurnOutcome> {
        let mut delay = POLL_MIN;
        let mut cancel_deadline = None;
        let mut emitted = String::new();
        if confirmed.turn_events
            && !status.is_some_and(terminal)
            && let Some(outcome) =
                self.wait_for_turn_events(id, cloud, confirmed, active, &mut emitted).await?
        {
            return Ok(outcome);
        }
        loop {
            match status {
                Some(TurnStatus::Cancelled) => return Ok(TurnOutcome::Cancelled),
                Some(TurnStatus::Failed) => return Ok(TurnOutcome::Failed),
                _ => {}
            }

            let completed = status == Some(TurnStatus::Completed);
            if completed {
                cancel_deadline = None;
            } else if active.cancelled.get() {
                let deadline =
                    *cancel_deadline.get_or_insert_with(|| Instant::now() + CANCEL_TIMEOUT);
                match time::timeout_at(
                    deadline,
                    self.agent.client.cancel_turn(cloud, &confirmed.turn_id),
                )
                .await
                {
                    Ok(Ok(turn))
                        if turn.turn_id == confirmed.turn_id
                            && turn.user_message_id == confirmed.user_message_id =>
                    {
                        status = Some(turn.status);
                        if terminal(turn.status) {
                            continue;
                        }
                    }
                    Ok(Err(error)) if uncertain(error) || error == CloudError::Conflict => {}
                    Ok(Err(error)) => return Err(cloud_error(error)),
                    _ => return Err(cancel_unconfirmed()),
                }
            }

            let history = self.agent.client.history(cloud);
            let result = if completed {
                history.await
            } else if let Some(deadline) = cancel_deadline {
                time::timeout_at(deadline, history).await.map_err(|_| cancel_unconfirmed())?
            } else {
                tokio::select! {
                    result = history => result,
                    _ = active.changed.notified() => continue,
                }
            };
            match result {
                Ok(history) => {
                    if !completed && let Some(turn) = find_turn(&history, confirmed) {
                        status = Some(turn.status);
                        if turn.status == TurnStatus::Responding
                            && !active.cancelled.get()
                            && turn.partial_reply.len() <= MAX_MESSAGE_BYTES
                            && let Some(suffix) = turn.partial_reply.strip_prefix(emitted.as_str())
                            && !suffix.is_empty()
                        {
                            self.send_reply_chunk(id, ContentChunk::new(suffix.to_owned().into()))
                                .await?;
                            emitted.push_str(suffix);
                            delay = POLL_MIN;
                        }
                    }
                    match status {
                        Some(TurnStatus::Completed) => {
                            if let Some(reply) = history.messages.into_iter().find(|message| {
                                message.sender == MessageSender::Employee
                                    && message.reply_to_message_id.as_deref()
                                        == Some(confirmed.user_message_id.as_str())
                            }) {
                                self.send_final_reply(id, reply, &emitted).await?;
                                return Ok(TurnOutcome::Completed);
                            }
                        }
                        Some(TurnStatus::Cancelled | TurnStatus::Failed) => continue,
                        _ => {}
                    }
                }
                Err(error) if uncertain(error) => {}
                Err(error) => return Err(cloud_error(error)),
            }

            if let Some(deadline) = cancel_deadline {
                time::timeout_at(deadline, time::sleep(delay))
                    .await
                    .map_err(|_| cancel_unconfirmed())?;
            } else {
                tokio::select! {
                    _ = time::sleep(delay) => {},
                    _ = active.changed.notified() => {},
                }
            }
            delay = (delay * 2).min(POLL_MAX);
        }
    }

    async fn send_reply_chunk(&self, id: &str, chunk: ContentChunk) -> Result<()> {
        self.updates
            .send(SessionNotification::new(id.to_owned(), SessionUpdate::AgentMessageChunk(chunk)))
            .await
    }

    async fn wait_for_turn_events(
        &self,
        id: &str,
        cloud: &str,
        confirmed: &ConfirmedSend,
        active: &ActivePrompt,
        emitted: &mut String,
    ) -> Result<Option<TurnOutcome>> {
        while !active.cancelled.get() {
            let result = tokio::select! {
                biased;
                _ = active.changed.notified() => return Ok(None),
                result = self.agent.client.turn_events(cloud, &confirmed.turn_id) => result,
            };
            let mut events = match result {
                Ok(events) => events,
                Err(error @ (CloudError::Unauthorized | CloudError::Forbidden)) => {
                    return Err(cloud_error(error));
                }
                Err(_) => return Ok(None),
            };
            let mut received = false;
            loop {
                if active.cancelled.get() {
                    return Ok(None);
                }
                let result = tokio::select! {
                    biased;
                    _ = active.changed.notified() => return Ok(None),
                    result = events.next_update() => result,
                };
                let update = match result {
                    Ok(Some(update)) => update,
                    Ok(None) | Err(CloudError::Unavailable) => break,
                    Err(error) => return Err(cloud_error(error)),
                };
                let turn = update.turn;
                if turn.turn_id != confirmed.turn_id
                    || turn.user_message_id != confirmed.user_message_id
                {
                    return Err(cloud_error(CloudError::InvalidResponse));
                }
                received = true;
                match turn.status {
                    TurnStatus::Cancelled => return Ok(Some(TurnOutcome::Cancelled)),
                    TurnStatus::Failed => return Ok(Some(TurnOutcome::Failed)),
                    TurnStatus::Completed => {
                        let Some(reply) = update.reply else {
                            return Ok(None);
                        };
                        if reply.sender != MessageSender::Employee
                            || reply.reply_to_message_id.as_deref()
                                != Some(confirmed.user_message_id.as_str())
                        {
                            return Err(cloud_error(CloudError::InvalidResponse));
                        }
                        self.send_final_reply(id, reply, emitted).await?;
                        return Ok(Some(TurnOutcome::Completed));
                    }
                    TurnStatus::Responding => {
                        if turn.partial_reply.len() > MAX_MESSAGE_BYTES {
                            return Err(cloud_error(CloudError::InvalidResponse));
                        }
                        if let Some(suffix) = turn.partial_reply.strip_prefix(emitted.as_str())
                            && !suffix.is_empty()
                        {
                            self.send_reply_chunk(id, ContentChunk::new(suffix.to_owned().into()))
                                .await?;
                            emitted.push_str(suffix);
                        }
                    }
                    _ => {}
                }
            }
            if !received {
                return Ok(None);
            }
            tokio::select! {
                _ = active.changed.notified() => return Ok(None),
                _ = time::sleep(POLL_MIN) => {}
            }
        }
        Ok(None)
    }

    async fn send_final_reply(
        &self,
        id: &str,
        reply: ConversationMessage,
        emitted: &str,
    ) -> Result<()> {
        if !valid_message(&reply) {
            return Err(cloud_error(CloudError::InvalidResponse));
        }
        let chunk = if emitted.is_empty() {
            message_chunk(reply)
        } else {
            let text = match reply.body.strip_prefix(emitted) {
                Some("") => return Ok(()),
                Some(suffix) => suffix.to_owned(),
                // Preserve the canonical answer if the runtime rewrote already displayed text.
                None => format!("\n\n{}", reply.body),
            };
            ContentChunk::new(text.into())
        };
        self.send_reply_chunk(id, chunk).await
    }
}

enum TurnOutcome {
    Completed,
    Cancelled,
    Failed,
}

fn valid_message(message: &ConversationMessage) -> bool {
    cloud_id(&message.message_id) && message.body.len() <= MAX_MESSAGE_BYTES
}

fn message_chunk(message: ConversationMessage) -> ContentChunk {
    ContentChunk::new(message.body.into()).message_id(message.message_id)
}

fn session_directory(cwd: &Path, no_mcp: bool) -> Result<PathBuf> {
    if !no_mcp {
        return Err(Error::new(-32602, "Employee ACP does not support MCP servers"));
    }
    canonical_directory(cwd)
}

fn prompt_text(prompt: &[ContentBlock]) -> Result<String> {
    let mut parts = Vec::with_capacity(prompt.len());
    for block in prompt {
        let ContentBlock::Text(text) = block else {
            return Err(Error::new(-32602, "Employee ACP accepts only text prompt blocks"));
        };
        parts.push(text.text.as_str());
    }
    let text = parts.join("\n");
    let text = text.trim();
    if text.is_empty() || text.len() > MAX_MESSAGE_BYTES {
        return Err(Error::new(
            -32602,
            "Employee prompt must contain 1 to 32768 UTF-8 bytes after trimming",
        ));
    }
    Ok(text.to_owned())
}

fn find_turn<'a>(
    history: &'a ConversationHistory,
    send: &ConfirmedSend,
) -> Option<&'a ConversationTurn> {
    history
        .turns
        .iter()
        .find(|turn| turn.turn_id == send.turn_id && turn.user_message_id == send.user_message_id)
}

fn terminal(status: TurnStatus) -> bool {
    matches!(status, TurnStatus::Cancelled | TurnStatus::Completed | TurnStatus::Failed)
}

fn uncertain(error: CloudError) -> bool {
    matches!(error, CloudError::Unavailable | CloudError::InvalidResponse)
}

fn cloud_error(error: CloudError) -> Error {
    let code = match error {
        CloudError::InvalidCredential | CloudError::Unauthorized => return Error::auth_required(),
        CloudError::InvalidRequest | CloudError::NotFound | CloudError::Forbidden => -32602,
        _ => -32603,
    };
    Error::new(code, error.to_string())
}

fn pending_error() -> Error {
    Error::new(-32603, "Employee turn is unconfirmed; retry the original message to recover it")
}

fn cancel_unconfirmed() -> Error {
    Error::new(-32603, "Employee cancellation is unconfirmed; the pending turn can be recovered")
}

#[cfg(test)]
#[path = "employee_tests.rs"]
mod tests;
