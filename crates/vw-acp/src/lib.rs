//! vw-acp 的库入口与公共导出聚合。
//!
//! 本模块负责组织 ACP 客户端运行所需的各个子系统，并向外暴露稳定的公共 API。
//! 这里不承载具体业务逻辑，重点是把 CLI、会话运行时、队列控制、持久化、
//! 输出格式化和错误处理等模块按统一边界导出给上层调用方。
//!
//! # 主要子系统
//!
//! - CLI 规划与参数解析：负责命令行入口的引导、参数归一化与公共命令装配
//! - Session Runtime：负责会话创建、恢复、发送消息与取消等运行时流程
//! - Queue Owner：负责后台队列所有者进程、套接字通信与租约管理
//! - Session Persistence：负责会话记录、索引和事件日志的磁盘持久化
//! - Output Formatting：负责文本、JSON 与静默模式的输出组织
//! - Error Handling：负责协议错误、运行时错误和对外输出错误的统一整理
//!
//! # 设计目的
//!
//! 通过集中导出公共符号，上层 crate 可以以较小的依赖面接入 vw-acp，
//! 同时保持内部模块仍按单一职责拆分，便于后续演进与回滚。

#![allow(clippy::result_large_err)]

mod acp_error_shapes;
mod acp_jsonrpc;
mod agent_registry;
mod agent_session_id;
pub mod cli;
mod cli_core;
mod cli_public;
mod client;
mod codex_compat;
mod config;
mod error;
mod error_normalization;
mod errors;
mod filesystem;
mod jsonrpc_error;
mod mcp_servers;
mod output;
mod output_json_formatter;
mod perf_metrics;
mod perf_metrics_capture;
mod permission_prompt;
mod permissions;
mod persisted_key_policy;
mod prompt_content;
mod queue_ipc;
mod queue_ipc_health;
mod queue_ipc_server;
mod queue_ipc_transport;
mod queue_lease_store;
mod queue_messages;
mod queue_owner_env;
mod queue_owner_turn_controller;
mod queue_paths;
mod read_output_suppression;
mod runtime_session_id;
pub mod server;
mod session_conversation_model;
mod session_event_log;
mod session_events;
mod session_mode_preference;
mod session_persistence;
mod session_runtime;
mod session_runtime_helpers;
mod spawn_command_options;
mod terminal;
mod types;
mod version;

#[cfg(test)]
#[path = "codex_compat_tests.rs"]
mod codex_compat_tests;
#[cfg(test)]
#[path = "error_normalization_tests.rs"]
mod error_normalization_tests;
#[cfg(test)]
#[path = "error_tests.rs"]
mod error_tests;
#[cfg(test)]
#[path = "errors_tests.rs"]
mod errors_tests;
#[cfg(test)]
#[path = "filesystem_tests.rs"]
mod filesystem_tests;
#[cfg(test)]
#[path = "jsonrpc_error_tests.rs"]
mod jsonrpc_error_tests;
#[cfg(test)]
#[path = "queue_owner_env_tests.rs"]
mod queue_owner_env_tests;

