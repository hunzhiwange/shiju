//! Prompt 执行流程中的连接、权限与恢复控制。
//!
//! 本模块承载发送 prompt 前后的主要编排逻辑，是会话运行时的核心执行器之一。
//! 它负责准备连接、装配控制器、处理权限、执行 prompt 并消费返回消息。
//!
//! 同时这里还承担会话恢复、模型切换、模式切换和配置更新等直接控制操作。

use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use agent_client_protocol::{McpServer, SetSessionConfigOptionResponse};
use parking_lot::Mutex;
use tokio::sync::Mutex as AsyncMutex;

use crate::error::AcpError;
use crate::errors::{AcpxErrorOptions, QueueConnectionError};
use crate::queue_ipc::{
    try_set_config_option_on_running_owner, try_set_mode_on_running_owner,
    try_set_model_on_running_owner,
};
use crate::queue_owner_turn_controller::{QueueControlFuture, QueueOwnerActiveSessionController};
use crate::session_mode_preference::{
    resolve_config_value, resolve_model_config_value, set_current_model_id,
    set_desired_config_option, set_desired_mode_id, set_desired_model_id, sync_config_options,
};
use crate::session_persistence::{
    absolute_path, iso_now, resolve_session_record, write_session_record,
};
use crate::session_runtime::connect_load::ConnectAndLoadClient;
use crate::session_runtime::lifecycle::apply_lifecycle_snapshot_to_record;
use crate::session_runtime_helpers::{
    InterruptedError, TimeoutError, WithInterruptError, with_interrupt, with_timeout,
};
use crate::types::{
    AcpAgentConfig, AcpSessionOptions, AuthPolicy, NonInteractivePermissionPolicy, PermissionMode,
    SessionRecord, SessionSetConfigOptionResult, SessionSetModeResult, SessionSetModelResult,
    SessionStateOptions,
};

#[cfg(test)]
#[path = "prompt_runner_tests.rs"]
mod prompt_runner_tests;

pub type ActiveSessionController = Arc<dyn QueueOwnerActiveSessionController>;
pub type ClientAvailableCallback = Arc<dyn Fn(ActiveSessionController) + Send + Sync>;
pub type ClientClosedCallback = Arc<dyn Fn() + Send + Sync>;

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, PromptRunnerError>> + 'a>>;

struct WithConnectedSessionOptions {
    session_record_id: String,
    mcp_servers: Option<Vec<McpServer>>,
    permission_mode: Option<PermissionMode>,
    non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    auth_credentials: Option<HashMap<String, String>>,
    auth_policy: Option<AuthPolicy>,
    verbose: bool,
    on_client_available: Option<ClientAvailableCallback>,
    on_client_closed: Option<ClientClosedCallback>,
}

