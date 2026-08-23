//! 验证运行时公共结果与选项结构保留取消、身份和控制字段。

use super::*;
use crate::prompt_content::text_prompt;
use crate::types::{
    AcpJsonRpcMessage, OutputErrorParams, OutputFormatterContext, SessionEventLog,
    SessionTokenUsage,
};

#[derive(Default)]
struct TestFormatter;

impl OutputFormatter for TestFormatter {
    fn set_context(&mut self, _context: OutputFormatterContext) {}

    fn on_acp_message(&mut self, _message: AcpJsonRpcMessage) {}

    fn on_error(&mut self, _params: OutputErrorParams) {}

    fn flush(&mut self) {}
}

fn test_record() -> SessionRecord {
    SessionRecord {
        schema: crate::SESSION_RECORD_SCHEMA.to_string(),
        vwacp_record_id: "record-1".to_string(),
        acp_session_id: "acp-1".to_string(),
        agent_session_id: None,
        agent_command: "agent".to_string(),
        agent_config: None,
        cwd: "/tmp/project".to_string(),
        name: None,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        last_used_at: "2026-01-01T00:00:00Z".to_string(),
        last_seq: 0,
        last_request_id: None,
        event_log: SessionEventLog {
            active_path: "/tmp/active.ndjson".to_string(),
            segment_count: 1,
            max_segment_bytes: 1024,
            max_segments: 3,
            last_write_at: None,
            last_write_error: None,
        },
        closed: Some(false),
        closed_at: None,
        pid: None,
        agent_started_at: None,
        last_prompt_at: None,
        last_agent_exit_code: None,
        last_agent_exit_signal: None,
        last_agent_exit_at: None,
        last_agent_disconnect_reason: None,
        protocol_version: None,
        agent_capabilities: None,
        title: None,
        messages: Vec::new(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
        cumulative_token_usage: SessionTokenUsage::default(),
        request_token_usage: HashMap::new(),
        vwacp: None,
    }
}

#[test]
fn session_cancel_result_keeps_cancel_status_fields() {
    let result = SessionCancelResult { session_id: "session-1".to_string(), cancelled: true };

    assert_eq!(result.session_id, "session-1");
    assert!(result.cancelled);
    assert_eq!(result.clone(), result);
}

#[test]
fn create_and_ensure_options_clone_preserves_identity_fields() {
    let create = SessionCreateOptions {
        agent_command: "agent".to_string(),
        agent_config: None,
        cwd: "/tmp/project".to_string(),
        name: Some("main".to_string()),
        resume_session_id: Some("resume-1".to_string()),
        mcp_servers: None,
        permission_mode: PermissionMode::DenyAll,
        non_interactive_permissions: Some(NonInteractivePermissionPolicy::Deny),
        auth_credentials: None,
        auth_policy: Some(AuthPolicy::Fail),
        verbose: true,
        session_options: Some(AcpSessionOptions {
            model: Some("gpt".to_string()),
            allowed_tools: None,
            max_turns: None,
        }),
        timeout_ms: Some(100),
    };
    let ensure = SessionEnsureOptions {
        walk_boundary: Some("/tmp".to_string()),
        agent_command: create.agent_command.clone(),
        agent_config: create.agent_config.clone(),
        cwd: create.cwd.clone(),
        name: create.name.clone(),
        resume_session_id: create.resume_session_id.clone(),
        mcp_servers: create.mcp_servers.clone(),
        permission_mode: create.permission_mode,
        non_interactive_permissions: create.non_interactive_permissions,
        auth_credentials: create.auth_credentials.clone(),
        auth_policy: create.auth_policy,
        verbose: create.verbose,
        session_options: create.session_options.clone(),
        timeout_ms: create.timeout_ms,
    };

    assert_eq!(create.clone().name.as_deref(), Some("main"));
    assert_eq!(ensure.clone().walk_boundary.as_deref(), Some("/tmp"));
    assert_eq!(ensure.permission_mode, PermissionMode::DenyAll);
}

#[test]
fn run_once_and_send_options_store_prompt_and_runtime_controls() {
    let mut run_formatter = TestFormatter;
    let mut send_formatter = TestFormatter;
    let run = RunOnceOptions {
        agent_command: "agent".to_string(),
        agent_config: None,
        cwd: "/tmp/project".to_string(),
        prompt: text_prompt("hello"),
        mcp_servers: None,
        permission_mode: PermissionMode::ApproveReads,
        non_interactive_permissions: None,
        auth_credentials: None,
        auth_policy: None,
        output_formatter: &mut run_formatter,
        on_acp_message: None,
        on_session_update: None,
        on_client_operation: None,
        suppress_sdk_console_errors: true,
        verbose: false,
        session_options: None,
        prompt_retries: Some(2),
        timeout_ms: Some(50),
    };
    let send = SessionSendOptions {
        session_id: "session-1".to_string(),
        prompt: text_prompt("again"),
        resume_policy: Some(SessionResumePolicy::SameSessionOnly),
        mcp_servers: None,
        permission_mode: PermissionMode::ApproveAll,
        non_interactive_permissions: None,
        auth_credentials: None,
        auth_policy: None,
        output_formatter: &mut send_formatter,
        on_acp_message: None,
        on_session_update: None,
        on_client_operation: None,
        error_emission_policy: Some(OutputErrorEmissionPolicy {
            queue_error_already_emitted: true,
        }),
        suppress_sdk_console_errors: false,
        verbose: true,
        wait_for_completion: true,
        ttl_ms: Some(DEFAULT_QUEUE_OWNER_TTL_MS),
        max_queue_depth: Some(3),
        prompt_retries: None,
        timeout_ms: None,
    };

    assert_eq!(run.agent_command, "agent");
    assert_eq!(run.prompt.len(), 1);
    assert_eq!(run.prompt_retries, Some(2));
    assert_eq!(send.session_id, "session-1");
    assert_eq!(send.resume_policy, Some(SessionResumePolicy::SameSessionOnly));
    assert_eq!(send.ttl_ms, Some(DEFAULT_QUEUE_OWNER_TTL_MS));
}

#[test]
fn session_create_with_client_result_is_cloneable() {
    let result = SessionCreateWithClientResult {
        record: test_record(),
        client: AcpClient::new("agent", Default::default()),
    };

    let cloned = result.clone();

    assert_eq!(cloned.record.vwacp_record_id, "record-1");
}
