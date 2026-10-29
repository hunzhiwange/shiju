use super::*;

#[test]
fn readiness_requires_matching_build_and_full_daemon() {
    assert!(!full_daemon_ready(&serde_json::json!({"build_id":"new"}), "new"));
    for (build, status, expected) in
        [("new", "ok", true), ("old", "ok", false), ("new", "error", false)]
    {
        let health = serde_json::json!({
            "build_id": build,
            "runtime": {"components": {"daemon": {"status": status}}}
        });
        assert_eq!(full_daemon_ready(&health, "new"), expected);
    }
}

#[cfg(unix)]
fn test_cli(dir: &Path, script: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let cli = dir.join("local cli");
    fs::write(&cli, format!("#!/bin/sh\n{script}\n")).unwrap();
    fs::set_permissions(&cli, fs::Permissions::from_mode(0o755)).unwrap();
    cli
}

#[cfg(unix)]
#[test]
fn daemon_launch_preserves_arguments_logs_and_child_across_retries() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("configuration with spaces");
    let cli = test_cli(dir.path(), "printf '%s\\n' \"$@\"; echo daemon-error >&2; exec sleep 30");
    let endpoint = GatewayRuntimeEndpoint::new("127.0.0.1", 1);
    let mut child = None;
    assert!(ensure_daemon(&cli, &config, &endpoint, "build", &mut child, Duration::ZERO).is_err());
    let pid = child.as_ref().unwrap().id();
    assert!(ensure_daemon(&cli, &config, &endpoint, "build", &mut child, Duration::ZERO).is_err());
    assert_eq!(child.as_ref().unwrap().id(), pid);
    let output = config.join(format!("logs/desktop-{}.daemon.stdout.log", std::process::id()));
    let error = config.join(format!("logs/desktop-{}.daemon.stderr.log", std::process::id()));
    for _ in 0..100 {
        if fs::read_to_string(&error).unwrap_or_default().contains("daemon-error") {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    child.as_mut().unwrap().kill().unwrap();
    child.as_mut().unwrap().wait().unwrap();
    assert_eq!(
        fs::read_to_string(output).unwrap(),
        format!("--config-dir\n{}\ndaemon\n", config.display())
    );
    assert!(fs::read_to_string(error).unwrap().contains("daemon-error"));
}

#[cfg(unix)]
#[test]
fn exited_retained_daemon_is_replaced_on_retry() {
    let dir = tempfile::tempdir().unwrap();
    let cli = test_cli(dir.path(), "echo restarted >&2; exit 7");
    let mut exited = std::process::Command::new("true").spawn().unwrap();
    exited.wait().unwrap();
    let mut child = Some(exited);

    let error = ensure_daemon(
        &cli,
        dir.path(),
        &GatewayRuntimeEndpoint::new("127.0.0.1", 1),
        "build",
        &mut child,
        Duration::from_secs(2),
    ).unwrap_err();

    assert!(error.contains("完整服务已退出"), "{error}");
    let log = dir.path().join(format!("logs/desktop-{}.daemon.stderr.log", std::process::id()));
    assert!(fs::read_to_string(log).unwrap().contains("restarted"));
    assert!(child.is_none());
}

#[cfg(unix)]
#[test]
fn exited_daemon_reports_failure_and_can_be_retried() {
    let dir = tempfile::tempdir().unwrap();
    let cli = test_cli(dir.path(), "echo startup-failed >&2; exit 7");
    let mut child = None;
    let result = ensure_daemon(
        &cli,
        dir.path(),
        &GatewayRuntimeEndpoint::new("127.0.0.1", 1),
        "build",
        &mut child,
        Duration::from_secs(2),
    );
    assert!(result.unwrap_err().contains("daemon.stderr.log"));
    assert!(child.is_none());
    assert!(
        fs::read_to_string(
            dir.path().join(format!("logs/desktop-{}.daemon.stderr.log", std::process::id()))
        )
        .unwrap()
        .contains("startup-failed")
    );
}

#[test]
fn healthy_full_daemon_is_reused_without_launching_cli() {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = GatewayRuntimeEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port());
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0; 1024];
        stream.read(&mut request).unwrap();
        let body = serde_json::json!({
            "status": "ok", "build_id": "build",
            "runtime": {"components": {"daemon": {"status": "ok"}}}
        })
        .to_string();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
    });
    let dir = tempfile::tempdir().unwrap();
    let mut child = None;
    ensure_daemon(
        &dir.path().join("missing-cli"),
        dir.path(),
        &endpoint,
        "build",
        &mut child,
        Duration::ZERO,
    )
    .unwrap();
    assert!(child.is_none());
    assert!(!dir.path().join("logs").exists());
    server.join().unwrap();
}