struct WithConnectedSessionResult<T> {
    value: T,
    record: SessionRecord,
    resumed: bool,
    load_error: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum PromptRunnerError {
    #[error(transparent)]
    SessionRepository(#[from] crate::session_persistence::SessionRepositoryError),
    #[error(transparent)]
    QueueConnection(#[from] QueueConnectionError),
    #[error(transparent)]
    ConnectAndLoad(#[from] crate::session_runtime::connect_load::ConnectAndLoadSessionError),
    #[error(transparent)]
    Acp(#[from] AcpError),
    #[error(transparent)]
    Timeout(#[from] TimeoutError),
    #[error(transparent)]
    Interrupted(#[from] InterruptedError),
    #[error("failed to install signal handler: {0}")]
    SignalHandler(#[source] std::io::Error),
}

struct PromptRunnerActiveSessionController {
    client: Arc<crate::AcpClient>,
    cwd: PathBuf,
    session_id: Arc<Mutex<String>>,
}

impl PromptRunnerActiveSessionController {
    fn new(client: Arc<crate::AcpClient>, cwd: PathBuf, session_id: Arc<Mutex<String>>) -> Self {
        Self { client, cwd, session_id }
    }
}

impl QueueOwnerActiveSessionController for PromptRunnerActiveSessionController {
    fn has_active_prompt(&self) -> bool {
        self.client.has_active_prompt()
    }

    fn request_cancel_active_prompt(&self) -> QueueControlFuture<bool> {
        let client = self.client.clone();
        queue_control_task(move || async move {
            client.request_cancel_active_prompt().await.map_err(queue_connection_error)
        })
    }

    fn set_session_mode(&self, mode_id: String) -> QueueControlFuture<()> {
        let client = self.client.clone();
        let cwd = self.cwd.clone();
        let session_id = self.session_id.lock().clone();
        queue_control_task(move || async move {
            client.set_session_mode(session_id, cwd, mode_id).await.map_err(queue_connection_error)
        })
    }

    fn set_session_model(&self, model_id: String) -> QueueControlFuture<()> {
        let client = self.client.clone();
        let cwd = self.cwd.clone();
        let session_id = self.session_id.lock().clone();
        queue_control_task(move || async move {
            client
                .set_session_model(session_id, cwd, model_id)
                .await
                .map_err(queue_connection_error)
        })
    }

    fn set_session_config_option(
        &self,
        config_id: String,
        value: String,
    ) -> QueueControlFuture<SetSessionConfigOptionResponse> {
        let client = self.client.clone();
        let cwd = self.cwd.clone();
        let session_id = self.session_id.lock().clone();
        queue_control_task(move || async move {
            client
                .set_session_config_option(session_id, cwd, config_id, value)
                .await
                .map_err(queue_connection_error)
        })
    }
}

pub struct RunSessionSetModeDirectOptions {
    pub session_record_id: String,
    pub mode_id: String,
    pub mcp_servers: Option<Vec<McpServer>>,
    pub non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    pub auth_credentials: Option<HashMap<String, String>>,
    pub auth_policy: Option<AuthPolicy>,
    pub timeout_ms: Option<u64>,
    pub verbose: bool,
    pub on_client_available: Option<ClientAvailableCallback>,
    pub on_client_closed: Option<ClientClosedCallback>,
}

pub struct SessionSetModeOptions {
    pub session_id: String,
    pub mode_id: String,
    pub mcp_servers: Option<Vec<McpServer>>,
    pub non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    pub auth_credentials: Option<HashMap<String, String>>,
    pub auth_policy: Option<AuthPolicy>,
    pub timeout_ms: Option<u64>,
    pub verbose: bool,
}

pub struct RunSessionSetConfigOptionDirectOptions {
    pub session_record_id: String,
    pub config_id: String,
    pub value: String,
    pub mcp_servers: Option<Vec<McpServer>>,
    pub non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    pub auth_credentials: Option<HashMap<String, String>>,
    pub auth_policy: Option<AuthPolicy>,
    pub timeout_ms: Option<u64>,
    pub verbose: bool,
    pub on_client_available: Option<ClientAvailableCallback>,
    pub on_client_closed: Option<ClientClosedCallback>,
}

pub struct SessionSetConfigOptionOptions {
    pub session_id: String,
    pub config_id: String,
    pub value: String,
    pub mcp_servers: Option<Vec<McpServer>>,
    pub non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    pub auth_credentials: Option<HashMap<String, String>>,
    pub auth_policy: Option<AuthPolicy>,
    pub timeout_ms: Option<u64>,
    pub verbose: bool,
}

pub struct RunSessionSetModelDirectOptions {
    pub session_record_id: String,
    pub model_id: String,
    pub mcp_servers: Option<Vec<McpServer>>,
    pub non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    pub auth_credentials: Option<HashMap<String, String>>,
    pub auth_policy: Option<AuthPolicy>,
    pub timeout_ms: Option<u64>,
    pub verbose: bool,
    pub on_client_available: Option<ClientAvailableCallback>,
    pub on_client_closed: Option<ClientClosedCallback>,
}

pub struct SessionSetModelOptions {
    pub session_id: String,
    pub model_id: String,
    pub mcp_servers: Option<Vec<McpServer>>,
    pub non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    pub auth_credentials: Option<HashMap<String, String>>,
    pub auth_policy: Option<AuthPolicy>,
    pub timeout_ms: Option<u64>,
    pub verbose: bool,
}

pub async fn set_session_mode(
    options: SessionSetModeOptions,
) -> Result<SessionSetModeResult, PromptRunnerError> {
    let submitted_to_owner = try_set_mode_on_running_owner(
        &options.session_id,
        &options.mode_id,
        options.timeout_ms,
        options.verbose,
    )
    .await?;

    if submitted_to_owner == Some(true) {
        let mut record = resolve_session_record(&options.session_id).await?;
        set_desired_mode_id(&mut record, Some(&options.mode_id));
        write_session_record(&record).await?;

        return Ok(SessionSetModeResult { record, resumed: false, load_error: None });
    }

    run_session_set_mode_direct(RunSessionSetModeDirectOptions {
        session_record_id: options.session_id,
        mode_id: options.mode_id,
        mcp_servers: options.mcp_servers,
        non_interactive_permissions: options.non_interactive_permissions,
        auth_credentials: options.auth_credentials,
        auth_policy: options.auth_policy,
        timeout_ms: options.timeout_ms,
        verbose: options.verbose,
        on_client_available: None,
        on_client_closed: None,
    })
    .await
}

pub async fn run_session_set_mode_direct(
    options: RunSessionSetModeDirectOptions,
) -> Result<SessionSetModeResult, PromptRunnerError> {
    let mode_id = options.mode_id.clone();
    let timeout_ms = options.timeout_ms;
    let result = with_connected_session(
        WithConnectedSessionOptions {
            session_record_id: options.session_record_id,
            mcp_servers: options.mcp_servers,
            permission_mode: None,
            non_interactive_permissions: options.non_interactive_permissions,
            auth_credentials: options.auth_credentials,
            auth_policy: options.auth_policy,
            verbose: options.verbose,
            on_client_available: options.on_client_available,
            on_client_closed: options.on_client_closed,
        },
        move |client, session_id, record| {
            let mode_id = mode_id.clone();
            Box::pin(async move {
                with_timeout(
                    client.set_session_mode(
                        session_id,
                        absolute_path(&record.cwd),
                        mode_id.clone(),
                    ),
                    timeout_ms,
                )
                .await??;
                set_desired_mode_id(record, Some(&mode_id));
                Ok(())
            })
        },
    )
    .await?;

    Ok(SessionSetModeResult {
        record: result.record,
        resumed: result.resumed,
        load_error: result.load_error,
    })
}

pub async fn set_session_model(
    options: SessionSetModelOptions,
) -> Result<SessionSetModelResult, PromptRunnerError> {
    let mut record = resolve_session_record(&options.session_id).await?;
    let model_config_id = resolve_model_config_value(
        record.vwacp.as_ref().and_then(|state| state.config_options.as_deref()),
        &options.model_id,
    )
    .map_err(AcpError::SetSessionConfigOption)?;
    if let Some(config_id) = model_config_id.as_deref()
        && let Some(response) = try_set_config_option_on_running_owner(
            &options.session_id,
            config_id,
            &options.model_id,
            options.timeout_ms,
            options.verbose,
        )
        .await?
    {
        set_desired_model_id(&mut record, Some(&options.model_id));
        set_current_model_id(&mut record, Some(&options.model_id));
        sync_config_options(&mut record, Some(response.config_options));
        write_session_record(&record).await?;
        return Ok(SessionSetModelResult { record, resumed: false, load_error: None });
    }

    let submitted_to_owner = if model_config_id.is_none() {
        try_set_model_on_running_owner(
            &options.session_id,
            &options.model_id,
            options.timeout_ms,
            options.verbose,
        )
        .await?
    } else {
        None
    };

    if submitted_to_owner == Some(true) {
        set_desired_model_id(&mut record, Some(&options.model_id));
        set_current_model_id(&mut record, Some(&options.model_id));
        write_session_record(&record).await?;

        return Ok(SessionSetModelResult { record, resumed: false, load_error: None });
    }

    run_session_set_model_direct(RunSessionSetModelDirectOptions {
        session_record_id: options.session_id,
        model_id: options.model_id,
        mcp_servers: options.mcp_servers,
        non_interactive_permissions: options.non_interactive_permissions,
        auth_credentials: options.auth_credentials,
        auth_policy: options.auth_policy,
        timeout_ms: options.timeout_ms,
        verbose: options.verbose,
        on_client_available: None,
        on_client_closed: None,
    })
    .await
}

pub async fn run_session_set_model_direct(
    options: RunSessionSetModelDirectOptions,
) -> Result<SessionSetModelResult, PromptRunnerError> {
    let model_id = options.model_id.clone();
    let timeout_ms = options.timeout_ms;
    let result = with_connected_session(
        WithConnectedSessionOptions {
            session_record_id: options.session_record_id,
            mcp_servers: options.mcp_servers,
            permission_mode: None,
            non_interactive_permissions: options.non_interactive_permissions,
            auth_credentials: options.auth_credentials,
            auth_policy: options.auth_policy,
            verbose: options.verbose,
            on_client_available: options.on_client_available,
            on_client_closed: options.on_client_closed,
        },
        move |client, session_id, record| {
            let model_id = model_id.clone();
            Box::pin(async move {
                let model_config_id = resolve_model_config_value(
                    record.vwacp.as_ref().and_then(|state| state.config_options.as_deref()),
                    &model_id,
                )
                .map_err(AcpError::SetSessionConfigOption)?;
                if let Some(config_id) = model_config_id {
                    let response = with_timeout(
                        client.set_session_config_option(
                            session_id,
                            absolute_path(&record.cwd),
                            config_id,
                            model_id.clone(),
                        ),
                        timeout_ms,
                    )
                    .await??;
                    sync_config_options(record, Some(response.config_options));
                } else {
                    with_timeout(
                        client.set_session_model(
                            session_id,
                            absolute_path(&record.cwd),
                            model_id.clone(),
                        ),
                        timeout_ms,
                    )
                    .await??;
                }
                set_desired_model_id(record, Some(&model_id));
                set_current_model_id(record, Some(&model_id));
                Ok(())
            })
        },
    )
    .await?;

    Ok(SessionSetModelResult {
        record: result.record,
        resumed: result.resumed,
        load_error: result.load_error,
    })
}

pub async fn set_session_config_option(
    options: SessionSetConfigOptionOptions,
) -> Result<SessionSetConfigOptionResult, PromptRunnerError> {
    let mut record = resolve_session_record(&options.session_id).await?;
    let resolved_config_id = resolve_config_value(
        record.vwacp.as_ref().and_then(|state| state.config_options.as_deref()),
        &options.config_id,
        &options.value,
    )
    .map_err(AcpError::SetSessionConfigOption)?;
    let owner_response = try_set_config_option_on_running_owner(
        &options.session_id,
        &resolved_config_id,
        &options.value,
        options.timeout_ms,
        options.verbose,
    )
    .await?;

    if let Some(response) = owner_response {
        if options.config_id == "mode" {
            set_desired_mode_id(&mut record, Some(&options.value));
        } else {
            set_desired_config_option(&mut record, &options.config_id, &options.value);
        }
        sync_config_options(&mut record, Some(response.config_options.clone()));
        write_session_record(&record).await?;

        return Ok(SessionSetConfigOptionResult {
            record,
            response,
            resumed: false,
            load_error: None,
        });
    }

    run_session_set_config_option_direct(RunSessionSetConfigOptionDirectOptions {
        session_record_id: options.session_id,
        config_id: options.config_id,
        value: options.value,
        mcp_servers: options.mcp_servers,
        non_interactive_permissions: options.non_interactive_permissions,
        auth_credentials: options.auth_credentials,
        auth_policy: options.auth_policy,
        timeout_ms: options.timeout_ms,
        verbose: options.verbose,
        on_client_available: None,
        on_client_closed: None,
    })
    .await
}

pub async fn run_session_set_config_option_direct(
    options: RunSessionSetConfigOptionDirectOptions,
) -> Result<SessionSetConfigOptionResult, PromptRunnerError> {
    let config_id = options.config_id.clone();
    let value = options.value.clone();
    let timeout_ms = options.timeout_ms;
    let result = with_connected_session(
        WithConnectedSessionOptions {
            session_record_id: options.session_record_id,
            mcp_servers: options.mcp_servers,
            permission_mode: None,
            non_interactive_permissions: options.non_interactive_permissions,
            auth_credentials: options.auth_credentials,
            auth_policy: options.auth_policy,
            verbose: options.verbose,
            on_client_available: options.on_client_available,
            on_client_closed: options.on_client_closed,
        },
        move |client, session_id, record| {
            let config_id = config_id.clone();
            let value = value.clone();
            Box::pin(async move {
                let resolved_config_id = resolve_config_value(
                    record.vwacp.as_ref().and_then(|state| state.config_options.as_deref()),
                    &config_id,
                    &value,
                )
                .map_err(AcpError::SetSessionConfigOption)?;
                let response = with_timeout(
                    client.set_session_config_option(
                        session_id,
                        absolute_path(&record.cwd),
                        resolved_config_id,
                        value.clone(),
                    ),
                    timeout_ms,
                )
                .await??;
                if config_id == "mode" {
                    set_desired_mode_id(record, Some(&value));
                } else {
                    set_desired_config_option(record, &config_id, &value);
                }
                sync_config_options(record, Some(response.config_options.clone()));
                Ok(response)
            })
        },
    )
    .await?;

    Ok(SessionSetConfigOptionResult {
        record: result.record,
        response: result.value,
        resumed: result.resumed,
        load_error: result.load_error,
    })
}

async fn with_connected_session<T, Run>(
    options: WithConnectedSessionOptions,
    run: Run,
) -> Result<WithConnectedSessionResult<T>, PromptRunnerError>
where
    Run: for<'a> FnOnce(Arc<crate::AcpClient>, String, &'a mut SessionRecord) -> BoxFuture<'a, T>,
{
    let record =
        Arc::new(AsyncMutex::new(resolve_session_record(&options.session_record_id).await?));
    let initial_record = record.lock().await.clone();
    let client = Arc::new(build_client(&initial_record, &options));
    let active_session_id = Arc::new(Mutex::new(initial_record.acp_session_id.clone()));
    let active_controller: ActiveSessionController =
        Arc::new(PromptRunnerActiveSessionController::new(
            client.clone(),
            absolute_path(&initial_record.cwd),
            active_session_id.clone(),
        ));
    let notified_client_available = Arc::new(AtomicBool::new(false));

    let result = with_interrupt(
        {
            let record = record.clone();
            let client = client.clone();
            let active_controller = active_controller.clone();
            let active_session_id = active_session_id.clone();
            let notified_client_available = notified_client_available.clone();
            let on_client_available = options.on_client_available.clone();

            move || async move {
                let connect_callback = on_client_available.map(|callback| {
                    let notified_client_available = notified_client_available.clone();
                    Arc::new(move |controller: ActiveSessionController| {
                        callback(controller);
                        notified_client_available.store(true, Ordering::SeqCst);
                    }) as Arc<dyn Fn(ActiveSessionController) + Send + Sync>
                });
                let session_id_callback = Arc::new(move |session_id: &str| {
                    *active_session_id.lock() = session_id.to_string();
                });

                let mut record_guard = record.lock().await;
                session_id_callback(&record_guard.acp_session_id);
                if let Some(callback) = connect_callback {
                    callback(active_controller.clone());
                }

                let value =
                    run(client.clone(), record_guard.acp_session_id.clone(), &mut record_guard)
                        .await?;

                record_guard.last_used_at = iso_now();
                record_guard.closed = Some(false);
                record_guard.closed_at = None;
                apply_lifecycle_snapshot_to_record(
                    &mut record_guard,
                    &<crate::AcpClient as ConnectAndLoadClient>::get_agent_lifecycle_snapshot(
                        client.as_ref(),
                    ),
                );
                write_session_record(&record_guard).await?;

                Ok(WithConnectedSessionResult {
                    value,
                    record: record_guard.clone(),
                    resumed: true, // run_prompt will try to resume
                    load_error: None,
                })
            }
        },
        {
            let record = record.clone();
            let client = client.clone();

            move || {
                let record = record.clone();
                let client = client.clone();

                async move {
                    let _ = client.cancel_active_prompt(2_500).await;
                    let mut record_guard = record.lock().await;
                    apply_lifecycle_snapshot_to_record(
                        &mut record_guard,
                        &<crate::AcpClient as ConnectAndLoadClient>::get_agent_lifecycle_snapshot(
                            client.as_ref(),
                        ),
                    );
                    record_guard.last_used_at = iso_now();
                    let _ = write_session_record(&record_guard).await;
                }
            }
        },
    )
    .await
    .map_err(map_with_interrupt_error);

    if notified_client_available.load(Ordering::SeqCst)
        && let Some(callback) = options.on_client_closed
    {
        callback();
    }
    let mut record_guard = record.lock().await;
    apply_lifecycle_snapshot_to_record(
        &mut record_guard,
        &<crate::AcpClient as ConnectAndLoadClient>::get_agent_lifecycle_snapshot(client.as_ref()),
    );
    let _ = write_session_record(&record_guard).await;

    result
}

fn map_with_interrupt_error(error: WithInterruptError<PromptRunnerError>) -> PromptRunnerError {
    match error {
        WithInterruptError::Inner(error) => error,
        WithInterruptError::Interrupted(error) => error.into(),
        WithInterruptError::SignalHandler(error) => PromptRunnerError::SignalHandler(error),
    }
}

fn queue_connection_error(error: AcpError) -> QueueConnectionError {
    QueueConnectionError::new(
        error.to_string(),
        AcpxErrorOptions {
            source: Some(Box::new(error)),
            retryable: Some(true),
            ..AcpxErrorOptions::default()
        },
    )
}

#[allow(clippy::result_large_err)]
fn queue_control_task<T, F, Fut>(run: F) -> QueueControlFuture<T>
where
    T: Send + 'static,
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, QueueConnectionError>> + 'static,
{
    Box::pin(async move {
        tokio::task::spawn_blocking(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| {
                    QueueConnectionError::new(
                        error.to_string(),
                        AcpxErrorOptions {
                            source: Some(Box::new(error)),
                            retryable: Some(true),
                            ..AcpxErrorOptions::default()
                        },
                    )
                })?;

            runtime.block_on(run())
        })
        .await
        .map_err(|error| {
            QueueConnectionError::new(
                error.to_string(),
                AcpxErrorOptions {
                    source: Some(Box::new(error)),
                    retryable: Some(true),
                    ..AcpxErrorOptions::default()
                },
            )
        })?
    })
}

fn build_client(record: &SessionRecord, options: &WithConnectedSessionOptions) -> crate::AcpClient {
    let agent_config = parse_agent_command(&record.agent_command);
    let mut client = crate::AcpClient::new(record.agent_command.clone(), agent_config)
        .with_mcp_servers(options.mcp_servers.clone().unwrap_or_default())
        .with_permission_mode(options.permission_mode.unwrap_or(PermissionMode::ApproveAll))
        .with_non_interactive_permissions(options.non_interactive_permissions)
        .with_auth_credentials(options.auth_credentials.clone().unwrap_or_default())
        .with_auth_policy(options.auth_policy.unwrap_or(AuthPolicy::Skip))
        .with_verbose(options.verbose)
        .with_session_options(session_options_from_record(record));

    if options.auth_credentials.is_none() {
        client = client.with_auth_credentials(HashMap::new());
    }

    client
}

pub(crate) fn session_options_from_record(record: &SessionRecord) -> Option<AcpSessionOptions> {
    let stored = record.vwacp.as_ref().and_then(|state| state.session_options.as_ref())?;

    let mut session_options = AcpSessionOptions::default();

    if let Some(model) = non_empty_trimmed(stored.model.as_deref()) {
        session_options.model = Some(model);
    }
    if let Some(allowed_tools) = clone_non_empty_tools(stored) {
        session_options.allowed_tools = Some(allowed_tools);
    }
    if let Some(max_turns) = stored.max_turns {
        session_options.max_turns = Some(max_turns);
    }

    (session_options.model.is_some()
        || session_options.allowed_tools.is_some()
        || session_options.max_turns.is_some())
    .then_some(session_options)
}

pub(crate) fn clone_non_empty_tools(stored: &SessionStateOptions) -> Option<Vec<String>> {
    let allowed_tools = stored
        .allowed_tools
        .as_ref()?
        .iter()
        .filter_map(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
        .collect::<Vec<_>>();
    (!allowed_tools.is_empty()).then_some(allowed_tools)
}

pub(crate) fn non_empty_trimmed(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

pub(crate) fn parse_agent_command(agent_command: &str) -> AcpAgentConfig {
    let mut parts = split_command_line(agent_command);
    let command = parts.first().cloned().unwrap_or_else(|| agent_command.trim().to_string());
    if !parts.is_empty() {
        parts.remove(0);
    }

    AcpAgentConfig { command, args: parts, env: HashMap::new() }
}

pub(crate) fn split_command_line(command: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut chars = command.chars().peekable();
    let mut quote = None::<char>;

    while let Some(ch) = chars.next() {
        match quote {
            Some(active_quote) if ch == active_quote => {
                quote = None;
            }
            Some('"') if ch == '\\' => {
                if let Some(escaped) = chars.next() {
                    current.push(escaped);
                }
            }
            Some(_) => current.push(ch),
            None if ch.is_whitespace() => {
                if !current.is_empty() {
                    parts.push(std::mem::take(&mut current));
                }
            }
            None if ch == '\'' || ch == '"' => {
                quote = Some(ch);
            }
            None if ch == '\\' => {
                if let Some(escaped) = chars.next() {
                    current.push(escaped);
                }
            }
            None => current.push(ch),
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts
}
