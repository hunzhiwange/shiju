//! ACP 客户端进程生命周期测试。
//!
//! 这些测试覆盖 actor 进程复用、空闲关闭、异常退出重启和进程组清理等边界。
//! Unix 测试使用临时 Python mock ACP 代理，避免依赖外部真实代理。

use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::{Duration as StdDuration, Instant, SystemTime, UNIX_EPOCH};

static UNIQUE_TEST_ID: AtomicU64 = AtomicU64::new(1);

#[test]
fn clone_shares_runtime_state_and_preserves_config() {
    let client = AcpClient::new(
        "test-agent",
        AcpAgentConfig {
            command: "agent".to_string(),
            args: vec!["--acp".to_string()],
            env: HashMap::new(),
        },
    );
    let clone = client.clone();
    *client.permission_stats.lock() =
        PermissionStats { requested: 2, approved: 1, denied: 1, cancelled: 0 };
    client.store_reusable_session(Some("session-1".to_string()));
    client.cancelling_session_ids.lock().insert("session-1".to_string());
    let (cancel_tx, _cancel_rx) = watch::channel(false);
    let (_completed_tx, completed_rx) = watch::channel(false);
    client.register_active_prompt("session-1".to_string(), cancel_tx, completed_rx);

    assert_eq!(clone.agent_name, "test-agent");
    assert_eq!(clone.config.command, "agent");
    assert_eq!(clone.config.args, vec!["--acp"]);
    assert_eq!(
        clone.permission_stats(),
        PermissionStats { requested: 2, approved: 1, denied: 1, cancelled: 0 }
    );
    assert!(clone.has_reusable_session("session-1"));
    assert!(clone.has_active_prompt());
    assert!(clone.cancelling_session_ids.lock().contains("session-1"));
    assert!(Arc::ptr_eq(&client.permission_stats, &clone.permission_stats));
    assert!(Arc::ptr_eq(&client.active_prompt, &clone.active_prompt));
    assert!(Arc::ptr_eq(&client.cancelling_session_ids, &clone.cancelling_session_ids));
    assert!(Arc::ptr_eq(&client.actor_state, &clone.actor_state));
}

#[test]
fn clone_preserves_builder_configuration_and_callbacks() {
    let mcp_servers =
        vec![acp::McpServer::Http(acp::McpServerHttp::new("project", "https://example.test/mcp"))];
    let session_options = AcpSessionOptions {
        model: Some("fast-model".to_string()),
        allowed_tools: Some(vec!["shell".to_string(), "read".to_string()]),
        max_turns: Some(3),
    };
    let mut auth_credentials = HashMap::new();
    auth_credentials.insert("api_key".to_string(), "secret".to_string());
    let acp_callback: AcpMessageCallback = Arc::new(|_direction, _message| {});
    let acp_output_callback: AcpMessageCallback = Arc::new(|_direction, _message| {});
    let session_update_callback: SessionUpdateCallback = Arc::new(|_notification| {});
    let client_operation_callback: ClientOperationCallback = Arc::new(|_operation| {});

    let mut client = AcpClient::new(
        "configured-agent",
        AcpAgentConfig {
            command: "agent".to_string(),
            args: vec!["--acp".to_string()],
            env: HashMap::from([("AGENT_ENV".to_string(), "1".to_string())]),
        },
    )
    .with_client_info("host-client", "1.2.3")
    .with_mcp_servers(mcp_servers.clone())
    .with_permission_mode(PermissionMode::ApproveReads)
    .with_non_interactive_permissions(Some(NonInteractivePermissionPolicy::Deny))
    .with_auth_credentials(auth_credentials.clone())
    .with_auth_policy(AuthPolicy::Fail)
    .with_session_options(Some(session_options.clone()))
    .with_verbose(true)
    .with_acp_message_callback(Some(acp_callback.clone()))
    .with_acp_output_message_callback(Some(acp_output_callback.clone()))
    .with_session_update_callback(Some(session_update_callback.clone()))
    .with_client_operation_callback(Some(client_operation_callback.clone()));

    let clone = client.clone();
    client.config.args.push("--mutated-after-clone".to_string());
    client.auth_credentials.insert("late".to_string(), "ignored".to_string());

    assert_eq!(clone.agent_name, "configured-agent");
    assert_eq!(clone.config.command, "agent");
    assert_eq!(clone.config.args, vec!["--acp"]);
    assert_eq!(clone.config.env.get("AGENT_ENV"), Some(&"1".to_string()));
    assert_eq!(clone.client_name, "host-client");
    assert_eq!(clone.client_version, "1.2.3");
    assert_eq!(clone.mcp_servers, mcp_servers);
    assert_eq!(clone.permission_mode, PermissionMode::ApproveReads);
    assert_eq!(clone.non_interactive_permissions, Some(NonInteractivePermissionPolicy::Deny));
    assert_eq!(clone.auth_credentials, auth_credentials);
    assert_eq!(clone.auth_policy, AuthPolicy::Fail);
    assert_eq!(clone.session_options, Some(session_options));
    assert!(clone.verbose);
    assert!(
        clone.on_acp_message.as_ref().is_some_and(|callback| Arc::ptr_eq(callback, &acp_callback))
    );
    assert!(
        clone
            .on_acp_output_message
            .as_ref()
            .is_some_and(|callback| Arc::ptr_eq(callback, &acp_output_callback))
    );
    assert!(
        clone
            .on_session_update
            .as_ref()
            .is_some_and(|callback| Arc::ptr_eq(callback, &session_update_callback))
    );
    assert!(
        clone
            .on_client_operation
            .as_ref()
            .is_some_and(|callback| Arc::ptr_eq(callback, &client_operation_callback))
    );
}

