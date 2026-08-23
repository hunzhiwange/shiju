//! 验证代理子进程启动、环境合并、错误包装与退出信息收集。
//!
//! 子进程与内存 I/O fixture 只暴露可观察的字节、退出状态和清理结果。

use std::collections::HashMap;
use std::path::PathBuf;
#[cfg(unix)]
use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::time::{Duration, sleep};

use crate::types::AcpAgentConfig;

use super::{AcpClient, AcpError, ProcessHandles};

fn client_with_config(config: AcpAgentConfig) -> AcpClient {
    AcpClient::new("test-agent", config)
}

fn shell_client(args: Vec<String>) -> AcpClient {
    client_with_config(AcpAgentConfig { command: "sh".to_string(), args, env: HashMap::new() })
}

fn unique_test_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    std::env::temp_dir().join(format!("vw-acp-process-{name}-{}-{nanos}", std::process::id()))
}

fn shell_args(script: &str, extra_args: Vec<String>) -> Vec<String> {
    let mut args = vec!["-c".to_string(), script.to_string(), "vw-acp-process-test".to_string()];
    args.extend(extra_args);
    args
}

#[cfg(unix)]
fn process_exists(process_id: u32) -> bool {
    let result = unsafe { libc::kill(process_id as i32, 0) };
    if result == 0 {
        return true;
    }
    !matches!(std::io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH))
}

// 下列行为用例覆盖成功转发、挂起或失败 I/O 以及进程终止边界。
#[test]
fn spawn_child_rejects_empty_command() {
    let client = client_with_config(AcpAgentConfig {
        command: " \t ".to_string(),
        args: Vec::new(),
        env: HashMap::new(),
    });

    let err = match client.spawn_child() {
        Ok(_) => panic!("empty command should fail"),
        Err(err) => err,
    };

    assert!(matches!(err, AcpError::EmptyCommand));
}

#[test]
fn spawn_child_wraps_spawn_error() {
    let client = client_with_config(AcpAgentConfig {
        command: "/definitely/missing/vw-acp-agent".to_string(),
        args: Vec::new(),
        env: HashMap::new(),
    });

    let err = match client.spawn_child() {
        Ok(_) => panic!("missing command should fail"),
        Err(err) => err,
    };

    assert!(matches!(err, AcpError::Spawn(_)));
}

#[tokio::test]
async fn spawn_child_trims_command_passes_args_and_merges_environment() {
    let output_path = unique_test_path("env.txt");
    let script = r#"
{
    printf 'arg=%s\n' "$2"
    printf 'config=%s\n' "$CONFIG_ONLY"
    printf 'auth_raw=%s\n' "$openai_api_key_raw"
    printf 'auth_normalized=%s\n' "$OPENAI_API_KEY_RAW"
    printf 'auth_prefixed=%s\n' "$VWACP_AUTH_OPENAI_API_KEY_RAW"
} > "$1"
"#;
    let mut env = HashMap::new();
    env.insert("CONFIG_ONLY".to_string(), "configured".to_string());
    env.insert("OPENAI_API_KEY_RAW".to_string(), "config-wins".to_string());
    let auth_credentials =
        HashMap::from([("openai_api_key_raw".to_string(), "auth-secret".to_string())]);
    let client = client_with_config(AcpAgentConfig {
        command: " sh ".to_string(),
        args: shell_args(
            script,
            vec![output_path.to_string_lossy().into_owned(), "child-arg".to_string()],
        ),
        env,
    })
    .with_auth_credentials(auth_credentials);

    let ProcessHandles { child, stderr_task } = client.spawn_child().expect("spawn should succeed");
    let finalized = client.finalize_child(child, stderr_task).await;
    let output = tokio::fs::read_to_string(&output_path).await.expect("env output");
    let _ = tokio::fs::remove_file(&output_path).await;

    assert_eq!(finalized.summary.exit_code, Some(0));
    assert_eq!(finalized.stderr_output, "");
    assert!(output.contains("arg=child-arg\n"));
    assert!(output.contains("config=configured\n"));
    #[cfg(not(windows))]
    assert!(output.contains("auth_raw=auth-secret\n"));
    #[cfg(windows)]
    assert!(output.contains("auth_raw=config-wins\n"));
    assert!(output.contains("auth_normalized=config-wins\n"));
    assert!(output.contains("auth_prefixed=auth-secret\n"));
}

#[tokio::test]
async fn finalize_child_returns_exit_summary_and_stderr_for_quiet_client() {
    let client =
        shell_client(shell_args("printf 'first line\\nsecond line\\n' >&2; exit 7", vec![]));

    let ProcessHandles { child, stderr_task } = client.spawn_child().expect("spawn should succeed");
    let finalized = client.finalize_child(child, stderr_task).await;

    assert_eq!(finalized.summary.exit_code, Some(7));
    assert_eq!(finalized.summary.signal, None);
    assert_eq!(finalized.stderr_output, "first line\nsecond line\n");
}

