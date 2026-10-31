//! 队列空闲时通过同一原生会话执行控制请求，避免重新进入队列 IPC。

use std::sync::Arc;

use crate::errors::{AcpxErrorOptions, QueueConnectionError};
use crate::queue_owner_turn_controller::QueueOwnerTurnControllerOptions;
use crate::session_runtime::prompt_runner::{
    PromptRunnerError, RunSessionSetConfigOptionDirectOptions, RunSessionSetModeDirectOptions,
    RunSessionSetModelDirectOptions, queue_control_task, run_session_set_config_option_direct,
    run_session_set_mode_direct, run_session_set_model_direct,
};
use crate::session_runtime::queue_owner_process::QueueOwnerRuntimeOptions;
use crate::session_runtime_helpers::with_timeout;
use crate::types::OutputErrorOrigin;

use super::queue_owner::timeout_queue_connection_error;

fn control_error(error: PromptRunnerError) -> QueueConnectionError {
    QueueConnectionError::new(
        error.to_string(),
        AcpxErrorOptions {
            source: Some(Box::new(error)),
            origin: Some(OutputErrorOrigin::Queue),
            ..AcpxErrorOptions::default()
        },
    )
}

pub(super) fn turn_controller_options(
    options: &QueueOwnerRuntimeOptions,
) -> QueueOwnerTurnControllerOptions {
    let mode_options = options.clone();
    let model_options = options.clone();
    let config_options = options.clone();
    QueueOwnerTurnControllerOptions {
        with_timeout: Arc::new(|future, timeout_ms| {
            Box::pin(async move {
                with_timeout(future, timeout_ms).await.map_err(timeout_queue_connection_error)?
            })
        }),
        with_timeout_config_option: Arc::new(|future, timeout_ms| {
            Box::pin(async move {
                with_timeout(future, timeout_ms).await.map_err(timeout_queue_connection_error)?
            })
        }),
        set_session_mode_fallback: Arc::new(move |mode_id, timeout_ms| {
            let options = mode_options.clone();
            queue_control_task(move || async move {
                run_session_set_mode_direct(RunSessionSetModeDirectOptions {
                    session_record_id: options.session_id,
                    mode_id,
                    mcp_servers: options.mcp_servers,
                    non_interactive_permissions: options.non_interactive_permissions,
                    auth_credentials: options.auth_credentials,
                    auth_policy: options.auth_policy,
                    timeout_ms,
                    verbose: options.verbose.unwrap_or(false),
                    on_client_available: None,
                    on_client_closed: None,
                })
                .await
                .map(|_| ())
                .map_err(control_error)
            })
        }),
        set_session_model_fallback: Arc::new(move |model_id, timeout_ms| {
            let options = model_options.clone();
            queue_control_task(move || async move {
                run_session_set_model_direct(RunSessionSetModelDirectOptions {
                    session_record_id: options.session_id,
                    model_id,
                    mcp_servers: options.mcp_servers,
                    non_interactive_permissions: options.non_interactive_permissions,
                    auth_credentials: options.auth_credentials,
                    auth_policy: options.auth_policy,
                    timeout_ms,
                    verbose: options.verbose.unwrap_or(false),
                    on_client_available: None,
                    on_client_closed: None,
                })
                .await
                .map(|_| ())
                .map_err(control_error)
            })
        }),
        set_session_config_option_fallback: Arc::new(move |config_id, value, timeout_ms| {
            let options = config_options.clone();
            queue_control_task(move || async move {
                run_session_set_config_option_direct(RunSessionSetConfigOptionDirectOptions {
                    session_record_id: options.session_id,
                    config_id,
                    value,
                    mcp_servers: options.mcp_servers,
                    non_interactive_permissions: options.non_interactive_permissions,
                    auth_credentials: options.auth_credentials,
                    auth_policy: options.auth_policy,
                    timeout_ms,
                    verbose: options.verbose.unwrap_or(false),
                    on_client_available: None,
                    on_client_closed: None,
                })
                .await
                .map(|result| result.response)
                .map_err(control_error)
            })
        }),
    }
}