#[cfg(unix)]
#[test]
fn shared_log_paths_cannot_block_desktop_daemon_startup() {
    let dir = tempfile::tempdir().unwrap();
    // Make the old shared log paths impossible to open as files.
    fs::create_dir_all(dir.path().join("logs/daemon.stdout.log")).unwrap();
    fs::create_dir_all(dir.path().join("logs/daemon.stderr.log")).unwrap();
    let cli = test_cli(dir.path(), "echo daemon-started; exit 7");
    let mut child = None;
    let error = ensure_daemon(
        &cli,
        dir.path(),
        &GatewayRuntimeEndpoint::new("127.0.0.1", 1),
        "build",
        &mut child,
        Duration::from_secs(2),
    )
    .unwrap_err();
    assert!(error.contains("完整服务已退出"), "{error}");
    let log = dir.path().join(format!("logs/desktop-{}.daemon.stdout.log", std::process::id()));
    assert!(fs::read_to_string(log).unwrap().contains("daemon-started"));
}

#[cfg(windows)]
#[test]
fn exclusively_locked_scheduled_task_logs_do_not_block_desktop_launch() {
    use std::os::windows::fs::OpenOptionsExt;
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("logs")).unwrap();
    let _stdout_lock = OpenOptions::new()
        .write(true)
        .create_new(true)
        .share_mode(0)
        .open(dir.path().join("logs/daemon.stdout.log"))
        .unwrap();
    let _stderr_lock = OpenOptions::new()
        .write(true)
        .create_new(true)
        .share_mode(0)
        .open(dir.path().join("logs/daemon.stderr.log"))
        .unwrap();
    let mut child = None;
    let error = ensure_daemon(
        &dir.path().join("missing.exe"),
        dir.path(),
        &GatewayRuntimeEndpoint::new("127.0.0.1", 1),
        "build",
        &mut child,
        Duration::ZERO,
    )
    .unwrap_err();
    // Reaching spawn proves both log opens succeeded while the original files are locked.
    assert!(error.contains("启动完整服务失败"), "{error}");
    assert!(
        dir.path().join(format!("logs/desktop-{}.daemon.stdout.log", std::process::id())).is_file()
    );
}

#[cfg(unix)]
#[test]
fn unavailable_desktop_logs_and_log_directory_do_not_block_daemon() {
    for block_directory in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let logs = dir.path().join("logs");
        if block_directory {
            fs::write(&logs, "not a directory").unwrap();
        } else {
            for stream in ["stdout", "stderr"] {
                fs::create_dir_all(
                    logs.join(format!("desktop-{}.daemon.{stream}.log", std::process::id())),
                )
                .unwrap();
            }
        }
        let cli = test_cli(dir.path(), "printf started > launched; exec sleep 30");
        let mut child = None;
        let error = ensure_daemon(
            &cli,
            dir.path(),
            &GatewayRuntimeEndpoint::new("127.0.0.1", 1),
            "build",
            &mut child,
            Duration::ZERO,
        )
        .unwrap_err();
        assert!(error.contains("尚未就绪"), "{error}");
        for _ in 0..100 {
            if dir.path().join("launched").exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        child.as_mut().unwrap().kill().unwrap();
        child.as_mut().unwrap().wait().unwrap();
        assert_eq!(fs::read_to_string(dir.path().join("launched")).unwrap(), "started");
    }
}

#[cfg(windows)]
#[test]
fn exclusively_locked_desktop_logs_fall_back_to_null_output() {
    use std::os::windows::fs::OpenOptionsExt;
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("logs")).unwrap();
    let locks: Vec<_> = ["stdout", "stderr"]
        .into_iter()
        .map(|stream| {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .share_mode(0)
                .open(
                    dir.path()
                        .join(format!("logs/desktop-{}.daemon.{stream}.log", std::process::id())),
                )
                .unwrap()
        })
        .collect();
    let mut child = None;
    let error = ensure_daemon(
        &dir.path().join("missing.exe"),
        dir.path(),
        &GatewayRuntimeEndpoint::new("127.0.0.1", 1),
        "build",
        &mut child,
        Duration::ZERO,
    )
    .unwrap_err();
    assert!(error.contains("启动完整服务失败"), "{error}");
    drop(locks);
}
