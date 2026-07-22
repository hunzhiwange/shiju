//! Agent 连接建立与已有会话加载流程。
//!
//! 本模块负责在“已有会话记录”与“当前 agent 连接状态”之间搭桥，
//! 既支持新建连接，也支持在需要时恢复并加载已有会话内容。
//!
//! 它的核心目标是把连接建立、session/load 和会话记录回放整合为单一流程。

use std::path::Path;
use std::sync::Arc;

use agent_client_protocol::{
    SessionConfigOption, SessionModelState, SetSessionConfigOptionResponse,
};
use async_trait::async_trait;

use crate::acp_error_shapes::is_acp_resource_not_found_error;
use crate::error::AcpError;
use crate::errors::{
    AcpxErrorOptions, SessionModeReplayError, SessionModelReplayError, SessionResumeRequiredError,
};
use crate::perf_metrics::increment_perf_counter;
use crate::queue_lease_store::is_process_alive;
use crate::queue_owner_turn_controller::QueueOwnerActiveSessionController;
use crate::session_mode_preference::{
    get_desired_config_options, get_desired_mode_id, get_desired_model_id, resolve_config_value,
    resolve_model_config_value, set_current_model_id, sync_advertised_model_state,
    sync_config_options,
};
use crate::session_runtime::lifecycle::{
    AgentLifecycleSnapshot, apply_lifecycle_snapshot_to_record, reconcile_agent_session_id,
    session_has_agent_messages,
};
use crate::session_runtime_helpers::TimeoutError;
use crate::types::{SessionInfo, SessionRecord, SessionResumePolicy};

type OnClientAvailable = Arc<dyn Fn(Arc<dyn QueueOwnerActiveSessionController>) + Send + Sync>;
type OnConnectedRecord = Arc<dyn Fn(&SessionRecord) + Send + Sync>;
type OnSessionIdResolved = Arc<dyn Fn(&str) + Send + Sync>;

const SESSION_LOAD_UNSUPPORTED_CODES: [&str; 2] = ["-32601", "-32602"];
const QUERY_CLOSED_BEFORE_RESPONSE_DETAIL: &str = "query closed before response";

#[derive(Debug, Clone, Default)]
pub struct ConnectAndLoadClientSession {
    pub session_id: String,
    pub agent_session_id: Option<String>,
    pub models: Option<SessionModelState>,
    pub config_options: Option<Vec<SessionConfigOption>>,
}

#[async_trait(?Send)]
pub trait ConnectAndLoadClient: Send + Sync {
    async fn start(&self) -> Result<(), AcpError>;
    async fn create_session(&self, cwd: &Path) -> Result<ConnectAndLoadClientSession, AcpError>;
    async fn load_session_with_options(
        &self,
        session_id: &str,
        cwd: &Path,
    ) -> Result<ConnectAndLoadClientSession, AcpError>;
    async fn set_session_mode(
        &self,
        session_id: &str,
        cwd: &Path,
        mode_id: &str,
    ) -> Result<(), AcpError>;
    async fn set_session_model(
        &self,
        session_id: &str,
        cwd: &Path,
        model_id: &str,
    ) -> Result<(), AcpError>;
    async fn set_session_config_option(
        &self,
        session_id: &str,
        cwd: &Path,
        config_id: &str,
        value: &str,
    ) -> Result<SetSessionConfigOptionResponse, AcpError>;
    fn has_reusable_session(&self, session_id: &str) -> bool;
    fn supports_load_session(&self) -> bool;
    fn get_agent_lifecycle_snapshot(&self) -> AgentLifecycleSnapshot;
}

#[async_trait(?Send)]
impl ConnectAndLoadClient for crate::AcpClient {
    async fn start(&self) -> Result<(), AcpError> {
        crate::AcpClient::start(self).await
    }

    async fn create_session(&self, cwd: &Path) -> Result<ConnectAndLoadClientSession, AcpError> {
        let SessionInfo { session_id, models, config_options } =
            crate::AcpClient::create_session(self, cwd).await?;
        Ok(ConnectAndLoadClientSession {
            session_id,
            agent_session_id: None,
            models,
            config_options,
        })
    }

