//! 验证 Actor 生命周期快照、可复用会话识别与失效清理。
//!
//! 最小客户端、通道和运行时 fixture 用于显式驱动异步状态转换与资源收尾。

use std::collections::HashMap;
use std::thread;

use tokio::sync::mpsc;

use crate::AgentLifecycleExit;
use crate::types::AcpAgentConfig;

use super::{AcpClient, AcpClientActorHandle, ChildExitSummary};

fn client() -> AcpClient {
    AcpClient::new(
        "test-agent",
        AcpAgentConfig { command: "agent".to_string(), args: Vec::new(), env: HashMap::new() },
    )
}

// 下列行为用例重点检查复用、取消、通道关闭和失败后的状态一致性。
#[test]
fn has_reusable_session_matches_only_current_session() {
    let client = client();

    assert!(!client.has_reusable_session("session-1"));

    client.store_reusable_session(Some("session-1".to_string()));

    assert!(client.has_reusable_session("session-1"));
    assert!(!client.has_reusable_session("session-2"));

    client.store_reusable_session(None);

    assert!(!client.has_reusable_session("session-1"));
}

#[test]
fn record_actor_start_sets_pid_and_clears_previous_exit() {
    let client = client();
    client.record_actor_exit(ChildExitSummary { exit_code: Some(7), signal: None }, None, false);

    client.record_actor_start(Some(42));
    let snapshot = client.get_agent_lifecycle_snapshot();

    assert_eq!(snapshot.pid, Some(42));
    assert!(snapshot.started_at.is_some());
    assert!(snapshot.last_exit.is_none());
}

#[test]
fn record_actor_start_accepts_missing_pid() {
    let client = client();

    client.record_actor_start(None);
    let snapshot = client.get_agent_lifecycle_snapshot();

    assert!(snapshot.pid.is_none());
    assert!(snapshot.started_at.is_some());
    assert!(snapshot.last_exit.is_none());
}

#[test]
fn record_actor_exit_stores_exit_summary_and_reason() {
    let client = client();

    client.record_actor_start(Some(42));
    client.record_actor_exit(
        ChildExitSummary { exit_code: Some(2), signal: Some("SIGTERM".to_string()) },
        Some("shutdown"),
        true,
    );
    let snapshot = client.get_agent_lifecycle_snapshot();
    let exited_at = snapshot.last_exit.as_ref().and_then(|exit| exit.exited_at.clone());

    assert!(snapshot.pid.is_none());
    assert_eq!(
        snapshot.last_exit,
        Some(AgentLifecycleExit {
            exit_code: Some(2),
            signal: Some("SIGTERM".to_string()),
            exited_at,
            reason: Some("shutdown".to_string()),
            unexpected_during_prompt: true,
        })
    );
}

#[test]
fn record_actor_exit_preserves_missing_exit_details() {
    let client = client();

    client.record_actor_exit(ChildExitSummary { exit_code: None, signal: None }, None, false);
    let snapshot = client.get_agent_lifecycle_snapshot();
    let exit = snapshot.last_exit.expect("last exit");

    assert!(snapshot.pid.is_none());
    assert!(exit.exit_code.is_none());
    assert!(exit.signal.is_none());
    assert!(exit.exited_at.is_some());
    assert!(exit.reason.is_none());
    assert!(!exit.unexpected_during_prompt);
}

#[test]
fn invalidate_actor_clears_handle_session_and_pid() {
    let client = client();
    let (command_tx, _command_rx) = mpsc::unbounded_channel();
    let thread = thread::spawn(|| {});

    {
        let mut state = client.actor_state.lock();
        state.handle = Some(AcpClientActorHandle { command_tx, thread: Some(thread) });
        state.reusable_session_id = Some("session-1".to_string());
        state.lifecycle.pid = Some(42);
        state.lifecycle.started_at = Some("2026-01-01T00:00:00Z".to_string());
    }

    client.invalidate_actor();
    let mut handle = {
        let mut state = client.actor_state.lock();
        assert!(state.handle.is_none());
        assert!(state.reusable_session_id.is_none());
        assert!(state.lifecycle.pid.is_none());
        assert_eq!(state.lifecycle.started_at.as_deref(), Some("2026-01-01T00:00:00Z"));
        state.handle.take()
    };

    if let Some(handle) = handle.as_mut()
        && let Some(thread) = handle.thread.take()
    {
        thread.join().expect("join actor handle thread");
    }
}
