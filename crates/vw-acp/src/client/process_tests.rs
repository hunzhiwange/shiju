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
async fn finalize_child_ignores_cancelled_stderr_task() {
    let client = shell_client(shell_args("exit 0", vec![]));
    // Cancellation supplies a real JoinError without depending on panic unwinding.
    let stderr_task = tokio::spawn(std::future::pending::<String>());
    stderr_task.abort();

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

#[tokio::test]
async fn acp_model_source_launch_profile_child_stderr_never_becomes_diagnostics() {
    let client = client_with_config(AcpAgentConfig {
        command: "sh".into(),
        args: vec!["-c".into(), "printf '%s' \"$TASK9_SECRET\" >&2".into()],
        env: HashMap::from([
            ("VW_ACP_LAUNCH_REFERENCE".into(), "vw-session-model:reference".into()),
            ("TASK9_SECRET".into(), "stderr-secret-9".into()),
        ]),
    });
    let handles = client.spawn_child().unwrap();
    assert!(handles.stderr_task.is_none());
    let result = client.finalize_child(handles.child, handles.stderr_task).await;
    assert!(result.stderr_output.is_empty());
    let restored = AcpClient::new("vw-session-model:reference", client.config.clone());
    let error = restored.spawn_child().err().unwrap().to_string();
    assert!(error.contains("reissued by its session owner"));
    assert!(!error.contains("stderr-secret-9"));
}

#[cfg(unix)]
#[tokio::test]
async fn acp_model_source_claude_managed_child_does_not_merge_account_credentials() {
    let output_path = unique_test_path("claude-managed-env");
    let config = AcpAgentConfig {
        command: "sh".into(),
        args: shell_args(
            "test -n \"$PATH\" && test -z \"$CARGO_TARGET_DIR\" && test -z \"$ACCOUNT_SECRET\" && test -z \"$VW_AUTH_ACCOUNT_SECRET\" && test \"$ANTHROPIC_AUTH_TOKEN\" = relay-only && printf isolated > \"$1\"",
            vec![output_path.to_string_lossy().into_owned()],
        ),
        env: HashMap::from([
            ("VW_ACP_LAUNCH_REFERENCE".into(), "vw-session-model:opaque".into()),
            ("VW_ACP_CLAUDE_SETTINGS".into(), "/private/settings.json".into()),
            ("ANTHROPIC_AUTH_TOKEN".into(), "relay-only".into()),
        ]),
    };
    let client = client_with_config(config).with_auth_credentials(HashMap::from([(
        "account-secret".into(),
        "account-must-not-leak".into(),
    )]));
    let ProcessHandles { child, stderr_task } = client.spawn_child().unwrap();
    let finalized = client.finalize_child(child, stderr_task).await;
    assert_eq!(finalized.summary.exit_code, Some(0));
    assert_eq!(std::fs::read_to_string(&output_path).unwrap(), "isolated");
    std::fs::remove_file(output_path).unwrap();
}

#[cfg(unix)]
#[test]
fn claude_managed_child_resolves_shebang_with_augmented_path() {
    const CHILD: &str = "VW_ACP_TEST_MANAGED_PATH_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        runtime.block_on(async {
            use tokio::io::AsyncReadExt;

            let client = client_with_config(AcpAgentConfig {
                command: "vw-acp-adapter-fixture".into(),
                args: vec![],
                env: HashMap::from([
                    ("VW_ACP_LAUNCH_REFERENCE".into(), "vw-session-model:opaque".into()),
                    ("VW_ACP_CLAUDE_SETTINGS".into(), "/private/settings.json".into()),
                ]),
            });
            let ProcessHandles { mut child, stderr_task } = client.spawn_child().unwrap();
            let mut output = String::new();
            child.stdout.take().unwrap().read_to_string(&mut output).await.unwrap();
            let finalized = client.finalize_child(child, stderr_task).await;
            assert_eq!(finalized.summary.exit_code, Some(0));
            assert_eq!(output, "runtime-found");
        });
        return;
    }

    use std::os::unix::fs::PermissionsExt;
    let home = tempfile::tempdir().unwrap();
    let bin = home.path().join("profile-bin");
    std::fs::create_dir(&bin).unwrap();
    for (name, source) in [
        ("vw-acp-adapter-fixture", "#!/usr/bin/env vw-acp-runtime-fixture\n"),
        ("vw-acp-runtime-fixture", "#!/bin/sh\nprintf runtime-found\n"),
    ] {
        let path = bin.join(name);
        std::fs::write(&path, source).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    std::fs::write(
        home.path().join(".profile"),
        format!("export PATH=\"{}:$PATH\"\n", bin.display()),
    )
    .unwrap();
    // A separate process gives the PATH cache a fresh desktop-like environment
    // without changing global environment variables in the test runner.
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "client::process_tests::claude_managed_child_resolves_shebang_with_augmented_path",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .env("HOME", home.path())
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "isolated PATH regression failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
async fn acp_full_access_launch_custom_bypass_requires_current_unrestricted_grant() {
    for args in [
        vec!["--approval-mode", "yolo"],
        vec!["--permission-mode=bypass_permissions"],
        vec!["--yolo"],
        vec!["--skip-trust"],
    ] {
        for (full, mode) in [
            (false, crate::PermissionMode::ApproveAll),
            (true, crate::PermissionMode::DenyAll),
            (true, crate::PermissionMode::ApproveReads),
        ] {
            let client = AcpClient::new(
                "custom",
                AcpAgentConfig {
                    command: "sh".into(),
                    args: args.iter().map(|s| (*s).into()).collect(),
                    env: HashMap::new(),
                },
            )
            .with_full_access(full)
            .with_permission_mode(mode);
            match client.spawn_child() {
                Err(AcpError::Initialize(message)) => assert!(message.contains("full access")),
                Ok(ProcessHandles { child, stderr_task }) => {
                    client.finalize_child(child, stderr_task).await;
                    panic!("conflicting custom bypass must be rejected");
                }
                Err(error) => panic!("unexpected error: {error}"),
            }
        }
    }
}

#[test]
fn acp_full_access_launch_official_and_managed_shapes_follow_current_grant() {
    for (agent, command, base, expected) in [
        ("gemini", "gemini", vec!["--experimental-acp"], vec![]),
        ("gemini", "npx", vec!["@google/gemini-cli@latest", "--experimental-acp"], vec![]),
        ("qwen", "qwen", vec!["--acp"], vec![]),
        ("qwen", "qwen", vec!["--acp", "--experimental-skills"], vec![]),
        ("qoder", "qodercli", vec!["--acp"], Vec::<&str>::new()),
    ] {
        let config = AcpAgentConfig {
            command: command.into(),
            args: base.iter().map(|s| (*s).into()).collect(),
            env: HashMap::new(),
        };
        for (full, mode) in [
            (false, crate::PermissionMode::ApproveAll),
            (true, crate::PermissionMode::ApproveAll),
            (true, crate::PermissionMode::DenyAll),
            (true, crate::PermissionMode::ApproveReads),
        ] {
            let client = AcpClient::new(agent, config.clone())
                .with_full_access(full)
                .with_permission_mode(mode);
            let mut wanted = config.args.clone();
            if full && mode == crate::PermissionMode::ApproveAll {
                wanted.extend(expected.iter().map(|s| (*s).into()));
            }
            assert_eq!(client.launch_args().unwrap(), wanted, "{agent} {full} {mode:?}");
            assert_eq!(client.config.args, config.args);
        }
    }
}

#[tokio::test]
async fn acp_full_access_launch_custom_arguments_and_environment_reach_child_unchanged() {
    let client = shell_client(shell_args(
        "printf '%s|%s' \"$1\" \"$CUSTOM_VALUE\" >&2",
        vec!["unknown --option=值".into()],
    ))
    .with_full_access(true);
    let mut client = client;
    client.config.env.insert("CUSTOM_VALUE".into(), "preserved".into());
    let ProcessHandles { child, stderr_task } = client.spawn_child().unwrap();
    let result = client.finalize_child(child, stderr_task).await;
    assert_eq!(result.summary.exit_code, Some(0));
    assert_eq!(result.stderr_output, "unknown --option=值|preserved");
}

#[cfg(unix)]
#[tokio::test]
async fn acp_full_access_launch_current_grant_is_materialized_at_process_boundary() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let command = temp.path().join("gemini");
    std::fs::write(&command, "#!/bin/sh\nprintf '%s\\n' \"$@\" >&2\n").unwrap();
    std::fs::set_permissions(&command, std::fs::Permissions::from_mode(0o755)).unwrap();
    let config = AcpAgentConfig {
        command: command.to_string_lossy().into_owned(),
        args: vec!["--experimental-acp".into()],
        env: HashMap::new(),
    };
    for full in [true, false] {
        let client = AcpClient::new("gemini", config.clone()).with_full_access(full);
        let ProcessHandles { child, stderr_task } = client.spawn_child().unwrap();
        let result = client.finalize_child(child, stderr_task).await;
        assert_eq!(result.summary.exit_code, Some(0));
        assert_eq!(result.stderr_output, "--experimental-acp\n");
    }
}