    async fn load_session_with_options(
        &self,
        session_id: &str,
        cwd: &Path,
    ) -> Result<ConnectAndLoadClientSession, AcpError> {
        let SessionInfo { session_id, models, config_options } =
            crate::AcpClient::load_session(self, session_id, cwd).await?;
        Ok(ConnectAndLoadClientSession {
            session_id,
            agent_session_id: None,
            models,
            config_options,
        })
    }

    async fn set_session_mode(
        &self,
        session_id: &str,
        cwd: &Path,
        mode_id: &str,
    ) -> Result<(), AcpError> {
        crate::AcpClient::set_session_mode(self, session_id, cwd, mode_id).await
    }

    async fn set_session_model(
        &self,
        session_id: &str,
        cwd: &Path,
        model_id: &str,
    ) -> Result<(), AcpError> {
        crate::AcpClient::set_session_model(self, session_id, cwd, model_id).await
    }

    async fn set_session_config_option(
        &self,
        session_id: &str,
        cwd: &Path,
        config_id: &str,
        value: &str,
    ) -> Result<SetSessionConfigOptionResponse, AcpError> {
        crate::AcpClient::set_session_config_option(self, session_id, cwd, config_id, value).await
    }

    fn has_reusable_session(&self, _session_id: &str) -> bool {
        crate::AcpClient::has_reusable_session(self, _session_id)
    }

    fn supports_load_session(&self) -> bool {
        true
    }

    fn get_agent_lifecycle_snapshot(&self) -> AgentLifecycleSnapshot {
        crate::AcpClient::get_agent_lifecycle_snapshot(self)
    }
}