#[tokio::test]
async fn finalize_child_returns_stderr_for_verbose_client() {
    let client = shell_client(shell_args("printf 'verbose stderr\\n' >&2; exit 3", vec![]))
        .with_verbose(true);

    let ProcessHandles { child, stderr_task } = client.spawn_child().expect("spawn should succeed");
    let finalized = client.finalize_child(child, stderr_task).await;

    assert_eq!(finalized.summary.exit_code, Some(3));
    assert_eq!(finalized.stderr_output, "verbose stderr\n");
}

#[tokio::test]
async fn finalize_child_handles_missing_stderr_task() {
    let client = shell_client(shell_args("exit 0", vec![]));

    let ProcessHandles { child, stderr_task: _ } =
        client.spawn_child().expect("spawn should succeed");
    let finalized = client.finalize_child(child, None).await;

    assert_eq!(finalized.summary.exit_code, Some(0));
    assert_eq!(finalized.stderr_output, "");
}

#[tokio::test]
async fn finalize_child_ignores_failed_stderr_task() {
    let client = shell_client(shell_args("exit 0", vec![]));
    let stderr_task = tokio::spawn(async move {
        panic!("stderr task failed");
    });

    let ProcessHandles { child, stderr_task: _ } =
        client.spawn_child().expect("spawn should succeed");
    let finalized = client.finalize_child(child, Some(stderr_task)).await;

    assert_eq!(finalized.summary.exit_code, Some(0));
    assert_eq!(finalized.stderr_output, "");
}

#[cfg(unix)]
#[tokio::test]
async fn finalize_child_times_out_then_terminates_process_group() {
    let client = client_with_config(AcpAgentConfig {
        command: "perl".to_string(),
        args: vec![
            "-e".to_string(),
            "$SIG{TERM} = sub { exit 13 }; select undef, undef, undef, 10 while 1;".to_string(),
        ],
        env: HashMap::new(),
    });

    let ProcessHandles { child, stderr_task } = client.spawn_child().expect("spawn should succeed");
    let finalized = client.finalize_child(child, stderr_task).await;

    assert_eq!(finalized.summary.exit_code, Some(13));
    assert_eq!(finalized.stderr_output, "");
}

#[cfg(unix)]
#[tokio::test]
async fn finalize_child_cleans_previously_captured_reparented_process_group() {
    let child_pid_path = unique_test_path("reparented-child.pid");
    let script = r#"
my $path = $ARGV[0];
my $pid = fork();
die "fork failed" unless defined $pid;
if ($pid == 0) {
    setpgrp(0, 0) or die "setpgrp failed: $!";
    open my $fh, '>', $path or die $!;
    print $fh $$;
    close $fh;
    $SIG{TERM} = 'IGNORE';
    select undef, undef, undef, 10 while 1;
    exit 0;
}
while (! -s $path) {
    select undef, undef, undef, 0.01;
}
select undef, undef, undef, 0.4;
exit 0;
"#;
    let client = client_with_config(AcpAgentConfig {
        command: "perl".to_string(),
        args: vec!["-e".to_string(), script.to_string(), child_pid_path.to_string_lossy().into()],
        env: HashMap::new(),
    });
    let ProcessHandles { child, stderr_task } = client.spawn_child().expect("spawn should succeed");

    let deadline = Instant::now() + Duration::from_secs(2);
    while !child_pid_path.exists() {
        if Instant::now() >= deadline {
            let _ = client.finalize_child(child, stderr_task).await;
            panic!("child pid file was not written");
        }
        sleep(Duration::from_millis(10)).await;
    }
    let cleanup_targets = super::process_cleanup_targets(child.id());
    let child_process_id = tokio::fs::read_to_string(&child_pid_path)
        .await
        .expect("child pid")
        .trim()
        .parse::<u32>()
        .expect("child pid should parse");

    sleep(Duration::from_millis(700)).await;
    let finalized =
        client.finalize_child_with_cleanup_targets(child, stderr_task, cleanup_targets).await;
    assert_eq!(finalized.summary.exit_code, Some(0));

    let deadline = Instant::now() + Duration::from_secs(2);
    while process_exists(child_process_id) {
        if Instant::now() >= deadline {
            unsafe {
                libc::kill(child_process_id as i32, libc::SIGKILL);
            }
            panic!("pre-captured process group was not cleaned up");
        }
        sleep(Duration::from_millis(10)).await;
    }
    let _ = tokio::fs::remove_file(&child_pid_path).await;
}

#[tokio::test]
async fn finalize_child_discards_slow_stderr_task() {
    let client = shell_client(shell_args("exit 0", vec![]));
    let stderr_task = tokio::spawn(async move {
        sleep(Duration::from_secs(1)).await;
        "late stderr".to_string()
    });

    let ProcessHandles { child, stderr_task: _ } =
        client.spawn_child().expect("spawn should succeed");
    let finalized = client.finalize_child(child, Some(stderr_task)).await;

    assert_eq!(finalized.summary.exit_code, Some(0));
    assert_eq!(finalized.stderr_output, "");
}
