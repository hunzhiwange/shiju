//! ACP 兼容导出门面；业务实现唯一归属各 acp-* 领域库。
//!
//! 二进制由 acp-cli-app 装配，版本元数据由原入口显式传入。
//! 模块归属、依赖方向、测试资源与后续验收见 ../分包说明.md。

#![allow(clippy::result_large_err)]

pub use acp_server::server;
pub use acp_agent_registry::{reviewed_agent_command, verify_reviewed_agent_executable};
pub use acp_access_policy::{AccessPolicy, AccessPolicyReason, FullAccessSupport};
pub use acp_types::session_mounts::*;

pub mod cli {
    pub use acp_cli_commands::cli::{config_command, session_target, status_command};
    pub use acp_cli_model::cli::flags;
    pub use acp_cli_render::cli::{json_output, output_render};
}

// 版本查询仍使用原入口包元数据，解析实现由 acp-cli-model 提供。
mod version;

#[cfg(test)]
#[path = "access_policy_tests.rs"]
mod access_policy_tests;

pub use acp_types::acp_error_shapes::{
    extract_acp_error, format_unknown_error_message, is_acp_resource_not_found_error,
};
pub use acp_types::acp_jsonrpc::{
    extract_session_update_notification, is_acp_json_rpc_message, is_json_rpc_notification,
    is_session_update_notification, parse_json_rpc_error_message, parse_prompt_stop_reason,
};
pub use acp_agent_registry::agent_registry::{
    AgentCommandSpec, DEFAULT_AGENT_NAME, built_in_agent_registry, built_in_agent_specs,
    list_built_in_agents, merge_agent_registry, merge_agent_specs, normalize_agent_name,
    resolve_agent_command, resolve_agent_spec, resolve_agent_spec_with_overrides,
};
pub use acp_types::agent_session_id::{
    AGENT_SESSION_ID_META_KEYS, extract_agent_session_id, normalize_agent_session_id,
};
pub use acp_cli_model::cli_core::{
    CliBootstrapPlan, CliCoreError, CliRuntimePlan, PerfCaptureRole, TOP_LEVEL_VERBS,
    apply_permission_exit_code, build_cli_bootstrap_plan, build_cli_runtime_plan, command_argv,
    detect_initial_cwd, detect_json_strict, detect_requested_output_format, is_queue_owner_mode,
    is_version_requested, read_prompt, read_prompt_input_from_stdin, resolve_compatible_config_id,
    resolve_requested_output_policy, should_maybe_handle_skillflag, top_level_verbs,
};
pub use acp_cli_model::cli_public::{
    AgentTokenScan, ConfigurePublicCliOptions, PUBLIC_CLI_HELP_TEXT, PublicCliError, PublicCliPlan,
    RootPromptAction, build_public_cli_plan, configure_public_cli, detect_agent_token,
    resolve_dynamic_agent_command, resolve_root_prompt_action,
};
pub use acp_client::{AcpClient, ConnectionDiagnostic};
pub use acp_access_policy::codex_compat::{is_codex_acp_command, is_codex_invocation};
pub use acp_config::config::{
    ConfigAgentEntry, ConfigDisplay, ConfigError, InitGlobalConfigFileResult, ResolvedAcpxConfig,
    default_global_config_path, init_global_config_file, init_global_config_file_at,
    load_resolved_config, load_resolved_config_from_paths, project_config_path, to_config_display,
};
pub use acp_types::error::AcpError;
pub use acp_types::error_normalization::{
    NormalizeOutputErrorOptions, exit_code_for_output_error_code, format_error_message,
    is_acp_query_closed_before_response_error, is_retryable_prompt_error, normalize_output_error,
};
pub use acp_types::errors::{
    AcpxErrorOptions, AcpxOperationalError, AgentDisconnectedError, AgentSpawnError,
    AuthPolicyError, ClaudeAcpSessionCreateTimeoutError, CopilotAcpUnsupportedError, ErrorSource,
    GeminiAcpStartupTimeoutError, PermissionDeniedError, PermissionPromptUnavailableError,
    QueueConnectionError, QueueProtocolError, SessionModeReplayError, SessionModelReplayError,
    SessionNotFoundError, SessionResolutionError, SessionResumeRequiredError,
};
pub use acp_host_io::filesystem::{
    FileSystemConfirmWriteFn, FileSystemConfirmWriteFuture, FileSystemHandlers,
    FileSystemHandlersOptions, FileSystemOperationCallback,
};
pub use acp_types::jsonrpc_error::{
    BuildJsonRpcErrorParams, JsonRpcErrorObject, JsonRpcErrorResponse,
    build_json_rpc_error_response, output_error_jsonrpc_code,
};
pub use acp_config::mcp_servers::{
    McpServerConfig, McpServerManagement, ParseMcpServersError, parse_mcp_server_configs,
    parse_mcp_server_configs_with_field_name, parse_mcp_servers, parse_mcp_servers_with_field_name,
    parse_optional_mcp_servers, parse_optional_mcp_servers_with_field_name,
};
pub use acp_output::output::{
    AnyOutputFormatter, OutputFormatterOptions, QuietOutputFormatter, TextOutputFormatter,
    create_output_formatter,
};
pub use acp_output::output_json_formatter::{JsonOutputFormatter, create_json_output_formatter};
pub use acp_metrics::perf_metrics::{
    format_perf_metric, get_perf_metrics_snapshot, increment_perf_counter, measure_perf,
    record_perf_duration, reset_perf_metrics, set_perf_gauge, start_perf_timer,
};
pub use acp_metrics::perf_metrics_capture::{
    CaptureReason, PERF_METRICS_FILE_ENV, PerfMetricsCaptureOptions,
    checkpoint_perf_metrics_capture, current_perf_metrics_capture_file_from_env,
    flush_perf_metrics_capture, install_perf_metrics_capture, perf_metrics_capture_file_from_env,
};
pub use acp_host_io::permission_prompt::{
    PermissionPromptOptions, can_prompt_for_permission, prompt_for_permission,
};
pub use acp_host_io::permissions::{
    PermissionConfirmation, PermissionDecision, classify_permission_decision,
    permission_mode_satisfies, resolve_permission_request,
};
pub use acp_storage::persisted_key_policy::{
    PersistedKeyPolicyError, assert_persisted_key_policy, find_persisted_key_policy_violations,
};
pub use acp_types::prompt_content::{
    PromptInput, PromptInputValidationError, is_prompt_input, merge_prompt_source_with_text,
    parse_prompt_source, prompt_to_display_text, text_prompt,
};
pub use acp_queue::queue_ipc::{
    MAX_MESSAGE_BUFFER_SIZE, SubmitToQueueOwnerOptions, next_queue_request_id,
    try_cancel_on_running_owner, try_set_config_option_on_running_owner,
    try_set_mode_on_running_owner, try_set_model_on_running_owner, try_submit_to_running_owner,
};
pub use acp_queue::queue_ipc_health::{QueueOwnerHealth, probe_queue_owner_health};
pub use acp_queue::queue_ipc_server::{
    QueueDepthChangedCallback, QueueOwnerControlHandlers, QueueOwnerSocketLease, QueueTask,
    SessionQueueOwner, SessionQueueOwnerOptions,
};
pub use acp_queue::queue_ipc_transport::{
    QUEUE_CONNECT_RETRY_MS, QueueOwnerConnection, SOCKET_CONNECTION_TIMEOUT_MS,
    connect_to_queue_owner,
};
pub use acp_lease::queue_lease_store::{
    QueueOwnerLease, QueueOwnerRecord, QueueOwnerStatus, ensure_owner_is_usable, is_process_alive,
    read_default_queue_owner_record, read_default_queue_owner_status, read_queue_owner_record,
    read_queue_owner_status, refresh_queue_owner_lease, refresh_queue_owner_lease_with_now,
    release_queue_owner_lease, terminate_default_queue_owner_for_session, terminate_process,
    terminate_queue_owner_for_session, try_acquire_default_queue_owner_lease,
    try_acquire_queue_owner_lease, try_acquire_queue_owner_lease_with_now, wait_ms,
};
pub use acp_queue::queue_messages::{
    QueueOwnerMessage, QueueRequest, parse_queue_owner_message, parse_queue_request,
};
pub use acp_queue::queue_owner_turn_controller::{
    QueueControlFuture, QueueOwnerActiveSessionController, QueueOwnerConfigOptionFallbackFn,
    QueueOwnerConfigOptionTimeoutFn, QueueOwnerModeFallbackFn, QueueOwnerModelFallbackFn,
    QueueOwnerTurnController, QueueOwnerTurnControllerOptions, QueueOwnerTurnState,
    QueueOwnerVoidTimeoutFn,
};
pub use acp_paths::queue_paths::{
    default_home_dir, default_queue_base_dir, default_queue_lock_file_path,
    default_queue_socket_base_dir, default_queue_socket_path, queue_base_dir,
    queue_key_for_session, queue_lock_file_path, queue_socket_base_dir, queue_socket_path,
};
pub use acp_output::read_output_suppression::{
    ReadLikeToolDescriptor, SUPPRESSED_READ_OUTPUT, is_read_like_tool,
};
pub use acp_types::runtime_session_id::{
    RUNTIME_SESSION_ID_META_KEYS, extract_runtime_session_id, normalize_runtime_session_id,
};
pub use acp_storage::session_conversation_model::{
    LegacyHistoryEntry, LegacyHistoryRole, append_legacy_history, clone_session_conversation,
    clone_session_vwacp_state, create_session_conversation, record_client_operation,
    record_prompt_submission, record_session_update, record_text_prompt_submission,
    trim_conversation_for_runtime,
};
pub use acp_types::session_event_log::{
    DEFAULT_EVENT_MAX_SEGMENTS, DEFAULT_EVENT_SEGMENT_MAX_BYTES, default_session_base_dir,
    default_session_event_active_path, default_session_event_lock_path, default_session_event_log,
    default_session_event_segment_path, safe_session_id, session_base_dir,
    session_event_active_path, session_event_lock_path, session_event_log,
    session_event_segment_path,
};
pub use acp_storage::session_events::{
    SessionEventAppendOptions, SessionEventWriter, SessionEventWriterOptions, list_session_events,
};
pub use acp_access_policy::session_mode_preference::{
    get_desired_config_options, get_desired_mode_id, get_desired_model_id, normalize_mode_id,
    resolve_config_value, set_current_model_id, set_desired_config_option, set_desired_mode_id,
    set_desired_model_id, sync_advertised_model_state, sync_config_options,
};
pub use acp_storage::session_persistence::{
    DEFAULT_HISTORY_LIMIT, FindSessionByDirectoryWalkOptions, FindSessionOptions,
    SESSION_INDEX_SCHEMA, SessionIndex, SessionIndexEntry, SessionRepositoryError,
    SessionRepositoryResult, absolute_path, close_session, find_git_repository_root, find_session,
    find_session_by_directory_walk, iso_now, list_sessions, list_sessions_for_agent,
    load_or_rebuild_session_index, parse_session_record, read_session_index, rebuild_session_index,
    resolve_session_record, serialize_session_record_for_disk, session_index_path,
    to_session_index_entry, write_session_index, write_session_record,
};
pub use acp_session_runtime::{
    QueueOwnerPayloadError, QueueOwnerRunFromEnvError, parse_queue_owner_payload,
    queue_owner_runtime_options_from_env, run_queue_owner_from_env,
};
pub use acp_session_connect::{
    ConnectAndLoadClient, ConnectAndLoadClientSession, ConnectAndLoadSessionError,
    ConnectAndLoadSessionOptions, ConnectAndLoadSessionResult, connect_and_load_session,
};
pub use acp_lifecycle::{
    AgentLifecycleExit, AgentLifecycleSnapshot, apply_conversation,
    apply_lifecycle_snapshot_to_record, reconcile_agent_session_id, session_has_agent_messages,
};
pub use acp_prompt_runner::{
    ActiveSessionController, ClientAvailableCallback, ClientClosedCallback, PromptRunnerError,
    RunSessionSetConfigOptionDirectOptions, RunSessionSetModeDirectOptions,
    RunSessionSetModelDirectOptions, SessionSetConfigOptionOptions, SessionSetModeOptions,
    SessionSetModelOptions, run_session_set_config_option_direct, run_session_set_mode_direct,
    run_session_set_model_direct, set_session_config_option, set_session_mode, set_session_model,
};
pub use acp_owner_process::{
    QUEUE_OWNER_ARGS_ENV, QUEUE_OWNER_PAYLOAD_ENV, QUEUE_OWNER_PROCESS_MARKER,
    QueueOwnerProcessError, QueueOwnerRuntimeOptions, QueueOwnerRuntimeSendOptions,
    QueueOwnerSpawnCommand, build_queue_owner_arg_override, queue_owner_runtime_options_from_send,
    resolve_queue_owner_spawn_args, resolve_queue_owner_spawn_command,
    sanitize_queue_owner_exec_argv, spawn_queue_owner_process,
};
pub use acp_session_runtime::{
    RunOnceOptions, SessionCancelOptions, SessionCancelResult, SessionCreateOptions,
    SessionEnsureOptions, SessionSendOptions, cancel_session_prompt, create_session,
    ensure_session, run_once, send_session, send_session_direct,
};

pub use acp_async_control::{
    InterruptedError, TimeoutError, WithInterruptError, with_interrupt, with_timeout,
};
pub use acp_host_io::spawn_command_options::build_spawn_command;
pub use acp_host_io::terminal::{
    TerminalConfirmExecuteFn, TerminalConfirmExecuteFuture, TerminalManager,
    TerminalManagerOptions, TerminalOperationCallback,
};
pub use acp_types::types::*;
pub use version::{
    ResolveAcpxVersionParams, UNKNOWN_VERSION, get_vwacp_version, resolve_vwacp_version,
};

pub use acp_agent_registry::install_builtin_agent;