pub struct ConnectAndLoadSessionOptions<'a, C>
where
    C: ConnectAndLoadClient,
{
    pub client: Arc<C>,
    pub record: &'a mut SessionRecord,
    pub resume_policy: Option<SessionResumePolicy>,
    pub timeout_ms: Option<u64>,
    pub verbose: bool,
    pub active_controller: Arc<dyn QueueOwnerActiveSessionController>,
    pub on_client_available: Option<OnClientAvailable>,
    pub on_connected_record: Option<OnConnectedRecord>,
    pub on_session_id_resolved: Option<OnSessionIdResolved>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectAndLoadSessionResult {
    pub session_id: String,
    pub agent_session_id: Option<String>,
    pub resumed: bool,
    pub load_error: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectAndLoadSessionError {
    #[error(transparent)]
    Timeout(#[from] TimeoutError),
    #[error(transparent)]
    Acp(#[from] AcpError),
    #[error(transparent)]
    SessionResumeRequired(#[from] SessionResumeRequiredError),
    #[error(transparent)]
    SessionModeReplay(#[from] SessionModeReplayError),
    #[error(transparent)]
    SessionModelReplay(#[from] SessionModelReplayError),
}

pub async fn connect_and_load_session<C>(
    options: ConnectAndLoadSessionOptions<'_, C>,
) -> Result<ConnectAndLoadSessionResult, ConnectAndLoadSessionError>
where
    C: ConnectAndLoadClient,
{
    let ConnectAndLoadSessionOptions {
        client,
        record,
        resume_policy,
        timeout_ms,
        verbose,
        active_controller,
        on_client_available,
        on_connected_record,
        on_session_id_resolved,
    } = options;
    let same_session_only = requires_same_session(resume_policy);
    let original_session_id = record.acp_session_id.clone();
    let original_agent_session_id = record.agent_session_id.clone();
    let desired_mode_id = get_desired_mode_id(record.vwacp.as_ref());
    let desired_model_id = get_desired_model_id(record.vwacp.as_ref());
    let desired_config_options = get_desired_config_options(record.vwacp.as_ref());
    let stored_process_alive = is_process_alive(record.pid);
    let should_reconnect = record.pid.is_some() && !stored_process_alive;
    let cwd = std::path::PathBuf::from(&record.cwd);

    if verbose {
        if stored_process_alive {
            eprintln!(
                "[vwacp] saved session pid {} is running; reconnecting with loadSession",
                record.pid.unwrap_or_default()
            );
        } else if should_reconnect {
            eprintln!(
                "[vwacp] saved session pid {} is dead; respawning agent and attempting session/load",
                record.pid.unwrap_or_default()
            );
        }
    }

    let reusing_loaded_session = client.has_reusable_session(&record.acp_session_id);
    if reusing_loaded_session {
        increment_perf_counter("runtime.connect_and_load.reused_session", 1);
    } else {
        with_timeout_result(client.start(), timeout_ms).await?;
    }
    if let Some(callback) = on_client_available.as_ref() {
        callback(active_controller.clone());
    }
    apply_lifecycle_snapshot_to_record(record, &client.get_agent_lifecycle_snapshot());
    record.closed = Some(false);
    record.closed_at = None;
    if let Some(callback) = on_connected_record.as_ref() {
        callback(record);
    }

    let mut resumed = false;
    let mut load_error = None;
    let mut session_id = record.acp_session_id.clone();
    let mut created_fresh_session = false;
    let mut pending_agent_session_id = record.agent_session_id.clone();
    let mut session_models = None;
    let mut session_config_options = None;

    if reusing_loaded_session {
        resumed = true;
    } else if client.supports_load_session() {
        let is_verbose = options.verbose;
        match with_timeout_result(
            client.load_session_with_options(&record.acp_session_id, cwd.as_path()),
            timeout_ms,
        )
        .await
        {
            Ok(load_result) => {
                reconcile_agent_session_id(record, load_result.agent_session_id.as_deref());
                session_models = load_result.models;
                session_config_options = load_result.config_options;
                resumed = true;
            }
            Err(error) => {
                load_error = Some(error.to_string());
                if same_session_only {
                    return Err(make_session_resume_required_error(
                        record,
                        &error.to_string(),
                        error,
                    ));
                }
                if !should_fallback_to_new_session(&error, record) {
                    return Err(error);
                }
                if is_verbose {
                    eprintln!("[vwacp] loadSession failed, started fresh session: {}", error);
                }
                let created_session =
                    with_timeout_result(client.create_session(cwd.as_path()), timeout_ms).await?;
                session_id = created_session.session_id;
                created_fresh_session = true;
                pending_agent_session_id = created_session.agent_session_id;
                session_models = created_session.models;
                session_config_options = created_session.config_options;
            }
        }
    } else {
        if same_session_only {
            return Err(
                SessionResumeRequiredError::new(
                    format!(
                        "Persistent ACP session {} could not be resumed: agent does not support session/load",
                        record.acp_session_id
                    ),
                    AcpxErrorOptions {
                        retryable: Some(true),
                        ..AcpxErrorOptions::default()
                    },
                )
                .into(),
            );
        }
        let created_session =
            with_timeout_result(client.create_session(cwd.as_path()), timeout_ms).await?;
        session_id = created_session.session_id;
        created_fresh_session = true;
        pending_agent_session_id = created_session.agent_session_id;
        session_models = created_session.models;
        session_config_options = created_session.config_options;
    }

    if created_fresh_session && let Some(desired_model_id) = desired_model_id.as_deref() {
        let model_config_id =
            resolve_model_config_value(session_config_options.as_deref(), desired_model_id)
                .map_err(AcpError::SetSessionConfigOption)?;
        let model_result = if let Some(config_id) = model_config_id {
            with_timeout_result(
                client.set_session_config_option(
                    &session_id,
                    cwd.as_path(),
                    &config_id,
                    desired_model_id,
                ),
                timeout_ms,
            )
            .await
            .map(|response| session_config_options = Some(response.config_options))
        } else if session_models
            .as_ref()
            .is_none_or(|models| desired_model_id != models.current_model_id.to_string())
        {
            with_timeout_result(
                client.set_session_model(&session_id, cwd.as_path(), desired_model_id),
                timeout_ms,
            )
            .await
        } else {
            Ok(())
        };
        if let Err(error) = model_result {
            let message = format!(
                "Failed to replay saved session model {desired_model_id} on fresh ACP session {session_id}: {error}"
            );
            record.acp_session_id = original_session_id;
            record.agent_session_id = original_agent_session_id;
            return Err(SessionModelReplayError::new(
                message,
                AcpxErrorOptions {
                    source: Some(Box::new(error)),
                    retryable: Some(true),
                    ..AcpxErrorOptions::default()
                },
            )
            .into());
        }
    }

    if created_fresh_session {
        let is_reasoning = |id: &str| matches!(id, "reasoning_effort" | "thought_level" | "effort");
        for (config_id, value) in desired_config_options
            .iter()
            .filter(|(config_id, _)| is_reasoning(config_id))
            .chain(desired_config_options.iter().filter(|(config_id, _)| !is_reasoning(config_id)))
        {
            let resolved_id =
                resolve_config_value(session_config_options.as_deref(), config_id, value)
                    .map_err(AcpError::SetSessionConfigOption)?;
            let response = with_timeout_result(
                client.set_session_config_option(&session_id, cwd.as_path(), &resolved_id, value),
                timeout_ms,
            )
            .await?;
            session_config_options = Some(response.config_options);
        }
    }

    if created_fresh_session
        && let Some(desired_mode_id) = desired_mode_id.as_deref()
        && let Err(error) = with_timeout_result(
            client.set_session_mode(&session_id, cwd.as_path(), desired_mode_id),
            timeout_ms,
        )
        .await
    {
        let message = format!(
            "Failed to replay saved session mode {desired_mode_id} on fresh ACP session {session_id}: {}",
            error
        );
        record.acp_session_id = original_session_id;
        record.agent_session_id = original_agent_session_id;
        if verbose {
            eprintln!("[vwacp] {message}");
        }
        return Err(SessionModeReplayError::new(
            message,
            AcpxErrorOptions {
                source: Some(Box::new(error)),
                retryable: Some(true),
                ..AcpxErrorOptions::default()
            },
        )
        .into());
    }

    if created_fresh_session {
        record.acp_session_id = session_id.clone();
        reconcile_agent_session_id(record, pending_agent_session_id.as_deref());
    }

    sync_advertised_model_state(record, session_models.as_ref());
    sync_config_options(record, session_config_options);
    if created_fresh_session
        && let Some(desired_model_id) = desired_model_id.as_deref()
        && session_models.is_some()
    {
        set_current_model_id(record, Some(desired_model_id));
    }

    if let Some(callback) = on_session_id_resolved.as_ref() {
        callback(&session_id);
    }

    Ok(ConnectAndLoadSessionResult {
        session_id,
        agent_session_id: record.agent_session_id.clone(),
        resumed,
        load_error,
    })
}

fn requires_same_session(resume_policy: Option<SessionResumePolicy>) -> bool {
    resume_policy == Some(SessionResumePolicy::SameSessionOnly)
}

fn should_fallback_to_new_session(
    error: &ConnectAndLoadSessionError,
    record: &SessionRecord,
) -> bool {
    if matches!(error, ConnectAndLoadSessionError::Timeout(_)) {
        return false;
    }

    let message = error.to_string();
    let normalized = message.to_ascii_lowercase();

    if is_resource_not_found_like(&message) {
        return true;
    }

    if SESSION_LOAD_UNSUPPORTED_CODES.iter().any(|code| normalized.contains(code)) {
        return true;
    }

    if !session_has_agent_messages(record)
        && (normalized.contains(QUERY_CLOSED_BEFORE_RESPONSE_DETAIL)
            || normalized.contains("-32603"))
    {
        return true;
    }

    false
}

fn is_resource_not_found_like(message: &str) -> bool {
    let value = serde_json::Value::String(message.to_string());
    if is_acp_resource_not_found_error(&value) {
        return true;
    }

    let normalized = message.to_ascii_lowercase();
    normalized.contains("resource not found")
        || normalized.contains("session not found")
        || normalized.contains("not found")
}

fn make_session_resume_required_error(
    record: &SessionRecord,
    reason: &str,
    cause: ConnectAndLoadSessionError,
) -> ConnectAndLoadSessionError {
    SessionResumeRequiredError::new(
        format!("Persistent ACP session {} could not be resumed: {reason}", record.acp_session_id),
        AcpxErrorOptions {
            source: Some(Box::new(cause)),
            retryable: Some(true),
            ..AcpxErrorOptions::default()
        },
    )
    .into()
}

async fn with_timeout_result<T, F, E>(
    future: F,
    timeout_ms: Option<u64>,
) -> Result<T, ConnectAndLoadSessionError>
where
    F: std::future::Future<Output = Result<T, E>>,
    E: Into<ConnectAndLoadSessionError>,
{
    crate::session_runtime_helpers::with_timeout(future, timeout_ms).await?.map_err(Into::into)
}
