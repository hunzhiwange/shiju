use super::terminal_session::TerminalSession;
use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
#[test]
fn cc_apps_terminal_session_real_tty_io_and_reaped_exit() {
    let root = tempfile::tempdir_in("/tmp").unwrap();
    let mut client = None;
    let session = TerminalSession::connect_with(root.path(), |socket, frontend| {
        assert!(!format!("{frontend:?}").contains("fixture-private-key"));
        let mut stream = UnixStream::connect(socket).unwrap();
        client = Some(std::thread::spawn(move || {
            stream.write_all(b"hello\n").unwrap();
            let mut output = Vec::new();
            stream.read_to_end(&mut output).unwrap();
            output
        }));
        Ok(())
    })
    .unwrap();
    let started = Arc::new(AtomicBool::new(false));
    let signal = started.clone();
    let status = session.run(Command::new("/bin/sh").args(["-c", "test -t 0 && test -t 1 || exit 9; read value; printf 'received:%s\\n' \"$value\"; exit 7"]), move || { signal.store(true, Ordering::SeqCst); }).unwrap();
    assert!(started.load(Ordering::SeqCst));
    assert_eq!(status.code(), Some(7));
    let output = String::from_utf8(client.unwrap().join().unwrap()).unwrap();
    assert!(output.contains("received:hello"), "{output}");
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}
#[test]
fn cc_apps_terminal_session_disconnect_kills_and_reaps_group() {
    let root = tempfile::tempdir_in("/tmp").unwrap();
    let session = TerminalSession::connect_with(root.path(), |socket, _| {
        let stream = UnixStream::connect(socket).unwrap();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(150));
            drop(stream);
        });
        Ok(())
    })
    .unwrap();
    let at = std::time::Instant::now();
    let status =
        session.run(Command::new("/bin/sh").args(["-c", "sleep 30 & wait"]), || {}).unwrap();
    assert!(!status.success());
    assert!(at.elapsed() < std::time::Duration::from_secs(5));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn cc_apps_terminal_session_normal_exit_is_not_a_disconnect_failure() {
    for _ in 0..10 {
        let root = tempfile::tempdir_in("/tmp").unwrap();
        let mut client = None;
        let session = TerminalSession::connect_with(root.path(), |socket, _| {
            let mut stream = UnixStream::connect(socket).unwrap();
            client = Some(std::thread::spawn(move || {
                let mut output = Vec::new();
                stream.read_to_end(&mut output).unwrap();
                output
            }));
            Ok(())
        })
        .unwrap();
        let status = session
            .run(Command::new("/bin/sh").args(["-c", "printf 'done'; exit 0"]), || {})
            .unwrap();
        assert!(status.success());
        assert_eq!(client.unwrap().join().unwrap(), b"done");
    }
}