#[test]
fn acp_full_access_launch_managed_routing_suffix_is_preserved_with_current_grant() {
    for (agent, base, suffix) in [
        ("gemini", vec!["--experimental-acp"], vec!["--shiju-model-source"]),
        (
            "qwen",
            vec!["--acp"],
            vec![
                "--shiju-model-source",
                "--auth-type",
                "openai",
                "--model",
                "alias",
                "--openai-base-url",
                "http://127.0.0.1:12345/v1",
            ],
        ),
    ] {
        let args: Vec<String> = base.iter().chain(&suffix).map(|s| (*s).into()).collect();
        let config = AcpAgentConfig {
            command: agent.into(),
            args: args.clone(),
            env: HashMap::from([
                ("VW_ACP_LAUNCH_REFERENCE".into(), format!("vw-session-model:{}", "a".repeat(64))),
                ("OPENAI_MODEL".into(), "alias".into()),
                ("OPENAI_BASE_URL".into(), "http://127.0.0.1:12345/v1".into()),
            ]),
        };
        for full in [false, true] {
            let client = AcpClient::new(agent, config.clone()).with_full_access(full);
            let actual = client.launch_args().unwrap();
            assert_eq!(&actual[..args.len()], &args);
            assert!(!actual.iter().any(|arg| arg == "yolo"));
            assert_eq!(client.config.env, config.env);
        }
    }
}

#[test]
fn acp_full_access_opencode_desktop_requires_current_grant() {
    use crate::PermissionMode;

    let config = AcpAgentConfig {
        command: "opencode".into(),
        args: vec!["acp".into()],
        ..Default::default()
    };
    let workspace = unique_test_path("opencode-workspace");
    for (mode, grant, allowed) in [
        (PermissionMode::ApproveReads, false, false),
        (PermissionMode::ApproveAll, true, true),
        (PermissionMode::ApproveAll, false, false),
        (PermissionMode::DenyAll, true, false),
    ] {
        let client = AcpClient::new("opencode", config.clone())
            .with_desktop_workspace(Some(workspace.clone()))
            .with_permission_mode(mode)
            .with_full_access(grant);
        assert_eq!(client.launch_args().is_ok(), allowed, "{mode:?} grant={grant}");
    }

    assert_eq!(AcpClient::new("opencode", config).launch_args().unwrap(), ["acp"]);
}