#[cfg(unix)]
#[tokio::test]
async fn finalize_child_kills_background_process_group() {
    let pid_file = unique_test_path("vw-acp-child-pid", "txt");
    let script = format!(
        "nohup sleep 30 >/dev/null 2>&1 & echo $! > '{}' ; cat >/dev/null",
        pid_file.display()
    );
    let client = AcpClient::new(
        "test",
        AcpAgentConfig {
            command: "sh".to_string(),
            args: vec!["-c".to_string(), script],
            env: HashMap::new(),
        },
    );

    let ProcessHandles { mut child, stderr_task } =
        client.spawn_child().expect("spawn should succeed");
    drop(child.stdin.take());

    let background_pid =
        wait_for_background_pid(&pid_file).await.expect("background pid should be recorded");
    assert!(
        process_exists(background_pid),
        "background process should still be alive before cleanup"
    );

    client.finalize_child(child, stderr_task).await;

    let exited = wait_for_process_exit(background_pid, StdDuration::from_secs(2)).await;
    if !exited {
        let _ = unsafe { libc::kill(background_pid, libc::SIGKILL) };
    }
    let _ = fs::remove_file(&pid_file);

    assert!(exited, "background process should be cleaned up");
}

#[test]
fn enrich_new_session_error_includes_exit_details() {
    let err = AcpError::NewSession(
        "Internal error: \"connection closed before request could be sent\"".to_string(),
    );
    let finalized = FinalizedChild {
        summary: ChildExitSummary { exit_code: Some(1), signal: None },
        stderr_output: "Authentication required\nRun claude login".to_string(),
    };

    let enriched = enrich_acp_error_with_process_context(err, &finalized);

    assert_eq!(
        enriched.to_string(),
        "acp new_session failed: Internal error: \"connection closed before request could be sent\" ACP agent process exited early (exit code 1, stderr: Authentication required | Run claude login)"
    );
}

#[test]
fn enrich_error_without_process_context_keeps_original_message() {
    let err = AcpError::Initialize("connection closed".to_string());
    let finalized = FinalizedChild::default();

    let enriched = enrich_acp_error_with_process_context(err, &finalized);

    assert_eq!(enriched.to_string(), "acp initialize failed: connection closed");
}