pub use acp_error_shapes::{
    extract_acp_error, format_unknown_error_message, is_acp_resource_not_found_error,
};
pub use acp_jsonrpc::{
    extract_session_update_notification, is_acp_json_rpc_message, is_json_rpc_notification,
    is_session_update_notification, parse_json_rpc_error_message, parse_prompt_stop_reason,
};
pub use agent_registry::{
    AgentCommandSpec, DEFAULT_AGENT_NAME, built_in_agent_registry, built_in_agent_specs,
    list_built_in_agents, merge_agent_registry, merge_agent_specs, normalize_agent_name,
    resolve_agent_command, resolve_agent_spec, resolve_agent_spec_with_overrides,
};
pub use agent_session_id::{
    AGENT_SESSION_ID_META_KEYS, extract_agent_session_id, normalize_agent_session_id,
};
pub use cli_core::{
    CliBootstrapPlan, CliCoreError, CliRuntimePlan, PerfCaptureRole, TOP_LEVEL_VERBS,
    apply_permission_exit_code, build_cli_bootstrap_plan, build_cli_runtime_plan, command_argv,
    detect_initial_cwd, detect_json_strict, detect_requested_output_format, is_queue_owner_mode,
    is_version_requested, read_prompt, read_prompt_input_from_stdin, resolve_compatible_config_id,
    resolve_requested_output_policy, should_maybe_handle_skillflag, top_level_verbs,
};
pub use cli_public::{
    AgentTokenScan, ConfigurePublicCliOptions, PUBLIC_CLI_HELP_TEXT, PublicCliError, PublicCliPlan,
    RootPromptAction, build_public_cli_plan, configure_public_cli, detect_agent_token,
    resolve_dynamic_agent_command, resolve_root_prompt_action,
};
pub use client::AcpClient;
pub use codex_compat::{is_codex_acp_command, is_codex_invocation};
pub use config::{
    ConfigAgentEntry, ConfigDisplay, ConfigError, InitGlobalConfigFileResult, ResolvedAcpxConfig,
    default_global_config_path, init_global_config_file, init_global_config_file_at,
    load_resolved_config, load_resolved_config_from_paths, project_config_path, to_config_display,
};
pub use error::AcpError;
pub use error_normalization::{
    NormalizeOutputErrorOptions, exit_code_for_output_error_code, format_error_message,
    is_acp_query_closed_before_response_error, is_retryable_prompt_error, normalize_output_error,
};
pub use errors::{
    AcpxErrorOptions, AcpxOperationalError, AgentDisconnectedError, AgentSpawnError,
    AuthPolicyError, ClaudeAcpSessionCreateTimeoutError, CopilotAcpUnsupportedError, ErrorSource,
    GeminiAcpStartupTimeoutError, PermissionDeniedError, PermissionPromptUnavailableError,
    QueueConnectionError, QueueProtocolError, SessionModeReplayError, SessionModelReplayError,
    SessionNotFoundError, SessionResolutionError, SessionResumeRequiredError,
};
pub use filesystem::{
    FileSystemConfirmWriteFn, FileSystemConfirmWriteFuture, FileSystemHandlers,
    FileSystemHandlersOptions, FileSystemOperationCallback,
};
pub use jsonrpc_error::{
    BuildJsonRpcErrorParams, JsonRpcErrorObject, JsonRpcErrorResponse,
    build_json_rpc_error_response, output_error_jsonrpc_code,
};
pub use mcp_servers::{
    McpServerConfig, McpServerManagement, ParseMcpServersError, parse_mcp_server_configs,
    parse_mcp_server_configs_with_field_name, parse_mcp_servers, parse_mcp_servers_with_field_name,
    parse_optional_mcp_servers, parse_optional_mcp_servers_with_field_name,
};
pub use output::{
    AnyOutputFormatter, OutputFormatterOptions, QuietOutputFormatter, TextOutputFormatter,
    create_output_formatter,
};
pub use output_json_formatter::{JsonOutputFormatter, create_json_output_formatter};
pub use perf_metrics::{
    format_perf_metric, get_perf_metrics_snapshot, increment_perf_counter, measure_perf,
    record_perf_duration, reset_perf_metrics, set_perf_gauge, start_perf_timer,
};
pub use perf_metrics_capture::{
    CaptureReason, PERF_METRICS_FILE_ENV, PerfMetricsCaptureOptions,
    checkpoint_perf_metrics_capture, current_perf_metrics_capture_file_from_env,
    flush_perf_metrics_capture, install_perf_metrics_capture, perf_metrics_capture_file_from_env,
};
pub use permission_prompt::{
    PermissionPromptOptions, can_prompt_for_permission, prompt_for_permission,
};
pub use permissions::{
    PermissionDecision, classify_permission_decision, permission_mode_satisfies,
    resolve_permission_request,
};
pub use persisted_key_policy::{
    PersistedKeyPolicyError, assert_persisted_key_policy, find_persisted_key_policy_violations,
};
pub use prompt_content::{
    PromptInput, PromptInputValidationError, is_prompt_input, merge_prompt_source_with_text,
    parse_prompt_source, prompt_to_display_text, text_prompt,
};
pub use queue_ipc::{
    MAX_MESSAGE_BUFFER_SIZE, SubmitToQueueOwnerOptions, next_queue_request_id,
    try_cancel_on_running_owner, try_set_config_option_on_running_owner,
    try_set_mode_on_running_owner, try_set_model_on_running_owner, try_submit_to_running_owner,
};
pub use queue_ipc_health::{QueueOwnerHealth, probe_queue_owner_health};
pub use queue_ipc_server::{
    QueueDepthChangedCallback, QueueOwnerControlHandlers, QueueOwnerSocketLease, QueueTask,
    SessionQueueOwner, SessionQueueOwnerOptions,
};
pub use queue_ipc_transport::{
    QUEUE_CONNECT_RETRY_MS, QueueOwnerConnection, SOCKET_CONNECTION_TIMEOUT_MS,
    connect_to_queue_owner,
};
pub use queue_lease_store::{
    QueueOwnerLease, QueueOwnerRecord, QueueOwnerStatus, ensure_owner_is_usable, is_process_alive,
    read_default_queue_owner_record, read_default_queue_owner_status, read_queue_owner_record,
    read_queue_owner_status, refresh_queue_owner_lease, refresh_queue_owner_lease_with_now,
    release_queue_owner_lease, terminate_default_queue_owner_for_session, terminate_process,
    terminate_queue_owner_for_session, try_acquire_default_queue_owner_lease,
    try_acquire_queue_owner_lease, try_acquire_queue_owner_lease_with_now, wait_ms,
};
pub use queue_messages::{
    QueueOwnerMessage, QueueRequest, parse_queue_owner_message, parse_queue_request,
};
pub use queue_owner_turn_controller::{
    QueueControlFuture, QueueOwnerActiveSessionController, QueueOwnerConfigOptionFallbackFn,
    QueueOwnerConfigOptionTimeoutFn, QueueOwnerModeFallbackFn, QueueOwnerModelFallbackFn,
    QueueOwnerTurnController, QueueOwnerTurnControllerOptions, QueueOwnerTurnState,
    QueueOwnerVoidTimeoutFn,
};
pub use queue_paths::{
    default_home_dir, default_queue_base_dir, default_queue_lock_file_path,
    default_queue_socket_base_dir, default_queue_socket_path, queue_base_dir,
    queue_key_for_session, queue_lock_file_path, queue_socket_base_dir, queue_socket_path,
};
pub use read_output_suppression::{
    ReadLikeToolDescriptor, SUPPRESSED_READ_OUTPUT, is_read_like_tool,
};
pub use runtime_session_id::{
    RUNTIME_SESSION_ID_META_KEYS, extract_runtime_session_id, normalize_runtime_session_id,
};
pub use session_conversation_model::{
    LegacyHistoryEntry, LegacyHistoryRole, append_legacy_history, clone_session_conversation,
    clone_session_vwacp_state, create_session_conversation, record_client_operation,
    record_prompt_submission, record_session_update, record_text_prompt_submission,
    trim_conversation_for_runtime,
};
pub use session_event_log::{
    DEFAULT_EVENT_MAX_SEGMENTS, DEFAULT_EVENT_SEGMENT_MAX_BYTES, default_session_base_dir,
    default_session_event_active_path, default_session_event_lock_path, default_session_event_log,
    default_session_event_segment_path, safe_session_id, session_base_dir,
    session_event_active_path, session_event_lock_path, session_event_log,
    session_event_segment_path,
};
pub use session_events::{
    SessionEventAppendOptions, SessionEventWriter, SessionEventWriterOptions, list_session_events,
};
pub use session_mode_preference::{
    get_desired_mode_id, get_desired_model_id, normalize_mode_id, set_current_model_id,
    set_desired_mode_id, set_desired_model_id, sync_advertised_model_state,
};
pub use session_persistence::{
    DEFAULT_HISTORY_LIMIT, FindSessionByDirectoryWalkOptions, FindSessionOptions,
    SESSION_INDEX_SCHEMA, SessionIndex, SessionIndexEntry, SessionRepositoryError,
    SessionRepositoryResult, absolute_path, close_session, find_git_repository_root, find_session,
    find_session_by_directory_walk, iso_now, list_sessions, list_sessions_for_agent,
    load_or_rebuild_session_index, parse_session_record, read_session_index, rebuild_session_index,
    resolve_session_record, serialize_session_record_for_disk, session_index_path,
    to_session_index_entry, write_session_index, write_session_record,
};
pub use session_runtime::{
    ActiveSessionController, AgentLifecycleExit, AgentLifecycleSnapshot, ClientAvailableCallback,
    ClientClosedCallback, ConnectAndLoadClient, ConnectAndLoadClientSession,
    ConnectAndLoadSessionError, ConnectAndLoadSessionOptions, ConnectAndLoadSessionResult,
    PromptRunnerError, QUEUE_OWNER_ARGS_ENV, QUEUE_OWNER_PAYLOAD_ENV, QUEUE_OWNER_PROCESS_MARKER,
    QueueOwnerPayloadError, QueueOwnerProcessError, QueueOwnerRunFromEnvError,
    QueueOwnerRuntimeOptions, QueueOwnerRuntimeSendOptions, QueueOwnerSpawnCommand, RunOnceOptions,
    RunSessionSetConfigOptionDirectOptions, RunSessionSetModeDirectOptions,
    RunSessionSetModelDirectOptions, SessionCancelOptions, SessionCancelResult,
    SessionCreateOptions, SessionEnsureOptions, SessionSendOptions, SessionSetConfigOptionOptions,
    SessionSetModeOptions, SessionSetModelOptions, apply_conversation,
    apply_lifecycle_snapshot_to_record, build_queue_owner_arg_override, cancel_session_prompt,
    connect_and_load_session, create_session, ensure_session, parse_queue_owner_payload,
    queue_owner_runtime_options_from_env, queue_owner_runtime_options_from_send,
    reconcile_agent_session_id, resolve_queue_owner_spawn_args, resolve_queue_owner_spawn_command,
    run_once, run_queue_owner_from_env, run_session_set_config_option_direct,
    run_session_set_mode_direct, run_session_set_model_direct, sanitize_queue_owner_exec_argv,
    send_session, send_session_direct, session_has_agent_messages, set_session_config_option,
    set_session_mode, set_session_model, spawn_queue_owner_process,
};
pub use session_runtime_helpers::{
    InterruptedError, TimeoutError, WithInterruptError, with_interrupt, with_timeout,
};
pub use spawn_command_options::build_spawn_command;
pub use terminal::{
    TerminalConfirmExecuteFn, TerminalConfirmExecuteFuture, TerminalManager,
    TerminalManagerOptions, TerminalOperationCallback,
};
pub use types::*;
pub use version::{
    ResolveAcpxVersionParams, UNKNOWN_VERSION, get_vwacp_version, resolve_vwacp_version,
};
