use crate::{
    host,
    request::LaunchMaterial,
    workbench_pty::{PtyAttachment, PtyOutput, PtySize, REPLAY_CAPACITY, WorkbenchPty},
};
use std::{
    io,
    os::unix::process::ExitStatusExt,
    process::Command,
    time::{Duration, Instant},
};

const SIZE: PtySize = PtySize { rows: 24, cols: 80 };
const DEADLINE: Duration = Duration::from_secs(5);

fn shell(root: &tempfile::TempDir, script: &str) -> Command {
    let cwd = root.path().canonicalize().unwrap();
    host::command(
        LaunchMaterial {
            program: "/bin/sh".into(),
            cwd: cwd.clone(),
            args: vec!["-c".into(), script.into()],
            skip_permissions: false,
        },
        &cwd,
        &[],
    )
    .unwrap()
}

fn read_until(session: &WorkbenchPty, attachment: &mut PtyAttachment, needle: &[u8]) -> Vec<u8> {
    let deadline = Instant::now() + DEADLINE;
    let mut bytes = Vec::new();
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(!remaining.is_zero(), "timed out waiting for {needle:?}; got {bytes:?}");
        let output = session.read(attachment, remaining).unwrap();
        assert!(!output.truncated);
        bytes.extend(output.bytes);
        if bytes.windows(needle.len()).any(|window| window == needle) {
            return bytes;
        }
        assert!(!output.finished, "exited before {needle:?}; got {bytes:?}");
    }
}

fn read_exit(session: &WorkbenchPty, attachment: &mut PtyAttachment) -> PtyOutput {
    let deadline = Instant::now() + DEADLINE;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(!remaining.is_zero(), "PTY did not exit");
        let output = session.read(attachment, remaining).unwrap();
        if output.finished {
            return output;
        }
    }
}

