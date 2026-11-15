//! ACP session protocol and bound mount/model state. Host owns native runtime actions.
use std::{collections::HashMap, path::PathBuf, sync::{Arc, Mutex}};
use async_trait::async_trait;
use agent_client_protocol::McpServer;
use acp_server::server::{BackendError, BackendHistoryEntry, BackendRole, BackendSession, BackendPrompt, PromptContent, SessionBackend};
use session_port::{SessionScope, SessionSnapshot, CreateSessionInput, LifecycleError};
use vw_api_transport::id::TenantScope;
use message_types as message;
use crate::{models, backend::prompt_query};

pub struct PreparedMounts<H: Host> {
    pub tools: H::Tools,
    pub skills: H::Skills,
    pub response: acp_types::SetSessionMountsResponse,
    pub mcp_catalog: Vec<acp_types::McpCatalogMount>,
}

/// Strong native handles are retained by the protocol state without copying registries.
#[async_trait]
pub trait Host: Send + Sync + Sized + 'static {
    type Tools: Clone + Send + Sync;
    type Skills: Clone + Send + Sync;
    type Managed: Send;
    async fn create_session(&self, input: CreateSessionInput) -> Result<SessionSnapshot, LifecycleError>;
    async fn load_session(&self, scope: SessionScope, id: &str) -> Result<SessionSnapshot, LifecycleError>;
    fn empty_mounts(&self) -> Self::Managed;
    fn resolve_mounts(&self, scope: &SessionScope, id: &str, project: &str, catalog: &[acp_types::McpCatalogMount], restore: bool) -> Result<Self::Managed, BackendError>;
    async fn prepare_revision(&self, scope: &SessionScope, id: &str, request: acp_server::server::BackendMountRequest, saved: &[acp_types::SessionSkillMount], revision: acp_types::SessionMountRevision, managed: Self::Managed) -> Result<PreparedMounts<Self>, BackendError>;
    async fn save_mounts(&self, scope: &SessionScope, id: &str, prepared: &PreparedMounts<Self>) -> Result<(), BackendError>;
    fn revoke(&self, skills: &Self::Skills);
    fn fingerprint<'a>(&self, skills: &'a Self::Skills) -> Option<&'a str>;
    fn shutdown(&self, tools: &Self::Tools);
    fn tool_count(&self, tools: &Self::Tools) -> usize;
    async fn session_models(&self, scope: SessionScope, selected: String) -> Result<agent_client_protocol::SessionModelState, BackendError>;
    async fn start_bound_prompt(&self, bound: BoundSession<Self>, operation: tokio::sync::OwnedMutexGuard<()>, id: &str, query: String) -> Result<BackendPrompt, BackendError>;
}
pub struct BoundSession<H: Host> {
    pub scope: SessionScope,
    pub tools: H::Tools,
    pub skills: H::Skills,
    pub revision: acp_types::SessionMountRevision,
    pub operation: Arc<tokio::sync::Mutex<()>>,
    pub model: Arc<tokio::sync::Mutex<String>>,
}

/// VibeWindow 原生会话和提示运行时的 ACP 后端。
pub struct VibeWindowBackend<H: Host> {
    host: H,
    tenant: TenantScope,
    sessions: Mutex<HashMap<String, BoundSession<H>>>,
}

impl<H: Host> VibeWindowBackend<H> {
    pub fn new(tenant: TenantScope) -> Self where H: Default { Self::with_host(tenant, H::default()) }
    /// 创建绑定固定租户作用域的 ACP 后端。
    pub fn with_host(tenant: TenantScope, host: H) -> Self {
        Self { host, tenant, sessions: Mutex::new(HashMap::new()) }
    }

    fn scope(&self, cwd: PathBuf) -> Result<SessionScope, BackendError> {
        SessionScope::new(self.tenant.clone(), cwd).map_err(lifecycle_error)
    }

