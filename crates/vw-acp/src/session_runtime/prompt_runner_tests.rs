//! Prompt 执行器纯函数逻辑的单元测试。

use std::collections::HashMap;
use std::error::Error as StdError;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use super::{
    PromptRunnerError, WithConnectedSessionOptions, clone_non_empty_tools,
    map_with_interrupt_error, non_empty_trimmed, parse_agent_command, queue_connection_error,
    queue_control_task, session_options_from_record, split_command_line, with_connected_session,
};
use crate::error::AcpError;
use crate::errors::{AcpxErrorOptions, QueueConnectionError};
use crate::queue_paths::TEST_HOME_ENV_LOCK;
use crate::session_runtime_helpers::{InterruptedError, TimeoutError, WithInterruptError};
use crate::types::{
    SessionAcpxState, SessionEventLog, SessionRecord, SessionStateOptions, SessionTokenUsage,
};

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(label: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("vw-acp-prompt-runner-{label}-{unique}"));
        fs::create_dir_all(&path).expect("create temp dir");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

struct HomeEnvGuard {
    previous: Option<OsString>,
}

impl HomeEnvGuard {
    fn set(path: &Path) -> Self {
        let previous = std::env::var_os("HOME");
        unsafe { std::env::set_var("HOME", path) };
        Self { previous }
    }
}

impl Drop for HomeEnvGuard {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => unsafe { std::env::set_var("HOME", value) },
            None => unsafe { std::env::remove_var("HOME") },
        }
    }
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

fn test_vwacp_state(session_options: Option<SessionStateOptions>) -> SessionAcpxState {
    SessionAcpxState {
        current_mode_id: None,
        desired_mode_id: None,
        current_model_id: None,
        available_models: None,
        available_commands: None,
        config_options: None,
        desired_config_options: None,
        session_options,
    }
}