fn assert_clean(root: &tempfile::TempDir, pid: u32) {
    // Only the PID of our own fixture is probed; signal zero sends no signal.
    assert_eq!(unsafe { libc::kill(pid as libc::pid_t, 0) }, -1);
    assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn cli_workbench_pty_same_process_survives_reattachment() {
    let root = tempfile::tempdir().unwrap();
    let script = r#"
        test -t 0 && test -t 1 && test -t 2 && test -d "$TMPDIR" || exit 9
        printf 'pid:%s\n' "$$"
        stty size
        printf 'ready\n'
        IFS= read -r value || exit 10
        printf 'received:%s\n' "$value"
        stty size
        printf 'same:%s\n' "$$"
        exit 7
    "#;
    let pty = WorkbenchPty::spawn(shell(&root, script), root.path(), SIZE).unwrap();
    let pid = pty.pid();
    let mut first = pty.attach(0).unwrap();
    let start = read_until(&pty, &mut first, b"ready\r\n");
    assert_eq!(start, format!("pid:{pid}\r\n24 80\r\nready\r\n").as_bytes());
    let cursor = first.cursor();
    first.detach();
    assert_eq!(pty.exit_status().unwrap(), None);

    let mut second = pty.attach(cursor).unwrap();
    pty.resize(&second, PtySize { rows: 35, cols: 91 }).unwrap();
    assert_eq!(pty.write(&second, b"hello\n").unwrap(), 6);
    let end = read_until(&pty, &mut second, format!("same:{pid}\r\n").as_bytes());
    assert_eq!(end, format!("hello\r\nreceived:hello\r\n35 91\r\nsame:{pid}\r\n").as_bytes());
    assert!(read_exit(&pty, &mut second).bytes.is_empty());
    let status = pty.exit_status().unwrap().unwrap();
    assert_eq!(status.code(), Some(7));
    assert_eq!(pty.stop().unwrap(), status);
    assert_eq!(pty.write(&second, b"x").unwrap_err().kind(), io::ErrorKind::BrokenPipe);
    assert_eq!(pty.resize(&second, SIZE).unwrap_err().kind(), io::ErrorKind::BrokenPipe);
    assert_clean(&root, pid);
    eprintln!(
        "PTY shell PID {pid} before and after reattach, exit 7, PID reaped and tempdir removed"
    );
}

#[test]
fn cli_workbench_pty_replays_bounded_tail_after_exit() {
    let root = tempfile::tempdir().unwrap();
    let script = format!("head -c {} /dev/zero | tr '\\000' x; printf 'tail'", REPLAY_CAPACITY * 2);
    let pty = WorkbenchPty::spawn(shell(&root, &script), root.path(), SIZE).unwrap();
    // A detached terminal must continue draining even when nobody consumes output.
    let deadline = Instant::now() + DEADLINE;
    while pty.exit_status().unwrap().is_none() {
        assert!(Instant::now() < deadline, "detached PTY did not exit");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(pty.exit_status().unwrap().unwrap().success());

    let mut replay = pty.attach(0).unwrap();
    let output = pty.read(&mut replay, Duration::ZERO).unwrap();
    assert!(output.finished && output.truncated);
    assert_eq!(output.start, (REPLAY_CAPACITY + 4) as u64);
    assert_eq!(output.next, (REPLAY_CAPACITY * 2 + 4) as u64);
    assert_eq!(output.bytes.len(), REPLAY_CAPACITY);
    assert!(output.bytes[..REPLAY_CAPACITY - 4].iter().all(|byte| *byte == b'x'));
    assert_eq!(&output.bytes[REPLAY_CAPACITY - 4..], b"tail");
    assert!(pty.read(&mut replay, Duration::ZERO).unwrap().bytes.is_empty());
    let mut resumed = pty.attach(output.next - 4).unwrap();
    let tail = pty.read(&mut resumed, Duration::ZERO).unwrap();
    assert!(!tail.truncated);
    assert_eq!(tail.bytes, b"tail");
    assert!(pty.attach(output.next + 1).is_err());
    assert_clean(&root, pty.pid());
}

#[test]
fn cli_workbench_pty_stop_and_drop_clean_only_owned_sessions() {
    let root = tempfile::tempdir().unwrap();
    let start = || {
        WorkbenchPty::spawn(shell(&root, "printf ready; read value"), root.path(), SIZE).unwrap()
    };
    let first = start();
    let second = start();
    let mut attachment = first.attach(0).unwrap();
    let mut other = second.attach(0).unwrap();
    read_until(&first, &mut attachment, b"ready");
    read_until(&second, &mut other, b"ready");
    assert_eq!(
        second.read(&mut attachment, Duration::ZERO).unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
    assert_eq!(second.write(&attachment, b"x").unwrap_err().kind(), io::ErrorKind::InvalidInput);
    assert_eq!(second.resize(&attachment, SIZE).unwrap_err().kind(), io::ErrorKind::InvalidInput);
    let status = first.stop().unwrap();
    assert_eq!(status.signal(), Some(libc::SIGKILL));
    assert_eq!(first.stop().unwrap(), status);
    drop(other);
    assert_eq!(second.exit_status().unwrap(), None);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    let second_pid = second.pid();
    drop(second);
    assert_clean(&root, second_pid);
    assert_clean(&root, first.pid());
}

#[test]
fn cli_workbench_pty_failed_spawn_removes_private_directory() {
    let root = tempfile::tempdir().unwrap();
    let mut command = Command::new(root.path().join("missing-executable"));
    command.current_dir(root.path());
    let error = WorkbenchPty::spawn(command, root.path(), SIZE).err().unwrap();
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn cli_workbench_pty_tracks_latest_title_without_consuming_output() {
    let root = tempfile::tempdir().unwrap();
    let script = r#"
        printf '\033]2;working\007'
        read -r line
        printf '\033]52;ignored\007'
        printf '\033]0;idle\033\\'
        printf 'updated'
    "#;
    let process = WorkbenchPty::spawn(shell(&root, script), root.path(), SIZE).unwrap();
    let mut reader = process.attach(0).unwrap();

    read_until(&process, &mut reader, b"working\x07");
    assert_eq!(process.terminal_title().as_deref(), Some("working"));
    process.write(&reader, b"continue\n").unwrap();
    read_until(&process, &mut reader, b"updated");
    assert_eq!(process.terminal_title().as_deref(), Some("idle"));
}
