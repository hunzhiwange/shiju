//! 验证 Actor 生命周期快照、可复用会话识别与失效清理。
//!
//! 最小客户端、通道和运行时 fixture 用于显式驱动异步状态转换与资源收尾。

use std::collections::HashMap;
use std::thread;

use tokio::sync::mpsc;

use crate::AgentLifecycleExit;
use crate::types::AcpAgentConfig;

use super::{AcpClient, AcpClientActorHandle, ChildExitSummary, ConnectionDiagnostic};

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

#[tokio::test]
async fn acp_diagnostic_classifies_failures_and_reaps_process() {
    let client = AcpClient::new(
        "diagnostic",
        AcpAgentConfig {
            command: "/nonexistent-acp-diagnostic".into(),
            args: vec![],
            env: HashMap::new(),
        },
    );
    assert_eq!(
        client.diagnose_connection(tokio_util::sync::CancellationToken::new()).await,
        ConnectionDiagnostic::StartupFailed
    );
}

#[cfg(unix)]
#[tokio::test]
async fn acp_diagnostic_classifies_failures_and_reaps_process_protocol_paths() {
    use super::ConnectionDiagnostic;
    for (mode, expected) in [
        ("success", ConnectionDiagnostic::Connected),
        ("auth", ConnectionDiagnostic::AuthenticationRequired),
        ("error", ConnectionDiagnostic::HandshakeFailed),
        ("cancel", ConnectionDiagnostic::Cancelled),
        ("timeout", ConnectionDiagnostic::TimedOut),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let pid_file = dir.path().join("pid");
        let script = r#"
import sys,json,os,time
open(sys.argv[2],'w').write(str(os.getpid()))
mode=sys.argv[1]
r=json.loads(sys.stdin.readline())
assert r['method']=='initialize'
assert not r['params'].get('clientCapabilities',{}).get('terminal',False)
if mode in ('cancel','timeout'):
    time.sleep(30)
else:
    result={'protocolVersion':1,'agentCapabilities':{},'authMethods':[]}
    if mode=='auth': result['authMethods']=[{'id':'login','name':'Login'}]
    response={'jsonrpc':'2.0','id':r['id']}
    if mode=='error': response['error']={'code':-32603,'message':'SECRET'}
    else: response['result']=result
    print(json.dumps(response),flush=True)
    for line in sys.stdin:
        raise RuntimeError('unexpected request: '+line)
"#;
        let client = AcpClient::new(
            "probe",
            AcpAgentConfig {
                command: "python3".into(),
                args: vec![
                    "-c".into(),
                    script.into(),
                    mode.into(),
                    pid_file.to_string_lossy().into_owned(),
                ],
                env: HashMap::new(),
            },
        );
        let cancel = tokio_util::sync::CancellationToken::new();
        if mode == "cancel" {
            let token = cancel.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                token.cancel();
            });
        }
        assert_eq!(client.diagnose_connection(cancel).await, expected, "{mode}");
        let pid: i32 = std::fs::read_to_string(pid_file).unwrap().parse().unwrap();
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1, "{mode}: process remains");
    }
}