#[test]
fn split_command_line_preserves_quoted_segments() {
    let parts = split_command_line(r#"npx "@scope/agent cli" --flag "two words""#);

    assert_eq!(
        parts,
        vec![
            "npx".to_string(),
            "@scope/agent cli".to_string(),
            "--flag".to_string(),
            "two words".to_string(),
        ]
    );
}

#[test]
fn split_command_line_handles_single_quotes_and_escaped_spaces() {
    let parts = split_command_line(r#"tool 'literal "value"' escaped\ space plain"#);

    assert_eq!(
        parts,
        vec![
            "tool".to_string(),
            "literal \"value\"".to_string(),
            "escaped space".to_string(),
            "plain".to_string(),
        ]
    );
}

#[test]
fn split_command_line_keeps_escaped_double_quote_content() {
    let parts = split_command_line(r#"agent "say \"hello\"" trailing\\"#);

    assert_eq!(
        parts,
        vec!["agent".to_string(), "say \"hello\"".to_string(), "trailing\\".to_string()]
    );
}

#[test]
fn split_command_line_preserves_unclosed_quote_content() {
    let parts = split_command_line(r#"agent "two words"#);

    assert_eq!(parts, vec!["agent".to_string(), "two words".to_string()]);
}

#[test]
fn split_command_line_omits_empty_quoted_segments() {
    let parts = split_command_line(r#"agent "" '' next"#);

    assert_eq!(parts, vec!["agent".to_string(), "next".to_string()]);
}

#[test]
fn parse_agent_command_extracts_command_and_args() {
    let config = parse_agent_command(r#"node ./bin/agent.js --mode fast"#);

    assert_eq!(config.command, "node");
    assert_eq!(
        config.args,
        vec!["./bin/agent.js".to_string(), "--mode".to_string(), "fast".to_string(),]
    );
    assert!(config.env.is_empty());
}

#[test]
fn parse_agent_command_handles_empty_input_without_args() {
    let config = parse_agent_command("   ");

    assert_eq!(config.command, "");
    assert!(config.args.is_empty());
    assert!(config.env.is_empty());
}

#[test]
fn non_empty_trimmed_filters_missing_and_blank_values() {
    assert_eq!(non_empty_trimmed(None), None);
    assert_eq!(non_empty_trimmed(Some(" \t\n ")), None);
    assert_eq!(non_empty_trimmed(Some("  model-a  ")), Some("model-a".to_string()));
}

#[test]
fn clone_non_empty_tools_trims_and_drops_blank_entries() {
    let stored = SessionStateOptions {
        model: None,
        allowed_tools: Some(vec![
            " shell ".to_string(),
            "".to_string(),
            "  ".to_string(),
            "read".to_string(),
        ]),
        max_turns: None,
    };

    assert_eq!(clone_non_empty_tools(&stored), Some(vec!["shell".to_string(), "read".to_string()]));
}

#[test]
fn clone_non_empty_tools_omits_missing_or_blank_tool_lists() {
    let missing = SessionStateOptions { model: None, allowed_tools: None, max_turns: None };
    let blank = SessionStateOptions {
        model: None,
        allowed_tools: Some(vec![" ".to_string(), "\t".to_string()]),
        max_turns: None,
    };

    assert_eq!(clone_non_empty_tools(&missing), None);
    assert_eq!(clone_non_empty_tools(&blank), None);
}

#[test]
fn session_options_from_record_preserves_max_turns_without_other_options() {
    let mut record = test_record();
    record.vwacp = Some(test_vwacp_state(Some(SessionStateOptions {
        model: None,
        allowed_tools: None,
        max_turns: Some(8),
    })));

    let options = session_options_from_record(&record).expect("session options");

    assert_eq!(options.model, None);
    assert_eq!(options.allowed_tools, None);
    assert_eq!(options.max_turns, Some(8));
}

#[test]
fn session_options_from_record_omits_missing_vwacp_state() {
    assert_eq!(session_options_from_record(&test_record()), None);
}

#[test]
fn session_options_from_record_filters_blank_options() {
    let mut record = test_record();
    record.vwacp = Some(test_vwacp_state(Some(SessionStateOptions {
        model: Some("  ".to_string()),
        allowed_tools: Some(vec!["".to_string(), "  ".to_string()]),
        max_turns: None,
    })));

    assert_eq!(session_options_from_record(&record), None);
}

#[test]
fn session_options_from_record_preserves_valid_options() {
    let mut record = test_record();
    record.vwacp = Some(test_vwacp_state(Some(SessionStateOptions {
        model: Some("  gpt-5  ".to_string()),
        allowed_tools: Some(vec![" shell ".to_string(), " ".to_string(), "read".to_string()]),
        max_turns: Some(4),
    })));

    let options = session_options_from_record(&record).expect("session options");

    assert_eq!(options.model.as_deref(), Some("gpt-5"));
    assert_eq!(options.allowed_tools, Some(vec!["shell".to_string(), "read".to_string()]));
    assert_eq!(options.max_turns, Some(4));
}

#[test]
fn map_with_interrupt_error_preserves_inner_error() {
    let error = map_with_interrupt_error(WithInterruptError::Inner(PromptRunnerError::Timeout(
        TimeoutError { timeout_ms: 7 },
    )));

    assert!(matches!(error, PromptRunnerError::Timeout(TimeoutError { timeout_ms: 7 })));
}

#[test]
fn map_with_interrupt_error_maps_interrupt_signal() {
    let error = map_with_interrupt_error(WithInterruptError::<PromptRunnerError>::Interrupted(
        InterruptedError,
    ));

    assert!(matches!(error, PromptRunnerError::Interrupted(_)));
}

#[test]
fn map_with_interrupt_error_maps_signal_handler_error() {
    let error = map_with_interrupt_error(WithInterruptError::<PromptRunnerError>::SignalHandler(
        io::Error::other("signal setup failed"),
    ));

    match error {
        PromptRunnerError::SignalHandler(source) => {
            assert_eq!(source.to_string(), "signal setup failed");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn queue_connection_error_marks_acp_failures_retryable() {
    let error = queue_connection_error(AcpError::Cancel("no active prompt".to_string()));

    assert_eq!(error.message(), "acp cancel failed: no active prompt");
    assert_eq!(error.retryable(), Some(true));
    assert_eq!(
        StdError::source(&error).map(ToString::to_string),
        Some("acp cancel failed: no active prompt".to_string())
    );
}

#[tokio::test]
async fn queue_control_task_returns_successful_result() {
    let value = queue_control_task(|| async { Ok::<_, QueueConnectionError>(42) })
        .await
        .expect("queue control task should succeed");

    assert_eq!(value, 42);
}

#[tokio::test]
async fn queue_control_task_returns_inner_error() {
    let error = queue_control_task(|| async {
        Err::<(), _>(QueueConnectionError::new("queue failed", AcpxErrorOptions::default()))
    })
    .await
    .expect_err("inner queue error should be returned");

    assert_eq!(error.message(), "queue failed");
}

#[tokio::test]
async fn with_connected_session_updates_record_and_runs_callbacks() {
    let _lock = TEST_HOME_ENV_LOCK.lock().await;
    let home = TempDir::new("home");
    let _home = HomeEnvGuard::set(home.path());
    let mut record = test_record();
    record.closed = Some(true);
    record.closed_at = Some("2026-01-02T00:00:00Z".to_string());
    crate::write_session_record(&record).await.expect("write initial record");
    let events = Arc::new(Mutex::new(Vec::new()));

    let result = with_connected_session(
        WithConnectedSessionOptions {
            session_record_id: record.vwacp_record_id.clone(),
            mcp_servers: None,
            permission_mode: None,
            non_interactive_permissions: None,
            auth_credentials: None,
            auth_policy: None,
            verbose: false,
            on_client_available: Some({
                let events = events.clone();
                Arc::new(move |controller| {
                    assert!(!controller.has_active_prompt());
                    events
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .push("available");
                })
            }),
            on_client_closed: Some({
                let events = events.clone();
                Arc::new(move || {
                    events.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).push("closed");
                })
            }),
        },
        {
            let events = events.clone();
            move |_client, session_id, record| {
                Box::pin(async move {
                    assert_eq!(session_id, "acp-1");
                    record.title = Some("updated title".to_string());
                    events.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).push("run");
                    Ok(9)
                })
            }
        },
    )
    .await
    .expect("connected session should run");

    assert_eq!(result.value, 9);
    assert!(result.resumed);
    assert_eq!(result.load_error, None);
    assert_eq!(result.record.title.as_deref(), Some("updated title"));
    assert_eq!(result.record.closed, Some(false));
    assert_eq!(result.record.closed_at, None);
    assert_eq!(
        *events.lock().unwrap_or_else(|poisoned| poisoned.into_inner()),
        vec!["available", "run", "closed"]
    );

    let persisted = crate::resolve_session_record("record-1").await.expect("resolve persisted");
    assert_eq!(persisted.title.as_deref(), Some("updated title"));
    assert_eq!(persisted.closed, Some(false));
    assert_eq!(persisted.closed_at, None);
}
