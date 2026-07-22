//! 会话运行时相关子模块的导出与聚合。
//!
//! 本模块是会话执行层的门面，统一导出连接加载、生命周期管理、
//! prompt 执行、队列所有者进程启动和运行时主流程等子模块。
//!
//! 上层调用通常只需要依赖这里导出的公共函数与类型，而无需感知内部拆分细节。

pub mod connect_load;
pub mod lifecycle;
pub mod prompt_runner;
pub mod queue_owner_process;
pub mod runtime;

pub use crate::queue_owner_env::{
    QueueOwnerPayloadError, QueueOwnerRunFromEnvError, parse_queue_owner_payload,
    queue_owner_runtime_options_from_env, run_queue_owner_from_env,
};
pub use connect_load::{
    ConnectAndLoadClient, ConnectAndLoadClientSession, ConnectAndLoadSessionError,
    ConnectAndLoadSessionOptions, ConnectAndLoadSessionResult, connect_and_load_session,
};
pub use lifecycle::{
    AgentLifecycleExit, AgentLifecycleSnapshot, apply_conversation,
    apply_lifecycle_snapshot_to_record, reconcile_agent_session_id, session_has_agent_messages,
};
pub use prompt_runner::{
    ActiveSessionController, ClientAvailableCallback, ClientClosedCallback, PromptRunnerError,
    RunSessionSetConfigOptionDirectOptions, RunSessionSetModeDirectOptions,
    RunSessionSetModelDirectOptions, SessionSetConfigOptionOptions, SessionSetModeOptions,
    SessionSetModelOptions, run_session_set_config_option_direct, run_session_set_mode_direct,
    run_session_set_model_direct, set_session_config_option, set_session_mode, set_session_model,
};
pub use queue_owner_process::{
    QUEUE_OWNER_ARGS_ENV, QUEUE_OWNER_PAYLOAD_ENV, QUEUE_OWNER_PROCESS_MARKER,
    QueueOwnerProcessError, QueueOwnerRuntimeOptions, QueueOwnerRuntimeSendOptions,
    QueueOwnerSpawnCommand, build_queue_owner_arg_override, queue_owner_runtime_options_from_send,
    resolve_queue_owner_spawn_args, resolve_queue_owner_spawn_command,
    sanitize_queue_owner_exec_argv, spawn_queue_owner_process,
};
pub use runtime::{
    RunOnceOptions, SessionCancelOptions, SessionCancelResult, SessionCreateOptions,
    SessionEnsureOptions, SessionSendOptions, cancel_session_prompt, create_session,
    ensure_session, run_once, send_session, send_session_direct,
};

#[cfg(test)]
mod connect_load_tests;
#[cfg(test)]
mod lifecycle_tests;
#[cfg(test)]
mod queue_owner_process_tests;
// #[cfg(test)]
// mod runtime_tests;