#[cfg(unix)]
#[tokio::test]
async fn actor_reuses_existing_process_for_same_cwd() {
    let fixture = MockAcpAgentFixture::new();
    let client = fixture.client(DEFAULT_ACTOR_IDLE_TIMEOUT);

    let first_session =
        client.create_session(&fixture.cwd).await.expect("first session should be created");
    let spawned_pids =
        wait_for_pid_log_len(&fixture.pid_log, 1).await.expect("first actor process should start");
    let first_pid = spawned_pids[0];

    let second_session =
        client.create_session(&fixture.cwd).await.expect("second session should be created");
    let reused_pids = read_logged_pids(&fixture.pid_log);

    assert_eq!(reused_pids, vec![first_pid], "same actor process should be reused");
    assert_ne!(
        first_session.session_id, second_session.session_id,
        "mock agent should still create distinct sessions"
    );
    assert_eq!(
        client.get_agent_lifecycle_snapshot().pid,
        Some(first_pid as u32),
        "lifecycle snapshot should keep the reused pid"
    );

    client.close().await.expect("client close should succeed");
    assert!(
        wait_for_process_exit(first_pid, StdDuration::from_secs(2)).await,
        "reused actor process should exit on close"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn actor_idle_timeout_closes_process_and_next_request_restarts_it() {
    let fixture = MockAcpAgentFixture::new();
    let client = fixture.client(Duration::from_millis(200));

    client
        .create_session(&fixture.cwd)
        .await
        .expect("session should be created before idle timeout");
    let first_pid = wait_for_pid_log_len(&fixture.pid_log, 1)
        .await
        .expect("initial actor process should start")[0];

    wait_for_idle_shutdown(&client).await.expect("idle timeout should close the actor runtime");
    assert!(
        wait_for_process_exit(first_pid, StdDuration::from_secs(2)).await,
        "idle timeout should terminate the actor process"
    );

    client
        .create_session(&fixture.cwd)
        .await
        .expect("session should restart actor after idle timeout");
    let restarted_pids = wait_for_pid_log_len(&fixture.pid_log, 2)
        .await
        .expect("second actor process should start after idle timeout");

    assert_ne!(restarted_pids[0], restarted_pids[1], "idle restart should use a new process");
    assert_eq!(
        client.get_agent_lifecycle_snapshot().pid,
        Some(restarted_pids[1] as u32),
        "lifecycle snapshot should point at the restarted process"
    );

    client.close().await.expect("client close should succeed");
    assert!(
        wait_for_process_exit(restarted_pids[1], StdDuration::from_secs(2)).await,
        "restarted actor process should exit on close"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn actor_restarts_when_agent_exits_between_requests() {
    let fixture = MockAcpAgentFixture::new_with_exit_after_new_session();
    let client = fixture.client(DEFAULT_ACTOR_IDLE_TIMEOUT);

    client
        .create_session(&fixture.cwd)
        .await
        .expect("first session should be created before agent exits");
    let first_pid = wait_for_pid_log_len(&fixture.pid_log, 1)
        .await
        .expect("initial actor process should start")[0];

    assert!(
        wait_for_process_exit(first_pid, StdDuration::from_secs(2)).await,
        "mock agent should exit after first session creation"
    );

    client
        .create_session(&fixture.cwd)
        .await
        .expect("client should restart actor after unexpected agent exit");
    let restarted_pids = wait_for_pid_log_len(&fixture.pid_log, 2)
        .await
        .expect("second actor process should start after unexpected exit");

    assert_ne!(
        restarted_pids[0], restarted_pids[1],
        "unexpected exit should trigger a new process"
    );
    assert_eq!(
        client.get_agent_lifecycle_snapshot().pid,
        Some(restarted_pids[1] as u32),
        "lifecycle snapshot should point at the restarted process"
    );

    client.close().await.expect("client close should succeed");
    assert!(
        wait_for_process_exit(restarted_pids[1], StdDuration::from_secs(2)).await,
        "restarted actor process should exit on close"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn actor_close_allows_wrapper_to_cleanup_detached_child() {
    let fixture = MockAcpAgentFixture::new_with_detached_worker();
    let client = fixture.client(DEFAULT_ACTOR_IDLE_TIMEOUT);

    client.create_session(&fixture.cwd).await.expect("session should be created before close");
    let worker_pid = wait_for_background_pid(
        fixture.worker_pid_log.as_deref().expect("detached worker pid log should exist"),
    )
    .await
    .expect("detached worker pid should be recorded");

    assert!(process_exists(worker_pid), "detached worker should be alive before close");

    client.close().await.expect("client close should succeed");

    assert!(
        wait_for_process_exit(worker_pid, StdDuration::from_secs(2)).await,
        "closing the client should let the wrapper clean up its detached child"
    );
}

#[cfg(unix)]
/// Unix 专用的 mock ACP 代理测试夹具。
///
/// 夹具负责创建临时工作目录、代理脚本和 PID 日志，并在 `Drop` 中清理文件。
struct MockAcpAgentFixture {
    cwd: PathBuf,
    script_path: PathBuf,
    pid_log: PathBuf,
    worker_pid_log: Option<PathBuf>,
    exit_after_new_session: bool,
}

#[cfg(unix)]
impl MockAcpAgentFixture {
    fn new() -> Self {
        Self::new_internal(false, false)
    }

    fn new_with_detached_worker() -> Self {
        Self::new_internal(true, false)
    }

    fn new_with_exit_after_new_session() -> Self {
        Self::new_internal(false, true)
    }

    fn new_internal(spawn_detached_worker: bool, exit_after_new_session: bool) -> Self {
        let cwd = unique_test_path("vw-acp-mock-cwd", "dir");
        let script_path = unique_test_path("vw-acp-mock-agent", "py");
        let pid_log = unique_test_path("vw-acp-mock-agent-pids", "log");
        let worker_pid_log =
            spawn_detached_worker.then(|| unique_test_path("vw-acp-mock-worker-pids", "log"));
        fs::create_dir_all(&cwd).expect("mock cwd should be created");
        fs::write(&pid_log, "").expect("pid log should be initialized");
        if let Some(path) = worker_pid_log.as_ref() {
            fs::write(path, "").expect("worker pid log should be initialized");
        }
        fs::write(&script_path, mock_acp_agent_script()).expect("mock ACP agent should be written");
        Self { cwd, script_path, pid_log, worker_pid_log, exit_after_new_session }
    }

    fn client(&self, actor_idle_timeout: Duration) -> AcpClient {
        let mut args = vec![
            "-u".to_string(),
            self.script_path.display().to_string(),
            self.pid_log.display().to_string(),
        ];
        if let Some(worker_pid_log) = self.worker_pid_log.as_ref() {
            args.push(worker_pid_log.display().to_string());
        }
        let mut env = HashMap::new();
        if self.exit_after_new_session {
            env.insert("VW_ACP_EXIT_AFTER_NEW_SESSION".to_string(), "1".to_string());
        }
        AcpClient::new("mock", AcpAgentConfig { command: "python3".to_string(), args, env })
            .with_actor_idle_timeout(actor_idle_timeout)
    }
}

#[cfg(unix)]
impl Drop for MockAcpAgentFixture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.script_path);
        let _ = fs::remove_file(&self.pid_log);
        if let Some(path) = self.worker_pid_log.as_ref() {
            let _ = fs::remove_file(path);
        }
        let _ = fs::remove_dir_all(&self.cwd);
    }
}

#[cfg(unix)]
fn mock_acp_agent_script() -> &'static str {
    // 脚本尽量保持自包含，测试可以验证真实子进程和进程组行为，而不是只模拟
    // Rust 内部状态。
    r#"import json
import os
import signal
import subprocess
import sys

pid_log = sys.argv[1]
with open(pid_log, "a", encoding="utf-8") as handle:
    handle.write(f"{os.getpid()}\n")
    handle.flush()

session_counter = 0
worker = None
exit_after_new_session = os.environ.get("VW_ACP_EXIT_AFTER_NEW_SESSION") == "1"

if len(sys.argv) > 2:
    worker_pid_log = sys.argv[2]
    worker = subprocess.Popen(
        [
            sys.executable,
            "-c",
            "import signal,time; signal.signal(signal.SIGTERM, lambda *_: None); time.sleep(30)",
        ],
        start_new_session=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    with open(worker_pid_log, "w", encoding="utf-8") as handle:
        handle.write(f"{worker.pid}\n")
        handle.flush()

try:
    for raw_line in sys.stdin:
        line = raw_line.strip()
        if not line:
            continue
        message = json.loads(line)
        method = message.get("method")
        request_id = message.get("id")

        if method == "initialize":
            result = {
                "protocolVersion": message["params"]["protocolVersion"],
                "agentCapabilities": {},
                "authMethods": [],
                "agentInfo": {
                    "name": "mock-acp-agent",
                    "version": "0.1.0"
                }
            }
        elif method == "session/new":
            session_counter += 1
            result = {
                "sessionId": f"session-{session_counter}"
            }
        elif method == "session/load" or method == "session/resume":
            result = {}
        else:
            if request_id is None:
                continue
            result = {}

        sys.stdout.write(json.dumps({
            "jsonrpc": "2.0",
            "id": request_id,
            "result": result
        }) + "\n")
        sys.stdout.flush()
        if method == "session/new" and exit_after_new_session:
            break
finally:
    if worker is not None and worker.poll() is None:
        worker.terminate()
        try:
            worker.wait(timeout=1)
        except subprocess.TimeoutExpired:
            worker.kill()
            worker.wait(timeout=1)
"#
}

#[cfg(unix)]
async fn wait_for_idle_shutdown(client: &AcpClient) -> Option<AgentLifecycleSnapshot> {
    let deadline = Instant::now() + StdDuration::from_secs(3);
    while Instant::now() < deadline {
        let snapshot = client.get_agent_lifecycle_snapshot();
        if snapshot.pid.is_none()
            && snapshot.last_exit.as_ref().and_then(|exit| exit.reason.as_deref())
                == Some("idle_timeout")
        {
            return Some(snapshot);
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    None
}

#[cfg(unix)]
async fn wait_for_pid_log_len(pid_log: &Path, expected_len: usize) -> Option<Vec<i32>> {
    let deadline = Instant::now() + StdDuration::from_secs(3);
    while Instant::now() < deadline {
        let pids = read_logged_pids(pid_log);
        if pids.len() >= expected_len {
            return Some(pids);
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    None
}

#[cfg(unix)]
fn read_logged_pids(pid_log: &Path) -> Vec<i32> {
    fs::read_to_string(pid_log)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.trim().parse::<i32>().ok())
        .collect()
}

#[cfg(unix)]
async fn wait_for_background_pid(pid_file: &Path) -> Option<i32> {
    let deadline = Instant::now() + StdDuration::from_secs(2);
    while Instant::now() < deadline {
        if let Ok(contents) = fs::read_to_string(pid_file)
            && let Ok(pid) = contents.trim().parse::<i32>()
        {
            return Some(pid);
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    None
}

#[cfg(unix)]
async fn wait_for_process_exit(pid: i32, timeout: StdDuration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if !process_exists(pid) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    !process_exists(pid)
}

#[cfg(unix)]
fn process_exists(pid: i32) -> bool {
    let result = unsafe { libc::kill(pid, 0) };
    if result == 0 {
        return true;
    }
    // ESRCH 表示内核找不到该 PID；其它错误更像权限或瞬态状态，因此保守地
    // 视为进程仍存在，避免测试误判清理成功。
    !matches!(std::io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH))
}

#[cfg(unix)]
fn unique_test_path(prefix: &str, extension: &str) -> PathBuf {
    let unique_id = UNIQUE_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let suffix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let name = if extension == "dir" {
        format!("{prefix}-{}-{unique_id}-{suffix}", std::process::id())
    } else {
        format!("{prefix}-{}-{unique_id}-{suffix}.{extension}", std::process::id())
    };
    std::env::temp_dir().join(name)
}