    fn remember_scope(
        &self,
        session_id: &str,
        scope: SessionScope,
        tools: H::Tools,
        skills: H::Skills,
        revision: acp_types::SessionMountRevision,
        stored_model: String,
    ) -> Result<(), BackendError> {
        let mut sessions = self.sessions.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let model = sessions
            .get(session_id)
            .map(|bound| bound.model.clone())
            .unwrap_or_else(|| Arc::new(tokio::sync::Mutex::new(stored_model)));
        let operation = sessions.get(session_id).map(|b| b.operation.clone()).unwrap_or_default();
        if let Some(old) = sessions.insert(
            session_id.to_string(),
            BoundSession { scope, tools, skills, model, revision, operation },
        ) {
            self.host.revoke(&old.skills);
            self.host.shutdown(&old.tools);
        }
        Ok(())
    }

    pub fn bound_session(&self, session_id: &str) -> Result<BoundSession<H>, BackendError> {
        self.sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(session_id)
            .cloned()
            .ok_or_else(|| BackendError::NotFound(session_id.to_string()))
    }

    pub fn lock_prompt(&self, id: &str) -> Result<(BoundSession<H>, tokio::sync::OwnedMutexGuard<()>), BackendError> {
        let operation = self.bound_session(id)?.operation.try_lock_owned()
            .map_err(|_| BackendError::InvalidRequest("session is busy".into()))?;
        Ok((self.bound_session(id)?, operation))
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn mounted_runtime(
        &self,
        session_id: &str,
    ) -> (H::Skills, H::Tools) {
        let bound = self.bound_session(session_id).expect("bound session");
        (bound.skills, bound.tools)
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn session_tool_count(&self, session_id: &str) -> Option<usize> {
        self.sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(session_id)
            .map(|bound| self.host.tool_count(&bound.tools))
    }
}

impl<H: Host> Clone for BoundSession<H> {
    fn clone(&self) -> Self {
        Self { scope: self.scope.clone(), tools: self.tools.clone(), skills: self.skills.clone(), revision: self.revision, operation: self.operation.clone(), model: self.model.clone() }
    }
}
pub fn lifecycle_error(error: session_port::LifecycleError) -> BackendError {
    use session_port::LifecycleError;
    let message = error.to_string();
    match error {
        LifecycleError::SessionNotFound { session_id } => BackendError::NotFound(session_id),
        LifecycleError::SessionOwnershipMismatch { session_id, .. } => {
            BackendError::OwnershipMismatch(session_id)
        }
        LifecycleError::DirectoryMustBeAbsolute { .. }
        | LifecycleError::DirectoryUnavailable { .. }
        | LifecycleError::DirectoryNotDirectory { .. } => BackendError::InvalidRequest(message),
        LifecycleError::Project(_) | LifecycleError::Session(_) => BackendError::Internal(message),
    }
}

fn message_time(info: &message::Info) -> u64 {
    match info {
        message::Info::User(info) => info.time.created,
        message::Info::Assistant(info) => info.time.created,
    }
}

fn message_text(parts: &[message::Part]) -> String {
    parts
        .iter()
        .filter_map(|part| match part {
            message::Part::Text(text) if text.synthetic != Some(true) => Some(text.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn backend_history(mut history: Vec<message::WithParts>) -> Vec<BackendHistoryEntry> {
    // 原生存储按新到旧返回；同毫秒内依靠单调递增消息 ID 恢复因果顺序。
    history.sort_by(|left, right| {
        message_time(&left.info)
            .cmp(&message_time(&right.info))
            .then_with(|| left.info.id().cmp(right.info.id()))
    });
    history
        .into_iter()
        .filter_map(|item| {
            let text = message_text(&item.parts);
            if text.trim().is_empty() {
                return None;
            }
            let role = match item.info {
                message::Info::User(_) => BackendRole::User,
                message::Info::Assistant(_) => BackendRole::Assistant,
            };
            Some(BackendHistoryEntry { role, text })
        })
        .collect()
}

fn backend_session(
    snapshot: session_port::SessionSnapshot,
    scope: &SessionScope,
) -> BackendSession {
    BackendSession {
        id: snapshot.info.id,
        cwd: scope.directory().to_path_buf(),
        history: backend_history(snapshot.history),
    }
}

#[async_trait]
impl<H: Host> SessionBackend for VibeWindowBackend<H> {
    fn supports_mount_updates(&self) -> bool {
        true
    }

    async fn set_session_mounts(
        &self,
        request: acp_types::SetSessionMountsRequest,
    ) -> Result<acp_types::SetSessionMountsResponse, BackendError> {
        let id = request.session_id.to_string();
        let bound = self.bound_session(&id)?;
        let operation = bound.operation.clone();
        let _operation = operation
            .try_lock()
            .map_err(|_| BackendError::InvalidRequest("session is busy".into()))?;
        let bound = self.bound_session(&id)?;
        let revision = bound
            .revision
            .next(request.expected_revision)
            .map_err(|e| BackendError::InvalidRequest(e.to_string()))?;
        let snapshot = self.host.load_session(bound.scope.clone(), &id).await.map_err(lifecycle_error)?;
        let managed = self.host.resolve_mounts(
            &bound.scope,
            &id,
            &snapshot.info.project_id,
            &request.mcp_catalog,
            false,
        )?;
        let mut prepared = self.host.prepare_revision(
            &bound.scope,
            &id,
            acp_server::server::BackendMountRequest {
                skills: Some(acp_types::SessionMountSelection { version: 1, skills: request.skills }),
                mcp_servers: request.mcp_servers,
            },
            &[],
            revision,
            managed,
        )
        .await?;
        // Invocation checkpoints are bound to the committed revision. Failed writes
        // retain both the previous mounts and their replayable checkpoint generation.
        self.host.save_mounts(&bound.scope, &id, &prepared).await?;
        prepared.response.revision = revision;
        let mut sessions = self.sessions.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let current = sessions.get_mut(&id).ok_or_else(|| BackendError::NotFound(id.clone()))?;
        if current.revision != bound.revision {
            return Err(BackendError::InvalidRequest("session mounts revision conflict".into()));
        }
        let old_tools = std::mem::replace(&mut current.tools, prepared.tools);
        self.host.revoke(&current.skills);
        current.skills = prepared.skills;
        current.revision = revision;
        self.host.shutdown(&old_tools);
        Ok(prepared.response)
    }

    fn supports_skill_mounts(&self) -> bool {
        cfg!(unix)
    }
    async fn create_session_with_mounts(
        &self,
        cwd: PathBuf,
        request: acp_server::server::BackendMountRequest,
    ) -> Result<acp_server::server::BackendMountedSession, BackendError> {
        let scope = self.scope(cwd)?;
        let snapshot = self.host.create_session(CreateSessionInput {
            scope: scope.clone(),
            parent_id: None,
            title: None,
            permission: None,
        })
        .await
        .map_err(lifecycle_error)?;
        let session = backend_session(snapshot, &scope);
        let prepared = self.host.prepare_revision(&scope, &session.id, request, &[], acp_types::SessionMountRevision(0), self.host.empty_mounts()).await?;
        let stored_model = models::load_selection(&scope, &session.id).await?;
        self.host.save_mounts(&scope, &session.id, &prepared).await?;
        self.remember_scope(
            &session.id,
            scope,
            prepared.tools,
            prepared.skills,
            prepared.response.revision,
            stored_model,
        )?;
        Ok(acp_server::server::BackendMountedSession { session, mounts: prepared.response })
    }

    async fn load_session_with_mounts(
        &self,
        session_id: &str,
        cwd: PathBuf,
        request: acp_server::server::BackendMountRequest,
    ) -> Result<acp_server::server::BackendMountedSession, BackendError> {
        let scope = self.scope(cwd)?;
        let existing = self.bound_session(session_id).ok();
        let _operation = existing
            .as_ref()
            .map(|bound| bound.operation.try_lock())
            .transpose()
            .map_err(|_| BackendError::InvalidRequest("session is busy".into()))?;
        let snapshot = self.host.load_session(scope.clone(), session_id).await.map_err(lifecycle_error)?;
        let record = crate::mount_store::load(&scope, session_id).await?;
        let restoring = request.skills.is_none();
        let previous_revision =
            record.as_ref().map(|r| r.revision).unwrap_or(acp_types::SessionMountRevision(0));
        let revision = if !restoring && record.is_some() {
            previous_revision
                .next(previous_revision)
                .map_err(|e| BackendError::InvalidRequest(e.to_string()))?
        } else {
            previous_revision
        };
        let managed = self.host.resolve_mounts(
            &scope,
            session_id,
            &snapshot.info.project_id,
            record.as_ref().map(|record| record.mcp_catalog.as_slice()).unwrap_or_default(),
            true,
        )?;
        let prepared = self.host.prepare_revision(
            &scope,
            session_id,
            request,
            record.as_ref().map(|r| r.skills.as_slice()).unwrap_or_default(),
            revision,
            managed,
        )
        .await?;
        if restoring
            && record.as_ref().is_some_and(|record| {
                record.fingerprint.as_deref() != self.host.fingerprint(&prepared.skills)
            })
        {
            return Err(BackendError::InvalidRequest(
                "persisted session skill source identity or content changed".into(),
            ));
        }
        let stored_model = models::load_selection(&scope, session_id).await?;
        self.host.save_mounts(&scope, session_id, &prepared).await?;
        let session = backend_session(snapshot, &scope);
        self.remember_scope(
            session_id,
            scope,
            prepared.tools,
            prepared.skills,
            revision,
            stored_model,
        )?;
        Ok(acp_server::server::BackendMountedSession { session, mounts: prepared.response })
    }

    async fn session_models(
        &self,
        session_id: &str,
    ) -> Result<Option<agent_client_protocol::SessionModelState>, BackendError> {
        let bound = self.bound_session(session_id)?;
        let selected = bound.model.lock().await.clone();
        self.host.session_models(bound.scope, selected).await.map(Some)
    }

    async fn set_session_model(
        &self,
        session_id: &str,
        model_id: &str,
    ) -> Result<agent_client_protocol::SessionModelState, BackendError> {
        let bound = self.bound_session(session_id)?;
        let mut selected = bound.model.lock().await;
        let models = self.host.session_models(bound.scope.clone(), model_id.to_string()).await?;
        models::save_selection(&bound.scope, session_id, model_id).await?;
        *selected = model_id.to_string();
        Ok(models)
    }

    async fn create_session(&self, cwd: PathBuf) -> Result<BackendSession, BackendError> {
        self.create_session_with_mcp(cwd, Vec::new()).await
    }

    async fn create_session_with_mcp(
        &self,
        cwd: PathBuf,
        mcp_servers: Vec<McpServer>,
    ) -> Result<BackendSession, BackendError> {
        Ok(self
            .create_session_with_mounts(
                cwd,
                acp_server::server::BackendMountRequest { skills: None, mcp_servers },
            )
            .await?
            .session)
    }

    async fn load_session(
        &self,
        session_id: &str,
        cwd: PathBuf,
    ) -> Result<BackendSession, BackendError> {
        self.load_session_with_mcp(session_id, cwd, Vec::new()).await
    }

    async fn load_session_with_mcp(
        &self,
        session_id: &str,
        cwd: PathBuf,
        mcp_servers: Vec<McpServer>,
    ) -> Result<BackendSession, BackendError> {
        Ok(self
            .load_session_with_mounts(
                session_id,
                cwd,
                acp_server::server::BackendMountRequest { skills: None, mcp_servers },
            )
            .await?
            .session)
    }

    async fn start_prompt(
        &self,
        session_id: &str,
        prompt: Vec<PromptContent>,
    ) -> Result<BackendPrompt, BackendError> {
        let (bound, operation) = self.lock_prompt(session_id)?;
        self.host.start_bound_prompt(bound, operation, session_id, prompt_query(prompt)).await
    }
}

impl<H: Host + Default> Default for VibeWindowBackend<H> { fn default() -> Self { Self::new(TenantScope::default()) } }
